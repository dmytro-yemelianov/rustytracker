# Tracker UI Look And Feel

## Purpose

RustyTracker should keep its Rust-native engine architecture while moving the
desktop UI toward the dense, pixel-oriented tracker feel of MilkyTracker. The
goal is visual and interaction compatibility where it matters to tracker use:
pattern editing, row/channel scanning, palette semantics, keyboard cursor
behavior, instrument/sample lists, and compact controls.

This is not a direct port of MilkyTracker's PPUI/Cocoa/OpenGL architecture.
RustyTracker should keep `egui`/`eframe` as the application shell until there is
a concrete performance or platform reason to replace it.

## Current UI Problem

The current `rustytracker-ui` crate renders the tracker as standard egui
widgets:

- a top menu and control bar
- side panels for order and instrument lists
- an `egui::Grid` with one widget per pattern cell
- egui groups, sliders, text fields, and combo boxes for instrument/sample
  editing

That is useful for bootstrapping, but it cannot match MilkyTracker's look
because the important surfaces are controlled by egui defaults: anti-aliased
system fonts, default spacing, default selection painting, generic widgets, and
ad hoc colors.

## Design Goals

- Keep domain behavior in `core`, `edit`, `play`, and format crates.
- Keep `eframe` for the native window, event loop, menus, file dialogs, and
  integration surfaces.
- Render tracker editor surfaces as custom-painted widgets instead of large
  grids of generic egui controls.
- Use semantic theme values named after tracker concepts: pattern note,
  instrument, effect, operand, cursor, cursor line, row highlights, selection,
  theme background, list background, waveform.
- Use fixed tracker metrics for rows, columns, channel cells, and cursor
  fields.
- Draw only visible rows/channels for large modules.
- Make pointer hit testing the inverse of layout math.
- Keep all compatibility values behind named constants.
- Add behavior in testable slices where logic can be separated from egui
  painting.

## Non-Goals

- Do not copy PPUI class structure into Rust.
- Do not rewrite the whole UI with raw `wgpu` until egui is proven to be the
  limiting factor.
- Do not mix tracker rendering decisions into parser/playback/editor crates.
- Do not start with a pixel-perfect clone of every MilkyTracker dialog.

## Target Architecture

```text
rustytracker-core / edit / play / xm / mod
        |
rustytracker-ui app state
        |
tracker_ui module
  - TrackerTheme
  - TrackerMetrics
  - custom pattern renderer
  - custom list/control renderers
  - bitmap font atlas
        |
egui / eframe shell
```

## Rendering Model

The pattern editor is the first surface to replace.

The renderer owns:

- desired content size
- visible row/channel range
- row highlight rectangles
- cursor line and active field rectangle
- per-field text colors
- pointer-to-cell hit testing

The renderer does not own:

- module mutation
- playback state
- edit command semantics
- file I/O

## Theme Model

Initial theme values should mirror MilkyTracker's default palette naming even
when exact color matching is refined later:

- `pattern_background`
- `pattern_note`
- `pattern_instrument`
- `pattern_volume`
- `pattern_effect`
- `pattern_operand`
- `theme_background`
- `foreground`
- `muted_foreground`
- `cursor`
- `cursor_line`
- `cursor_line_highlight`
- `selection`
- `row_highlight_primary`
- `row_highlight_secondary`
- `channel_header`
- `border`

## Font Plan

Phase 1 uses `egui::FontId::monospace` with fixed tracker metrics to get the
layout and palette model in place.

Phase 2 adds a bitmap font atlas:

- include an 8x8-compatible tracker font asset or generated atlas
- render glyphs by character cell instead of using system font shaping
- keep nearest-neighbor texture filtering
- expose tiny/system/large size choices as tracker metrics

## Phases

### Phase 1: Pattern Surface Foundation

- [x] Write this specification.
- [x] Add `tracker_ui` module.
- [x] Add `TrackerTheme` and `TrackerMetrics`.
- [x] Replace `egui::Grid` pattern editor with one custom-painted widget.
- [x] Draw visible rows only.
- [x] Draw semantic pattern text colors.
- [x] Draw active row, active channel, and active field.
- [x] Map pointer clicks back to row/channel.

### Phase 2: Palette And Settings

- [x] Route custom tracker surfaces through shared UI theme/metric resources.
- [x] Add Milky-style preset palettes.
- [x] Replace scattered `Color32::*` use in tracker-painted surfaces.
- [x] Store selected tracker palette in UI settings.
- [x] Apply theme to order/instrument lists.

### Phase 3: Bitmap Font

- [x] Add bitmap font asset/atlas.
- [x] Render pattern text through the atlas.
- [ ] Add tiny/system/large font metrics.
- [ ] Add tests for text layout and click hit testing.

### Phase 4: Tracker Controls

- [x] Add custom-painted list rows.
- [x] Add tracker-style transport buttons, order actions, and view tabs.
- [x] Add tracker-painted instrument/sample panels.
- [x] Render sample waveform through the tracker painter.
- [x] Replace envelope toggles and sample loop mode with tracker controls.
- [ ] Add tracker-style scrollbars where egui defaults visibly clash.
- [ ] Replace remaining instrument/sample editor text fields, sliders, and numeric steppers.

### Phase 5: Interaction Parity

- [ ] Match Milky cursor sub-field navigation.
- [ ] Match selection rectangle behavior.
- [ ] Match row centering and follow-playhead behavior.
- [ ] Add editing affordances that do not depend on egui widget focus.

## Acceptance

- Pattern editor no longer allocates one egui widget per cell.
- Pattern rendering uses semantic theme values, not ad hoc color literals.
- Pattern layout is fixed-metric and hit testing is deterministic.
- Large channel counts remain responsive while scrolling and playing.
- Future bitmap font work can reuse the same layout and painter structure.
