---
type: reference
title: How this plugin finds and names the device
description: The Maestro service UUID used for discovery, and why model identification is hardcoded rather than read off the wire
tags: [pixelbuds, bluetooth, identity]
status: stable
verified:
  - by: reading libmaestro/src/lib.rs's UUID constant and maestro_pw.proto for anything resembling a model field
    at: 2026-08-17
---

# Discovery

`pixelbudsd --device <MAC>` skips discovery. Without it, the daemon asks
BlueZ for every paired device's advertised UUIDs and picks the first one that
lists the Maestro service UUID:

```
25e97ff7-24ce-4c4c-8951-f764a708f7b5
```

This is `maestro::UUID`, a constant in `libmaestro/src/lib.rs`. It is not
Pixel-Buds-Pro-2-specific — it is whatever the Maestro protocol implementation
on the bud's firmware advertises, and `pbpctrl`'s own README hedges that the
tool "might or might not work on other Pixel Buds devices." A second AllHub
device (a Pixel Watch, say) that also spoke Maestro would confuse this
discovery; there is no code here to disambiguate by name or model, only by
protocol support.

# Why `model_name` is a constant, not a read

AirPods send a model identifier in their BLE advertisement, which is how
omapods tells an AirPods Pro 3 from an AirPods 4 and fills `model_name`
dynamically. `maestro_pw.proto` has no equivalent field: `SoftwareInfo` and
`HardwareInfo` carry firmware and serial numbers per component (case, left,
right) but nothing that names the product line. `pixelbudsd` therefore
hardcodes `model_name: "Pixel Buds Pro 2"` — this plugin's target, not
something it detects — and leaves `device_name` as the live BlueZ alias, the
same split omapods uses for the two fields, just with one side fixed instead
of measured.

If this daemon is ever pointed at a different Maestro-speaking device, the
panel will still label it "Pixel Buds Pro 2" while showing that device's real
alias and settings. That mislabel is the cost of not having a wire-level
model field to check against.
