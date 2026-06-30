use eframe::egui;
use egui::{Color32, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2};
use rustytracker_core::{EffectCommand, Note, Pattern, PatternCell, SampleData, SampleLoopKind};

use crate::app::ActiveField;

const SYSTEM_FONT_BYTES: &[u8] = include_bytes!("../assets/fonts/J-FLUX.8X8");
const FONT_GLYPH_COUNT: usize = 256;
const FONT_GLYPH_WIDTH: usize = 8;
const FONT_GLYPH_HEIGHT: usize = 8;
const FONT_GLYPH_BYTES: usize = FONT_GLYPH_HEIGHT;
const FONT_REQUIRED_BYTES: usize = FONT_GLYPH_COUNT * FONT_GLYPH_BYTES;
const FONT_BIT_MASK_START: u8 = 0x80;
const FONT_ATLAS_COLUMNS: usize = 16;
const FONT_ATLAS_ROWS: usize = 16;
const FONT_ATLAS_WIDTH: usize = FONT_ATLAS_COLUMNS * FONT_GLYPH_WIDTH;
const FONT_ATLAS_HEIGHT: usize = FONT_ATLAS_ROWS * FONT_GLYPH_HEIGHT;

const NOTE_NAMES: [&str; 12] = [
    "C-", "C#", "D-", "D#", "E-", "F-", "F#", "G-", "G#", "A-", "A#", "B-",
];

const ROW_NUMBER_COLUMNS: usize = 3;
const ROW_GUTTER_COLUMNS: usize = 2;
const NOTE_FIELD_COLUMNS: usize = 4;
const INSTRUMENT_FIELD_COLUMNS: usize = 2;
const EFFECT_FIELD_COLUMNS: usize = 3;
const FIELD_GAP_COLUMNS: usize = 1;
const CHANNEL_GAP_COLUMNS: usize = 2;
const PRIMARY_HIGHLIGHT_SPACING: u16 = 16;
const SECONDARY_HIGHLIGHT_SPACING: u16 = 4;
const ROW_LABEL_OFFSET_Y: f32 = 2.0;
const CHANNEL_HEADER_OFFSET_Y: f32 = 3.0;
const CELL_TEXT_OFFSET_Y: f32 = 2.0;
const LIST_TEXT_OFFSET_X: f32 = 4.0;
const LIST_TEXT_OFFSET_Y: f32 = 3.0;
const CONTROL_HORIZONTAL_PADDING: f32 = 8.0;
const CONTROL_TEXT_OFFSET_Y: f32 = 5.0;
const CONTROL_EXTRA_HEIGHT: f32 = 6.0;
const CONTROL_MIN_WIDTH: f32 = 48.0;
const STATUS_HORIZONTAL_PADDING: f32 = 6.0;
const TOOLBAR_SEPARATOR_WIDTH: f32 = 8.0;
const TOOLBAR_SEPARATOR_INSET_Y: f32 = 3.0;
const WAVEFORM_HEIGHT: f32 = 120.0;
const WAVEFORM_MIN_WIDTH: f32 = 240.0;
const WAVEFORM_EDGE_PADDING: f32 = 5.0;
const WAVEFORM_STROKE_WIDTH: f32 = 1.5;
const PCM8_NORMALIZATION_FACTOR: f32 = 128.0;
const PCM16_NORMALIZATION_FACTOR: f32 = 32768.0;
const NO_WAVEFORM_TEXT: &str = "NO AUDIO WAVEFORM DATA";
const CURSOR_BORDER_WIDTH: f32 = 1.0;

const NOTE_FIELD_COLUMN: usize = 0;
const INSTRUMENT_FIELD_COLUMN: usize = NOTE_FIELD_COLUMNS + FIELD_GAP_COLUMNS;
const EFFECT0_FIELD_COLUMN: usize =
    INSTRUMENT_FIELD_COLUMN + INSTRUMENT_FIELD_COLUMNS + FIELD_GAP_COLUMNS;
const EFFECT1_FIELD_COLUMN: usize = EFFECT0_FIELD_COLUMN + EFFECT_FIELD_COLUMNS + FIELD_GAP_COLUMNS;
const CELL_TEXT_COLUMNS: usize = EFFECT1_FIELD_COLUMN + EFFECT_FIELD_COLUMNS;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TrackerPalette {
    #[default]
    MilkyDefault,
    MilkyWarm,
    HighContrast,
}

impl TrackerPalette {
    pub const ALL: [Self; 3] = [Self::MilkyDefault, Self::MilkyWarm, Self::HighContrast];

    pub const fn label(self) -> &'static str {
        match self {
            Self::MilkyDefault => "Milky Default",
            Self::MilkyWarm => "Milky Warm",
            Self::HighContrast => "High Contrast",
        }
    }

    pub const fn theme(self) -> TrackerTheme {
        match self {
            Self::MilkyDefault => TrackerTheme::milky_default(),
            Self::MilkyWarm => TrackerTheme::milky_warm(),
            Self::HighContrast => TrackerTheme::high_contrast(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TrackerTheme {
    pub pattern_background: Color32,
    pub pattern_note: Color32,
    pub pattern_instrument: Color32,
    pub pattern_effect: Color32,
    pub pattern_operand: Color32,
    pub theme_background: Color32,
    pub foreground: Color32,
    pub muted_foreground: Color32,
    pub cursor: Color32,
    pub cursor_line: Color32,
    pub cursor_line_highlight: Color32,
    pub selection: Color32,
    pub row_highlight_primary: Color32,
    pub row_highlight_secondary: Color32,
    pub channel_header: Color32,
    pub border: Color32,
    pub waveform: Color32,
    pub waveform_center: Color32,
    pub waveform_loop: Color32,
    pub waveform_loop_edge: Color32,
}

impl TrackerTheme {
    pub const fn milky_default() -> Self {
        Self {
            pattern_background: Color32::from_rgb(0x16, 0x1b, 0x1d),
            pattern_note: Color32::from_rgb(0xff, 0xff, 0xff),
            pattern_instrument: Color32::from_rgb(0x93, 0x93, 0xff),
            pattern_effect: Color32::from_rgb(0xff, 0xff, 0xff),
            pattern_operand: Color32::from_rgb(0x7f, 0x7f, 0x80),
            theme_background: Color32::from_rgb(0x20, 0x28, 0x29),
            foreground: Color32::from_rgb(0xa7, 0xa7, 0xa7),
            muted_foreground: Color32::from_rgb(0x5d, 0x64, 0x6b),
            cursor: Color32::from_rgb(0x64, 0x72, 0x78),
            cursor_line: Color32::from_rgb(0x2e, 0x35, 0x3c),
            cursor_line_highlight: Color32::from_rgb(0xa0, 0x18, 0x30),
            selection: Color32::from_rgb(0x10, 0x30, 0x60),
            row_highlight_primary: Color32::from_rgb(0x20, 0x21, 0x20),
            row_highlight_secondary: Color32::from_rgb(0x10, 0x10, 0x00),
            channel_header: Color32::from_rgb(0x49, 0x57, 0x6b),
            border: Color32::from_rgb(0x28, 0x43, 0x6b),
            waveform: Color32::from_rgb(0x93, 0x93, 0xff),
            waveform_center: Color32::from_rgb(0x39, 0x42, 0x47),
            waveform_loop: Color32::from_rgba_premultiplied(0x10, 0x30, 0x60, 0x40),
            waveform_loop_edge: Color32::from_rgb(0x49, 0x57, 0x6b),
        }
    }

    pub const fn milky_warm() -> Self {
        Self {
            pattern_background: Color32::from_rgb(0x19, 0x18, 0x15),
            pattern_note: Color32::from_rgb(0xff, 0xf6, 0xd6),
            pattern_instrument: Color32::from_rgb(0xff, 0xb8, 0x6b),
            pattern_effect: Color32::from_rgb(0xf2, 0xe8, 0xc8),
            pattern_operand: Color32::from_rgb(0xa6, 0x91, 0x70),
            theme_background: Color32::from_rgb(0x25, 0x23, 0x1e),
            foreground: Color32::from_rgb(0xd0, 0xc4, 0xa8),
            muted_foreground: Color32::from_rgb(0x77, 0x70, 0x62),
            cursor: Color32::from_rgb(0x78, 0x64, 0x48),
            cursor_line: Color32::from_rgb(0x39, 0x32, 0x29),
            cursor_line_highlight: Color32::from_rgb(0x90, 0x28, 0x30),
            selection: Color32::from_rgb(0x55, 0x38, 0x20),
            row_highlight_primary: Color32::from_rgb(0x24, 0x22, 0x1c),
            row_highlight_secondary: Color32::from_rgb(0x16, 0x14, 0x0f),
            channel_header: Color32::from_rgb(0x60, 0x50, 0x38),
            border: Color32::from_rgb(0x68, 0x54, 0x34),
            waveform: Color32::from_rgb(0xff, 0xb8, 0x6b),
            waveform_center: Color32::from_rgb(0x48, 0x40, 0x34),
            waveform_loop: Color32::from_rgba_premultiplied(0x55, 0x38, 0x20, 0x50),
            waveform_loop_edge: Color32::from_rgb(0x96, 0x78, 0x4a),
        }
    }

    pub const fn high_contrast() -> Self {
        Self {
            pattern_background: Color32::from_rgb(0x00, 0x00, 0x00),
            pattern_note: Color32::from_rgb(0xff, 0xff, 0xff),
            pattern_instrument: Color32::from_rgb(0x54, 0xff, 0xff),
            pattern_effect: Color32::from_rgb(0xff, 0xff, 0xff),
            pattern_operand: Color32::from_rgb(0xd0, 0xd0, 0xd0),
            theme_background: Color32::from_rgb(0x12, 0x12, 0x12),
            foreground: Color32::from_rgb(0xe8, 0xe8, 0xe8),
            muted_foreground: Color32::from_rgb(0x88, 0x88, 0x88),
            cursor: Color32::from_rgb(0x40, 0x70, 0x90),
            cursor_line: Color32::from_rgb(0x20, 0x28, 0x2c),
            cursor_line_highlight: Color32::from_rgb(0xb8, 0x20, 0x40),
            selection: Color32::from_rgb(0x00, 0x3c, 0x78),
            row_highlight_primary: Color32::from_rgb(0x14, 0x14, 0x14),
            row_highlight_secondary: Color32::from_rgb(0x08, 0x08, 0x08),
            channel_header: Color32::from_rgb(0x28, 0x38, 0x48),
            border: Color32::from_rgb(0x70, 0x90, 0xc0),
            waveform: Color32::from_rgb(0x54, 0xff, 0xff),
            waveform_center: Color32::from_rgb(0x40, 0x40, 0x40),
            waveform_loop: Color32::from_rgba_premultiplied(0x00, 0x3c, 0x78, 0x60),
            waveform_loop_edge: Color32::from_rgb(0x70, 0x90, 0xc0),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TrackerMetrics {
    pub char_width: f32,
    pub row_height: f32,
    pub header_height: f32,
}

impl TrackerMetrics {
    pub const fn milky_system() -> Self {
        Self {
            char_width: 8.0,
            row_height: 14.0,
            header_height: 18.0,
        }
    }

    fn row_header_width(self) -> f32 {
        columns_to_width(ROW_NUMBER_COLUMNS + ROW_GUTTER_COLUMNS, self.char_width)
    }

    fn channel_width(self) -> f32 {
        columns_to_width(CELL_TEXT_COLUMNS + CHANNEL_GAP_COLUMNS, self.char_width)
    }

    fn cell_text_width(self) -> f32 {
        columns_to_width(CELL_TEXT_COLUMNS, self.char_width)
    }

    fn content_size(self, rows: u16, channels: u16) -> Vec2 {
        egui::vec2(
            self.row_header_width() + self.channel_width() * channels as f32,
            self.header_height + self.row_height * rows as f32,
        )
    }
}

pub struct TrackerUiResources {
    font_texture: egui::TextureHandle,
    palette: TrackerPalette,
    theme: TrackerTheme,
    metrics: TrackerMetrics,
}

#[derive(Clone, Copy)]
struct PatternPaint<'a> {
    metrics: TrackerMetrics,
    theme: TrackerTheme,
    resources: &'a TrackerUiResources,
}

impl TrackerUiResources {
    pub fn new(ctx: &egui::Context) -> Self {
        let font_texture = ctx.load_texture(
            "rustytracker-j-flux-8x8",
            build_font_atlas_image(),
            egui::TextureOptions::NEAREST,
        );

        let palette = TrackerPalette::default();

        Self {
            font_texture,
            palette,
            theme: palette.theme(),
            metrics: TrackerMetrics::milky_system(),
        }
    }

    pub fn palette(&self) -> TrackerPalette {
        self.palette
    }

    pub fn set_palette(&mut self, palette: TrackerPalette) {
        self.palette = palette;
        self.theme = palette.theme();
    }

    pub fn theme(&self) -> TrackerTheme {
        self.theme
    }

    pub fn metrics(&self) -> TrackerMetrics {
        self.metrics
    }

    fn font_texture_id(&self) -> egui::TextureId {
        self.font_texture.id()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PatternView {
    pub active_pattern_index: usize,
    pub active_row: u16,
    pub active_channel: u16,
    pub active_field: ActiveField,
    pub edit_mode: bool,
}

pub struct WaveformView<'a> {
    pub data: &'a SampleData,
    pub sample_length: u32,
    pub loop_kind: SampleLoopKind,
    pub loop_start: u32,
    pub loop_length: u32,
}

pub fn show_list_heading(ui: &mut Ui, resources: &TrackerUiResources, text: &str) {
    let theme = resources.theme();
    let metrics = resources.metrics();
    let desired_size = egui::vec2(ui.available_width(), metrics.header_height);
    let (rect, _response) = ui.allocate_exact_size(desired_size, Sense::hover());

    if ui.is_rect_visible(rect) {
        ui.painter().rect_filled(rect, 0.0, theme.theme_background);
        draw_text(
            ui.painter(),
            egui::pos2(
                rect.min.x + LIST_TEXT_OFFSET_X,
                rect.min.y + LIST_TEXT_OFFSET_Y,
            ),
            text,
            theme.foreground,
            metrics,
            resources,
        );
    }
}

pub fn show_list_row(
    ui: &mut Ui,
    resources: &TrackerUiResources,
    text: &str,
    selected: bool,
    accent: Color32,
) -> Response {
    let theme = resources.theme();
    let metrics = resources.metrics();
    let desired_size = egui::vec2(ui.available_width(), metrics.row_height);
    let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());

    if ui.is_rect_visible(rect) {
        let background = if selected {
            theme.selection
        } else {
            theme.pattern_background
        };
        ui.painter().rect_filled(rect, 0.0, background);

        if selected {
            ui.painter().rect_stroke(
                rect,
                0.0,
                Stroke::new(CURSOR_BORDER_WIDTH, accent),
                StrokeKind::Inside,
            );
        }

        let text_color = if selected { accent } else { theme.foreground };
        draw_text(
            ui.painter(),
            egui::pos2(
                rect.min.x + LIST_TEXT_OFFSET_X,
                rect.min.y + LIST_TEXT_OFFSET_Y,
            ),
            text,
            text_color,
            metrics,
            resources,
        );
    }

    response
}

pub fn show_toolbar_button(
    ui: &mut Ui,
    resources: &TrackerUiResources,
    text: &str,
    selected: bool,
    accent: Color32,
) -> Response {
    let theme = resources.theme();
    let metrics = resources.metrics();
    let desired_size = egui::vec2(
        toolbar_button_width(text, metrics),
        toolbar_control_height(metrics),
    );
    let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());

    if ui.is_rect_visible(rect) {
        let background = if selected {
            theme.selection
        } else if response.hovered() {
            theme.cursor_line
        } else {
            theme.theme_background
        };
        let stroke_color = if selected || response.hovered() {
            accent
        } else {
            theme.border
        };
        let text_color = if selected || response.hovered() {
            accent
        } else {
            theme.foreground
        };

        ui.painter().rect_filled(rect, 0.0, background);
        ui.painter().rect_stroke(
            rect,
            0.0,
            Stroke::new(CURSOR_BORDER_WIDTH, stroke_color),
            StrokeKind::Inside,
        );
        draw_text(
            ui.painter(),
            egui::pos2(
                rect.min.x + CONTROL_HORIZONTAL_PADDING,
                rect.min.y + CONTROL_TEXT_OFFSET_Y,
            ),
            text,
            text_color,
            metrics,
            resources,
        );
    }

    response
}

pub fn show_status_label(ui: &mut Ui, resources: &TrackerUiResources, text: &str, accent: Color32) {
    let theme = resources.theme();
    let metrics = resources.metrics();
    let desired_size = egui::vec2(
        status_label_width(text, metrics),
        toolbar_control_height(metrics),
    );
    let (rect, _response) = ui.allocate_exact_size(desired_size, Sense::hover());

    if ui.is_rect_visible(rect) {
        ui.painter()
            .rect_filled(rect, 0.0, theme.pattern_background);
        ui.painter().rect_stroke(
            rect,
            0.0,
            Stroke::new(CURSOR_BORDER_WIDTH, theme.border),
            StrokeKind::Inside,
        );
        draw_text(
            ui.painter(),
            egui::pos2(
                rect.min.x + STATUS_HORIZONTAL_PADDING,
                rect.min.y + CONTROL_TEXT_OFFSET_Y,
            ),
            text,
            accent,
            metrics,
            resources,
        );
    }
}

pub fn show_toolbar_separator(ui: &mut Ui, resources: &TrackerUiResources) {
    let theme = resources.theme();
    let metrics = resources.metrics();
    let desired_size = egui::vec2(TOOLBAR_SEPARATOR_WIDTH, toolbar_control_height(metrics));
    let (rect, _response) = ui.allocate_exact_size(desired_size, Sense::hover());

    if ui.is_rect_visible(rect) {
        let x = rect.center().x;
        ui.painter().line_segment(
            [
                egui::pos2(x, rect.min.y + TOOLBAR_SEPARATOR_INSET_Y),
                egui::pos2(x, rect.max.y - TOOLBAR_SEPARATOR_INSET_Y),
            ],
            Stroke::new(CURSOR_BORDER_WIDTH, theme.border),
        );
    }
}

pub fn show_waveform(ui: &mut Ui, resources: &TrackerUiResources, view: WaveformView<'_>) {
    let theme = resources.theme();
    let metrics = resources.metrics();
    let width = ui.available_width().max(WAVEFORM_MIN_WIDTH);
    let (rect, _response) =
        ui.allocate_exact_size(egui::vec2(width, WAVEFORM_HEIGHT), Sense::hover());

    if !ui.is_rect_visible(rect) {
        return;
    }

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, theme.pattern_background);
    painter.rect_stroke(
        rect,
        0.0,
        Stroke::new(CURSOR_BORDER_WIDTH, theme.border),
        StrokeKind::Inside,
    );

    let mid_y = rect.center().y;
    painter.line_segment(
        [egui::pos2(rect.min.x, mid_y), egui::pos2(rect.max.x, mid_y)],
        Stroke::new(CURSOR_BORDER_WIDTH, theme.waveform_center),
    );

    if let Some(loop_rect) = waveform_loop_rect(
        rect,
        view.sample_length,
        view.loop_kind,
        view.loop_start,
        view.loop_length,
    ) {
        painter.rect_filled(loop_rect, 0.0, theme.waveform_loop);
        painter.line_segment(
            [
                egui::pos2(loop_rect.min.x, rect.min.y),
                egui::pos2(loop_rect.min.x, rect.max.y),
            ],
            Stroke::new(CURSOR_BORDER_WIDTH, theme.waveform_loop_edge),
        );
        painter.line_segment(
            [
                egui::pos2(loop_rect.max.x, rect.min.y),
                egui::pos2(loop_rect.max.x, rect.max.y),
            ],
            Stroke::new(CURSOR_BORDER_WIDTH, theme.waveform_loop_edge),
        );
    }

    let sample_len = view.data.frame_count();
    if sample_len == 0 {
        let text_width = text_width(NO_WAVEFORM_TEXT, metrics);
        draw_text(
            &painter,
            egui::pos2(
                rect.center().x - text_width / 2.0,
                mid_y - FONT_GLYPH_HEIGHT as f32 / 2.0,
            ),
            NO_WAVEFORM_TEXT,
            theme.muted_foreground,
            metrics,
            resources,
        );
        return;
    }

    let pixel_width = rect.width().round().max(1.0) as usize;
    let amplitude = rect.height() / 2.0 - WAVEFORM_EDGE_PADDING;
    let mut previous = None;

    for x in 0..pixel_width {
        let sample_index = waveform_sample_index(x, pixel_width, sample_len);
        let sample_value = waveform_sample_value(view.data, sample_index);
        let point = egui::pos2(rect.min.x + x as f32, mid_y - sample_value * amplitude);

        if let Some(previous_point) = previous {
            painter.line_segment(
                [previous_point, point],
                Stroke::new(WAVEFORM_STROKE_WIDTH, theme.waveform),
            );
        }

        previous = Some(point);
    }
}

pub fn show_pattern_editor(
    ui: &mut Ui,
    resources: &TrackerUiResources,
    pattern: &Pattern,
    view: PatternView,
) -> Option<(u16, u16)> {
    let theme = resources.theme();
    let metrics = resources.metrics();
    let paint = PatternPaint {
        metrics,
        theme,
        resources,
    };
    let rows = pattern.rows();
    let channels = pattern.channels();
    let desired_size = metrics.content_size(rows, channels);
    let mut clicked_cell = None;
    let heading_size = egui::vec2(ui.available_width(), metrics.header_height);
    let (heading_rect, _heading_response) = ui.allocate_exact_size(heading_size, Sense::hover());

    if ui.is_rect_visible(heading_rect) {
        ui.painter()
            .rect_filled(heading_rect, 0.0, theme.theme_background);
        draw_text(
            ui.painter(),
            egui::pos2(
                heading_rect.min.x + LIST_TEXT_OFFSET_X,
                heading_rect.min.y + LIST_TEXT_OFFSET_Y,
            ),
            &format!(
                "Pattern {:02X}  Rows {:02X}  Channels {:02X}",
                view.active_pattern_index, rows, channels
            ),
            theme.foreground,
            metrics,
            resources,
        );
    }

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show_viewport(ui, |ui, viewport| {
            let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());
            let painter = ui.painter_at(rect);

            painter.rect_filled(rect, 0.0, theme.pattern_background);
            painter.rect_stroke(
                rect,
                0.0,
                Stroke::new(CURSOR_BORDER_WIDTH, theme.border),
                StrokeKind::Inside,
            );

            draw_headers(&painter, rect, viewport, paint, channels);
            draw_visible_rows(&painter, rect, viewport, paint, pattern, view);

            if response.clicked() {
                if let Some(pointer_pos) = response.interact_pointer_pos() {
                    clicked_cell = hit_test_cell(rect, metrics, pointer_pos, rows, channels);
                }
            }
        });

    clicked_cell
}

fn draw_headers(
    painter: &egui::Painter,
    rect: Rect,
    viewport: Rect,
    paint: PatternPaint<'_>,
    channels: u16,
) {
    let metrics = paint.metrics;
    let theme = paint.theme;

    if viewport.max.y < 0.0 || viewport.min.y > metrics.header_height {
        return;
    }

    let header_rect =
        Rect::from_min_size(rect.min, egui::vec2(rect.width(), metrics.header_height));
    painter.rect_filled(header_rect, 0.0, theme.theme_background);

    draw_text(
        painter,
        egui::pos2(rect.min.x, rect.min.y + CHANNEL_HEADER_OFFSET_Y),
        "Row",
        theme.muted_foreground,
        metrics,
        paint.resources,
    );

    let first_channel = first_visible_channel(viewport, metrics, channels);
    let last_channel = last_visible_channel(viewport, metrics, channels);

    for channel in first_channel..last_channel {
        let x = rect.min.x + metrics.row_header_width() + channel as f32 * metrics.channel_width();
        let channel_rect = Rect::from_min_size(
            egui::pos2(x, rect.min.y),
            egui::vec2(metrics.cell_text_width(), metrics.header_height),
        );
        painter.rect_filled(channel_rect, 0.0, theme.channel_header);
        draw_text(
            painter,
            egui::pos2(x, rect.min.y + CHANNEL_HEADER_OFFSET_Y),
            &format!("CH{:02}", channel + 1),
            theme.foreground,
            metrics,
            paint.resources,
        );
    }
}

fn draw_visible_rows(
    painter: &egui::Painter,
    rect: Rect,
    viewport: Rect,
    paint: PatternPaint<'_>,
    pattern: &Pattern,
    view: PatternView,
) {
    let metrics = paint.metrics;
    let theme = paint.theme;
    let rows = pattern.rows();
    let channels = pattern.channels();
    let first_row = first_visible_row(viewport, metrics, rows);
    let last_row = last_visible_row(viewport, metrics, rows);
    let first_channel = first_visible_channel(viewport, metrics, channels);
    let last_channel = last_visible_channel(viewport, metrics, channels);
    let default_cell = PatternCell::default();

    for row in first_row..last_row {
        let row_top = rect.min.y + metrics.header_height + row as f32 * metrics.row_height;
        let row_rect = Rect::from_min_size(
            egui::pos2(rect.min.x, row_top),
            egui::vec2(rect.width(), metrics.row_height),
        );

        if row == view.active_row {
            let color = if view.edit_mode {
                theme.cursor_line_highlight
            } else {
                theme.cursor_line
            };
            painter.rect_filled(row_rect, 0.0, color);
        } else if row % PRIMARY_HIGHLIGHT_SPACING == 0 {
            painter.rect_filled(row_rect, 0.0, theme.row_highlight_primary);
        } else if row % SECONDARY_HIGHLIGHT_SPACING == 0 {
            painter.rect_filled(row_rect, 0.0, theme.row_highlight_secondary);
        }

        draw_text(
            painter,
            egui::pos2(rect.min.x, row_top + ROW_LABEL_OFFSET_Y),
            &format!("{row:02X}"),
            if row == view.active_row {
                theme.pattern_note
            } else {
                theme.muted_foreground
            },
            metrics,
            paint.resources,
        );

        for channel in first_channel..last_channel {
            let cell = pattern.cell(channel, row).unwrap_or(&default_cell);
            let cell_x =
                rect.min.x + metrics.row_header_width() + channel as f32 * metrics.channel_width();
            let cell_y = row_top + CELL_TEXT_OFFSET_Y;

            if row == view.active_row && channel == view.active_channel {
                let cursor_rect = active_field_rect(cell_x, row_top, metrics, view.active_field);
                painter.rect_filled(cursor_rect, 0.0, theme.cursor);
                painter.rect_stroke(
                    cursor_rect,
                    0.0,
                    Stroke::new(CURSOR_BORDER_WIDTH, theme.pattern_note),
                    StrokeKind::Inside,
                );
            }

            draw_pattern_cell(painter, egui::pos2(cell_x, cell_y), paint, cell);
        }
    }
}

fn draw_pattern_cell(
    painter: &egui::Painter,
    origin: Pos2,
    paint: PatternPaint<'_>,
    cell: &PatternCell,
) {
    let theme = paint.theme;

    draw_field(
        painter,
        origin,
        NOTE_FIELD_COLUMN,
        &format_note(cell.note),
        theme.pattern_note,
        paint,
    );
    draw_field(
        painter,
        origin,
        INSTRUMENT_FIELD_COLUMN,
        &format_instrument(cell.instrument),
        theme.pattern_instrument,
        paint,
    );

    let effect0 = cell.effects.first().copied().unwrap_or_default();
    let effect1 = cell.effects.get(1).copied().unwrap_or_default();
    draw_effect_field(painter, origin, EFFECT0_FIELD_COLUMN, effect0, paint);
    draw_effect_field(painter, origin, EFFECT1_FIELD_COLUMN, effect1, paint);
}

fn draw_effect_field(
    painter: &egui::Painter,
    origin: Pos2,
    column: usize,
    effect: EffectCommand,
    paint: PatternPaint<'_>,
) {
    let theme = paint.theme;
    let text = format_effect(effect);
    let mut chars = text.chars();
    let command = chars.next().unwrap_or('.');
    let operand: String = chars.collect();

    draw_field(
        painter,
        origin,
        column,
        &command.to_string(),
        theme.pattern_effect,
        paint,
    );
    draw_field(
        painter,
        origin,
        column + 1,
        &operand,
        theme.pattern_operand,
        paint,
    );
}

fn draw_field(
    painter: &egui::Painter,
    origin: Pos2,
    column: usize,
    text: &str,
    color: Color32,
    paint: PatternPaint<'_>,
) {
    let metrics = paint.metrics;

    draw_text(
        painter,
        egui::pos2(
            origin.x + columns_to_width(column, metrics.char_width),
            origin.y,
        ),
        text,
        color,
        metrics,
        paint.resources,
    );
}

fn draw_text(
    painter: &egui::Painter,
    pos: Pos2,
    text: &str,
    color: Color32,
    metrics: TrackerMetrics,
    resources: &TrackerUiResources,
) {
    let clip = painter.clip_rect();

    for (char_index, ch) in text.chars().enumerate() {
        let glyph_x = pos.x + char_index as f32 * metrics.char_width;
        let destination_rect = Rect::from_min_size(
            egui::pos2(glyph_x, pos.y),
            egui::vec2(FONT_GLYPH_WIDTH as f32, FONT_GLYPH_HEIGHT as f32),
        );

        if !clip.intersects(destination_rect) {
            continue;
        }

        painter.image(
            resources.font_texture_id(),
            destination_rect,
            glyph_uv_rect(ch),
            color,
        );
    }
}

fn build_font_atlas_image() -> egui::ColorImage {
    debug_assert!(SYSTEM_FONT_BYTES.len() >= FONT_REQUIRED_BYTES);

    let mut image = egui::ColorImage {
        size: [FONT_ATLAS_WIDTH, FONT_ATLAS_HEIGHT],
        pixels: vec![Color32::TRANSPARENT; FONT_ATLAS_WIDTH * FONT_ATLAS_HEIGHT],
    };

    for glyph_index in 0..FONT_GLYPH_COUNT {
        let glyph_x = (glyph_index % FONT_ATLAS_COLUMNS) * FONT_GLYPH_WIDTH;
        let glyph_y = (glyph_index / FONT_ATLAS_COLUMNS) * FONT_GLYPH_HEIGHT;

        for row in 0..FONT_GLYPH_HEIGHT {
            let row_bits = SYSTEM_FONT_BYTES[glyph_index * FONT_GLYPH_BYTES + row];
            for column in 0..FONT_GLYPH_WIDTH {
                let mask = FONT_BIT_MASK_START >> column;
                if row_bits & mask == 0 {
                    continue;
                }

                let atlas_x = glyph_x + column;
                let atlas_y = glyph_y + row;
                image.pixels[atlas_y * FONT_ATLAS_WIDTH + atlas_x] = Color32::WHITE;
            }
        }
    }

    image
}

fn glyph_uv_rect(ch: char) -> Rect {
    let codepoint = ch as usize;
    let glyph_index = if codepoint < FONT_GLYPH_COUNT {
        codepoint
    } else {
        b'?' as usize
    };
    let glyph_x = (glyph_index % FONT_ATLAS_COLUMNS) * FONT_GLYPH_WIDTH;
    let glyph_y = (glyph_index / FONT_ATLAS_COLUMNS) * FONT_GLYPH_HEIGHT;

    Rect::from_min_max(
        egui::pos2(
            glyph_x as f32 / FONT_ATLAS_WIDTH as f32,
            glyph_y as f32 / FONT_ATLAS_HEIGHT as f32,
        ),
        egui::pos2(
            (glyph_x + FONT_GLYPH_WIDTH) as f32 / FONT_ATLAS_WIDTH as f32,
            (glyph_y + FONT_GLYPH_HEIGHT) as f32 / FONT_ATLAS_HEIGHT as f32,
        ),
    )
}

fn active_field_rect(
    cell_x: f32,
    row_top: f32,
    metrics: TrackerMetrics,
    active_field: ActiveField,
) -> Rect {
    let (column, width_columns) = match active_field {
        ActiveField::Note => (NOTE_FIELD_COLUMN, NOTE_FIELD_COLUMNS),
        ActiveField::Instrument => (INSTRUMENT_FIELD_COLUMN, INSTRUMENT_FIELD_COLUMNS),
        ActiveField::Effect0 => (EFFECT0_FIELD_COLUMN, EFFECT_FIELD_COLUMNS),
        ActiveField::Effect1 => (EFFECT1_FIELD_COLUMN, EFFECT_FIELD_COLUMNS),
    };

    Rect::from_min_size(
        egui::pos2(
            cell_x + columns_to_width(column, metrics.char_width),
            row_top,
        ),
        egui::vec2(
            columns_to_width(width_columns, metrics.char_width),
            metrics.row_height,
        ),
    )
}

fn hit_test_cell(
    content_rect: Rect,
    metrics: TrackerMetrics,
    pointer_pos: Pos2,
    rows: u16,
    channels: u16,
) -> Option<(u16, u16)> {
    if !content_rect.contains(pointer_pos) {
        return None;
    }

    let local_x = pointer_pos.x - content_rect.min.x;
    let local_y = pointer_pos.y - content_rect.min.y;

    if local_y < metrics.header_height || local_x < metrics.row_header_width() {
        return None;
    }

    let row = ((local_y - metrics.header_height) / metrics.row_height).floor() as u16;
    let channel = ((local_x - metrics.row_header_width()) / metrics.channel_width()).floor() as u16;

    if row < rows && channel < channels {
        Some((channel, row))
    } else {
        None
    }
}

fn first_visible_row(viewport: Rect, metrics: TrackerMetrics, rows: u16) -> u16 {
    (((viewport.min.y - metrics.header_height).max(0.0) / metrics.row_height).floor() as u16)
        .min(rows)
}

fn last_visible_row(viewport: Rect, metrics: TrackerMetrics, rows: u16) -> u16 {
    (((viewport.max.y - metrics.header_height).max(0.0) / metrics.row_height).ceil() as u16)
        .saturating_add(1)
        .min(rows)
}

fn first_visible_channel(viewport: Rect, metrics: TrackerMetrics, channels: u16) -> u16 {
    (((viewport.min.x - metrics.row_header_width()).max(0.0) / metrics.channel_width()).floor()
        as u16)
        .min(channels)
}

fn last_visible_channel(viewport: Rect, metrics: TrackerMetrics, channels: u16) -> u16 {
    (((viewport.max.x - metrics.row_header_width()).max(0.0) / metrics.channel_width()).ceil()
        as u16)
        .saturating_add(1)
        .min(channels)
}

fn columns_to_width(columns: usize, char_width: f32) -> f32 {
    columns as f32 * char_width
}

fn toolbar_control_height(metrics: TrackerMetrics) -> f32 {
    metrics.row_height + CONTROL_EXTRA_HEIGHT
}

fn toolbar_button_width(text: &str, metrics: TrackerMetrics) -> f32 {
    (text.chars().count() as f32 * metrics.char_width + CONTROL_HORIZONTAL_PADDING * 2.0)
        .max(CONTROL_MIN_WIDTH)
}

fn status_label_width(text: &str, metrics: TrackerMetrics) -> f32 {
    text.chars().count() as f32 * metrics.char_width + STATUS_HORIZONTAL_PADDING * 2.0
}

fn text_width(text: &str, metrics: TrackerMetrics) -> f32 {
    text.chars().count() as f32 * metrics.char_width
}

fn waveform_loop_rect(
    rect: Rect,
    sample_length: u32,
    loop_kind: SampleLoopKind,
    loop_start: u32,
    loop_length: u32,
) -> Option<Rect> {
    if loop_kind == SampleLoopKind::None || sample_length == 0 || loop_length == 0 {
        return None;
    }

    let loop_end = loop_start.saturating_add(loop_length).min(sample_length);
    if loop_start >= loop_end {
        return None;
    }

    let start_ratio = loop_start as f32 / sample_length as f32;
    let end_ratio = loop_end as f32 / sample_length as f32;
    let start_x = rect.min.x + start_ratio * rect.width();
    let end_x = rect.min.x + end_ratio * rect.width();

    Some(Rect::from_min_max(
        egui::pos2(start_x, rect.min.y),
        egui::pos2(end_x, rect.max.y),
    ))
}

fn waveform_sample_index(x: usize, pixel_width: usize, sample_len: usize) -> usize {
    if pixel_width <= 1 || sample_len <= 1 {
        return 0;
    }

    let x_ratio = x as f32 / (pixel_width - 1) as f32;
    (x_ratio * (sample_len - 1) as f32).round() as usize
}

fn waveform_sample_value(data: &SampleData, index: usize) -> f32 {
    match data {
        SampleData::Pcm8(values) => values
            .get(index)
            .map(|&value| value as f32 / PCM8_NORMALIZATION_FACTOR)
            .unwrap_or(0.0),
        SampleData::Pcm16(values) => values
            .get(index)
            .map(|&value| value as f32 / PCM16_NORMALIZATION_FACTOR)
            .unwrap_or(0.0),
        SampleData::Empty => 0.0,
    }
    .clamp(-1.0, 1.0)
}

fn format_note(note: Note) -> String {
    match note {
        Note::Empty => fit_field("...", NOTE_FIELD_COLUMNS),
        Note::Off => fit_field("====", NOTE_FIELD_COLUMNS),
        Note::Key(value) => {
            let zero_based = value.saturating_sub(1);
            let octave = zero_based / NOTE_NAMES.len() as u8;
            let name = NOTE_NAMES
                .get((zero_based % NOTE_NAMES.len() as u8) as usize)
                .copied()
                .unwrap_or("??");
            fit_field(&format!("{name}{octave}"), NOTE_FIELD_COLUMNS)
        }
    }
}

fn format_instrument(instrument: u8) -> String {
    if instrument == 0 {
        fit_field("..", INSTRUMENT_FIELD_COLUMNS)
    } else {
        fit_field(&format!("{instrument:02X}"), INSTRUMENT_FIELD_COLUMNS)
    }
}

fn format_effect(command: EffectCommand) -> String {
    let text = if command.effect == 0 && command.operand == 0 {
        "...".to_string()
    } else if (0x30..=0x3f).contains(&command.effect) {
        let ext_type = command.effect - 0x30;
        format!("E{:X}{:X}", ext_type, command.operand & 0x0f)
    } else {
        let effect_char = match command.effect {
            0x00..=0x09 => char::from(b'0' + command.effect),
            0x0a => 'A',
            0x0b => 'B',
            0x0c => 'C',
            0x0d => 'D',
            0x0f => 'F',
            0x20 => '0',
            _ => '?',
        };
        if effect_char == '?' {
            "???".to_string()
        } else {
            format!("{}{:02X}", effect_char, command.operand)
        }
    };

    fit_field(&text, EFFECT_FIELD_COLUMNS)
}

fn fit_field(text: &str, width: usize) -> String {
    let mut value: String = text.chars().take(width).collect();
    while value.chars().count() < width {
        value.push(' ');
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hit_test_maps_pointer_to_channel_and_row() {
        let metrics = TrackerMetrics::milky_system();
        let rect = Rect::from_min_size(Pos2::ZERO, metrics.content_size(64, 8));
        let x = metrics.row_header_width() + metrics.channel_width() * 2.0 + 1.0;
        let y = metrics.header_height + metrics.row_height * 10.0 + 1.0;

        assert_eq!(
            hit_test_cell(rect, metrics, egui::pos2(x, y), 64, 8),
            Some((2, 10))
        );
    }

    #[test]
    fn hit_test_ignores_header_and_row_gutter() {
        let metrics = TrackerMetrics::milky_system();
        let rect = Rect::from_min_size(Pos2::ZERO, metrics.content_size(64, 8));

        assert_eq!(
            hit_test_cell(
                rect,
                metrics,
                egui::pos2(1.0, metrics.header_height - 1.0),
                64,
                8
            ),
            None
        );
        assert_eq!(
            hit_test_cell(
                rect,
                metrics,
                egui::pos2(
                    metrics.row_header_width() - 1.0,
                    metrics.header_height + 1.0
                ),
                64,
                8
            ),
            None
        );
    }

    #[test]
    fn embedded_system_font_contains_cp437_glyph_grid() {
        assert!(SYSTEM_FONT_BYTES.len() >= FONT_REQUIRED_BYTES);
    }

    #[test]
    fn glyph_uv_rect_maps_ascii_to_16_by_16_atlas() {
        let glyph_index = b'A' as usize;
        let glyph_x = (glyph_index % FONT_ATLAS_COLUMNS) * FONT_GLYPH_WIDTH;
        let glyph_y = (glyph_index / FONT_ATLAS_COLUMNS) * FONT_GLYPH_HEIGHT;
        let uv = glyph_uv_rect('A');

        assert_eq!(uv.min.x, glyph_x as f32 / FONT_ATLAS_WIDTH as f32);
        assert_eq!(uv.min.y, glyph_y as f32 / FONT_ATLAS_HEIGHT as f32);
        assert_eq!(
            uv.max.x,
            (glyph_x + FONT_GLYPH_WIDTH) as f32 / FONT_ATLAS_WIDTH as f32
        );
        assert_eq!(
            uv.max.y,
            (glyph_y + FONT_GLYPH_HEIGHT) as f32 / FONT_ATLAS_HEIGHT as f32
        );
    }

    #[test]
    fn toolbar_button_width_uses_fixed_character_metrics() {
        let metrics = TrackerMetrics::milky_system();

        assert_eq!(
            toolbar_button_width("PLAY", metrics),
            CONTROL_MIN_WIDTH.max(4.0 * metrics.char_width + CONTROL_HORIZONTAL_PADDING * 2.0)
        );
    }

    #[test]
    fn tracker_palettes_expose_default_and_distinct_accents() {
        assert_eq!(TrackerPalette::default(), TrackerPalette::MilkyDefault);
        assert_eq!(TrackerPalette::ALL[0], TrackerPalette::MilkyDefault);
        assert_ne!(
            TrackerPalette::MilkyDefault.theme().pattern_instrument,
            TrackerPalette::MilkyWarm.theme().pattern_instrument
        );
        assert_ne!(
            TrackerPalette::MilkyDefault.theme().border,
            TrackerPalette::HighContrast.theme().border
        );
    }

    #[test]
    fn waveform_loop_rect_clamps_loop_end_to_sample_length() {
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(100.0, 20.0));
        let loop_rect = waveform_loop_rect(rect, 100, SampleLoopKind::Forward, 80, 40).unwrap();

        assert_eq!(loop_rect.min.x, 80.0);
        assert_eq!(loop_rect.max.x, 100.0);
    }

    #[test]
    fn waveform_loop_rect_ignores_disabled_or_empty_loops() {
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(100.0, 20.0));

        assert!(waveform_loop_rect(rect, 100, SampleLoopKind::None, 10, 20).is_none());
        assert!(waveform_loop_rect(rect, 100, SampleLoopKind::Forward, 10, 0).is_none());
        assert!(waveform_loop_rect(rect, 0, SampleLoopKind::Forward, 10, 20).is_none());
    }

    #[test]
    fn waveform_sample_index_spreads_pixels_across_sample_range() {
        assert_eq!(waveform_sample_index(0, 5, 9), 0);
        assert_eq!(waveform_sample_index(2, 5, 9), 4);
        assert_eq!(waveform_sample_index(4, 5, 9), 8);
    }

    #[test]
    fn waveform_sample_value_normalizes_pcm_data() {
        assert_eq!(waveform_sample_value(&SampleData::pcm8(vec![64]), 0), 0.5);
        assert_eq!(
            waveform_sample_value(&SampleData::pcm16(vec![-16384]), 0),
            -0.5
        );
        assert_eq!(waveform_sample_value(&SampleData::Empty, 0), 0.0);
    }
}
