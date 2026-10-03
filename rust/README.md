# Port knock (Rust)

GUI + CLI port knocking client, port of the AutoIt version in `../autoit`.

Same flow as the AutoIt form: pick a saved profile, edit its IP and its list of
knocks (type TCP/UDP, port, payload), press Knock. The sequence runs with a
configurable delay between packets, then the optional `after` command runs
(the equivalent of the `KNOCK_EXE_TARGET` in the PowerShell version).

The GUI was rewritten around the same flow: a sidebar of saved hosts beside the
editor, collapsing to a selector on narrow windows, one primary action per
view, and a light/dark theme. See [UI.md](UI.md) for the design tokens,
responsive rules and accessibility notes.

## GUI

| Key | Action |
| --- | --- |
| `Enter` | Knock the selected host |
| `Ctrl+S` | Write `config.toml` |

`Enter` is ignored while a text field has focus, so typing a host never fires a
knock. `Delete` arms a 3-second inline confirmation instead of opening a modal.

## Build

```sh
cargo build --release
```

The binary lands in `target/release/portknock.exe` (`target/release/portknock`
elsewhere). On first GUI start it writes a `config.toml` next to the
executable.

## CLI

```sh
portknock                        # GUI (same as --gui)
portknock --list                 # saved profiles
portknock --knock srv            # knock one profile, then run its `after`
portknock --config other.toml    # use a different config
portknock --knock srv --after "ssh srv"   # override the profile's command
```

## config.toml

```toml
delay_ms = 300          # wait between knocks; knockd's seq_timeout must exceed this
after = "mstsc /v:10.0.0.5 /prompt"   # default command for every profile
theme = "system"        # "system" (follows the OS), "dark" or "light"

[[profile]]
desc = "srv"            # the name shown in the list, and what --knock takes
host = "10.0.0.5"
after = "ssh srv"       # optional, wins over the global `after`

[[profile.knocks]]
proto = "tcp"
port = 7151

[[profile.knocks]]
proto = "tcp"
port = 10888

[[profile.knocks]]
proto = "udp"
port = 8899
text = "opensesame"     # UDP payload; empty sends a single zero-width space
```

## Notes

- A refused TCP connect is the knock, not an error: closed ports are the point.
  Only DNS/socket failures are reported.
- The delay defaults to the AutoIt's `sleep(300)`. If knockd drops your
  sequence, raise it (knockd's `seq_timeout` has to be larger than `delay_ms`).
- A TCP connect gives up after 2s, so an unreachable host costs 2s per knock.
- `theme` is optional: a config written before it existed still loads, and falls
  back to `system`. Saving from the GUI adds the key.

## Test

```sh
cargo test
```
