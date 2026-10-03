# Port knock

Various portknockers clients.

| Directory | Type | How to run | Binary |
| --- | --- | --- | --- |
| [`rust/`](rust/) | GUI | `cd rust && cargo run --release` | [release `rust-gui-v0.1.0`](https://github.com/lyma/portknock/releases/tag/rust-gui-v0.1.0), Windows x64 |
| [`autoit/`](autoit/) | CLI | — | `autoit/porknock.exe`, `autoit/tcp.exe` |
| [`bash/`](bash/) | CLI | `bash/portknock.sh <port1> <port2> <port3> <host>` | — |
| [`powershell/`](powershell/) | CLI | edit the constants at the top, then `.\powershell\portknocker.ps1` | — |
| [`c/`](c/) | CLI | `portknock <host> <port1> <port2> <portN>` | — |

Each one is independent; pick whichever fits. Everything but the Rust client is
a script or a single C file, so there is nothing to build.

The Rust client is the only one with a graphical interface — host profiles,
knock sequences and a dry run per step. It also reads and writes a config file;
the others take their arguments on the command line. See
[`rust/UI.md`](rust/UI.md) for how its layout works.

:)

