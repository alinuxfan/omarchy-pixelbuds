---
type: log
title: Change log for this knowledge bundle
description: What changed in the plugin's understanding of its own platform, and when
tags: [omarchy, pixelbuds, log]
---

# 2026-08-17

Initial bundle, written alongside the plugin itself, following
[omapods](https://github.com/thisisgm/omarchy-pods)'s structure and its
AirPods knowledge bundle. Built without Pixel Buds Pro 2 hardware in hand —
see `not-measured-on-hardware.md` for exactly what that does and does not
cover. `daemon/` was verified to compile, link, and pass
`cargo clippy --workspace --all-targets` with zero warnings against the real
`maestro` crate from `qzed/pbpctrl` at commit `2620367a`.
