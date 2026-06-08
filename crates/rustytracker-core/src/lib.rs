//! Typed core model for RustyTracker.
//!
//! This crate intentionally starts with domain invariants only. File formats,
//! playback, and editing commands live in separate crates once their tests are
//! in place.

pub const DEFAULT_SONG_CHANNELS: u16 = 8;
pub const EDITOR_PATTERN_CHANNELS: u16 = 32;
pub const MIN_CHANNEL_COUNT: u16 = 1;
pub const DEFAULT_PATTERN_ROWS: u16 = 64;
pub const DEFAULT_EFFECT_SLOTS: u8 = 2;
pub const DEFAULT_BPM: u16 = 125;
pub const DEFAULT_TICK_SPEED: u16 = 6;
pub const DEFAULT_MAIN_VOLUME: u16 = 255;
pub const EMPTY_PATTERN_NUMBER: u8 = 0;
pub const MIN_ACTIVE_ORDERS: usize = 1;
pub const INSERT_AFTER_OFFSET: usize = 1;
pub const ORDER_SEQUENCE_STEP: u8 = 1;
pub const EMPTY_SAMPLE_LENGTH: u32 = 0;

pub const MAX_ORDERS: usize = 256;
pub const MAX_ACTIVE_ORDERS: usize = 255;
pub const MAX_PATTERNS: usize = 256;
pub const MAX_INSTRUMENTS: usize = 255;
pub const DEFAULT_INSTRUMENTS: usize = 128;
pub const SAMPLES_PER_INSTRUMENT: usize = 16;
pub const DEFAULT_SAMPLE_COUNT: usize = DEFAULT_INSTRUMENTS * SAMPLES_PER_INSTRUMENT;
pub const MAX_XM_NOTES: u8 = 96;
pub const NOTE_OFF_VALUE: u8 = 121;
pub const TITLE_TEXT_LEN: usize = 20;
pub const INSTRUMENT_NAME_LEN: usize = 22;
pub const SAMPLE_NAME_LEN: usize = 22;
pub const SAMPLE_DEFAULT_VOLUME: u8 = 0xff;
pub const SAMPLE_DEFAULT_PANNING: u8 = 0x80;
pub const SAMPLE_DEFAULT_FLAGS: u8 = 3;
pub const SAMPLE_DEFAULT_VOLUME_FADEOUT: u16 = 65_535;
pub const NOTES_PER_OCTAVE: u8 = 12;
pub const FIRST_XM_NOTE_VALUE: u8 = 1;
pub const EMPTY_NOTE_VALUE: u8 = 0;
pub const DEFAULT_INSTRUMENT_NUMBER: u8 = 0;
pub const DEFAULT_NOTE_SAMPLE_INDEX: usize = 0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    InvalidChannelCount(u16),
    InvalidChannel { channel: u16, capacity: u16 },
    InvalidRow { row: u16, rows: u16 },
    InvalidEffectSlot { slot: u8, slots: u8 },
    InvalidNote { octave: u8, note: u8 },
    EmptyOrderList,
    TooManyOrders { requested: usize, maximum: usize },
    InvalidOrderIndex { index: usize, len: usize },
    PatternNumberOverflow,
}

pub type CoreResult<T> = Result<T, CoreError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedText<const CAPACITY: usize> {
    text: String,
}

impl<const CAPACITY: usize> FixedText<CAPACITY> {
    pub fn new(value: &str) -> Self {
        let mut text = String::new();

        for ch in value.chars() {
            if text.len() + ch.len_utf8() > CAPACITY {
                break;
            }
            text.push(ch);
        }

        Self { text }
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }

    pub const fn capacity(&self) -> usize {
        CAPACITY
    }
}

impl<const CAPACITY: usize> Default for FixedText<CAPACITY> {
    fn default() -> Self {
        Self {
            text: String::new(),
        }
    }
}

pub type ModuleTitle = FixedText<TITLE_TEXT_LEN>;
pub type InstrumentName = FixedText<INSTRUMENT_NAME_LEN>;
pub type SampleName = FixedText<SAMPLE_NAME_LEN>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrequencyTable {
    Amiga,
    Linear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteName {
    C = 0,
    CSharp = 1,
    D = 2,
    DSharp = 3,
    E = 4,
    F = 5,
    FSharp = 6,
    G = 7,
    GSharp = 8,
    A = 9,
    ASharp = 10,
    B = 11,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Note {
    Empty,
    Key(u8),
    Off,
}

impl Note {
    pub fn key(octave: u8, name: NoteName) -> CoreResult<Self> {
        let value = octave
            .saturating_mul(NOTES_PER_OCTAVE)
            .saturating_add(name as u8)
            .saturating_add(FIRST_XM_NOTE_VALUE);

        if (FIRST_XM_NOTE_VALUE..=MAX_XM_NOTES).contains(&value) {
            Ok(Self::Key(value))
        } else {
            Err(CoreError::InvalidNote {
                octave,
                note: name as u8,
            })
        }
    }

    pub fn raw(self) -> u8 {
        match self {
            Self::Empty => EMPTY_NOTE_VALUE,
            Self::Key(value) => value,
            Self::Off => NOTE_OFF_VALUE,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EffectCommand {
    pub effect: u8,
    pub operand: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternCell {
    pub note: Note,
    pub instrument: u8,
    pub effects: Vec<EffectCommand>,
}

impl Default for PatternCell {
    fn default() -> Self {
        Self {
            note: Note::Empty,
            instrument: DEFAULT_INSTRUMENT_NUMBER,
            effects: vec![EffectCommand::default(); DEFAULT_EFFECT_SLOTS as usize],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pattern {
    rows: u16,
    channels: u16,
    effect_slots: u8,
    cells: Vec<PatternCell>,
}

impl Pattern {
    pub fn empty_editor_pattern() -> Self {
        Self::new(
            DEFAULT_PATTERN_ROWS,
            EDITOR_PATTERN_CHANNELS,
            DEFAULT_EFFECT_SLOTS,
        )
    }

    pub fn new(rows: u16, channels: u16, effect_slots: u8) -> Self {
        let cell = PatternCell {
            effects: vec![EffectCommand::default(); effect_slots as usize],
            ..PatternCell::default()
        };
        let len = rows as usize * channels as usize;

        Self {
            rows,
            channels,
            effect_slots,
            cells: vec![cell; len],
        }
    }

    pub fn rows(&self) -> u16 {
        self.rows
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    pub fn effect_slots(&self) -> u8 {
        self.effect_slots
    }

    pub fn cell(&self, channel: u16, row: u16) -> CoreResult<&PatternCell> {
        let index = self.index(channel, row)?;
        Ok(&self.cells[index])
    }

    pub fn set_cell(&mut self, channel: u16, row: u16, cell: PatternCell) -> CoreResult<()> {
        if cell.effects.len() != self.effect_slots as usize {
            return Err(CoreError::InvalidEffectSlot {
                slot: cell.effects.len() as u8,
                slots: self.effect_slots,
            });
        }

        let index = self.index(channel, row)?;
        self.cells[index] = cell;
        Ok(())
    }

    fn index(&self, channel: u16, row: u16) -> CoreResult<usize> {
        if channel >= self.channels {
            return Err(CoreError::InvalidChannel {
                channel,
                capacity: self.channels,
            });
        }

        if row >= self.rows {
            return Err(CoreError::InvalidRow {
                row,
                rows: self.rows,
            });
        }

        Ok(row as usize * self.channels as usize + channel as usize)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderList {
    orders: Vec<u8>,
}

impl Default for OrderList {
    fn default() -> Self {
        Self {
            orders: vec![EMPTY_PATTERN_NUMBER],
        }
    }
}

impl OrderList {
    pub fn from_orders(orders: Vec<u8>) -> CoreResult<Self> {
        if orders.is_empty() {
            return Err(CoreError::EmptyOrderList);
        }

        if orders.len() > MAX_ACTIVE_ORDERS {
            return Err(CoreError::TooManyOrders {
                requested: orders.len(),
                maximum: MAX_ACTIVE_ORDERS,
            });
        }

        Ok(Self { orders })
    }

    pub fn len(&self) -> usize {
        self.orders.len()
    }

    pub fn is_empty(&self) -> bool {
        self.orders.is_empty()
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.orders
    }

    pub fn set_len_clamped(&mut self, requested: usize) {
        let target = requested.clamp(MIN_ACTIVE_ORDERS, MAX_ACTIVE_ORDERS);
        self.orders.resize(target, EMPTY_PATTERN_NUMBER);
    }

    pub fn insert_duplicate_after(&mut self, index: usize) -> CoreResult<()> {
        let pattern = *self.orders.get(index).ok_or(CoreError::InvalidOrderIndex {
            index,
            len: self.orders.len(),
        })?;
        self.insert_after(index, pattern)
    }

    pub fn sequence_after(&mut self, index: usize) -> CoreResult<u8> {
        if index >= self.orders.len() {
            return Err(CoreError::InvalidOrderIndex {
                index,
                len: self.orders.len(),
            });
        }

        let highest = self
            .orders
            .iter()
            .copied()
            .max()
            .unwrap_or(EMPTY_PATTERN_NUMBER);
        let next = highest
            .checked_add(ORDER_SEQUENCE_STEP)
            .ok_or(CoreError::PatternNumberOverflow)?;
        self.insert_after(index, next)?;
        Ok(next)
    }

    pub fn delete(&mut self, index: usize) -> CoreResult<()> {
        if index >= self.orders.len() {
            return Err(CoreError::InvalidOrderIndex {
                index,
                len: self.orders.len(),
            });
        }

        if self.orders.len() > MIN_ACTIVE_ORDERS {
            self.orders.remove(index);
        }

        Ok(())
    }

    fn insert_after(&mut self, index: usize, pattern: u8) -> CoreResult<()> {
        if self.orders.len() >= MAX_ACTIVE_ORDERS {
            return Err(CoreError::TooManyOrders {
                requested: self.orders.len() + INSERT_AFTER_OFFSET,
                maximum: MAX_ACTIVE_ORDERS,
            });
        }

        if index >= self.orders.len() {
            return Err(CoreError::InvalidOrderIndex {
                index,
                len: self.orders.len(),
            });
        }

        self.orders.insert(index + INSERT_AFTER_OFFSET, pattern);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instrument {
    pub name: InstrumentName,
    pub sample_slots: Vec<Option<usize>>,
    pub note_sample_map: Vec<usize>,
}

impl Instrument {
    pub fn empty(index: usize) -> Self {
        let first_sample = index * SAMPLES_PER_INSTRUMENT;
        let sample_slots = (0..SAMPLES_PER_INSTRUMENT)
            .map(|offset| Some(first_sample + offset))
            .collect();

        Self {
            name: InstrumentName::default(),
            sample_slots,
            note_sample_map: vec![DEFAULT_NOTE_SAMPLE_INDEX; MAX_XM_NOTES as usize],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sample {
    pub name: SampleName,
    pub length: u32,
    pub loop_start: u32,
    pub loop_length: u32,
    pub volume: u8,
    pub panning: u8,
    pub flags: u8,
    pub volume_fadeout: u16,
}

impl Default for Sample {
    fn default() -> Self {
        Self {
            name: SampleName::default(),
            length: EMPTY_SAMPLE_LENGTH,
            loop_start: EMPTY_SAMPLE_LENGTH,
            loop_length: EMPTY_SAMPLE_LENGTH,
            volume: SAMPLE_DEFAULT_VOLUME,
            panning: SAMPLE_DEFAULT_PANNING,
            flags: SAMPLE_DEFAULT_FLAGS,
            volume_fadeout: SAMPLE_DEFAULT_VOLUME_FADEOUT,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleHeader {
    pub title: ModuleTitle,
    pub channel_count: u16,
    pub frequency_table: FrequencyTable,
    pub bpm: u16,
    pub tick_speed: u16,
    pub main_volume: u16,
    pub restart_position: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    pub header: ModuleHeader,
    pub orders: Vec<u8>,
    pub patterns: Vec<Pattern>,
    pub instruments: Vec<Instrument>,
    pub samples: Vec<Sample>,
}

impl Module {
    pub fn empty() -> Self {
        Self::empty_with_channels(DEFAULT_SONG_CHANNELS)
            .expect("default channel count must be valid")
    }

    pub fn empty_with_channels(channel_count: u16) -> CoreResult<Self> {
        if channel_count < MIN_CHANNEL_COUNT || channel_count > EDITOR_PATTERN_CHANNELS {
            return Err(CoreError::InvalidChannelCount(channel_count));
        }

        Ok(Self {
            header: ModuleHeader {
                title: ModuleTitle::default(),
                channel_count,
                frequency_table: FrequencyTable::Linear,
                bpm: DEFAULT_BPM,
                tick_speed: DEFAULT_TICK_SPEED,
                main_volume: DEFAULT_MAIN_VOLUME,
                restart_position: EMPTY_PATTERN_NUMBER as u16,
            },
            orders: vec![EMPTY_PATTERN_NUMBER],
            patterns: vec![Pattern::empty_editor_pattern()],
            instruments: (0..DEFAULT_INSTRUMENTS).map(Instrument::empty).collect(),
            samples: vec![Sample::default(); DEFAULT_SAMPLE_COUNT],
        })
    }
}
