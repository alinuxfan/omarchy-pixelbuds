---
type: log
title: Change log for this knowledge bundle
description: What changed in the plugin's understanding of its own platform, and when
tags: [omarchy, pixelbuds, log]
---

# 2026-08-18

First real Pixel Buds Pro 2 hardware connected. `pixelbudsd` compiled and
ran against it; found and fixed a deadlock in `maestro_link.rs`'s
`run_once()` that made this the daemon's first real bug, not a hardware
surprise:

- **`client.run()` must race every RPC call, not follow them.** `run_once`
  called `seed_initial_settings()` — five `read_setting_var` awaits — before
  ever starting `client.run()` as a concurrent task. Every RPC issued through
  a `ClientHandle` only queues a request; nothing sends it or reads the
  reply except `Client::run()`'s own loop. With no one polling that loop yet,
  the first `read_setting_var` call hung forever, and so did everything
  after it: settings stayed at their `Status::default()` values
  (`anc_mode: 0`/Unknown, all bools false), no `RuntimeInfo` (battery) ever
  arrived, and a `pbp2ctl anc:active` issued from another terminal hung for
  the full timeout with no response and no error. Confirmed against
  `qzed/pbpctrl`'s own `maestro_get_battery.rs`/`maestro_listen.rs`
  examples, which always drive `client.run()` concurrently with any RPC call
  via `tokio::select!`, never sequentially before it. Fixed by moving
  `seed_initial_settings` and the two subscription listeners into a second
  future raced against `client_task` in `run_once`'s outer `tokio::select!`,
  the same shape the upstream examples use.
- Once fixed, a real device round-tripped correctly: initial seed read real
  ANC mode, on-head detection, speech detection and volume-exposure state;
  `RuntimeInfo` delivered real battery levels for both buds (case battery
  reads `available: false` while the buds are out of the case, which reads
  as correct rather than broken); every `pbp2ctl` verb (`anc:*`,
  `multipoint`, `ohd`, `speech`, `volumeexposure`, `refresh`) applied
  instantly and pushed back through the settings-change stream into
  `status.json`.
- The initial RFCOMM `connect_profile` took about 42 seconds on first
  connection (`bluetoothctl` already showed the device as
  paired/bonded/trusted/connected at the ACL level throughout). Not
  reproduced a second time in this session but worth watching for — nothing
  in `daemon/README.md` sets an expectation either way.
- `Model.js`/`tests/model.test.js` and `cargo test --workspace` /
  `cargo clippy --workspace --all-targets` still pass after the fix.
- Not yet exercised on hardware: the panel (`Panel.qml`/`Service.qml`)
  itself as an installed Omarchy plugin, and the mid-air reconnect path
  (`os error 104`) `pbpctrl`'s examples call out.

# 2026-08-17

Initial bundle, written alongside the plugin itself, following
[omapods](https://github.com/thisisgm/omarchy-pods)'s structure and its
AirPods knowledge bundle. Built without Pixel Buds Pro 2 hardware in hand —
see `not-measured-on-hardware.md` for exactly what that does and does not
cover. `daemon/` was verified to compile, link, and pass
`cargo clippy --workspace --all-targets` with zero warnings against the real
`maestro` crate from `qzed/pbpctrl` at commit `2620367a`.
