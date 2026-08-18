---
type: reference
title: Maestro, and why this plugin uses it instead of Fast Pair
description: Which RPCs the daemon calls, which fields they return, and the protocol that was rejected
tags: [pixelbuds, maestro, gfps, fastpair, rfcomm]
status: stable
verified:
  - by: reading maestro_pw.proto and libmaestro/src in qzed/pbpctrl, and compiling pixelbudsd against the real crate
    at: 2026-08-17
---

# Two protocols were available, one was chosen

Pixel Buds Pro speak two different RFCOMM protocols that a Linux client can
join:

**Google Fast Pair Service (GFPS)**, an openly [specified](https://developers.google.com/nearby/fast-pair/spec)
protocol used for pairing handoff and a coarse battery broadcast
(`DeviceEventCode::BatteryInfo`: one byte per bud and the case, no charging
state, no case-placement, no settings access at all).

**Maestro**, the protocol the actual Google Buds Android app uses for
everything else: ANC, multipoint, on-head detection, EQ, gestures, and
per-device battery with charging state. It is not documented by Google; the
`maestro_pw.proto` schema in `pbpctrl` was recovered from the app.

This plugin uses Maestro exclusively — `pixelbudsd` does not link `gfps` at
all — because every field the panel shows (charging state, case placement,
ANC mode, the four toggles) requires it. GFPS would only be useful as a
fallback data source while Maestro's RFCOMM channel is renegotiating, and
`pbpctrl`'s own README already says as much: "For more detailed information,
use `pbpctrl show battery`", i.e. Maestro over GFPS.

# The two RPCs that carry everything the panel draws

`maestro_pw.Maestro/SubscribeRuntimeInfo` — a server-streamed
`google.protobuf.Empty → stream RuntimeInfo`. `RuntimeInfo.battery_info` is a
`BatteryInfo { case, left, right }`, each an `Option<DeviceBatteryInfo {
level: i32, state: BatteryState }>`. Each of the three can be independently
absent, which `pixelbudsd::maestro_link::bud_from`/`case_from` treats as
`available: false` rather than `level: 0`, the same "no packet, no value"
handling omapods' `Model.podFrom` uses for AirPods.
`RuntimeInfo.placement` is a `PlacementInfo { left_bud_in_case,
right_bud_in_case }` — case placement, not ear/on-head presence. There is no
proto field for whether a bud is in an ear at all; on-head detection (`ohd`)
is a device-side behavior toggle in this protocol, not a readable live state,
so unlike AirPods' three-way ear-detection behavior this panel only has an
on/off setting for it.

`maestro_pw.Maestro/SubscribeToSettingsChanges` — a server-streamed
`Empty → stream SettingsRsp`, one `SettingValue` per change. `pixelbudsd`
also calls the unary `ReadSetting` once per setting at startup
(`maestro_link::seed_initial_settings`) to get an initial value before the
first change event, since subscribing alone only reports *changes*.

Five settings, out of the ~17 `maestro_pw.SettingValue` carries, have a row
in this panel:

| Setting | `SettingId` | Panel control |
|---|---|---|
| `CurrentAncrState` | 13 | ANC mode (Off / Active / Aware / Adaptive) |
| `MultipointEnable` | 11 | Multipoint toggle |
| `OhdEnable` | 2 | On-head detection toggle |
| `SpeechDetection` | 22 | Speech Detection toggle |
| `VolumeExposureNotifications` | 21 | Volume Notifications toggle |

`AncState`'s wire values (`ANC_STATE_OFF = 1`, `ACTIVE = 2`, `AWARE = 3`,
`ADAPTIVE = 4`, `UNKNOWN = 0`) are copied verbatim into `pixelbuds_common::anc` and
into `Model.js`'s `ANC_*` constants, so nothing in this codebase re-numbers
them. "Aware" is Maestro's name for what the Pixel Buds app calls
Transparency mode; the panel shows the app's name and keeps the protocol's
constant.

# Why battery does not survive a disconnect here

AirPods keep broadcasting a battery BLE advertisement independent of the
audio link, so omapods' daemon can show battery with `connected: false`.
Every field this plugin reads — battery included — arrives over the same
Maestro RFCOMM session, which only exists while connected. There is nothing
to gate separately: `Service.qml`'s `hasBattery` is `hasBuds &&
(any level known)` rather than omapods' independent gate. See
`plugin-design-decisions.md`.
