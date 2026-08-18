---
type: reference
title: This bundle was written without Pixel Buds Pro on hand
description: What was actually verified before shipping, and what a first real pairing should re-check
tags: [pixelbuds, maestro, caveat]
status: needs-verification
---

# What "verified" means in this bundle

omapods measured its facts against a running daemon and a connected AirPods
Pro 3. This plugin was built against the `maestro` and `gfps` source in
[qzed/pbpctrl](https://github.com/qzed/pbpctrl) at commit `2620367a` and
Google's published Fast Pair specification, with no Pixel Buds Pro attached
to the machine it was written on.

What that verification did cover:

- `daemon/` compiles, links, and passes `cargo clippy --workspace --all-targets`
  with zero warnings against the real `maestro` crate at that commit — not a
  stub, not a mock.
- `pixelbudsd` run with no paired device fails exactly the way the code
  says it should: `no paired device advertises the Maestro service UUID
  (25e97ff7-24ce-4c4c-8951-f764a708f7b5); pair your Pixel Buds Pro first, or
  pass --device <MAC>`.
- `pixelbudsctl` run with no daemon listening fails exactly the way the code says
  it should: `could not reach pixelbudsd at $XDG_RUNTIME_DIR/pixelbudspro.sock`.
- `tests/model.test.js` passes against a hand-written sample line built from
  the schema in `status-schema.md`, not a line a daemon actually wrote.

What it did not cover, because it requires the hardware:

- That `maestro_pw.AncState`'s four values actually round-trip through a real
  device's `WriteSetting`/`ReadSetting` calls the way `pbpctrl`'s own
  documentation implies.
- That `RuntimeInfo.placement` is populated the way its field names suggest
  (`right_bud_in_case`, `left_bud_in_case`) rather than meaning something
  adjacent, like on-head state.
- Reconnect behavior after the buds hand audio processing off between each
  other, which `pbpctrl`'s own examples specifically call out as a cause of
  connection resets (`os error 104`) worth retrying rather than failing on.
- Whether the Maestro UUID advertisement and RFCOMM channel resolution behave
  identically across the whole Pixel Buds Pro line. Everything this bundle
  has actually confirmed on hardware (see `log.md`) was tested against a
  Pixel Buds Pro 2 unit specifically, not the original (2021) Pixel Buds Pro
  `pbpctrl` was itself written against; its own README hedges with "might or
  might not work on other Pixel Buds devices."

# What to do on first real pairing

Run `pixelbudsd` with `RUST_LOG=pixelbudsd=debug,maestro=trace` in a terminal,
open the panel, and work through every row once. Anything that does not match
this bundle is this bundle's bug, not the daemon's; file it against
`log.md` with what was actually observed.
