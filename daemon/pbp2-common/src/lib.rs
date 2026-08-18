//! Shared between `pixelbudsd` and `pbp2ctl`, so the status schema and the
//! two paths that matter (state file, control socket) have exactly one
//! definition. Two copies of a path string is precisely the kind of thing
//! that drifts; see `knowledge/ipc-socket-location.md` in the plugin root.

use std::env;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Highest schema version this crate writes. Bump this, and the version this
/// plugin's `Model.js` reads, together.
pub const SCHEMA_VERSION: u32 = 1;

/// Wire values for ANC mode. These are not invented: they are
/// `maestro_pw.AncState` from the Maestro protobuf schema, copied so the
/// panel and the daemon never disagree about what an int meant.
pub mod anc {
    pub const UNKNOWN: i32 = 0;
    pub const OFF: i32 = 1;
    pub const ACTIVE: i32 = 2;
    pub const AWARE: i32 = 3;
    pub const ADAPTIVE: i32 = 4;
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct BudStatus {
    pub available: bool,
    pub level: i32,
    pub charging: bool,
    pub in_case: bool,
}

impl BudStatus {
    pub fn unknown() -> Self {
        Self { available: false, level: -1, charging: false, in_case: false }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CaseStatus {
    pub available: bool,
    pub level: i32,
    pub charging: bool,
}

impl CaseStatus {
    pub fn unknown() -> Self {
        Self { available: false, level: -1, charging: false }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Status {
    pub schema_version: u32,
    pub connected: bool,
    pub device_name: String,
    pub model_name: String,
    pub anc_mode: i32,
    pub multipoint_enabled: bool,
    pub on_head_detection_enabled: bool,
    pub speech_detection_enabled: bool,
    pub volume_exposure_notifications_enabled: bool,
    pub left: BudStatus,
    pub right: BudStatus,
    pub case: CaseStatus,
}

impl Default for Status {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            connected: false,
            device_name: String::new(),
            model_name: String::new(),
            anc_mode: anc::UNKNOWN,
            multipoint_enabled: false,
            on_head_detection_enabled: false,
            speech_detection_enabled: false,
            volume_exposure_notifications_enabled: false,
            left: BudStatus::unknown(),
            right: BudStatus::unknown(),
            case: CaseStatus::unknown(),
        }
    }
}

impl Status {
    /// One line of compact JSON, the whole wire format. `Model.js` parses
    /// this by key, not by position, so field order here is not load-bearing.
    pub fn render(&self) -> String {
        serde_json::to_string(self).expect("Status always serializes")
    }
}

/// `$XDG_STATE_HOME/pixelbudspro2/status.json`, falling back to
/// `$HOME/.local/state` the same way the XDG base directory spec does.
pub fn state_path() -> PathBuf {
    let base = env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = env::var_os("HOME").expect("HOME must be set");
            PathBuf::from(home).join(".local/state")
        });
    base.join("pixelbudspro2").join("status.json")
}

/// `$XDG_RUNTIME_DIR/pixelbudspro2.sock`. Returns `None` when
/// `XDG_RUNTIME_DIR` is unset rather than falling back to `/tmp`: a fallback
/// would quietly restore the world-visible control socket the runtime dir is
/// there to avoid. Every context that matters here (the graphical session,
/// the Quickshell process running the panel) has one.
pub fn socket_path() -> Option<PathBuf> {
    let dir = env::var_os("XDG_RUNTIME_DIR")?;
    if dir.is_empty() {
        return None;
    }
    Some(PathBuf::from(dir).join("pixelbudspro2.sock"))
}

/// A verb sent over the control socket, one line, newline-terminated.
/// The full set pbp2ctl and Service.qml's `_send` agree on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verb {
    Anc(i32),
    Multipoint(bool),
    OnHeadDetection(bool),
    SpeechDetection(bool),
    VolumeExposureNotifications(bool),
    Refresh,
}

/// Parses a verb like `anc:aware` or `multipoint:on`. Extracted from the
/// daemon's dispatch loop so it can be unit-tested without a running
/// Bluetooth session, the same reasoning `ipcverb.hpp` gives upstream.
pub fn parse_verb(raw: &str) -> Result<Verb, String> {
    let raw = raw.trim();
    if raw == "refresh" {
        return Ok(Verb::Refresh);
    }

    let (name, payload) = raw
        .split_once(':')
        .ok_or_else(|| format!("malformed verb (expected \"name:value\"): {raw:?}"))?;

    match name {
        "anc" => match payload {
            "off" => Ok(Verb::Anc(anc::OFF)),
            "active" => Ok(Verb::Anc(anc::ACTIVE)),
            "aware" => Ok(Verb::Anc(anc::AWARE)),
            "adaptive" => Ok(Verb::Anc(anc::ADAPTIVE)),
            other => Err(format!("unknown anc mode: {other:?}")),
        },
        "multipoint" => parse_bool(payload).map(Verb::Multipoint),
        "ohd" => parse_bool(payload).map(Verb::OnHeadDetection),
        "speech" => parse_bool(payload).map(Verb::SpeechDetection),
        "volumeexposure" => parse_bool(payload).map(Verb::VolumeExposureNotifications),
        "refresh" => Ok(Verb::Refresh),
        other => Err(format!("unknown verb: {other:?}")),
    }
}

fn parse_bool(payload: &str) -> Result<bool, String> {
    match payload {
        "on" => Ok(true),
        "off" => Ok(false),
        other => Err(format!("expected \"on\" or \"off\", got {other:?}")),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn status_round_trips_through_json() {
        let status = Status {
            connected: true,
            device_name: "Test Buds".into(),
            anc_mode: anc::AWARE,
            left: BudStatus { available: true, level: 74, charging: false, in_case: false },
            ..Default::default()
        };

        let line = status.render();
        let back: Status = serde_json::from_str(&line).unwrap();
        assert_eq!(status, back);
    }

    #[test]
    fn parses_anc_verbs() {
        assert_eq!(parse_verb("anc:off").unwrap(), Verb::Anc(anc::OFF));
        assert_eq!(parse_verb("anc:adaptive").unwrap(), Verb::Anc(anc::ADAPTIVE));
        assert!(parse_verb("anc:sideways").is_err());
    }

    #[test]
    fn parses_bool_verbs() {
        assert_eq!(parse_verb("multipoint:on").unwrap(), Verb::Multipoint(true));
        assert_eq!(parse_verb("ohd:off").unwrap(), Verb::OnHeadDetection(false));
        assert!(parse_verb("speech:maybe").is_err());
    }

    #[test]
    fn parses_refresh_with_no_payload() {
        assert_eq!(parse_verb("refresh").unwrap(), Verb::Refresh);
    }

    #[test]
    fn rejects_verbs_with_no_colon() {
        assert!(parse_verb("anc").is_err());
        assert!(parse_verb("").is_err());
    }

    #[test]
    fn socket_path_refuses_to_fall_back() {
        // SAFETY: test-only mutation of the process environment, single-threaded test.
        unsafe { env::remove_var("XDG_RUNTIME_DIR") };
        assert_eq!(socket_path(), None);

        unsafe { env::set_var("XDG_RUNTIME_DIR", "/run/user/1000") };
        assert_eq!(socket_path(), Some(PathBuf::from("/run/user/1000/pixelbudspro2.sock")));
    }
}
