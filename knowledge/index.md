---
type: index
title: Platform facts behind this plugin
description: Machine-readable index of the facts this plugin depends on
tags: [omarchy, pixelbuds, maestro, quickshell]
---

# Knowledge bundle

An [Open Knowledge Format](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md)
bundle, following the same shape [omapods](https://github.com/thisisgm/omarchy-pods)
uses for its AirPods facts.

Every fact here is sourced from reading the `maestro` and `gfps` crates in
[qzed/pbpctrl](https://github.com/qzed/pbpctrl) at commit `2620367a`, and from
Google's public [Fast Pair specification](https://developers.google.com/nearby/fast-pair/spec),
not from a Pixel Buds Pro 2 connected to a running machine — this plugin was
built without the hardware in hand. Where that matters for a fact's
reliability, the file says so. `log.md` records what changed and when.

| Fact | Why it matters |
|---|---|
| [maestro-protocol](maestro-protocol.md) | which RPC gives which field, and the two protocols this plugin could have used instead |
| [pixel-buds-pro2-identity](pixel-buds-pro2-identity.md) | the service UUID, and why there is no on-wire model name to read |
| [status-schema](status-schema.md) | the wire format the panel parses and the verbs the control socket accepts |
| [ipc-socket-location](ipc-socket-location.md) | why the control socket lives under `XDG_RUNTIME_DIR`, ported from omapods' AirPods daemon |
| [plugin-design-decisions](plugin-design-decisions.md) | what this panel owns, what it deliberately leaves out, and where its shape differs from omapods' |
| [not-measured-on-hardware](not-measured-on-hardware.md) | the single biggest caveat on everything else in this bundle |
