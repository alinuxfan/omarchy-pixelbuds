// Run with: deno run --allow-read tests/model.test.js
// Model.js has no exports, so it is evaluated here rather than imported.

const source = Deno.readTextFileSync(new URL("../Model.js", import.meta.url))
const Model = new Function(
  source + "; return { parseStatus, budFrom, caseFrom, defaultBud, defaultCase, ancModeName, ancModeVerb, availableModes, levelFraction, levelText, budMeta, elideError, LEVEL_UNKNOWN, ANC_UNKNOWN, ANC_OFF, ANC_ACTIVE, ANC_AWARE, ANC_ADAPTIVE, MAX_ERROR_CHARS }"
)()

let failures = 0

function check(name, actual, expected) {
  const ok = JSON.stringify(actual) === JSON.stringify(expected)
  if (!ok) {
    failures++
    console.log("FAIL " + name + "\n  expected " + JSON.stringify(expected) + "\n  got      " + JSON.stringify(actual))
  }
}

// A line pixelbudsd actually writes, copied from the schema doc.
const live = '{"anc_mode":2,"case":{"available":true,"charging":false,"level":88},"connected":true,"device_name":"alinuxfan’s Pixel Buds Pro","left":{"available":true,"charging":false,"in_case":false,"level":74},"model_name":"Pixel Buds Pro","multipoint_enabled":true,"on_head_detection_enabled":true,"right":{"available":true,"charging":false,"in_case":false,"level":81},"schema_version":1,"speech_detection_enabled":true,"volume_exposure_notifications_enabled":true}'

const good = Model.parseStatus(live)
check("live line parses", good.ok, true)
check("live modelName", good.modelName, "Pixel Buds Pro")
check("live deviceName keeps the daemon's apostrophe", good.deviceName, "alinuxfan’s Pixel Buds Pro")
check("live left level", good.left.level, 74)
check("live case", good.caseBattery, { level: 88, charging: false })
check("live ancMode", good.ancMode, Model.ANC_ACTIVE)
check("live multipointEnabled", good.multipointEnabled, true)

// A fresh daemon has not received a RuntimeInfo packet yet: no left, right or case object at all.
const fresh = Model.parseStatus('{"connected":false,"anc_mode":0,"schema_version":1}')
check("fresh line parses", fresh.ok, true)
check("fresh left is the default bud", fresh.left, Model.defaultBud())
check("fresh case is unknown", fresh.caseBattery.level, Model.LEVEL_UNKNOWN)
check("fresh ancMode is Unknown", fresh.ancMode, Model.ANC_UNKNOWN)

// available:false means the daemon's BatteryInfo carried no entry for this bud (a real, distinct proto state).
const gone = Model.budFrom({ available: false, level: 82, charging: true, in_case: true })
check("unavailable bud reports no level", gone.level, Model.LEVEL_UNKNOWN)
check("unavailable bud reports no charging", gone.charging, false)
check("unavailable bud reports no in_case", gone.inCase, false)
check("unavailable bud shows no meta", Model.budMeta(gone), "")

// Every failure path returns the full default shape rather than throwing.
const empty = Model.parseStatus("")
check("empty input is not ok", empty.ok, false)
check("empty input names the failure", empty.lastError !== "", true)
check("empty input still has a left bud", empty.left, Model.defaultBud())

const garbage = Model.parseStatus("not json at all")
check("garbage is not ok", garbage.ok, false)
check("garbage is not schemaTooNew", garbage.schemaTooNew, false)

const noVersion = Model.parseStatus('{"connected":true}')
check("a line with no schema_version is not ok", noVersion.ok, false)

const tooNew = Model.parseStatus('{"schema_version":2,"connected":true}')
check("a newer schema is not ok", tooNew.ok, false)
check("a newer schema is flagged", tooNew.schemaTooNew, true)
check("a newer schema reports both versions", tooNew.lastError, "pixelbudsd speaks status schema 2, this panel reads 1")

check("null parses to the default shape", Model.parseStatus("null").ok, false)

// Verbs, straight off the wire enum, and the modes list they come from.
check("anc verb for Adaptive", Model.ancModeVerb(Model.ANC_ADAPTIVE), "anc:adaptive")
check("anc verb for Unknown is empty", Model.ancModeVerb(Model.ANC_UNKNOWN), "")
check("anc verb for a made-up mode is empty", Model.ancModeVerb(99), "")
check("available modes are the four Maestro states, Off first", Model.availableModes(), [Model.ANC_OFF, Model.ANC_ACTIVE, Model.ANC_AWARE, Model.ANC_ADAPTIVE])

// Meter and label edges.
check("an unknown level draws an empty track", Model.levelFraction(Model.LEVEL_UNKNOWN), 0)
check("a level above 100 is clamped", Model.levelFraction(140), 1)
check("a negative level is clamped", Model.levelFraction(-5), 0)
check("an unknown level reads as dashes", Model.levelText(Model.LEVEL_UNKNOWN), "--")

// Errors are elided to one line rather than dumped.
const long = Model.elideError("x\n\n   y".repeat(60))
check("an elided error is one line", long.indexOf("\n"), -1)
check("an elided error fits the row", long.length <= Model.MAX_ERROR_CHARS, true)
check("elideError copes with nothing", Model.elideError(null), "")

if (failures > 0) {
  console.log(failures + " failed")
  Deno.exit(1)
}
console.log("model.test.js: all checks passed")
