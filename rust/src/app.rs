use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use eframe::egui;
use eframe::egui::Color32;

use crate::ui::{self as kit, Kind};
use crate::{knock_seq, run_after, save_config, Config, Knock, Profile, Proto, ThemePref};

enum Msg {
    Knock(String),
    Done(Result<(), String>),
}

/// Colour of the status line. Drives the dot, not the text.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tone {
    Idle,
    Busy,
    Ok,
    Error,
}

pub struct App {
    path: PathBuf,
    cfg: Config,
    selected: Option<usize>,
    busy: bool,
    status: String,
    tone: Tone,
    clear_status_at: Option<Instant>,
    events: Receiver<Msg>,
    sender: Sender<Msg>,
    ctx: egui::Context,
    /// `Delete` becomes `Confirm?` until this instant.
    arm_delete_until: Option<Instant>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, path: PathBuf, cfg: Config) -> Self {
        let (sender, events) = channel();
        let app = App {
            path,
            cfg,
            selected: None,
            busy: false,
            status: String::new(),
            tone: Tone::Idle,
            clear_status_at: None,
            events,
            sender,
            ctx: cc.egui_ctx.clone(),
            arm_delete_until: None,
        };
        app.apply_theme();
        app
    }

    fn apply_theme(&self) {
        let pref = match self.cfg.theme {
            ThemePref::System => egui::ThemePreference::System,
            ThemePref::Dark => egui::ThemePreference::Dark,
            ThemePref::Light => egui::ThemePreference::Light,
        };
        self.ctx.set_theme(pref);
        // set_theme rebuilds Visuals from the theme defaults, so our rounding
        // and spacing have to be re-applied on top.
        kit::install(&self.ctx);
    }

    fn toggle_theme(&mut self) {
        self.cfg.theme = match self.ctx.theme() {
            egui::Theme::Dark => ThemePref::Light,
            egui::Theme::Light => ThemePref::Dark,
        };
        self.apply_theme();
        self.persist();
    }

    fn say(&mut self, msg: impl Into<String>, tone: Tone) {
        self.status = msg.into();
        self.tone = tone;
        self.clear_status_at = None;
    }

    fn say_timed(&mut self, msg: impl Into<String>, tone: Tone, ttl: Duration) {
        self.say(msg, tone);
        self.clear_status_at = Some(Instant::now() + ttl);
    }

    fn poll(&mut self) {
        while let Ok(msg) = self.events.try_recv() {
            match msg {
                Msg::Knock(s) => self.say(s, Tone::Busy),
                Msg::Done(result) => {
                    self.busy = false;
                    match result {
                        Ok(()) => {
                            self.say_timed("knock complete", Tone::Ok, Duration::from_secs(5))
                        }
                        Err(e) => self.say(format!("failed: {e}"), Tone::Error),
                    }
                }
            }
        }
        if self.clear_status_at.is_some_and(|t| Instant::now() >= t) {
            self.status.clear();
            self.tone = Tone::Idle;
            self.clear_status_at = None;
        }
        if self.arm_delete_until.is_some_and(|t| Instant::now() >= t) {
            self.arm_delete_until = None;
        }
    }

    fn persist(&mut self) {
        if let Err(e) = save_config(&self.path, &self.cfg) {
            self.say(format!("save failed: {e}"), Tone::Error);
        }
    }

    fn commit(&mut self) {
        let Some(i) = self.selected else {
            self.say("nothing selected", Tone::Error);
            return;
        };
        let Some(p) = self.cfg.profile.get(i) else {
            return;
        };
        if p.desc.trim().is_empty() {
            self.say("a host needs a name", Tone::Error);
            return;
        }
        let desc = p.desc.clone();
        match self.cfg.profile.iter().position(|o| o.desc == desc) {
            // Renaming onto an existing name: keep the old slot's identity so
            // the list does not jump, then drop the duplicate.
            Some(other) if other != i => {
                self.cfg.profile[other] = self.cfg.profile[i].clone();
                self.cfg.profile.remove(i);
                self.selected = Some(other);
            }
            _ => {}
        }
        self.persist();
        self.say(format!("saved {desc}"), Tone::Ok);
    }

    fn new_profile(&mut self) {
        let desc = format!("Host {}", self.cfg.profile.len() + 1);
        self.cfg.profile.push(Profile {
            desc,
            ..Profile::default()
        });
        self.selected = Some(self.cfg.profile.len() - 1);
        self.arm_delete_until = None;
        self.say("fill in the host, then Save", Tone::Idle);
    }

    fn delete(&mut self) {
        let Some(i) = self.selected else {
            self.say("nothing selected", Tone::Error);
            return;
        };
        let Some(removed) = self.cfg.profile.get(i).map(|p| p.desc.clone()) else {
            return;
        };
        self.cfg.profile.remove(i);
        self.selected = None;
        self.arm_delete_until = None;
        self.persist();
        self.say(format!("deleted {removed}"), Tone::Ok);
    }

    fn knock(&mut self) {
        if self.busy {
            return;
        }
        let Some(i) = self.selected else {
            self.say("pick a host first", Tone::Error);
            return;
        };
        let Some(profile) = self.cfg.profile.get(i).cloned() else {
            return;
        };
        if profile.host.trim().is_empty() {
            self.say("host is required", Tone::Error);
            return;
        }
        if profile.knocks.iter().any(|k| k.port == 0) {
            self.say("every knock needs a port", Tone::Error);
            return;
        }

        let after = profile.after.clone().or_else(|| self.cfg.after.clone());
        let delay = Duration::from_millis(self.cfg.delay_ms);
        let host = profile.host.clone();
        self.busy = true;
        self.say("knocking…", Tone::Busy);

        let sender = self.sender.clone();
        thread::spawn(move || {
            let knocked = knock_seq(&profile, delay, |k| {
                let _ = sender.send(Msg::Knock(format!(
                    "knocking {} {}:{}",
                    k.proto.label(),
                    host,
                    k.port
                )));
            });
            let result = match (knocked, after) {
                (Err(e), _) => Err(e),
                (Ok(()), Some(cmd)) => run_after(&cmd),
                (Ok(()), None) => Ok(()),
            };
            let _ = sender.send(Msg::Done(result));
        });
    }

    // ---------------------------------------------------------------- layout

    /// Toolbar height. Fixed, so it has to fit its row — see the
    /// `nothing_is_cut_off_in_the_toolbar` test.
    const TOOLBAR_H: f32 = 56.0;
    const STATUS_H: f32 = 34.0;

    fn shell(&mut self, ui: &mut egui::Ui) {
        let wide = ui.available_width() >= kit::NARROW;

        egui::Panel::top("toolbar")
            .exact_size(Self::TOOLBAR_H)
            .resizable(false)
            .show_separator_line(false)
            .show(ui, |ui| self.toolbar(ui));

        if wide {
            egui::Panel::left("sidebar")
                .exact_size(kit::SIDEBAR_W)
                .resizable(false)
                .show_separator_line(false)
                .show(ui, |ui| self.sidebar(ui));
        }

        egui::Frame::central_panel(ui.style())
            .fill(kit::tokens(ui).card)
            .show(ui, |ui| {
                if !wide {
                    self.narrow_picker(ui);
                    ui.add_space(kit::UNIT);
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    // Without this the content is laid out at its natural width
                    // and simply painted off the edge of the window.
                    ui.set_max_width(ui.available_width());
                    self.detail(ui);
                });
            });

        egui::Panel::bottom("status")
            .exact_size(Self::STATUS_H)
            .resizable(false)
            .show_separator_line(false)
            .show(ui, |ui| self.status_bar(ui));
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        let t = kit::tokens(ui);
        // Fixed budget for the delay/theme cluster, so the title beside it can
        // shrink instead of shoving the cluster off the right edge.
        let right_w = 196.0;
        kit::split_row(
            ui,
            right_w,
            |ui| {
                ui.label(egui::RichText::new("Port Knock").size(16.0).strong());
                if ui.available_width() > 130.0 {
                    kit::clipped(
                        ui,
                        egui::RichText::new("port knocking client")
                            .small()
                            .color(t.muted),
                    );
                }
            },
            |ui| {
                let (glyph, tip) = if self.ctx.theme() == egui::Theme::Dark {
                    ("\u{2600}", "Switch to light theme")
                } else {
                    ("\u{263c}", "Switch to dark theme")
                };
                if kit::icon_button(ui, tip, glyph).clicked() {
                    self.toggle_theme();
                }
                kit::clipped(ui, egui::RichText::new("Delay").small().color(t.muted));
                let mut ms = self.cfg.delay_ms as i64;
                if ui
                    .add(
                        egui::DragValue::new(&mut ms)
                            .range(0..=60_000)
                            .suffix(" ms"),
                    )
                    .changed()
                {
                    self.cfg.delay_ms = ms.max(0) as u64;
                    self.persist();
                }
            },
        );
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.set_max_width(ui.available_width());
            ui.horizontal(|ui| {
                kit::label(ui, "Hosts");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if kit::button(ui, Kind::Secondary, "+ New").clicked() {
                        self.new_profile();
                    }
                });
            });
            ui.add_space(kit::UNIT);

            if self.cfg.profile.is_empty() {
                kit::empty_state(
                    ui,
                    "No hosts yet",
                    "Create one to knock its ports in order.",
                );
                return;
            }

            for i in 0..self.cfg.profile.len() {
                let p = self.cfg.profile[i].clone();
                let selected = self.selected == Some(i);
                let card = kit::card(ui, selected, |ui| {
                    let t = kit::tokens(ui);
                    kit::split_row(
                        ui,
                        kit::BADGE_SLOT,
                        |ui| {
                            kit::clipped(ui, egui::RichText::new(p.desc.clone()).strong());
                        },
                        |ui| {
                            let n = p.knocks.len();
                            let text = match n {
                                0 => "empty".to_string(),
                                1 => "1 knock".to_string(),
                                n => format!("{n} knocks"),
                            };
                            kit::badge_right(
                                ui,
                                kit::BADGE_SLOT,
                                &text,
                                t.muted,
                                kit::mix(t.card_border, t.muted, 0.2),
                            );
                        },
                    );
                    let host = if p.host.trim().is_empty() {
                        "no host set".to_string()
                    } else {
                        p.host.clone()
                    };
                    kit::clipped(ui, egui::RichText::new(host).small().color(t.muted));
                    if let Some(after) = p.after.as_deref().filter(|a| !a.trim().is_empty()) {
                        kit::clipped(
                            ui,
                            egui::RichText::new(format!("then {after}"))
                                .small()
                                .color(t.muted),
                        );
                    }
                });
                if card.clicked() {
                    self.selected = Some(i);
                    self.arm_delete_until = None;
                }
            }
        });
    }

    /// Below [`kit::NARROW`] there is no room for a sidebar, so the same choice
    /// becomes a dropdown above the form.
    fn narrow_picker(&mut self, ui: &mut egui::Ui) {
        if self.cfg.profile.is_empty() {
            return;
        }
        let names: Vec<String> = self
            .cfg
            .profile
            .iter()
            .map(|p| {
                if p.desc.trim().is_empty() {
                    if p.host.trim().is_empty() {
                        "unnamed".to_string()
                    } else {
                        p.host.clone()
                    }
                } else {
                    p.desc.clone()
                }
            })
            .collect();
        let current = self.selected.unwrap_or(0).min(names.len() - 1);
        let mut picked = self.selected;
        egui::ComboBox::from_id_salt("host-picker")
            .selected_text(names[current].clone())
            .width(ui.available_width())
            .show_ui(ui, |ui| {
                for (i, name) in names.iter().enumerate() {
                    ui.selectable_value(&mut picked, Some(i), name.clone());
                }
            });
        if let Some(i) = picked {
            self.selected = Some(i);
        }
    }

    fn detail(&mut self, ui: &mut egui::Ui) {
        if self.cfg.profile.is_empty() {
            kit::empty_state(
                ui,
                "No host selected",
                "A host is a name, an address and the ports to knock, in order.",
            );
            ui.vertical_centered(|ui| {
                if kit::button(ui, Kind::Primary, "+ New host").clicked() {
                    self.new_profile();
                }
            });
            return;
        }
        if self.selected.is_none() {
            self.selected = Some(0);
        }

        let i = self.selected.unwrap();
        let t = kit::tokens(ui);
        let (desc, host, n_knocks) = {
            let p = &self.cfg.profile[i];
            (p.desc.clone(), p.host.clone(), p.knocks.len())
        };

        kit::split_row(
            ui,
            kit::BADGE_SLOT,
            |ui| {
                kit::clipped(
                    ui,
                    egui::RichText::new(if desc.is_empty() { "New host" } else { &desc })
                        .size(22.0)
                        .strong(),
                );
            },
            |ui| {
                let (label, fg, bg) = if self.busy {
                    ("busy", t.accent, kit::mix(t.card, t.accent, 0.16))
                } else if n_knocks == 0 {
                    ("no knocks", t.muted, kit::mix(t.card_border, t.muted, 0.2))
                } else {
                    (
                        "ready",
                        kit::mix(t.text, t.accent, 0.35),
                        kit::mix(t.card, t.accent, 0.12),
                    )
                };
                kit::badge_right(ui, kit::BADGE_SLOT, label, fg, bg);
            },
        );
        kit::clipped(
            ui,
            egui::RichText::new(if host.is_empty() { "—" } else { &host })
                .small()
                .color(t.muted),
        );

        ui.add_space(kit::UNIT * 2.0);
        kit::label(ui, "Name");
        {
            let p = &mut self.cfg.profile[i];
            let mut d = p.desc.clone();
            if kit::input(ui, &mut d, "what to call this host").changed() {
                p.desc = d;
            }
        }
        kit::label(ui, "Host");
        {
            let p = &mut self.cfg.profile[i];
            let mut h = p.host.clone();
            if kit::input(ui, &mut h, "10.0.0.5 or host.example.com").changed() {
                p.host = h;
            }
        }

        self.knock_rows(ui, i);
        self.after_field(ui, i);

        ui.add_space(kit::UNIT * 2.0);
        self.actions(ui);
    }

    /// Edit the knock list in place. Removal is deferred past the borrow so we
    /// never invalidate the iterator we are walking.
    fn knock_rows(&mut self, ui: &mut egui::Ui, i: usize) {
        let t = kit::tokens(ui);
        kit::section(ui, "Knock sequence");

        // Budget every column from the row width instead of letting each widget
        // take its natural size: a natural-size row is what overflows the pane.
        let gap = ui.spacing().item_spacing.x;
        let idx_w = kit::ROW_H;
        let proto_w = 72.0;
        let port_w = 96.0;
        let rm_w = kit::ROW_H;
        let fixed = idx_w + proto_w + port_w + rm_w + 4.0 * gap;
        let payload_w = (ui.available_width() - fixed).max(48.0);

        // Only a UDP knock has a payload, so only a UDP row reserves the column.
        let any_udp = self.cfg.profile[i]
            .knocks
            .iter()
            .any(|k| k.proto == Proto::Udp);

        let mut remove = None;

        if any_udp {
            ui.horizontal(|ui| {
                ui.allocate_space(egui::vec2(idx_w, kit::ROW_H));
                ui.allocate_space(egui::vec2(proto_w, kit::ROW_H));
                ui.allocate_space(egui::vec2(port_w, kit::ROW_H));
                kit::label(ui, "Payload");
            });
        }

        for n in 0..self.cfg.profile[i].knocks.len() {
            let proto = self.cfg.profile[i].knocks[n].proto;
            ui.horizontal(|ui| {
                ui.add_sized(
                    egui::vec2(idx_w, kit::ROW_H),
                    egui::Label::new(
                        egui::RichText::new(format!("{}", n + 1))
                            .small()
                            .color(t.muted),
                    ),
                );

                egui::ComboBox::from_id_salt(("proto", i, n))
                    .selected_text(proto.label())
                    .width(proto_w)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.cfg.profile[i].knocks[n].proto,
                            Proto::Tcp,
                            "TCP",
                        );
                        ui.selectable_value(
                            &mut self.cfg.profile[i].knocks[n].proto,
                            Proto::Udp,
                            "UDP",
                        );
                    });

                let mut port_text = self.cfg.profile[i].knocks[n].port.to_string();
                if kit::input_w(ui, &mut port_text, "0", port_w).changed() {
                    // Anything unparseable means "no port", which `knock`
                    // already rejects with a readable message.
                    self.cfg.profile[i].knocks[n].port =
                        port_text.trim().parse::<u16>().unwrap_or(0);
                }

                if proto == Proto::Udp {
                    kit::input_w(
                        ui,
                        &mut self.cfg.profile[i].knocks[n].text,
                        "UDP only",
                        payload_w,
                    );
                } else {
                    ui.allocate_space(egui::vec2(payload_w, kit::ROW_H));
                }

                if kit::icon_button(ui, "Remove this knock", "\u{2715}").clicked() {
                    remove = Some(n);
                }
            });
            if remove.is_some() {
                break;
            }
        }

        if let Some(n) = remove {
            self.cfg.profile[i].knocks.remove(n);
        }

        ui.horizontal(|ui| {
            if kit::button(ui, Kind::Secondary, "+ Add knock").clicked() {
                self.cfg.profile[i].knocks.push(Knock {
                    proto: Proto::Tcp,
                    port: 0,
                    text: String::new(),
                });
            }
            kit::clipped(
                ui,
                egui::RichText::new("knocked top to bottom, with the global delay between")
                    .small()
                    .color(t.muted),
            );
        });
    }

    fn after_field(&mut self, ui: &mut egui::Ui, i: usize) {
        ui.add_space(kit::UNIT * 2.0);
        kit::section(ui, "After");
        let t = kit::tokens(ui);
        let mut text = self.cfg.profile[i].after.clone().unwrap_or_default();
        let inherited = self.cfg.after.clone().filter(|a| !a.trim().is_empty());
        let hint = match &inherited {
            Some(a) => format!("inherits: {a}"),
            None => "ssh root@host".to_string(),
        };
        let p = &mut self.cfg.profile[i];
        if kit::input(ui, &mut text, &hint).changed() {
            p.after = (!text.trim().is_empty()).then_some(text);
        }
        if inherited.is_some() {
            kit::clipped(
                ui,
                egui::RichText::new("blank falls back to the global command")
                    .small()
                    .color(t.muted),
            );
        }
    }

    fn actions(&mut self, ui: &mut egui::Ui) {
        kit::rule(ui);
        ui.add_space(kit::UNIT);

        let armed = self.arm_delete_until.is_some();
        ui.horizontal(|ui| {
            if kit::button(ui, Kind::Secondary, "Save")
                .on_hover_text("Write to config.toml  (Ctrl+S)")
                .clicked()
            {
                self.commit();
            }

            let width = kit::width(ui, 140.0);
            if self.busy {
                ui.allocate_space(egui::vec2(width, kit::ROW_H));
                kit::label(ui, "knocking…");
            } else if kit::button_wide(ui, Kind::Primary, "Knock now", width)
                .on_hover_text("Run the sequence  (Enter)")
                .clicked()
            {
                self.knock();
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let kind = if armed {
                    Kind::DangerSolid
                } else {
                    Kind::Danger
                };
                let label = if armed { "Confirm?" } else { "Delete" };
                if kit::button(ui, kind, label).clicked() {
                    if armed {
                        self.delete();
                    } else {
                        self.arm_delete_until = Some(Instant::now() + kit::CONFIRM_TTL);
                        self.say_timed("click again to delete", Tone::Idle, kit::CONFIRM_TTL);
                    }
                }
            });
        });

        // Keep repainting while the button is armed so it disarms itself.
        if armed {
            self.ctx.request_repaint_after(Duration::from_millis(250));
        }
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        let t = kit::tokens(ui);
        // The config path is the longest string in the app, so it gets a fixed
        // slot and the message truncates instead of pushing it off the edge.
        let path_w = 220.0;
        kit::split_row(
            ui,
            path_w,
            |ui| {
                let dot = match self.tone {
                    Tone::Idle => None,
                    Tone::Busy => Some(t.accent),
                    Tone::Ok => Some(Color32::from_rgb(22, 163, 74)),
                    Tone::Error => Some(t.danger),
                };
                if let Some(c) = dot {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 4.0, c);
                }
                let text = if self.status.is_empty() {
                    "Ready"
                } else {
                    self.status.as_str()
                };
                kit::clipped(
                    ui,
                    egui::RichText::new(text).color(match self.tone {
                        Tone::Idle => t.muted,
                        _ => t.text,
                    }),
                );
            },
            |ui| {
                let path = self.path.display().to_string();
                kit::clipped(ui, egui::RichText::new(path).small().color(t.muted))
                    .on_hover_text(format!("config: {}", self.path.display()));
            },
        );
    }

    /// `Enter` knocks only when no field has focus, so it cannot fire in the
    /// middle of typing. `Ctrl+S` always saves.
    fn shortcuts(&mut self, ctx: &egui::Context) {
        let enter = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
        let save = ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::S));
        let idle = ctx.memory(|m| m.focused()).is_none();
        if save {
            self.commit();
        }
        if enter && idle {
            self.knock();
        }
    }
    /// Whole window: polls results, lays out, then handles keys.
    ///
    /// Split from the [`eframe::App`] impl so a test can drive the layout with
    /// nothing but a [`egui::Ui`], at any size.
    pub fn render(&mut self, ui: &mut egui::Ui) {
        self.poll();
        self.shell(ui);
        self.shortcuts(ui.ctx());
        if self.busy {
            self.ctx.request_repaint();
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.render(ui);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Worst case the real app can be in: names and payloads longer than any
    /// column, a long `after`, a full knock list and a long status line.
    fn worst_case() -> App {
        let cfg = Config {
            delay_ms: 60_000,
            after: Some(
                "ssh -p 2222 verylongusername@a-fairly-long-hostname.example.internal".into(),
            ),
            theme: ThemePref::Dark,
            profile: vec![
                Profile {
                    desc: "A host with a really quite absurdly long descriptive name".into(),
                    host: "an-extremely-long-hostname.that-keeps-going.example.internal".into(),
                    knocks: (0..6)
                        .map(|i| Knock {
                            proto: if i % 2 == 0 { Proto::Tcp } else { Proto::Udp },
                            port: 65535,
                            text: "a-fairly-long-udp-payload-for-sure".into(),
                        })
                        .collect(),
                    after: Some(
                        "ssh -p 2222 averylongusername@alonghostname.example.internal".into(),
                    ),
                },
                Profile {
                    desc: "second".into(),
                    ..Profile::default()
                },
            ],
        };
        let ctx = egui::Context::default();
        crate::ui::install(&ctx);
        let (sender, events) = channel();
        App {
            path: PathBuf::from(
                r"C:\Users\somebody\AppData\Roaming\portknock\a-fairly-long-name.toml",
            ),
            cfg,
            selected: Some(0),
            busy: false,
            status:
                "knocked 6 ports on an-extremely-long-hostname.that-keeps-going.example.internal"
                    .into(),
            tone: Tone::Ok,
            clear_status_at: None,
            events,
            sender,
            ctx: ctx.clone(),
            arm_delete_until: None,
        }
    }

    /// Render one frame of `render` into a window of `screen.size()` and return
    /// the rightmost x that is laid out past the window edge.
    ///
    /// egui panels do not squash their contents: a widget wider than its pane
    /// keeps its natural width and is drawn off the edge, unreachable. Nothing
    /// is ever *visible* outside the window — the root clip rect stops it — so
    /// the only signal available here is geometry.
    ///
    /// Text is exempt when its own widget clip keeps it inside the window: a
    /// `TextEdit` lays its text out at full length and relies on clipping, and
    /// [`crate::ui::label`] truncates its galley. The field box itself is still
    /// measured, so a field wider than the window is caught.
    fn overflowing_max_x(
        ctx: &egui::Context,
        screen: egui::Rect,
        mut render: impl FnMut(&mut egui::Ui),
    ) -> f32 {
        const SLOP: f32 = 1.0;

        /// Right edge of `c` if it is laid out past `window_right`, else `None`.
        fn overflowing(c: &egui::epaint::ClippedShape, window_right: f32) -> Option<f32> {
            let right = match &c.shape {
                egui::Shape::Rect(r) => r.rect.right(),
                // One Text shape per run of text, positioned at `pos`.
                egui::Shape::Text(t) => t.pos.x + t.galley.rect.width(),
                egui::Shape::Mesh(m) => m
                    .vertices
                    .iter()
                    .filter(|v| c.clip_rect.contains(v.pos))
                    .map(|v| v.pos.x)
                    .fold(None, |a: Option<f32>, x| Some(a.map_or(x, |a| a.max(x))))?,
                _ => return None,
            };
            // Text whose own widget clip stays inside the window is fine: a
            // `TextEdit` lays its text out at full length and clips it.
            let clipped_in_window = matches!(c.shape, egui::Shape::Text(_))
                && c.clip_rect.right() <= window_right + SLOP;
            (right > window_right + SLOP && !clipped_in_window).then_some(right)
        }

        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ui| render(ui),
        );
        let window_right = screen.right();
        let max_x = out
            .shapes
            .iter()
            .filter_map(|c| overflowing(c, window_right))
            .filter(|x| x.is_finite())
            .fold(f32::MIN, f32::max);
        // No renderer here to receive the font atlas.
        out.textures_delta.clear();
        max_x
    }

    /// The toolbar is a fixed-height panel, so anything that does not fit in it
    /// is clipped and then painted over by the central panel — invisible, with
    /// no error anywhere. This is what a `split_row` in a vertical `Ui` looks
    /// like: the right slot lands on its own line, below the panel.
    ///
    /// Text is exempt because `TextEdit` lays text out at full length and lets
    /// its own box clip it; the box around it is checked.
    #[test]
    fn nothing_is_cut_off_in_the_toolbar() {
        for width in [480.0, 820.0, 1600.0] {
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 900.0));
            let bar =
                egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, App::TOOLBAR_H));
            let mut app = worst_case();
            let mut out = app.ctx.clone().run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ui| app.render(ui),
            );
            let cut: Vec<String> = out
                .shapes
                .iter()
                .filter(|c| c.clip_rect == bar)
                .filter_map(|c| match &c.shape {
                    egui::Shape::Rect(r) if !bar.contains_rect(r.rect) => {
                        Some(format!("{:?}", r.rect))
                    }
                    egui::Shape::Text(t) => {
                        let r = egui::Rect::from_min_size(
                            t.pos,
                            egui::vec2(t.galley.rect.width(), t.galley.rect.height()),
                        );
                        (!bar.contains_rect(r)).then(|| {
                            format!(
                                "{:?} {:?}",
                                r,
                                t.galley.job.text.chars().take(16).collect::<String>()
                            )
                        })
                    }
                    _ => None,
                })
                .collect();
            assert!(
                cut.is_empty(),
                "at width {width}: cut off by the toolbar: {cut:?}"
            );
            out.textures_delta.clear();
        }
    }

    #[test]
    fn nothing_is_laid_out_past_the_window_edge() {
        for width in [480.0, 560.0, 640.0, 820.0, 1100.0, 1600.0] {
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 900.0));
            let mut app = worst_case();
            let max_x = overflowing_max_x(&app.ctx.clone(), screen, |ui| app.render(ui));
            assert!(
                max_x <= width + 1.0,
                "at width {width}: content is laid out to x={max_x:.0}, past the window edge"
            );
        }
    }

    /// The measurement above has to be able to fail, otherwise the test above
    /// proves nothing. This is the pattern that used to break: a vertical
    /// scroll area whose content width is never pinned, so it is laid out at
    /// its natural width and hangs off the side of the window.
    #[test]
    fn the_overflow_check_can_actually_fail() {
        let width = 400.0;
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 900.0));
        let max_x = overflowing_max_x(&egui::Context::default(), screen, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("a host name far too long to ever fit in this window");
                        ui.label("and something else beside it");
                    });
                });
        });
        assert!(
            max_x > width + 1.0,
            "expected the unpinned scroll area to overflow, it reached only x={max_x:.0}"
        );
    }
}
