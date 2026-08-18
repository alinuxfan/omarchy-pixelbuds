# pixelbudsd

The background half of this plugin. omapods (the AirPods panel this project
follows) vendors a fork of `librepods` here because there was no packaged
library for the AirPods Accessory Protocol on Linux. Pixel Buds Pro are in
the better position: [`pbpctrl`](https://github.com/qzed/pbpctrl) already
implements the Maestro protocol as two reusable, MIT/Apache-2.0 crates
(`maestro`, `gfps`), so `pixelbudsd` depends on `maestro` directly rather than
vendoring a copy. All credit for the protocol reverse-engineering belongs to
that project; this daemon is thin plumbing on top of it.

Two binaries:

- **`pixelbudsd`** — connects to the Maestro RFCOMM channel, subscribes to
  battery, case-placement and settings updates, and publishes them as one
  line of JSON to `$XDG_STATE_HOME/pixelbudspro/status.json` on every
  change. Also opens a control socket at
  `$XDG_RUNTIME_DIR/pixelbudspro.sock`.
- **`pixelbudsctl`** — sends one verb to that socket and prints the reply. This is
  what the panel's `Service.qml` spawns for every control.

See `../knowledge/maestro-protocol.md` for where each published field comes
from on the wire, and `../knowledge/status-schema.md` for the file format and
verb list.

## Building

```bash
cd daemon
cargo build --release
```

Requires a Rust toolchain (edition 2021) and, transitively through `bluer`,
D-Bus development headers:

```bash
# Arch Linux
sudo pacman -S dbus

# Debian/Ubuntu
sudo apt-get install libdbus-1-dev pkg-config
```

`maestro` is pulled from `qzed/pbpctrl` as a git dependency; `Cargo.lock`
pins the exact commit this daemon was built and checked against, so builds
are reproducible even though that repository is not versioned on crates.io.

## Installing

```bash
install -Dm755 target/release/pixelbudsd "$HOME/.local/bin/pixelbudsd"
install -Dm755 target/release/pixelbudsctl "$HOME/.local/bin/pixelbudsctl"
install -Dm644 pixelbudsd.service "$HOME/.config/systemd/user/pixelbudsd.service"
systemctl --user daemon-reload
systemctl --user enable --now pixelbudsd.service
```

Pair your Pixel Buds Pro first (`bluetoothctl` or the stock Omarchy
Bluetooth panel). `pixelbudsd` looks for a paired device advertising the
Maestro service UUID and needs no further configuration; pass `--device
<MAC>` to skip discovery.

## Running by hand

```bash
RUST_LOG=pixelbudsd=debug,maestro=info cargo run -- --device AA:BB:CC:DD:EE:FF
```

```bash
pixelbudsctl anc:aware      # Off | Active | Aware | Adaptive, i.e. anc:off / anc:active / anc:aware / anc:adaptive
pixelbudsctl multipoint:on
pixelbudsctl ohd:off
pixelbudsctl speech:on
pixelbudsctl volumeexposure:on
pixelbudsctl refresh
```

## What is not here

Equalizer bands, gesture-control targets, volume balance, mono output and
OTA/diagnostics/OOBE flags are all real Maestro settings — `maestro`'s own
`pbpctrl` CLI reads and writes every one of them — but none has a row in this
panel. Same reasoning as omapods' mic-mode omission: a bar widget is not the
place to reproduce the whole Google Buds app, and PipeWire already owns
per-app audio.
