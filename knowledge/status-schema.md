---
type: reference
title: The pixelbudsd status file and control verbs, key by key
description: The exact wire format Model.js parses, and every verb pixelbudsctl accepts
tags: [pixelbudsd, ipc, schema]
status: stable
verified:
  - by: reading pixelbuds-common's Status struct and running `cargo test --workspace`, which round-trips this schema through serde
    at: 2026-08-17
---

# Where it comes from

`pixelbudsd` writes the whole status object, one line of compact JSON,
through a temp-file-plus-rename (`StatusWriter::write_atomic`), so a reader
never sees a half-written line. Two properties matter to the panel, both
carried over from omapods' AirPods daemon:

- **It writes only on change.** `StatusWriter::publish` compares the
  rendered line against the last one written and returns early on a match, so
  a control verb the buds ignored produces no write.
- **It is removed on shutdown**, from the `main()` cleanup path after the
  signal-or-task-exit select, for both `SIGINT` and `SIGTERM`. An absent
  state file is how `Service.qml`'s `FileView` learns the daemon stopped.

Unlike omapods, this daemon does **not** independently gate battery on the
audio link: every field arrives over the same Maestro session, so `connected:
false` means every other field is stale, not just silent. See
`maestro-protocol.md`.

# The keys the panel reads

| Key | Type | Meaning |
|---|---|---|
| `schema_version` | int | currently 1, gates incompatible bumps |
| `connected` | bool | the Maestro RFCOMM session, not the A2DP audio link |
| `device_name` | string | the BlueZ alias |
| `model_name` | string | always `"Pixel Buds Pro"`; see `pixel-buds-pro-identity.md` |
| `anc_mode` | int | 0 unknown, 1 off, 2 active (ANC), 3 aware (Transparency), 4 adaptive — Maestro's own `AncState` values, unrenumbered |
| `multipoint_enabled` | bool | `MultipointEnable` setting |
| `on_head_detection_enabled` | bool | `OhdEnable` setting |
| `speech_detection_enabled` | bool | `SpeechDetection` setting — auto-Transparency when you start talking |
| `volume_exposure_notifications_enabled` | bool | `VolumeExposureNotifications` setting |
| `left`, `right` | object | `{available, level, charging, in_case}` |
| `case` | object | `{available, level, charging}`, no `in_case` |

`left`, `right` and `case` are always present as keys, each with its own
`available` flag — a deliberate simplification from omapods, whose AirPods
daemon omits the key entirely until a packet arrives because that is what its
Qt `QJsonObject` forced. Nothing here forces that, so a fresh `pixelbudsd`
just publishes `available: false` from `Status::default()` instead.
`Model.js`'s `budFrom`/`caseFrom` still treat `available !== true` as
"discard whatever else this object claims," so a parser bug on either side
would fail the same way regardless of which encoding was chosen.

# Control verbs

Sent as one line to `$XDG_RUNTIME_DIR/pixelbudspro.sock`, newline-terminated,
answered with one line back (`ok` or `error: ...`). Parsed by
`pixelbuds_common::parse_verb`, shared by both binaries so the grammar cannot
drift between them:

```
anc:off  anc:active  anc:aware  anc:adaptive
multipoint:on   multipoint:off
ohd:on          ohd:off
speech:on        speech:off
volumeexposure:on   volumeexposure:off
refresh
```

`refresh` does not touch the buds; it re-publishes the daemon's current
in-memory status. Useful from the terminal (`pixelbudsctl refresh`) to confirm the
daemon is alive and writing. The panel itself never sends it: pressing `r`
in `Service.qml` calls `stateFile.reload()`, a local re-read of the file that
does not touch the socket at all, exactly like omapods' `refresh()`.
