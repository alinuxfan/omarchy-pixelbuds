//! Small control client for `pixelbudsd`, in the same spirit as
//! `librepods-ctl`: send one verb over the daemon's Unix socket, print
//! whatever it says back, and exit non-zero if it refused.
//!
//! This is what Service.qml's `_send()` spawns for every control (ANC mode,
//! multipoint, on-head detection, ...); see `knowledge/status-schema.md` for
//! the full verb list.

use std::process::ExitCode;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let Some(verb) = std::env::args().nth(1) else {
        eprintln!("usage: pbp2ctl <verb>");
        eprintln!("  e.g.: pbp2ctl anc:aware");
        return ExitCode::from(2);
    };

    // Validated here too, not just daemon-side, so a typo fails locally
    // with a message instead of a silent round trip to the socket.
    if let Err(msg) = pbp2_common::parse_verb(&verb) {
        eprintln!("{msg}");
        return ExitCode::from(2);
    }

    match send(&verb).await {
        Ok(response) if response == "ok" => ExitCode::SUCCESS,
        Ok(response) => {
            eprintln!("{response}");
            ExitCode::FAILURE
        }
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

async fn send(verb: &str) -> anyhow::Result<String> {
    let Some(path) = pbp2_common::socket_path() else {
        anyhow::bail!("XDG_RUNTIME_DIR is not set; pixelbudsd cannot be reached");
    };

    let stream = tokio::time::timeout(Duration::from_secs(3), UnixStream::connect(&path))
        .await
        .map_err(|_| anyhow::anyhow!("timed out connecting to pixelbudsd"))?
        .map_err(|err| anyhow::anyhow!("could not reach pixelbudsd at {}: {err}", path.display()))?;

    let (read_half, mut write_half) = stream.into_split();
    write_half.write_all(verb.as_bytes()).await?;
    write_half.write_all(b"\n").await?;

    let mut line = String::new();
    BufReader::new(read_half).read_line(&mut line).await?;
    Ok(line.trim_end().to_string())
}
