pub use rustytracker_core::{
    EffectCommand, Envelope, EnvelopePoint, FrequencyTable, Module, Note, Pattern, PatternCell,
    SampleData, SampleLoopKind, DEFAULT_EFFECT_SLOTS,
};
pub use rustytracker_play::{
    ChannelSampleFrame, PlaybackChannelState, PlaybackClock, PlaybackCursor, PlaybackEnvelopeState,
    PlaybackError, PlaybackMixerMode, PlaybackSampleValue, PlaybackState, PlaybackTiming,
    RawMonoPcmFrame, RawStereoPcmFrame, RowAdvance, TickAdvance, EFFECT_ARPEGGIO_ZERO,
    EFFECT_PATTERN_BREAK, EFFECT_POSITION_JUMP, EFFECT_SET_SPEED_BPM, EFFECT_TONE_PORTAMENTO,
    EFFECT_VOLUME_SLIDE, PLAYBACK_FIRST_ORDER_INDEX, PLAYBACK_FIRST_ROW, PLAYBACK_FIRST_TICK,
    PLAYBACK_MONO_SILENCE, PLAYBACK_ORDER_STEP, PLAYBACK_ROW_STEP, PLAYBACK_STEREO_SILENCE,
    PLAYBACK_TICK_STEP, SPEED_BPM_THRESHOLD, VIB_TAB,
};

pub const PLAY_TEST_CHANNELS: u16 = 1;
pub const PLAY_TEST_TWO_CHANNELS: u16 = 2;
pub const PLAY_TEST_CHANNEL_ZERO: u16 = 0;
pub const PLAY_TEST_CHANNEL_ONE: u16 = 1;
pub const PLAY_TEST_CHANNEL_TWO: u16 = 2;
pub const PLAY_TEST_CHANNEL_THREE: u16 = 3;
pub const PLAY_TEST_PATTERN_ZERO: u8 = 0;
pub const PLAY_TEST_PATTERN_ONE: u8 = 1;
pub const PLAY_TEST_FIRST_PATTERN_INDEX: usize = 0;
pub const PLAY_TEST_SECOND_PATTERN_INDEX: usize = 1;
pub const PLAY_TEST_ZERO_ROWS: u16 = 0;
pub const PLAY_TEST_ONE_ROW: u16 = 1;
pub const PLAY_TEST_TWO_ROWS: u16 = 2;
pub const PLAY_TEST_THREE_ROWS: u16 = 3;
pub const PLAY_TEST_DEFAULT_TICK_SPEED: u16 = 6;
pub const PLAY_TEST_ONE_TICK_PER_ROW: u16 = 1;
pub const PLAY_TEST_THREE_TICKS_PER_ROW: u16 = 3;
pub const PLAY_TEST_DEFAULT_BPM: u16 = 125;
pub const PLAY_TEST_FAST_BPM: u16 = 250;
pub const PLAY_TEST_ZERO_TICK_SPEED: u16 = 0;
pub const PLAY_TEST_ZERO_BPM: u16 = 0;
pub const PLAY_TEST_ZERO_SAMPLE_RATE: u32 = 0;
pub const PLAY_TEST_DEFAULT_TICK_NANOS: u64 = 20_000_000;
pub const PLAY_TEST_DEFAULT_ROW_NANOS: u64 =
    PLAY_TEST_DEFAULT_TICK_NANOS * PLAY_TEST_DEFAULT_TICK_SPEED as u64;
pub const PLAY_TEST_FAST_TICK_NANOS: u64 = 10_000_000;
pub const PLAY_TEST_FAST_ROW_NANOS: u64 =
    PLAY_TEST_FAST_TICK_NANOS * PLAY_TEST_THREE_TICKS_PER_ROW as u64;
pub const PLAY_TEST_CHANNEL_ZERO_NOTE: u8 = 49;
pub const PLAY_TEST_CHANNEL_ZERO_AMIGA_PERIOD: u32 = 1712;
pub const PLAY_TEST_CHANNEL_ONE_NOTE: u8 = 50;
pub const PLAY_TEST_ROW_ONE_NOTE: u8 = 51;
pub const PLAY_TEST_CHANNEL_ZERO_INSTRUMENT: u8 = 1;
pub const PLAY_TEST_CHANNEL_ONE_INSTRUMENT: u8 = 2;
pub const PLAY_TEST_ROW_ONE_INSTRUMENT: u8 = 3;
pub const PLAY_TEST_FIRST_INSTRUMENT_INDEX: usize = 0;
pub const PLAY_TEST_SECOND_INSTRUMENT_INDEX: usize = 1;
pub const PLAY_TEST_FIRST_SAMPLE_INDEX: usize = 0;
pub const PLAY_TEST_SECOND_SAMPLE_INDEX: usize = 1;
pub const PLAY_TEST_SAMPLE_START_FRAME: usize = 0;
pub const PLAY_TEST_SECOND_SAMPLE_FRAME: usize = 1;
pub const PLAY_TEST_SAMPLE_VOLUME: u8 = 48;
pub const PLAY_TEST_SAMPLE_PANNING: u8 = 96;
pub const PLAY_TEST_MISSING_INSTRUMENT: u8 = 200;
pub const PLAY_TEST_PCM8_FIRST_VALUE: i8 = -2;
pub const PLAY_TEST_PCM8_SECOND_VALUE: i8 = 3;
pub const PLAY_TEST_PCM16_FIRST_VALUE: i16 = -512;
pub const PLAY_TEST_PCM16_SECOND_VALUE: i16 = 1024;
pub const PLAY_TEST_RENDER_FRAMES: usize = 3;
pub const PLAY_TEST_PCM8_FIRST_MONO: i32 = -512;
pub const PLAY_TEST_PCM16_HIGH_VALUE: i16 = 1024;
pub const PLAY_TEST_FIRST_MIXED_MONO: i32 = 512;
pub const PLAY_TEST_SILENCE_MONO: i32 = 0;
pub const PLAY_TEST_ENVELOPE_ENABLED_FLAG: u8 = 0x01;

pub fn module_with_orders_and_pattern_rows(orders: Vec<u8>, rows: &[u16]) -> Module {
    let mut module = Module::empty_with_channels(PLAY_TEST_CHANNELS).unwrap();
    module.orders = orders;
    module.patterns = rows
        .iter()
        .map(|rows| Pattern::new(*rows, PLAY_TEST_CHANNELS, DEFAULT_EFFECT_SLOTS))
        .collect();

    module
}

pub fn module_with_two_channel_cells(rows: u16, cells: &[(u16, u16, PatternCell)]) -> Module {
    let mut module = Module::empty_with_channels(PLAY_TEST_TWO_CHANNELS).unwrap();
    module.orders = vec![PLAY_TEST_PATTERN_ZERO];
    let mut pattern = Pattern::new(rows, PLAY_TEST_TWO_CHANNELS, DEFAULT_EFFECT_SLOTS);

    for (channel, row, cell) in cells {
        pattern.set_cell(*channel, *row, cell.clone()).unwrap();
    }

    module.patterns = vec![pattern];
    module
}

pub fn test_cell(note: u8, instrument: u8) -> PatternCell {
    PatternCell {
        note: Note::Key(note),
        instrument,
        ..PatternCell::default()
    }
}

pub fn note_off_cell() -> PatternCell {
    PatternCell {
        note: Note::Off,
        ..PatternCell::default()
    }
}

pub fn note_only_cell(note: u8) -> PatternCell {
    PatternCell {
        note: Note::Key(note),
        ..PatternCell::default()
    }
}

pub fn map_instrument_to_sample(module: &mut Module, instrument_index: usize, sample_index: usize) {
    module.instruments[instrument_index].note_sample_map =
        vec![Some(sample_index); module.instruments[instrument_index].note_sample_map.len()];
}

mod cursor {
    mod basic;
    mod traverse;
    mod timing;
    mod playback;
    mod render;
    mod effects;
}
