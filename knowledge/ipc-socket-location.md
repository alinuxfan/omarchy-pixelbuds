---
type: reference
title: The control socket lives under XDG_RUNTIME_DIR, ported straight from omapods
description: Same rule as the AirPods daemon, applied here in Rust instead of Qt
tags: [pixelbudsd, ipc, security]
status: stable
verified:
  - by: reading pixelbuds-common::socket_path and its unit test, socket_path_refuses_to_fall_back
    at: 2026-08-17
---

# What this plugin copied

omapods' `knowledge/ipc-socket-location.md` documents moving the AirPods
daemon's control socket off a predictable, world-visible `/tmp/app_server`
and onto `$XDG_RUNTIME_DIR/librepods.sock` (mode 0700 under a systemd
session's `/run/user/<uid>`). `pixelbudsd` does the same thing from the
start: `pixelbuds_common::socket_path()` returns
`$XDG_RUNTIME_DIR/pixelbudspro.sock`, and `main.rs` sets its permissions to
`0o700` right after `bind`.

# Why a shared crate instead of a shared header

omapods keeps the daemon and `librepods-ctl` in sync on the socket path with
a shared C++ header, `linux/ipcpath.hpp`, because two copies of a path string
is exactly the kind of thing that drifts. This project has the same failure
mode and the same fix, just in Rust: `pixelbuds-common::socket_path()` and
`state_path()` are called by both `pixelbudsd` and `pixelbudsctl`, and `parse_verb`
(the verb grammar) lives there too, so the daemon's dispatcher and the CLI's
own pre-flight validation parse the exact same code rather than two hand-kept
lists.

# Why it refuses to fall back

`socket_path()` returns `None` when `XDG_RUNTIME_DIR` is unset or empty, and
both binaries treat that as a startup error rather than falling back to
`/tmp`: `pixelbudsd` refuses to start (`main()` returns
`Err` via `.context(...)` before touching the network at all), and `pixelbudsctl`
prints `XDG_RUNTIME_DIR is not set; pixelbudsd cannot be reached` and exits
non-zero. A fallback would quietly restore the world-visible socket the
runtime dir exists to avoid, and every context that matters — the graphical
session, the Quickshell process running the panel — has one.
`pixelbuds-common`'s `socket_path_refuses_to_fall_back` test asserts both halves:
an empty environment gives `None`, a set one gives the expected path.
