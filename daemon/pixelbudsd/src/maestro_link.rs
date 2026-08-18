//! Owns the Bluetooth RFCOMM connection to the Pixel Buds Pro and the
//! Maestro RPC session running over it: connecting, reconnecting after the
//! buds hand audio off between each other (which resets the socket), and
//! translating protocol events into `Status` updates.
//!
//! See `knowledge/maestro-protocol.md` for what each field is measured from.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use bluer::rfcomm::{Profile, ProfileHandle, ReqError, Role, Stream};
use bluer::{Address, Device, Session};
use futures::StreamExt;
use tokio::sync::Mutex;

use maestro::protocol::codec::Codec;
use maestro::protocol::types::{settings_rsp, DeviceBatteryInfo, RuntimeInfo};
use maestro::protocol::utils;
use maestro::pwrpc::client::Client;
use maestro::service::settings::{AncState, SettingId, SettingValue};
use maestro::service::MaestroService;

use pixelbuds_common::{anc, BudStatus, CaseStatus, Status, Verb};

use crate::status_writer::StatusWriter;

/// The daemon's live handle to the buds, shared with the control socket so
/// `pixelbudsctl` verbs can be applied without the socket handler owning the
/// Bluetooth connection itself. `None` whenever no session is up.
pub type SharedService = Arc<Mutex<Option<MaestroService>>>;

const SETTINGS_TO_SEED: [SettingId; 5] = [
    SettingId::CurrentAncrState,
    SettingId::MultipointEnable,
    SettingId::OhdEnable,
    SettingId::SpeechDetection,
    SettingId::VolumeExposureNotifications,
];

/// Runs forever: connect, serve, and on any disconnect (including the
/// mid-air handoff between the two buds, which resets the RFCOMM socket)
/// wait briefly and try again.
pub async fn run(
    session: Session,
    addr: Address,
    status: Arc<Mutex<Status>>,
    service_slot: SharedService,
    writer: StatusWriter,
) -> Result<()> {
    let adapter = session.default_adapter().await?;
    let dev = adapter.device(addr)?;

    loop {
        {
            let mut st = status.lock().await;
            st.device_name = dev.alias().await.unwrap_or_default();
            st.model_name = "Pixel Buds Pro".to_string();
            writer.publish(&st).await;
        }

        match run_once(&session, &dev, &status, &service_slot, &writer).await {
            Ok(()) => tracing::warn!("maestro session ended, reconnecting"),
            Err(err) => tracing::warn!(error = ?err, "maestro session failed, reconnecting"),
        }

        *service_slot.lock().await = None;
        {
            // The Maestro RFCOMM session resets when the buds hand audio
            // processing off between each other, even though the Bluetooth
            // link itself never drops (pbpctrl's own examples call this out
            // as `os error 104`, expected and worth retrying rather than
            // failing on). Only report `connected: false` — which hides the
            // panel's icon — once BlueZ agrees the device is actually gone,
            // so a mid-air handoff doesn't flicker the bar.
            let still_paired = dev.is_connected().await.unwrap_or(false);
            let mut st = status.lock().await;
            st.connected = still_paired;
            writer.publish(&st).await;
        }

        tokio::time::sleep(Duration::from_millis(1500)).await;
    }
}

async fn run_once(
    session: &Session,
    dev: &Device,
    status: &Arc<Mutex<Status>>,
    service_slot: &SharedService,
    writer: &StatusWriter,
) -> Result<()> {
    tracing::info!("connecting to Maestro profile");
    let stream = connect_maestro_rfcomm(session, dev).await?;
    tracing::info!("Maestro profile connected");

    let codec = Codec::new();
    let stream = codec.wrap(stream);

    let mut client = Client::new(stream);
    let handle = client.handle();
    let channel = utils::resolve_channel(&mut client)
        .await
        .context("failed to resolve maestro rpc channel")?;

    let mut service = MaestroService::new(handle, channel);

    // `client.run()` is what actually drains the request queue and reads
    // responses off the wire; every RPC issued through `service` (seeding,
    // subscriptions) just queues a request and awaits a reply that only
    // `client.run()` can deliver. It must be racing alongside those calls
    // from the start, not started only once they've all already awaited —
    // otherwise the very first RPC call deadlocks forever.
    let client_task = async move { client.run().await.map_err(anyhow::Error::from) };

    let session_task = async move {
        {
            let mut st = status.lock().await;
            st.connected = true;
            writer.publish(&st).await;
        }
        *service_slot.lock().await = Some(service.clone());

        seed_initial_settings(&mut service, status, writer).await;

        let runtime_task = listen_runtime_info(service.clone(), status.clone(), writer.clone());
        let settings_task = listen_settings(service.clone(), status.clone(), writer.clone());

        tokio::select! {
            res = runtime_task => res,
            res = settings_task => res,
        }
    };

    tokio::select! {
        res = client_task => res,
        res = session_task => res,
    }
}

async fn connect_maestro_rfcomm(session: &Session, dev: &Device) -> Result<Stream> {
    let profile = Profile {
        uuid: maestro::UUID,
        role: Some(Role::Client),
        require_authentication: Some(false),
        require_authorization: Some(false),
        auto_connect: Some(false),
        ..Default::default()
    };

    let mut handle = session.register_profile(profile).await?;

    let stream = tokio::try_join!(try_connect_profile(dev), accept_profile_request(&mut handle, dev.address()),)?.1;

    Ok(stream)
}

async fn try_connect_profile(dev: &Device) -> Result<()> {
    const MAX_TRIES: u32 = 3;
    let mut attempt = 0;

    loop {
        let _ = dev.connect().await;
        match dev.connect_profile(&maestro::UUID).await {
            Ok(()) => return Ok(()),
            Err(err) if attempt < MAX_TRIES => {
                attempt += 1;
                tracing::warn!(error = ?err, attempt, "connecting maestro profile failed, retrying");
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            Err(err) => return Err(err.into()),
        }
    }
}

async fn accept_profile_request(handle: &mut ProfileHandle, address: Address) -> Result<Stream> {
    while let Some(req) = handle.next().await {
        if req.device() == address {
            return Ok(req.accept()?);
        }
        req.reject(ReqError::Rejected);
    }
    anyhow::bail!("profile handle closed without a connection request")
}

async fn seed_initial_settings(service: &mut MaestroService, status: &Arc<Mutex<Status>>, writer: &StatusWriter) {
    for id in SETTINGS_TO_SEED {
        match service.read_setting_var(id).await {
            Ok(value) => {
                let mut st = status.lock().await;
                apply_setting(&mut st, value);
                writer.publish(&st).await;
            }
            Err(err) => tracing::debug!(?id, error = ?err, "failed to read initial setting"),
        }
    }
}

async fn listen_runtime_info(mut service: MaestroService, status: Arc<Mutex<Status>>, writer: StatusWriter) -> Result<()> {
    let mut call = service.subscribe_to_runtime_info()?;
    while let Some(msg) = call.stream().next().await {
        let info = msg?;
        let mut st = status.lock().await;
        apply_runtime_info(&mut st, info);
        writer.publish(&st).await;
    }
    anyhow::bail!("runtime info stream ended")
}

async fn listen_settings(mut service: MaestroService, status: Arc<Mutex<Status>>, writer: StatusWriter) -> Result<()> {
    let mut call = service.subscribe_to_settings_changes()?;
    while let Some(msg) = call.stream().next().await {
        let rsp = msg?;
        let Some(settings_rsp::ValueOneof::Value(raw)) = rsp.value_oneof else { continue };
        let Some(value_oneof) = raw.value_oneof else { continue };

        let mut st = status.lock().await;
        apply_setting(&mut st, value_oneof.into());
        writer.publish(&st).await;
    }
    anyhow::bail!("settings stream ended")
}

fn apply_setting(status: &mut Status, value: SettingValue) {
    match value {
        SettingValue::CurrentAncrState(state) => status.anc_mode = anc_wire(state),
        SettingValue::MultipointEnable(v) => status.multipoint_enabled = v,
        SettingValue::OhdEnable(v) => status.on_head_detection_enabled = v,
        SettingValue::SpeechDetection(v) => status.speech_detection_enabled = v,
        SettingValue::VolumeExposureNotifications(v) => status.volume_exposure_notifications_enabled = v,
        // Everything else (EQ, gestures, mono, OTA...) has no row in this panel.
        _ => {}
    }
}

fn anc_wire(state: AncState) -> i32 {
    match state {
        AncState::Off => anc::OFF,
        AncState::Active => anc::ACTIVE,
        AncState::Aware => anc::AWARE,
        AncState::Adaptive => anc::ADAPTIVE,
        AncState::Unknown(_) => anc::UNKNOWN,
    }
}

fn apply_runtime_info(status: &mut Status, info: RuntimeInfo) {
    let in_case_left = info.placement.as_ref().map(|p| p.left_bud_in_case).unwrap_or(false);
    let in_case_right = info.placement.as_ref().map(|p| p.right_bud_in_case).unwrap_or(false);

    if let Some(battery) = info.battery_info {
        status.left = bud_from(battery.left, in_case_left);
        status.right = bud_from(battery.right, in_case_right);
        // Unlike the buds, the case has no independent link to report its own
        // charge over: the buds only learn it while docked (the pogo pins),
        // so `battery.case` goes absent the moment they're picked up. Treating
        // that absence as "unavailable" would blank out a perfectly good
        // reading a few seconds after every undock; keep the last known case
        // reading on screen instead, same as a phone's Fast Pair companion
        // does, and only overwrite it when a fresh docked reading arrives.
        if let Some(case) = battery.case {
            status.case = case_from(Some(case));
        }
    }
}

// BatteryState::BATTERY_CHARGING = 2 on the wire; see maestro-protocol.md.
fn bud_from(info: Option<DeviceBatteryInfo>, in_case: bool) -> BudStatus {
    match info {
        Some(d) => BudStatus { available: true, level: d.level, charging: d.state == 2, in_case },
        None => BudStatus::unknown(),
    }
}

fn case_from(info: Option<DeviceBatteryInfo>) -> CaseStatus {
    match info {
        Some(d) => CaseStatus { available: true, level: d.level, charging: d.state == 2 },
        None => CaseStatus::unknown(),
    }
}

/// Maps a control-socket verb onto the Maestro setting it writes. `Refresh`
/// has no setting, and is handled directly by the control socket instead.
pub fn setting_for_verb(verb: Verb) -> Option<SettingValue> {
    match verb {
        Verb::Anc(mode) => {
            let state = if mode == anc::OFF {
                AncState::Off
            } else if mode == anc::ACTIVE {
                AncState::Active
            } else if mode == anc::AWARE {
                AncState::Aware
            } else if mode == anc::ADAPTIVE {
                AncState::Adaptive
            } else {
                return None;
            };
            Some(SettingValue::CurrentAncrState(state))
        }
        Verb::Multipoint(v) => Some(SettingValue::MultipointEnable(v)),
        Verb::OnHeadDetection(v) => Some(SettingValue::OhdEnable(v)),
        Verb::SpeechDetection(v) => Some(SettingValue::SpeechDetection(v)),
        Verb::VolumeExposureNotifications(v) => Some(SettingValue::VolumeExposureNotifications(v)),
        Verb::Refresh => None,
    }
}
