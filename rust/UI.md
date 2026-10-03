# UI design

How the GUI looks and why. Tokens live in `src/ui.rs`; the numbers below are
that file's constants, so this document and the code cannot drift far apart.

## Layout

Two panes, one task. The left pane picks a host, the right pane edits it.

```
┌──────────────────────────────────────────────┐
│ toolbar: title · delay ms · [theme]          │  56px, fixed top
├──────────────┬───────────────────────────────┤
│ + New host   │  HOST NAME                    │
│              │  ──────────────                │
│ ▍GregRocks 3 │  Host      [______________]   │
│  10.0.0.1    │  After cmd [______________]   │
│    then ssh  │                               │
│ ▍Bordinha  4 │  KNOCKS                       │
│  189.89.…    │  [tcp▾] [7151] [_____] [x]   │
│              │  [udp▾] [8899] [opensesame]…  │
│              │  + Add knock                   │
├──────────────┴───────────────────────────────┤
│ status message                        Ctrl+S │  28px, fixed bottom
└──────────────────────────────────────────────┘
```

- Above `NARROW` (640px) both panes are visible; the sidebar is `SIDEBAR_W`
  (264px) and the form scrolls independently in the rest.
- Below `NARROW` the sidebar is replaced by a single `ComboBox` above the form.
  Same data, same selection, one column — no horizontal scrolling, no hidden
  way to switch host.
- Selection is shared between the two, so resizing across the breakpoint never
  changes which host you are editing.
- The detail pane is empty-state-first: "No host selected" plus the reason, not
  a blank rectangle.

## Tokens

| Token | Value | Why |
| --- | --- | --- |
| `UNIT` | `4.0` | Every gap, padding and stroke is a multiple. Change this to re-scale the whole UI. |
| `ROW_H` | `32.0` | Height of every input, button and knock row. One row rhythm, so the form reads as a table. |
| `RADIUS` | `8` | Cards and inputs. |
| `RADIUS_SM` | `6` | Buttons and badges — one step tighter than the surface they sit on. |
| `ACCENT` | `#2563EB` | Brand blue, selection, primary action, focus ring. |
| `DANGER` | `#DC2626` | Irreversible actions only. |

Sizes and colours are not scattered: `Tokens` is resolved from the live theme
once per widget (`tokens(ui)`), so every component works in light and dark with
no `if dark` of its own.

## Button hierarchy

Five weights, and the row tells you what is expected of you:

| Kind | Look | Use for |
| --- | --- | --- |
| `Primary` | Filled accent | Exactly one per view: *Knock now*. The thing you came to do. |
| `Secondary` | Bordered neutral | Everything you might do: *Save*. |
| `Ghost` | Text only | Tertiary: row remove, theme toggle. |
| `Danger` | Red text, no fill until hover | *Delete*. |
| `DangerSolid` | Solid red | The armed state of `Danger`. |

`Danger` → `DangerSolid` → back is the whole delete flow, in place, in 3s. It is
two clicks and no modal, so it cannot become a habit of confirming dialogs.

## Theme

`ThemePref` is `system` (default), `dark` or `light`, stored in `config.toml`
next to the other settings rather than in eframe's storage — one file, one
format, no feature flag.

`ui::install` styles **both** themes once at startup, so `Context::set_theme`
just swaps between two prepared `Style`s. Nothing has to re-derive a colour
after the swap. The toolbar button shows which theme is active (`◐` system,
`☀` light, `☾` dark) and toggles through all three.

## Interaction rules

- **Knock** runs off the UI thread; while it runs, the button is replaced by a
  `knocking…` label at the same width so the toolbar does not jump.
- **Status bar** is the only place results appear, coloured by tone (idle, ok,
  warn, err) *and* prefixed with a word (`knocked`, `saved`, `failed`), so tone
  is never the only signal.
- **Shortcuts**: `Enter` knocks, `Ctrl+S` saves. `Enter` is suppressed while a
  text field has focus, so typing a host does not fire a knock. Every shortcut
  is also written in the hover text of the button it duplicates.
- **Empty host** falls back to `no host set` in the sidebar rather than
  rendering an empty line.

## Accessibility

- `ACCENT` on white is **5.2:1** and `DANGER` on white is **4.8:1**, both above
  WCAG AA for normal text. The values are recorded here so nobody "brightens"
  them without re-checking.
- Focus is drawn as a 1.5px accent outline on the focused button, independent of
  fill, so it is visible on every weight including `Ghost`.
- Disabled controls fade toward the panel background instead of only dimming
  the text.
- Sidebar labels truncate with an ellipsis and keep their full value in a hover
  tooltip; long hosts never stretch the layout.
- Nothing depends on colour alone: selection is a fill *and* a border, and
  status is a tone *and* a word.

## Why these are hand-rolled

egui is immediate mode and ships primitives, not a component library. Three
places needed real work rather than configuration:

- `Button::fill` pins one colour across all widget states, so a filled button
  has no hover feedback. `ui.rs` allocates, interacts, then picks the fill from
  the response — the only way to get idle/hover/press from one widget.
- `TextEdit`'s default frame is `Frame::new()`, so `install`'s rounded corners
  never reach a text field. Every field passes its own frame.
- `egui_extras` (and its `StripBuilder`) is not a dependency, so responsive
  columns are explicit `Layout` switches rather than declarative flex.

Keep that in mind before reaching for a new dependency: most of what a design
system needs here is `allocate_exact_size`, `interact`, and `Painter`.

## Keeping a window narrow

egui does not squash a widget that is wider than its container. It lays the
widget out at its natural width and paints it off the edge of the window, where
it cannot be clicked. It also never reserves space: a widget keeps its natural
size and pushes whatever follows it. Five rules keep every layout inside the
frame, and they only work together:

- **Pin the scroll area's content width.** `ScrollArea::vertical()` with
  `auto_shrink([false, false])` builds its inner `Ui` with no width limit, so
  everything inside is laid out at its natural width. `.show(ui, |ui| {
  ui.set_max_width(ui.available_width()); .. })` is what keeps the pane honest.
- **Reserve fixed widths with `allocate_exact_size`, not
  `allocate_ui_with_layout`.** The latter builds a child of the requested *max*
  size but returns a content-sized rect and advances the cursor by that, so the
  requested width is silently dropped. `split_row` reserves both slots this way
  and hands each child a `Ui` of exactly that rect.
- **Build rows inside `split_row`, not at the call site.** Every panel and
  scroll area in this app is a vertical `Ui`, where two `allocate_*` calls
  stack *vertically* — the right slot drops onto its own line, outside the
  panel, where it is clipped and then painted over by the panel below. No error,
  no warning, just a control that vanished.
- **Cap text fields with `add_sized`, not `desired_width`.** A `TextEdit` grows
  to fit its text: `desired_width(200)` on a long payload gives a 285px field.
  `add_sized` fixes the box and lets the text scroll inside it.
- **Truncate with `clipped`**, never `ui.label`, for anything user-supplied.

`app.rs` checks this headlessly rather than by eye. `nothing_is_laid_out_past_the_window_edge`
renders a worst-case window (long host names, long payloads, many rows) at six
widths from 480px up and fails if anything is laid out past the edge. Text is
exempt when its own widget clip stays inside the window, since `TextEdit` always
lays text out at full length and relies on clipping.
`nothing_is_cut_off_in_the_toolbar` renders the same window and fails if anything
is laid out past the toolbar's fixed height — the symptom of the vertical-stack
bug above, which is invisible because the content is simply gone.
`the_overflow_check_can_actually_fail` lays out the unpinned scroll area that
started all this, to prove the first check can still fail.

## Adding a widget

1. Take colours from `Tokens`, never from a literal, unless it is `ACCENT` or
   `DANGER`.
2. Space it in `UNIT` multiples and size it to `ROW_H` if it belongs in a form.
3. Reuse `clipped` for any text that can be longer than its pane, and
   `split_row` for anything that has to stay pinned to the right of it.
4. Give it a `Kind` that matches what it does to the user's data. If it does not
   fit the five weights, that is a sign it may not belong on that row.
