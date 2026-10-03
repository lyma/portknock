//! Design tokens and widgets. See `UI.md` for why they look the way they do.

use std::sync::Arc;
use std::time::Duration;

use eframe::egui::{
    pos2, vec2, Align, Color32, Context, CornerRadius, FontId, Frame, Galley, Label, Layout,
    Margin, Rect, Response, RichText, Sense, Stroke, StrokeKind, TextEdit, Theme, Ui, UiBuilder,
};

/// Base unit of the spacing scale. Every gap below is a multiple of it.
pub const UNIT: f32 = 4.0;
/// Corner radius of cards and inputs.
pub const RADIUS: u8 = 8;
/// Corner radius of buttons and badges. One step smaller than `RADIUS`.
pub const RADIUS_SM: u8 = 6;
/// Height of inputs, buttons and knock rows.
pub const ROW_H: f32 = 32.0;
/// Width reserved for a right-pinned badge. Fixed, so the label beside it can
/// budget its own width instead of guessing: that reservation is what keeps the
/// row inside the window when the label is long.
pub const BADGE_SLOT: f32 = 76.0;
/// Inner margin of a card, horizontally.
const CARD_PAD_X: i8 = 16;
/// Inner margin of a card, vertically.
const CARD_PAD_Y: i8 = 12;
/// Width of the host sidebar.
pub const SIDEBAR_W: f32 = 264.0;
/// Below this width the sidebar collapses into a selector above the form.
pub const NARROW: f32 = 640.0;
/// How long `Delete` stays armed as `Confirm?` before disarming itself.
pub const CONFIRM_TTL: Duration = Duration::from_secs(3);

/// Brand blue. Dark enough for white text at AA contrast (5.1:1).
const ACCENT: Color32 = Color32::from_rgb(37, 99, 235);
/// Destructive red, same contrast rules as `ACCENT`.
const DANGER: Color32 = Color32::from_rgb(220, 38, 38);
/// Font size of buttons and inputs.
const FS_BUTTON: f32 = 14.0;
/// Font size of badges.
const FS_BADGE: f32 = 11.0;

/// Blend `a` toward `b`. `t` of 0.0 gives `a`.
pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let [ar, ag, ab, aa] = a.to_array();
    let [br, bg, bb, ba] = b.to_array();
    let ch = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(ch(ar, br), ch(ag, bg), ch(ab, bb), ch(aa, ba))
}

/// Colours resolved against the live theme, so every widget in this module
/// works in light and dark without a branch of its own.
pub struct Tokens {
    pub accent: Color32,
    pub on_accent: Color32,
    pub danger: Color32,
    /// Raised surface: a card sitting above the panel background.
    pub card: Color32,
    pub card_hover: Color32,
    pub card_selected: Color32,
    pub card_border: Color32,
    pub field: Color32,
    pub field_border: Color32,
    pub muted: Color32,
    pub text: Color32,
}

pub fn tokens(ui: &Ui) -> Tokens {
    let v = ui.visuals();
    // Dark panels want a *lighter* raised surface, light panels want white.
    let card = if v.dark_mode {
        mix(v.panel_fill, Color32::WHITE, 0.06)
    } else {
        Color32::WHITE
    };
    Tokens {
        accent: ACCENT,
        on_accent: Color32::WHITE,
        danger: DANGER,
        card,
        card_hover: mix(card, v.text_color(), 0.07),
        card_selected: mix(card, ACCENT, 0.14),
        card_border: v.widgets.inactive.bg_stroke.color,
        field: v.extreme_bg_color,
        field_border: v.widgets.inactive.bg_stroke.color,
        muted: v.weak_text_color(),
        text: v.text_color(),
    }
}

/// Round the stock egui widgets and open up the spacing so built-ins sit in
/// the same rhythm as the hand-painted ones.
///
/// Applied to *both* themes, so `Context::set_theme` just swaps between two
/// prepared styles and never has to be followed by another call.
pub fn install(ctx: &Context) {
    for theme in [Theme::Dark, Theme::Light] {
        let mut style = (*ctx.style_of(theme)).clone();
        style.spacing.item_spacing = vec2(UNIT * 2.0, UNIT * 2.0);
        style.spacing.button_padding = vec2(UNIT * 3.0, UNIT * 1.75);
        style.spacing.scroll.bar_width = UNIT * 2.0;
        for w in [
            &mut style.visuals.widgets.noninteractive,
            &mut style.visuals.widgets.inactive,
            &mut style.visuals.widgets.hovered,
            &mut style.visuals.widgets.active,
            &mut style.visuals.widgets.open,
        ] {
            w.corner_radius = RADIUS.into();
        }
        ctx.set_style_of(theme, style);
    }
}

/// Visual weight of a button. One row may mix any of these.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Filled accent. Exactly one per view: the thing you came to do.
    Primary,
    /// Bordered neutral. Everything you might do.
    Secondary,
    /// Text only. Tertiary actions.
    Ghost,
    /// Text in red, no fill until hovered. Irreversible actions.
    Danger,
    /// Solid red. The armed state of a `Danger` button, so that "are you
    /// sure?" cannot be missed.
    DangerSolid,
}

struct Palette {
    base: Option<Color32>,
    hover: Option<Color32>,
    press: Option<Color32>,
    fg: Color32,
    outline: Option<Stroke>,
}

impl Kind {
    fn palette(self, t: &Tokens, panel: Color32) -> Palette {
        match self {
            Kind::Primary => Palette {
                base: Some(t.accent),
                hover: Some(mix(t.accent, Color32::WHITE, 0.12)),
                press: Some(mix(t.accent, Color32::BLACK, 0.14)),
                fg: t.on_accent,
                outline: None,
            },
            Kind::Secondary => Palette {
                base: Some(t.card),
                hover: Some(mix(t.card, t.text, 0.08)),
                press: Some(mix(t.card, t.text, 0.14)),
                fg: t.text,
                outline: Some(Stroke::new(1.0, t.card_border)),
            },
            Kind::Ghost => Palette {
                base: None,
                hover: Some(mix(panel, t.muted, 0.22)),
                press: Some(mix(panel, t.muted, 0.34)),
                fg: t.muted,
                outline: None,
            },
            Kind::Danger => Palette {
                base: None,
                hover: Some(mix(panel, t.danger, 0.20)),
                press: Some(mix(panel, t.danger, 0.32)),
                fg: t.danger,
                outline: None,
            },
            Kind::DangerSolid => Palette {
                base: Some(t.danger),
                hover: Some(mix(t.danger, Color32::WHITE, 0.14)),
                press: Some(mix(t.danger, Color32::BLACK, 0.14)),
                fg: t.on_accent,
                outline: None,
            },
        }
    }
}

fn galley(ui: &Ui, label: &str, size: f32, color: Color32) -> Arc<Galley> {
    ui.painter()
        .layout_no_wrap(label.to_owned(), FontId::proportional(size), color)
}

/// Paint `g` centred in `rect` using the fill the interaction implies.
///
/// egui's own `Button::fill` pins one colour across every widget state, so a
/// filled button gets no hover feedback. We allocate, interact, then pick the
/// fill from the response, which is the only way to get all three states.
fn paint(ui: &Ui, rect: Rect, g: &Arc<Galley>, kind: Kind, resp: &Response) {
    let t = tokens(ui);
    let p = kind.palette(&t, ui.visuals().panel_fill);
    let enabled = ui.is_enabled();

    let fill = if !enabled {
        p.base.map(|c| mix(c, ui.visuals().panel_fill, 0.55))
    } else if resp.is_pointer_button_down_on() {
        p.press.or(p.base).or(p.hover)
    } else if resp.hovered() {
        p.hover.or(p.base)
    } else {
        p.base
    };
    if let Some(c) = fill {
        ui.painter()
            .rect_filled(rect, CornerRadius::from(RADIUS_SM), c);
    }

    let outline = if resp.has_focus() && enabled {
        Some(Stroke::new(1.5, t.accent))
    } else {
        p.outline
    };
    if let Some(s) = outline {
        ui.painter()
            .rect_stroke(rect, CornerRadius::from(RADIUS_SM), s, StrokeKind::Inside);
    }

    let fg = if enabled {
        p.fg
    } else {
        mix(p.fg, ui.visuals().panel_fill, 0.45)
    };
    let size = g.size();
    ui.painter().galley(
        pos2(
            rect.center().x - size.x / 2.0,
            rect.center().y - size.y / 2.0,
        ),
        g.clone(),
        fg,
    );
}

/// Button sized to its label.
pub fn button(ui: &mut Ui, kind: Kind, label: impl AsRef<str>) -> Response {
    button_impl(ui, kind, label.as_ref(), None)
}

/// Button stretched to `width`. Used for the primary action so it reads as the
/// end of a toolbar rather than another item in it.
pub fn button_wide(ui: &mut Ui, kind: Kind, label: impl AsRef<str>, width: f32) -> Response {
    button_impl(ui, kind, label.as_ref(), Some(width))
}

fn button_impl(ui: &mut Ui, kind: Kind, label: &str, min_w: Option<f32>) -> Response {
    let t = tokens(ui);
    let p = kind.palette(&t, ui.visuals().panel_fill);
    let enabled = ui.is_enabled();
    let fg = if enabled {
        p.fg
    } else {
        mix(p.fg, ui.visuals().panel_fill, 0.45)
    };
    let g = galley(ui, label, FS_BUTTON, fg);
    let natural = g.size().x + UNIT * 6.0;
    let (rect, resp) = ui.allocate_exact_size(
        vec2(min_w.unwrap_or(natural).max(natural), ROW_H),
        Sense::click(),
    );
    paint(ui, rect, &g, kind, &resp);
    resp
}

/// Square button holding one glyph. `tip` is the hover text.
pub fn icon_button(ui: &mut Ui, tip: &str, glyph: &str) -> Response {
    let t = tokens(ui);
    let g = galley(ui, glyph, FS_BADGE + 2.0, t.muted);
    let (rect, resp) = ui.allocate_exact_size(vec2(ROW_H, ROW_H), Sense::click());
    paint(ui, rect, &g, Kind::Ghost, &resp);
    resp.on_hover_text(tip)
}

/// A full-width clickable surface. `add` paints the contents; the card picks
/// its fill from the pointer afterwards, so hover works without knowing the
/// content height up front.
pub fn card(ui: &mut Ui, selected: bool, add: impl FnOnce(&mut Ui)) -> Response {
    let t = tokens(ui);
    let mut p = Frame::new()
        .corner_radius(RADIUS)
        .inner_margin(Margin::symmetric(CARD_PAD_X, CARD_PAD_Y))
        .begin(ui);

    let id = p.content_ui.next_auto_id();
    let hit = p
        .content_ui
        .interact(p.content_ui.max_rect(), id, Sense::click());
    add(&mut p.content_ui);

    let hovered = hit.hovered() && ui.is_enabled();
    p.frame = p
        .frame
        .fill(if selected {
            t.card_selected
        } else if hovered {
            t.card_hover
        } else {
            t.card
        })
        .stroke(if selected {
            Stroke::new(1.5, t.accent)
        } else {
            Stroke::new(1.0, t.card_border)
        });
    p.allocate_space(ui);
    p.paint(ui);
    hit
}

/// Row split into a flexible left part and a fixed-width right part.
///
/// egui rows never shrink and never clip, so a long label on the left pushes
/// whatever follows it straight out of the window. Budgeting the fixed part
/// first is the only way to keep a row inside its container: the left side is
/// laid out in a box of exactly the width that is left over — so it still has to
/// truncate its own text, see [`label`].
///
/// The width is reserved with `allocate_exact_size`, not
/// `allocate_ui_with_layout`: the latter builds a child of the requested *max*
/// size but returns a content-sized rect and advances the cursor by that, so
/// nothing is actually reserved and the right slot lands next to the left text
/// instead of at the far edge.
pub fn split_row(
    ui: &mut Ui,
    right_w: f32,
    left: impl FnOnce(&mut Ui),
    right: impl FnOnce(&mut Ui),
) {
    let gap = ui.spacing().item_spacing.x;
    let left_w = (ui.available_width() - right_w - gap).max(48.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        let (left_rect, _) = ui.allocate_exact_size(vec2(left_w, ROW_H), Sense::hover());
        let (right_rect, _) = ui.allocate_exact_size(vec2(right_w, ROW_H), Sense::hover());
        left(
            &mut ui.new_child(
                UiBuilder::new()
                    .max_rect(left_rect)
                    .layout(Layout::left_to_right(Align::Center)),
            ),
        );
        right(
            &mut ui.new_child(
                UiBuilder::new()
                    .max_rect(right_rect)
                    .layout(Layout::right_to_left(Align::Center)),
            ),
        );
    });
}

/// Right-aligned small rounded label, used for counts and state.
///
/// Pair it with [`split_row`]: the row reserves `slot_w` first, so the label
/// can never be pushed past the window edge by whatever is on the left.
pub fn badge_right(ui: &mut Ui, slot_w: f32, text: &str, fg: Color32, bg: Color32) {
    let (slot, _) = ui.allocate_exact_size(vec2(slot_w, ROW_H), Sense::hover());
    let g = galley(ui, text, FS_BADGE, fg);
    let pill = vec2(g.size().x + UNIT * 2.0, g.size().y + UNIT);
    let r = Rect::from_min_size(
        pos2(slot.right() - pill.x, slot.center().y - pill.y / 2.0),
        pill,
    );
    ui.painter()
        .rect_filled(r, CornerRadius::from((pill.y / 2.0).round() as u8), bg);
    let s = g.size();
    ui.painter().galley(
        pos2(r.center().x - s.x / 2.0, r.center().y - s.y / 2.0),
        g,
        fg,
    );
}

/// Uppercase section heading with a hairline under it.
pub fn section(ui: &mut Ui, text: &str) {
    let t = tokens(ui);
    ui.label(
        RichText::new(text.to_uppercase())
            .small()
            .strong()
            .color(t.muted),
    );
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, 0.0, mix(t.card_border, t.muted, 0.25));
}

/// Caption above a widget. Truncates rather than widening its container.
pub fn label(ui: &mut Ui, text: &str) {
    let t = tokens(ui);
    clipped(ui, RichText::new(text).small().color(t.muted));
}

/// Single-line text field: filled, rounded, stretched to the available width.
///
/// egui's default `TextEdit` frame is bare (`Frame::new()`), so `install`'s
/// rounded corners never reach it. Every field has to pass its own frame.
pub fn input(ui: &mut Ui, text: &mut String, hint: &str) -> Response {
    input_w(ui, text, hint, f32::INFINITY)
}

/// As [`input`], but capped at `width`.
///
/// `add_sized`, not `desired_width`: a `TextEdit` grows to fit its text, so a
/// long payload widens the field and pushes the rest of the row off screen.
/// A sized box clips the text instead and lets it scroll inside the field.
pub fn input_w(ui: &mut Ui, text: &mut String, hint: &str, width: f32) -> Response {
    let t = tokens(ui);
    let field = TextEdit::singleline(text)
        .hint_text(RichText::new(hint).weak())
        .desired_width(width)
        .desired_rows(1)
        .frame(
            Frame::new()
                .fill(t.field)
                .stroke(Stroke::new(1.0, t.field_border))
                .corner_radius(RADIUS_SM)
                .inner_margin(Margin::symmetric(UNIT as i8 * 2, UNIT as i8)),
        );
    ui.add_sized(
        vec2(
            if width.is_finite() {
                width
            } else {
                ui.available_width()
            },
            ROW_H,
        ),
        field,
    )
}

/// One-line text that shortens with an ellipsis instead of overflowing.
pub fn clipped(ui: &mut Ui, text: RichText) -> Response {
    ui.add(Label::new(text).truncate().show_tooltip_when_elided(true))
}

/// Empty state: a short explanation plus the one action that resolves it.
pub fn empty_state(ui: &mut Ui, title: &str, body: &str) {
    let t = tokens(ui);
    ui.add_space(UNIT * 6.0);
    ui.vertical_centered(|ui| {
        clipped(ui, RichText::new(title).size(18.0).strong());
        clipped(ui, RichText::new(body).color(t.muted));
        ui.add_space(UNIT);
    });
}

/// Available width, clamped: containers can report infinity.
pub fn width(ui: &Ui, max: f32) -> f32 {
    let w = ui.available_width();
    if w.is_finite() {
        w.min(max)
    } else {
        max
    }
}

/// A 1px horizontal rule.
pub fn rule(ui: &mut Ui) {
    let t = tokens(ui);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, 0.0, mix(t.card_border, t.muted, 0.25));
}
