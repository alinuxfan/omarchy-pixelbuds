---
type: reference
title: What this panel owns, and where its shape differs from omapods
description: The split against the stock audio and Bluetooth panels, and every deliberate divergence from the AirPods plugin this one follows
tags: [omarchy, quickshell, design]
status: stable
---

# The split

Same division omapods drew for AirPods, unchanged here:

| Control | Owner |
|---|---|
| Volume, output device list, per-app mixer | stock `omarchy.audio` |
| Connect, disconnect, forget, pairing | stock `omarchy.bluetooth` |
| Per-bud and case battery | this plugin |
| ANC mode, Multipoint, Speech Detection | this plugin |
| On-head detection, hearing-safety notifications | this plugin |
| Spatial Audio, EQ, gesture mapping | nobody: see below |

# Rows this plugin does not draw, on purpose

`maestro_pw.SettingValue` carries roughly seventeen settings; five have rows
here (see `status-schema.md`). The rest were left out for the same reason
omapods leaves out AirPods' mic-mode and Spatial Audio:

- **5-band EQ, volume balance, mono output** — real controls, but a
  five-slider EQ editor does not belong in a bar panel's popup any more than
  a PipeWire graph would. `pbpctrl set eq` already exists for this from a
  terminal.
- **Gesture-control targets, hold-gesture loop** — configuring what a tap or
  hold does is a one-time setup task, not something reached for during a
  session the way ANC mode is.
- **Auto-OTA, diagnostics, OOBE flags** — device provisioning state, not
  listening controls. Nothing here should let a stray keypress re-arm the
  out-of-box experience.

# Where this plugin's shape differs from omapods', and why

**No adaptive-level slider.** AirPods' Adaptive mode has a separate 0–100
intensity the AirPods panel exposes with `SliderRow`. Maestro's `AncState`
has no comparable field — Adaptive is just one of four discrete states — so
there is nothing to attach a slider to. See `maestro-protocol.md`.

**No three-way ear-detection cycle.** AirPods expose "pause when one is out
/ both are out / never" as a readable behavior omapods cycles through.
Maestro's on-head detection (`OhdEnable`) is a plain on/off setting, so this
panel has a `ToggleRow`, not omapods' `ValueRow` with a three-state cycle.

**No lid state.** `RuntimeInfo.placement` reports whether each bud is in the
case, not whether the case itself is open or closed — there is no field for
that on this wire. The AirPods panel's case row shows "Open"/"Closed"; this
one shows nothing extra for the case beyond battery and charging.

**`hasBattery` needs only `hasBuds`.** omapods gates battery on "any level
known" independent of the audio link, because AirPods keep broadcasting
battery over BLE while disconnected. Every field this plugin reads shares one
RFCOMM session, so there is no independent battery channel to gate
separately — see `maestro-protocol.md`'s closing section.

**The daemon is a real dependency, not a vendored fork.** omapods'
`daemon/` is a GPL-3.0 fork of `librepods`, vendored wholesale because there
was no packaged crate for the AirPods protocol. `pixelbudsd` instead
`git`-depends on `qzed/pbpctrl`'s `maestro` crate (MIT/Apache-2.0), because
that project already ships one. See `daemon/README.md`.

# No polling, same as omapods

`Service.qml` watches `$XDG_STATE_HOME/pixelbudspro/status.json` with a
`FileView` and runs no process while idle; a click spawns `pixelbudsctl` once and
the process exits. The optimistic-state handling (`_pendingField`,
`settleTimer`, the single-slot `_queued`) is copied from omapods' `Service.qml`
unchanged, because the problem it solves — a click should move the control at
once, without a write in flight getting snapped back by a stale read — has
nothing to do with which device is on the other end of the daemon.
