//! Background daemon for the Pixel Buds Pro 2 Omarchy panel.
//!
//! Connects to the Maestro RFCOMM channel (the same protocol the Google Buds
//! Android app speaks), subscribes to battery, placement and settings
//! updates, and publishes them to `$XDG_STATE_HOME/pixelbudspro2/status.json`
//! on change. A Unix socket at `$XDG_RUNTIME_DIR/pixelbudspro2.sock` accepts
//! control verbs from `pbp2ctl`.
//!
//! See `knowledge/maestro-protocol.md` in the plugin root for where each
//! field in `Status` comes from on the wire.

mod maestro_link;
mod status_writer;

use std::str::FromStr;
use std::sync::Arc;

use anyhow::{Context, Result};
use bluer::{Address, Session};
use clap::Parser;
use tokio::sync::Mutex;

use maestro_link::SharedService;
use pbp2_common::Status;

#[derive(Debug, Parser)]
#[command(author, version, about = "Daemon that publishes Pixel Buds Pro 2 status for the Omarchy panel")]
struct Args {
    /// Bluetooth address of the Pixel Buds Pro 2 (searches paired devices for
    /// the Maestro service UUID if unspecified).
    #[arg(short, long)]
    device: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();

    let state_path = pbp2_common::state_path();
    let socket_path = pbp2_common::socket_path().context(
        "XDG_RUNTIME_DIR is not set; refusing to fall back to a world-visible socket location",
    )?;

    let status = Arc::new(Mutex::new(Status::default()));

    // MaestroService is not Send (raw-pointer phantom markers on its typed
    // RPC handles), so this Arc never crosses a tokio::spawn boundary: both
    // the maestro link and the control socket run as unspawned futures
    // combined by the top-level select! below.
    #[allow(clippy::arc_with_non_send_sync)]
    let service: SharedService = Arc::new(Mutex::new(None));

    let writer = status_writer::StatusWriter::new(state_path.clone());

    let session = Session::new().await.context("failed to open a BlueZ session")?;
    let addr = resolve_device_address(&session, args.device.as_deref()).await?;

    tracing::info!(%addr, "targeting device");

    let link_task = maestro_link::run(session, addr, status.clone(), service.clone(), writer.clone());
    let control_task = control_socket::serve(socket_path.clone(), status.clone(), service.clone(), writer.clone());

    tokio::select! {
        res = link_task => {
            if let Err(err) = res {
                tracing::error!(error = ?err, "maestro link task ended");
            }
        }
        res = control_task => {
            if let Err(err) = res {
                tracing::error!(error = ?err, "control socket task ended");
            }
        }
        _ = shutdown_signal() => {
            tracing::info!("shutting down");
        }
    }

    // An absent state file is how a watcher (the Omarchy panel's FileView)
    // learns the daemon stopped, so this is not an optional cleanup step.
    let _ = tokio::fs::remove_file(&state_path).await;
    let _ = tokio::fs::remove_file(&socket_path).await;

    Ok(())
}

async fn resolve_device_address(session: &Session, device: Option<&str>) -> Result<Address> {
    if let Some(device) = device {
        return Address::from_str(device).with_context(|| format!("invalid device address: {device}"));
    }

    let adapter = session.default_adapter().await?;
    for addr in adapter.device_addresses().await? {
        let dev = adapter.device(addr)?;
        let Ok(Some(uuids)) = dev.uuids().await else { continue };
        if uuids.contains(&maestro::UUID) {
            return Ok(addr);
        }
    }

    anyhow::bail!(
        "no paired device advertises the Maestro service UUID ({}); pair your Pixel Buds Pro 2 first, \
         or pass --device <MAC>",
        maestro::UUID
    )
}

async fn shutdown_signal() -> Result<()> {
    use tokio::signal::unix::{signal, SignalKind};

    let mut sigterm = signal(SignalKind::terminate())?;
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = sigterm.recv() => {}
    }
    Ok(())
}

mod control_socket {
    use std::sync::Arc;

    use anyhow::{Context, Result};
    use std::os::unix::fs::PermissionsExt;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixListener;
    use tokio::sync::Mutex;

    use pbp2_common::{parse_verb, Status, Verb};

    use crate::maestro_link::SharedService;
    use crate::status_writer::StatusWriter;

    pub async fn serve(
        path: std::path::PathBuf,
        status: Arc<Mutex<Status>>,
        service: SharedService,
        writer: StatusWriter,
    ) -> Result<()> {
        let _ = std::fs::remove_file(&path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let listener = UnixListener::bind(&path)
            .with_context(|| format!("failed to bind control socket at {}", path.display()))?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))
            .context("failed to restrict control socket permissions")?;

        tracing::info!(path = %path.display(), "control socket listening");

        loop {
            let (stream, _) = listener.accept().await?;

            // Handled in place rather than spawned: MaestroService holds
            // non-Send internals (raw-pointer phantom markers on its typed
            // RPC handles), so a clone of it cannot cross into a spawned
            // task. Control connections are one verb, one reply each, so
            // serving them one at a time off this loop costs nothing.
            if let Err(err) = handle_connection(stream, status.clone(), service.clone(), writer.clone()).await {
                tracing::debug!(error = ?err, "control connection ended with an error");
            }
        }
    }

    async fn handle_connection(
        stream: tokio::net::UnixStream,
        status: Arc<Mutex<Status>>,
        service: SharedService,
        writer: StatusWriter,
    ) -> Result<()> {
        let (read_half, mut write_half) = stream.into_split();
        let mut lines = BufReader::new(read_half).lines();

        let Some(line) = lines.next_line().await? else {
            return Ok(());
        };

        let response = match parse_verb(&line) {
            Ok(verb) => apply_verb(verb, &status, &service, &writer).await,
            Err(msg) => format!("error: {msg}"),
        };

        write_half.write_all(response.as_bytes()).await?;
        write_half.write_all(b"\n").await?;
        Ok(())
    }

    async fn apply_verb(
        verb: Verb,
        status: &Arc<Mutex<Status>>,
        service: &SharedService,
        writer: &StatusWriter,
    ) -> String {
        if verb == Verb::Refresh {
            let snapshot = status.lock().await.clone();
            writer.publish(&snapshot).await;
            return "ok".to_string();
        }

        let svc = { service.lock().await.clone() };
        let Some(mut svc) = svc else {
            return "error: not connected to Pixel Buds Pro 2".to_string();
        };

        let setting = match crate::maestro_link::setting_for_verb(verb) {
            Some(setting) => setting,
            None => return "error: refresh has no setting to write".to_string(),
        };

        match svc.write_setting(setting).await {
            Ok(()) => "ok".to_string(),
            Err(err) => format!("error: {err}"),
        }
    }
}
