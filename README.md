<h1 align="center">Pixel Buds Pro 2 for Omarchy</h1>

<p align="center">
  Battery for each bud and the case, ANC mode, Multipoint, on-head detection, Speech Detection and hearing-safety notifications, drawn in Omarchy's own panel idiom.
</p>

<p align="center">
  Follows the shape of <a href="https://github.com/thisisgm/omarchy-pods">omarchy-pods</a>, the AirPods panel for Omarchy, adapted to what Pixel Buds Pro 2 actually expose over Google's Maestro protocol.
</p>

## What it shows

- **Battery** for the left bud, the right bud and the case, each with a
  charging hint and, for the buds, whether they're sitting in the case.
  Nothing else on a Linux box knows these numbers: this comes from the
  Maestro RFCOMM channel, not `org.bluez.Battery1`.
- **ANC mode** — Off, Active (Noise Cancellation), Aware (Transparency) or
  Adaptive. All four are always present; unlike AirPods, Maestro does not
  vary the mode list by model.
- **Multipoint**, **Speech Detection** (auto-Transparency when you start
  talking) and **Volume Notifications** (hearing-safety warnings), as
  toggles.
- **On-head detection**, as a toggle. Maestro exposes this as a plain on/off
  setting rather than AirPods' three-way pause behavior.

## Deliberately absent

- **Volume and output device** live in the stock Audio panel.
- **Connect, disconnect and forget** live in the stock Bluetooth panel, and
  in `omarchy bluetooth device`.
- **5-band EQ, volume balance, mono output, gesture mapping, auto-OTA,
  diagnostics** are real Maestro settings this daemon's own libraries can
  read and write, but none has a row here. See
  [knowledge/plugin-design-decisions.md](knowledge/plugin-design-decisions.md)
  for why.

## Requirements

- **The daemon in [`daemon/`](daemon/), built and running.** Unlike
  omapods' AirPods daemon, this one is not a vendored fork: it depends
  directly on the `maestro` crate from
  [qzed/pbpctrl](https://github.com/qzed/pbpctrl), which already implements
  the protocol as a proper library. See [daemon/README.md](daemon/README.md).
- Pixel Buds Pro 2 paired to the machine through the usual Bluetooth flow.

## How it works

The plugin does not poll. The daemon writes its status to
`$XDG_STATE_HOME/pixelbudspro2/status.json` whenever that status changes, and
removes the file when it stops. The panel watches it with a `FileView`, so an
idle desktop runs no processes at all on its behalf. `pbp2ctl` is spawned only
when you actually change something, over a control socket at
`$XDG_RUNTIME_DIR/pixelbudspro2.sock`.

The plugin never talks to Bluetooth itself. If `pbp2ctl` is missing or the
daemon is not running, the panel says so in one line instead of drawing an
empty surface.

## Install

```bash
omarchy plugin add https://github.com/alinuxfan/omarchy-pixelbuds --enable
omarchy bar move io.github.alinuxfan.pixelbudspro2
```

Then build the daemon out of the copy that just cloned, and hand it to
systemd. Building it needs a Rust toolchain and D-Bus development headers
(`dbus` on Arch, `libdbus-1-dev pkg-config` on Debian/Ubuntu):

```bash
cd ~/.config/omarchy/plugins/io.github.alinuxfan.pixelbudspro2/daemon
cargo build --release
install -Dm755 target/release/pixelbudsd "$HOME/.local/bin/pixelbudsd"
install -Dm755 target/release/pbp2ctl "$HOME/.local/bin/pbp2ctl"
install -Dm644 pixelbudsd.service "$HOME/.config/systemd/user/pixelbudsd.service"
systemctl --user daemon-reload
systemctl --user enable --now pixelbudsd.service
```

`~/.local/bin` is where the unit expects the binaries and where the panel
finds `pbp2ctl`; Omarchy already puts it on `PATH`. The unit is bound to
`graphical-session.target`, so the daemon comes back after a reboot.

## Remove

```bash
systemctl --user disable --now pixelbudsd.service
rm -f ~/.local/bin/pixelbudsd ~/.local/bin/pbp2ctl ~/.config/systemd/user/pixelbudsd.service
omarchy plugin remove io.github.alinuxfan.pixelbudspro2
```

## Keyboard

| Key | Action |
|-----|--------|
| `j` / `k`, `↓` / `↑` | move between rows |
| `enter` / `space` | activate the current row |
| `o` | ANC Off |
| `n` | Noise Cancellation (Active) |
| `t` | Transparency (Aware) |
| `a` | Adaptive |
| `m` | toggle Multipoint |
| `h` | toggle on-head detection |
| `s` | toggle Speech Detection |
| `v` | toggle Volume Notifications |
| `r` | refresh |
| `tab` | move to the next panel |
| `esc` | close |

Left click opens the panel. Right click cycles the ANC mode without opening
anything.

## Settings

| Setting | Default | Notes |
|---------|---------|-------|
| Hide when disconnected | on | Leaves the bar entirely rather than sitting there with nothing to say. |
| Path to pbp2ctl | empty | Leave empty to find it on `PATH`. |

## Tests

`Model.js` holds the parsing and formatting, with no QML imports, so it runs
outside the shell:

```bash
deno run --allow-read tests/model.test.js
```

`daemon/` has its own unit tests, plus a real compile-and-link check against
the `maestro` crate:

```bash
cd daemon && cargo test --workspace && cargo clippy --workspace --all-targets
```

## A caveat worth reading before you file an issue

This plugin was built without a Pixel Buds Pro 2 unit connected to the
machine it was written on. The daemon compiles, links and passes its tests
against the real `maestro` protocol library, and fails exactly the way it
should when nothing is paired — but nothing here has been confirmed against
a live device yet. See
[knowledge/not-measured-on-hardware.md](knowledge/not-measured-on-hardware.md)
for exactly what that does and doesn't cover, and please open an issue with
what you actually saw on first real pairing.

## Credits

The hard part is not this panel, or even this daemon: it's
[pbpctrl](https://github.com/qzed/pbpctrl) by **Maximilian Luz**, which
recovered Google's Maestro protocol and shipped it as reusable, permissively
licensed Rust crates. `daemon/pixelbudsd` is a thin wrapper around that work, depending on
`maestro` as a normal Cargo git dependency rather than a vendored fork.

## Licence

MIT, see [LICENSE](LICENSE). `daemon/`'s own dependency on `maestro` and
`gfps` is dual MIT/Apache-2.0 upstream; see
[daemon/README.md](daemon/README.md).
