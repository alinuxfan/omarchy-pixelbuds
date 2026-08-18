// No QML imports on purpose, so every function here runs in a plain JS harness.

// Wire values match maestro_pw.proto's AncState enum exactly, so the daemon
// never has to translate and the panel never has to guess what a raw int meant.
var ANC_UNKNOWN = 0
var ANC_OFF = 1
var ANC_ACTIVE = 2
var ANC_AWARE = 3
var ANC_ADAPTIVE = 4

// Highest schema_version this panel knows how to read.
var SUPPORTED_SCHEMA = 1

// Level the daemon reports when a bud or the case has not been heard from.
var LEVEL_UNKNOWN = -1

// nf-md-check U+F012C, the same glyph omapods measured rendering correctly at bar size.
var GLYPH_CHECK = "󰄬"

// Longest error the panel will show inside a row, and the cut that leaves room for the ellipsis.
var MAX_ERROR_CHARS = 140
var ELIDED_ERROR_CHARS = 137

function defaultBud() {
  return { level: LEVEL_UNKNOWN, charging: false, inCase: false }
}

function defaultCase() {
  return { level: LEVEL_UNKNOWN, charging: false }
}

// Full shape on every path, so the panel never reads undefined off a parse failure.
function defaultStatus() {
  return {
    ok: false,
    lastError: "",
    schemaVersion: 0,
    schemaTooNew: false,
    connected: false,
    deviceName: "",
    modelName: "",
    ancMode: ANC_UNKNOWN,
    multipointEnabled: false,
    onHeadDetectionEnabled: false,
    speechDetectionEnabled: false,
    volumeExposureNotificationsEnabled: false,
    left: defaultBud(),
    right: defaultBud(),
    caseBattery: defaultCase()
  }
}

function intOr(value, fallback) {
  var n = parseInt(value, 10)
  return isFinite(n) ? n : fallback
}

function budFrom(raw) {
  var bud = defaultBud()
  if (!raw || typeof raw !== "object") return bud
  // available:false means the daemon has stopped hearing from this bud, so its charging and in_case are stale too.
  if (raw.available !== true) return bud
  bud.level = intOr(raw.level, LEVEL_UNKNOWN)
  bud.charging = raw.charging === true
  bud.inCase = raw.in_case === true
  return bud
}

function caseFrom(raw) {
  var c = defaultCase()
  if (!raw || typeof raw !== "object") return c
  if (raw.available !== true) return c
  c.level = intOr(raw.level, LEVEL_UNKNOWN)
  c.charging = raw.charging === true
  return c
}

// The whole of $XDG_STATE_HOME/pixelbudspro/status.json, one line of compact JSON:
// {"anc_mode":2,"case":{"available":true,"charging":false,"level":88},
//  "connected":true,"device_name":"alinuxfan's Pixel Buds Pro",
//  "left":{"available":true,"charging":false,"in_case":false,"level":74},
//  "model_name":"Pixel Buds Pro","multipoint_enabled":true,
//  "on_head_detection_enabled":true,"right":{"available":true,"charging":false,"in_case":false,"level":81},
//  "schema_version":1,"speech_detection_enabled":true,"volume_exposure_notifications_enabled":true}
function parseStatus(raw) {
  var status = defaultStatus()
  var text = String(raw || "").trim()
  if (text === "") {
    status.lastError = "The pixelbudsd status file is empty"
    return status
  }

  var parsed
  try {
    parsed = JSON.parse(text)
  } catch (e) {
    status.lastError = "Could not read the pixelbudsd status file"
    return status
  }
  if (!parsed || typeof parsed !== "object" || parsed.schema_version === undefined) {
    status.lastError = "The pixelbudsd status file carried no schema_version"
    return status
  }

  status.schemaVersion = intOr(parsed.schema_version, 0)
  if (status.schemaVersion > SUPPORTED_SCHEMA) {
    // Newer daemon: report the version rather than draw fields we may be misreading.
    status.schemaTooNew = true
    status.lastError = "pixelbudsd speaks status schema " + status.schemaVersion + ", this panel reads " + SUPPORTED_SCHEMA
    return status
  }

  status.ok = true
  status.connected = parsed.connected === true
  status.deviceName = String(parsed.device_name || "")
  status.modelName = String(parsed.model_name || "")
  status.ancMode = intOr(parsed.anc_mode, ANC_UNKNOWN)
  status.multipointEnabled = parsed.multipoint_enabled === true
  status.onHeadDetectionEnabled = parsed.on_head_detection_enabled === true
  status.speechDetectionEnabled = parsed.speech_detection_enabled === true
  status.volumeExposureNotificationsEnabled = parsed.volume_exposure_notifications_enabled === true
  status.left = budFrom(parsed.left)
  status.right = budFrom(parsed.right)
  status.caseBattery = caseFrom(parsed["case"])
  return status
}

function ancModeName(mode) {
  if (mode === ANC_OFF) return "Off"
  if (mode === ANC_ACTIVE) return "Noise Cancellation"
  if (mode === ANC_AWARE) return "Transparency"
  if (mode === ANC_ADAPTIVE) return "Adaptive"
  return "Unknown"
}

// The four Maestro ANC states, indexed to match ancMode, in the order pixelbudsctl accepts them.
function ancModeVerb(mode) {
  if (mode === ANC_OFF) return "anc:off"
  if (mode === ANC_ACTIVE) return "anc:active"
  if (mode === ANC_AWARE) return "anc:aware"
  if (mode === ANC_ADAPTIVE) return "anc:adaptive"
  return ""
}

// Fixed: Maestro exposes exactly these four states, unlike AirPods where the
// mode list depends on which model answered. See knowledge/pixel-buds-pro-identity.md.
function availableModes() {
  return [ANC_OFF, ANC_ACTIVE, ANC_AWARE, ANC_ADAPTIVE]
}

function levelText(level) {
  return level === LEVEL_UNKNOWN ? "--" : String(level) + "%"
}

// 0 to 1 for the meter; an unknown level draws an empty track rather than a full one.
function levelFraction(level) {
  if (level === LEVEL_UNKNOWN) return 0
  return Math.max(0, Math.min(100, level)) / 100
}

function budMeta(bud) {
  if (bud.charging) return "Charging"
  if (bud.inCase) return "In case"
  return ""
}

// Collapse pixelbudsctl's stderr into one line the panel can show inside a row.
function elideError(text) {
  var value = String(text || "").replace(/\s+/g, " ").trim()
  return value.length > MAX_ERROR_CHARS ? value.substring(0, ELIDED_ERROR_CHARS) + "…" : value
}
