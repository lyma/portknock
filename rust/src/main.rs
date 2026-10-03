#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;
mod ui;

use std::net::{TcpStream, ToSocketAddrs, UdpSocket};
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};

const DEFAULT_DELAY_MS: u64 = 300;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);
const NOOP: &str = "\u{200b}";

/// No console of its own, so a failure reached by double-click has nowhere to
/// print. Windows gets a message box; anywhere else stderr is enough.
fn fail(msg: &str) -> ! {
    eprintln!("{msg}");
    #[cfg(windows)]
    popup(msg);
    std::process::exit(1);
}

#[cfg(windows)]
fn popup(msg: &str) {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    extern "system" {
        fn MessageBoxW(
            hwnd: *mut std::ffi::c_void,
            text: *const u16,
            caption: *const u16,
            kind: u32,
        ) -> i32;
    }

    let wide = |s: &str| {
        OsStr::new(s)
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<u16>>()
    };
    let (text, caption) = (wide(msg), wide("portknock"));
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            caption.as_ptr(),
            0x10, // MB_ICONERROR
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Proto {
    Tcp,
    Udp,
}

impl Proto {
    fn label(self) -> &'static str {
        match self {
            Proto::Tcp => "TCP",
            Proto::Udp => "UDP",
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Knock {
    proto: Proto,
    port: u16,
    /// Payload sent on UDP knocks. Ignored for TCP.
    #[serde(default)]
    text: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct Profile {
    desc: String,
    #[serde(alias = "ip")]
    host: String,
    #[serde(default)]
    knocks: Vec<Knock>,
    /// Runs after the sequence. Falls back to the global `after`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    after: Option<String>,
}

impl Default for Profile {
    fn default() -> Self {
        Profile {
            desc: String::new(),
            host: String::new(),
            knocks: vec![Knock {
                proto: Proto::Tcp,
                port: 0,
                text: String::new(),
            }],
            after: None,
        }
    }
}

/// Persisted in `config.toml`, so the toolbar toggle survives a restart without
/// pulling in eframe's `persistence` feature. `System` follows the OS and is
/// what a fresh config gets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ThemePref {
    #[default]
    System,
    Dark,
    Light,
}

#[derive(Serialize, Deserialize)]
struct Config {
    #[serde(default = "default_delay")]
    delay_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    after: Option<String>,
    #[serde(default)]
    theme: ThemePref,
    #[serde(default)]
    profile: Vec<Profile>,
}

fn default_delay() -> u64 {
    DEFAULT_DELAY_MS
}

impl Default for Config {
    fn default() -> Self {
        Config {
            delay_ms: DEFAULT_DELAY_MS,
            after: None,
            theme: ThemePref::System,
            profile: vec![Profile {
                desc: "GregRocks".into(),
                host: "127.0.0.1".into(),
                knocks: vec![Knock {
                    proto: Proto::Udp,
                    port: 0,
                    text: String::new(),
                }],
                after: None,
            }],
        }
    }
}

/// One TCP SYN or one UDP datagram. Failures are not the caller's problem:
/// a refused TCP connect *is* the knock.
fn knock_one(host: &str, k: &Knock) -> Result<(), String> {
    match k.proto {
        Proto::Tcp => {
            let addr = (host, k.port)
                .to_socket_addrs()
                .map_err(|e| format!("resolve {host}: {e}"))?
                .next()
                .ok_or_else(|| format!("no address for {host}"))?;
            let _ = TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT);
            Ok(())
        }
        Proto::Udp => {
            // ponytail: empty payload sends one zero-width space, not a
            // zero-length datagram. Some stacks drop those, some firewalls ignore them.
            let payload = if k.text.is_empty() { NOOP } else { &k.text };
            let sock = UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
            let sent = sock
                .send_to(payload.as_bytes(), (host, k.port))
                .map_err(|e| format!("{host}:{}: {e}", k.port))?;
            debug_assert!(sent > 0);
            Ok(())
        }
    }
}

fn knock_seq(
    profile: &Profile,
    delay: Duration,
    mut on_knock: impl FnMut(&Knock),
) -> Result<(), String> {
    for (i, k) in profile.knocks.iter().enumerate() {
        on_knock(k);
        knock_one(&profile.host, k)?;
        if i + 1 < profile.knocks.len() {
            thread::sleep(delay);
        }
    }
    Ok(())
}

fn run_after(cmd: &str) -> Result<(), String> {
    let mut child = if cfg!(windows) {
        Command::new("cmd").args(["/C", cmd]).spawn()
    } else {
        Command::new("sh").args(["-c", cmd]).spawn()
    }
    .map_err(|e| format!("after `{cmd}`: {e}"))?;
    // Don't become a zombie; the launched app outlives us anyway.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

fn config_path(explicit: Option<&str>) -> PathBuf {
    match explicit {
        Some(p) => PathBuf::from(p),
        None => std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|dir| dir.join("config.toml")))
            .unwrap_or_else(|| PathBuf::from("config.toml")),
    }
}

fn load_config(path: &PathBuf) -> Result<Config, String> {
    match std::fs::read_to_string(path) {
        Ok(s) => toml::from_str(&s).map_err(|e| format!("{}: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

fn save_config(path: &PathBuf, cfg: &Config) -> Result<(), String> {
    let body = toml::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    std::fs::write(path, body).map_err(|e| format!("{}: {e}", path.display()))
}

fn find_profile<'a>(cfg: &'a Config, desc: &str) -> Result<&'a Profile, String> {
    cfg.profile
        .iter()
        .find(|p| p.desc.eq_ignore_ascii_case(desc))
        .ok_or_else(|| format!("no profile named `{desc}`"))
}

fn run_cli(cfg: &Config, desc: &str, after: Option<&str>) -> i32 {
    let profile = match find_profile(cfg, desc) {
        Ok(p) => p.clone(),
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };
    let delay = Duration::from_millis(cfg.delay_ms);
    let host = profile.host.clone();
    let label = profile.desc.clone();

    if let Err(e) = knock_seq(&profile, delay, |k| {
        println!("Knocking {} port {}...", k.proto.label(), k.port);
    }) {
        eprintln!("knock failed: {e}");
        return 1;
    }
    println!("Knock ends! ({label} @ {host})");

    let cmd = after
        .map(String::from)
        .or_else(|| profile.after.clone())
        .or_else(|| cfg.after.clone());
    match cmd {
        Some(cmd) => {
            println!("Executing: {cmd}");
            if let Err(e) = run_after(&cmd) {
                eprintln!("{e}");
                return 1;
            }
        }
        None => println!("Open sesame!"),
    }
    0
}

const HELP: &str = "\
portknock - port knocking client (Rust)

Usage:
  portknock                    open the GUI
  portknock --list             list saved profiles
  portknock --knock <desc>     knock the ports of a saved profile

Options:
  --config <path>   config file (default: config.toml next to the executable)
  --after <cmd>     command to run after the sequence, overrides the profile
  --list
  --knock <desc>
  -h, --help
  -V, --version
";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut config = None;
    let mut after = None;
    let mut knock = None;
    let mut list = false;

    let mut i = 0;
    let need = |i: usize| -> String {
        match args.get(i + 1) {
            Some(v) => v.clone(),
            None => {
                eprintln!("{} needs a value", args[i]);
                std::process::exit(2);
            }
        }
    };
    while i < args.len() {
        let step = match args[i].as_str() {
            "-h" | "--help" => {
                print!("{HELP}");
                return;
            }
            "-V" | "--version" => {
                println!("portknock {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            "--gui" => 1,
            "--list" => {
                list = true;
                1
            }
            "--config" => {
                config = Some(need(i));
                2
            }
            "--after" => {
                after = Some(need(i));
                2
            }
            "--knock" => {
                knock = Some(need(i));
                2
            }
            other => {
                eprintln!("unknown option `{other}`\n\n{HELP}");
                std::process::exit(2);
            }
        };
        i += step;
    }

    let path = config_path(config.as_deref());
    let cfg = match load_config(&path) {
        Ok(c) => c,
        Err(e) => fail(&e),
    };

    if list {
        if cfg.profile.is_empty() {
            println!("no profiles in {}", path.display());
        }
        for p in &cfg.profile {
            println!("{}\t{}", p.desc, p.host);
        }
        return;
    }

    if let Some(desc) = knock {
        std::process::exit(run_cli(&cfg, &desc, after.as_deref()));
    }

    if !path.exists() {
        if let Err(e) = save_config(&path, &cfg) {
            eprintln!("warning: could not create {}: {e}", path.display());
        }
    }

    // Below this the form cannot lay out its columns; a floor is better than a
    // window that scrolls sideways. See `ui::split_row` for the width rules.
    let mut opts = eframe::NativeOptions::default();
    opts.viewport.min_inner_size = Some(eframe::egui::Vec2::new(480.0, 420.0));

    if let Err(e) = eframe::run_native(
        "portknock",
        opts,
        Box::new(|cc| Ok(Box::new(app::App::new(cc, path, cfg)))),
    ) {
        fail(&format!("gui failed: {e}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::TcpListener;

    fn free_port() -> u16 {
        TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    #[test]
    fn tcp_sequence_reaches_ports_in_order() {
        let listeners: Vec<TcpListener> = (0..3)
            .map(|_| TcpListener::bind(("127.0.0.1", 0)).unwrap())
            .collect();
        let ports: Vec<u16> = listeners
            .iter()
            .map(|l| l.local_addr().unwrap().port())
            .collect();

        let profile = Profile {
            desc: "t".into(),
            host: "127.0.0.1".into(),
            knocks: ports
                .iter()
                .map(|p| Knock {
                    proto: Proto::Tcp,
                    port: *p,
                    text: String::new(),
                })
                .collect(),
            after: None,
        };

        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink = seen.clone();
        let collector = std::thread::spawn(move || {
            for l in listeners {
                let (mut s, _) = l.accept().unwrap();
                sink.lock().unwrap().push(s.local_addr().unwrap().port());
                let _ = s.write_all(b"x");
            }
        });

        let mut order = Vec::new();
        knock_seq(&profile, Duration::from_millis(20), |k| order.push(k.port)).unwrap();
        collector.join().unwrap();

        assert_eq!(order, ports);
        assert_eq!(*seen.lock().unwrap(), ports);
    }

    #[test]
    fn closed_tcp_port_is_not_an_error() {
        let profile = Profile {
            desc: "t".into(),
            host: "127.0.0.1".into(),
            knocks: vec![Knock {
                proto: Proto::Tcp,
                port: free_port(),
                text: String::new(),
            }],
            after: None,
        };
        assert!(knock_seq(&profile, Duration::ZERO, |_| {}).is_ok());
    }

    #[test]
    fn config_round_trips_through_toml() {
        let cfg = Config {
            delay_ms: 500,
            after: Some("echo hi".into()),
            theme: ThemePref::Dark,
            profile: vec![Profile {
                desc: "srv".into(),
                host: "10.0.0.5".into(),
                knocks: vec![Knock {
                    proto: Proto::Udp,
                    port: 1234,
                    text: "ping".into(),
                }],
                after: Some("ssh srv".into()),
            }],
        };
        let back: Config = toml::from_str(&toml::to_string(&cfg).unwrap()).unwrap();
        assert_eq!(back.delay_ms, 500);
        assert_eq!(back.theme, ThemePref::Dark);
        assert_eq!(back.profile[0].knocks[0].proto, Proto::Udp);
        assert_eq!(back.profile[0].after.as_deref(), Some("ssh srv"));
    }

    #[test]
    fn missing_config_falls_back_to_default() {
        let p = PathBuf::from("definitely-not-here-9182.toml");
        let cfg = load_config(&p).unwrap();
        assert_eq!(cfg.delay_ms, DEFAULT_DELAY_MS);
        assert_eq!(cfg.profile.len(), 1);
    }

    /// Configs written before the theme toggle existed must still load: the
    /// field is `#[serde(default)]`, so an absent key means "follow the OS".
    #[test]
    fn config_without_theme_key_still_loads() {
        let cfg: Config =
            toml::from_str("delay_ms = 300\n\n[[profile]]\ndesc = \"srv\"\nhost = \"10.0.0.5\"\n")
                .unwrap();
        assert_eq!(cfg.theme, ThemePref::System);
        assert_eq!(cfg.profile.len(), 1);
    }
}
