//! Protracker MOD file parser for RustyTracker.
//!
//! Handles 15-instrument and 31-instrument MOD modules, Amiga period note decoding,
//! standard effect mapping, and signed 8-bit PCM sample loading.

mod error;

use rustytracker_core::{
    EffectCommand, Envelope, Instrument, Module, Note, Pattern, PatternCell, Sample, SampleData,
    SampleLoopKind, DEFAULT_BPM, DEFAULT_EFFECT_SLOTS, DEFAULT_INSTRUMENTS, DEFAULT_MAIN_VOLUME,
    DEFAULT_SAMPLE_COUNT, DEFAULT_TICK_SPEED, EDITOR_PATTERN_CHANNELS, MAX_XM_NOTES,
    MIN_CHANNEL_COUNT, SAMPLES_PER_INSTRUMENT, SAMPLE_DEFAULT_PANNING,
    SAMPLE_DEFAULT_RELATIVE_NOTE, SAMPLE_DEFAULT_TYPE, SAMPLE_DEFAULT_VOLUME_FADEOUT,
};

pub use error::{ModParseError, ModWriteError};

const MOD_MIN_15_INSTRUMENT_BYTES: usize = 600;
const MOD_TITLE_LEN: usize = 20;
const MOD_15_INSTRUMENT_COUNT: usize = 15;
const MOD_31_INSTRUMENT_COUNT: usize = 31;
const MOD_INSTRUMENT_HEADER_LEN: usize = 30;
const MOD_INSTRUMENT_NAME_LEN: usize = 22;
const MOD_INSTRUMENT_FIELDS_AFTER_NAME_LEN: usize =
    MOD_INSTRUMENT_HEADER_LEN - MOD_INSTRUMENT_NAME_LEN;
const MOD_SONG_LENGTH_RESTART_LEN: usize = 2;
const MOD_ORDER_TABLE_LEN: usize = 128;
const MOD_SIGNATURE_OFFSET: usize = 1080;
const MOD_SIGNATURE_LEN: usize = 4;
const MOD_SIGNATURE_END: usize = MOD_SIGNATURE_OFFSET + MOD_SIGNATURE_LEN;
const MOD_DEFAULT_CHANNEL_COUNT: u16 = 4;
const MOD_PATTERN_ROWS: u16 = 64;
const MOD_CELL_BYTES: usize = 4;
const MOD_SAMPLE_LENGTH_WORD_BYTES: u32 = 2;
const MOD_LOOP_ENABLED_MIN_BYTES: u32 = 2;
const MOD_MAX_SAMPLE_BYTES: usize = 131_070;
const MOD_MAX_CHANNELS: u16 = EDITOR_PATTERN_CHANNELS;
const MOD_MAX_PATTERNS: usize = MOD_ORDER_TABLE_LEN;
const MOD_MAX_INSTRUMENT_NUMBER: u8 = 31;
const MOD_RESTART_POSITION_MASK: u16 = 0x7f;
const MOD_EMPTY_LOOP_START_WORDS: u16 = 0;
const MOD_EMPTY_LOOP_LENGTH_WORDS: u16 = 1;
const MOD_SAMPLE_8_BIT_FLAGS: u8 = 1;

/// Parses a MOD file byte buffer into a core `Module`.
pub fn parse_mod_module(bytes: &[u8]) -> Result<Module, ModParseError> {
    if bytes.len() < MOD_MIN_15_INSTRUMENT_BYTES {
        return Err(ModParseError::Truncated {
            expected: MOD_MIN_15_INSTRUMENT_BYTES,
            actual: bytes.len(),
        });
    }

    let mut is_31_ins = false;
    let mut channel_count = MOD_DEFAULT_CHANNEL_COUNT;
    let mut instrument_count = MOD_15_INSTRUMENT_COUNT;

    if bytes.len() >= MOD_SIGNATURE_END {
        let mut sig = [0u8; MOD_SIGNATURE_LEN];
        sig.copy_from_slice(&bytes[MOD_SIGNATURE_OFFSET..MOD_SIGNATURE_END]);
        if let Some(ch) = get_pt_num_channels(&sig) {
            is_31_ins = true;
            channel_count = ch;
            instrument_count = MOD_31_INSTRUMENT_COUNT;
        }
    }

    let expected_header_len = MOD_TITLE_LEN
        + instrument_count * MOD_INSTRUMENT_HEADER_LEN
        + MOD_SONG_LENGTH_RESTART_LEN
        + MOD_ORDER_TABLE_LEN
        + if is_31_ins { MOD_SIGNATURE_LEN } else { 0 };

    if bytes.len() < expected_header_len {
        return Err(ModParseError::Truncated {
            expected: expected_header_len,
            actual: bytes.len(),
        });
    }

    if !(MIN_CHANNEL_COUNT..=MOD_MAX_CHANNELS).contains(&channel_count) {
        return Err(ModParseError::InvalidChannelCount {
            channel_count,
            minimum: MIN_CHANNEL_COUNT,
            maximum: MOD_MAX_CHANNELS,
        });
    }

    // 1. Read Title
    let title_bytes = &bytes[0..MOD_TITLE_LEN];
    let title = clean_string(title_bytes);

    // 2. Read Instruments & Samples
    let mut cursor = MOD_TITLE_LEN;
    let mut samples = Vec::new();
    let mut instruments = Vec::new();

    for i in 0..instrument_count {
        let name_bytes = &bytes[cursor..cursor + MOD_INSTRUMENT_NAME_LEN];
        let name = clean_string(name_bytes);
        cursor += MOD_INSTRUMENT_NAME_LEN;

        let smplen = u16::from_be_bytes([bytes[cursor], bytes[cursor + 1]]) as u32
            * MOD_SAMPLE_LENGTH_WORD_BYTES;
        let finetune_nibble = bytes[cursor + 2] & 0x0f;
        let volume_64 = bytes[cursor + 3];
        let loop_start = u16::from_be_bytes([bytes[cursor + 4], bytes[cursor + 5]]) as u32
            * MOD_SAMPLE_LENGTH_WORD_BYTES;
        let loop_len = u16::from_be_bytes([bytes[cursor + 6], bytes[cursor + 7]]) as u32
            * MOD_SAMPLE_LENGTH_WORD_BYTES;
        cursor += MOD_INSTRUMENT_FIELDS_AFTER_NAME_LEN;

        let finetune = mod_finetunes(finetune_nibble);
        let volume = vol64_to_255(volume_64);

        let mut sample = Sample {
            name: rustytracker_core::SampleName::new(&name),
            length: smplen,
            loop_start,
            loop_length: loop_len,
            loop_kind: if loop_len > MOD_LOOP_ENABLED_MIN_BYTES {
                SampleLoopKind::Forward
            } else {
                SampleLoopKind::None
            },
            volume,
            panning: SAMPLE_DEFAULT_PANNING,
            flags: MOD_SAMPLE_8_BIT_FLAGS,
            volume_fadeout: SAMPLE_DEFAULT_VOLUME_FADEOUT,
            sample_type: SAMPLE_DEFAULT_TYPE,
            finetune,
            relative_note: SAMPLE_DEFAULT_RELATIVE_NOTE,
            data: SampleData::Empty,
        };

        // Correct loops like MilkyTracker does:
        if sample.loop_start + sample.loop_length > sample.length {
            let diff = (sample.loop_start + sample.loop_length).saturating_sub(sample.length);
            sample.loop_start = sample.loop_start.saturating_sub(diff);
            if sample.loop_start + sample.loop_length > sample.length {
                let diff2 = (sample.loop_start + sample.loop_length).saturating_sub(sample.length);
                sample.loop_length = sample.loop_length.saturating_sub(diff2);
            }
        }
        if sample.loop_length <= MOD_LOOP_ENABLED_MIN_BYTES {
            sample.loop_length = 0;
            sample.loop_kind = SampleLoopKind::None;
        }

        samples.push(sample);

        // Core instruments map note to sample index
        let note_sample_map = vec![Some(i); MAX_XM_NOTES as usize];
        let mut sample_slots = vec![None; SAMPLES_PER_INSTRUMENT];
        sample_slots[0] = Some(i);

        let instrument = Instrument {
            name: rustytracker_core::InstrumentName::new(&name),
            sample_slots,
            note_sample_map,
            volume_envelope: Envelope::default(),
            panning_envelope: Envelope::default(),
            vibrato: rustytracker_core::Vibrato::default(),
            volume_fadeout: SAMPLE_DEFAULT_VOLUME_FADEOUT,
        };
        instruments.push(instrument);
    }

    // 3. Read Orders
    let song_length = bytes[cursor] as usize;
    let restart_position = bytes[cursor + 1];
    cursor += MOD_SONG_LENGTH_RESTART_LEN;

    if song_length == 0 || song_length > MOD_ORDER_TABLE_LEN {
        return Err(ModParseError::InvalidOrderCount {
            orders: song_length,
            maximum: MOD_ORDER_TABLE_LEN,
        });
    }

    let order_list_bytes = &bytes[cursor..cursor + MOD_ORDER_TABLE_LEN];
    let orders = order_list_bytes[0..song_length].to_vec();
    cursor += MOD_ORDER_TABLE_LEN;

    if is_31_ins {
        cursor += MOD_SIGNATURE_LEN;
    }

    // Determine number of patterns
    let mut max_pattern = 0u8;
    for &pat in &orders {
        if pat > max_pattern {
            max_pattern = pat;
        }
    }
    let num_patterns = max_pattern as usize + 1;

    // 4. Read Patterns
    let pattern_size = channel_count as usize * MOD_PATTERN_ROWS as usize * MOD_CELL_BYTES;
    let total_patterns_len = num_patterns * pattern_size;
    if bytes.len() < cursor + total_patterns_len {
        return Err(ModParseError::Truncated {
            expected: cursor + total_patterns_len,
            actual: bytes.len(),
        });
    }

    let mut patterns = Vec::new();
    for _ in 0..num_patterns {
        let mut pattern = Pattern::new(MOD_PATTERN_ROWS, channel_count, DEFAULT_EFFECT_SLOTS);
        let pat_bytes = &bytes[cursor..cursor + pattern_size];
        cursor += pattern_size;

        let mut byte_idx = 0;
        for r in 0..MOD_PATTERN_ROWS {
            for c in 0..channel_count {
                let b1 = pat_bytes[byte_idx];
                let b2 = pat_bytes[byte_idx + 1];
                let b3 = pat_bytes[byte_idx + 2];
                let b4 = pat_bytes[byte_idx + 3];
                byte_idx += MOD_CELL_BYTES;

                let note_period = (((b1 & 0x0f) as u16) << 8) | b2 as u16;
                let ins_num = (b1 & 0xf0) | (b3 >> 4);
                let mut effect = b3 & 0x0f;
                let mut operand = b4;

                // Adjust effects
                if effect == 0x0e {
                    effect = (operand >> 4) + 0x30;
                    operand &= 0x0f;
                } else if effect == 0x00 && operand != 0 {
                    effect = 0x20; // Arpeggio (nonzero)
                } else if (effect == 0x01 || effect == 0x02 || effect == 0x0a) && operand == 0 {
                    effect = 0;
                } else if effect == 0x05 && operand == 0 {
                    effect = 0x03;
                } else if effect == 0x06 && operand == 0 {
                    effect = 0x04;
                } else if effect == 0x0c {
                    operand = vol64_to_255(operand);
                }

                let note = if note_period > 0 {
                    let notenum = amiga_period_to_note(note_period);
                    if notenum > 0 {
                        Note::Key(notenum)
                    } else {
                        Note::Empty
                    }
                } else {
                    Note::Empty
                };

                let cell = PatternCell {
                    note,
                    instrument: ins_num,
                    effects: vec![EffectCommand { effect, operand }, EffectCommand::default()],
                };
                pattern
                    .set_cell(c, r, cell)
                    .expect("cell indices must be in bounds");
            }
        }
        patterns.push(pattern);
    }

    // 5. Read Sample Data (8-bit signed PCM)
    for sample in samples.iter_mut().take(instrument_count) {
        if sample.length > 0 {
            if bytes.len() < cursor + sample.length as usize {
                return Err(ModParseError::Truncated {
                    expected: cursor + sample.length as usize,
                    actual: bytes.len(),
                });
            }
            let sample_bytes = &bytes[cursor..cursor + sample.length as usize];
            cursor += sample.length as usize;

            let mut data_vec = vec![0i8; sample.length as usize];
            for j in 0..sample.length as usize {
                data_vec[j] = sample_bytes[j] as i8;
            }
            sample.data = SampleData::pcm8(data_vec);
        }
    }

    // Pad instruments and samples to standard core counts:
    while instruments.len() < DEFAULT_INSTRUMENTS {
        instruments.push(rustytracker_core::Instrument::empty(instruments.len()));
    }
    while samples.len() < DEFAULT_SAMPLE_COUNT {
        samples.push(Sample::default());
    }

    Ok(Module {
        header: rustytracker_core::ModuleHeader {
            title: rustytracker_core::ModuleTitle::new(&title),
            channel_count,
            frequency_table: rustytracker_core::FrequencyTable::Amiga,
            bpm: DEFAULT_BPM,
            tick_speed: DEFAULT_TICK_SPEED,
            main_volume: DEFAULT_MAIN_VOLUME,
            restart_position: restart_position as u16,
            is_mod: true,
        },
        orders,
        patterns,
        instruments,
        samples,
    })
}

fn get_pt_num_channels(sig: &[u8; MOD_SIGNATURE_LEN]) -> Option<u16> {
    if sig == b"M.K." || sig == b"M!K!" || sig == b"FLT4" {
        return Some(4);
    }
    if sig == b"FLT8" || sig == b"OKTA" || sig == b"OCTA" || sig == b"FA08" || sig == b"CD81" {
        return Some(8);
    }
    if sig[0] >= b'1' && sig[0] <= b'9' && &sig[1..4] == b"CHN" {
        return Some((sig[0] - b'0') as u16);
    }
    if sig[0] >= b'1'
        && sig[0] <= b'9'
        && sig[1] >= b'0'
        && sig[1] <= b'9'
        && (&sig[2..4] == b"CH" || &sig[2..4] == b"CN")
    {
        return Some(((sig[0] - b'0') * 10 + (sig[1] - b'0')) as u16);
    }
    None
}

fn mod_finetunes(nibble: u8) -> i8 {
    let modfinetunes = [
        0, 16, 32, 48, 64, 80, 96, 112, -128, -112, -96, -80, -64, -48, -32, -16,
    ];
    modfinetunes[(nibble & 0x0f) as usize]
}

const VOL64_TO_255_SCALE: u32 = 261_120;
const VOL64_TO_255_ROUNDING: u32 = 65_535;
const VOL64_TO_255_SHIFT: u32 = 16;
const BYTE_MASK: u32 = 0xff;
const XM_VOLUME_MAX: u8 = 64;

fn vol64_to_255(volume: u8) -> u8 {
    (((volume.min(XM_VOLUME_MAX) as u32 * VOL64_TO_255_SCALE + VOL64_TO_255_ROUNDING)
        >> VOL64_TO_255_SHIFT)
        & BYTE_MASK) as u8
}

fn amiga_period_to_note(period: u16) -> u8 {
    if period == 0 {
        return 0;
    }
    let periods = [
        1712, 1616, 1524, 1440, 1356, 1280, 1208, 1140, 1076, 1016, 960, 907,
    ];
    for y in 0..120 {
        let per = ((periods[y % 12] * 16) >> (y / 12)) >> 2;
        if period >= per as u16 {
            return (y + 1) as u8;
        }
    }
    0
}

fn clean_string(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes {
        if b == 0 {
            break;
        }
        if b.is_ascii() && !b.is_ascii_control() {
            s.push(b as char);
        } else {
            s.push(' ');
        }
    }
    s.trim_end().to_owned()
}

pub fn write_mod_module(module: &Module) -> Result<Vec<u8>, ModWriteError> {
    if module.header.channel_count < MIN_CHANNEL_COUNT
        || module.header.channel_count > MOD_MAX_CHANNELS
    {
        return Err(ModWriteError::TooManyChannels {
            channels: module.header.channel_count,
        });
    }

    if module.orders.is_empty() || module.orders.len() > MOD_ORDER_TABLE_LEN {
        return Err(ModWriteError::TooManyOrders {
            orders: module.orders.len(),
        });
    }

    // Determine number of patterns to write
    let mut max_pattern = 0u8;
    for &pat in &module.orders {
        if pat > max_pattern {
            max_pattern = pat;
        }
    }
    let num_patterns = max_pattern as usize + 1;

    if num_patterns > MOD_MAX_PATTERNS {
        return Err(ModWriteError::TooManyPatterns {
            patterns: num_patterns,
        });
    }

    let mut bytes = Vec::new();

    // 1. Title
    bytes.extend_from_slice(&pad_string(module.header.title.as_str(), MOD_TITLE_LEN));

    // 2. Instrument/Sample Headers
    let mut sample_data_to_write = Vec::new();

    for i in 0..MOD_31_INSTRUMENT_COUNT {
        let name = if let Some(ins) = module.instruments.get(i) {
            ins.name.as_str()
        } else {
            ""
        };
        bytes.extend_from_slice(&pad_string(name, MOD_INSTRUMENT_NAME_LEN));

        let mut length_words = 0u16;
        let mut finetune_nibble = 0u8;
        let mut volume_64 = 0u8;
        let loop_start_words;
        let loop_length_words;

        if let Some((sample_index, sample)) = mod_instrument_sample(module, i)? {
            let sample_bytes = match &sample.data {
                SampleData::Pcm8(data) => {
                    let mut data = (**data).clone();
                    if !data.len().is_multiple_of(2) {
                        data.push(0);
                    }
                    data
                }
                SampleData::Pcm16(data) => {
                    let mut data_8: Vec<i8> = data.iter().map(|&val| (val >> 8) as i8).collect();
                    if !data_8.len().is_multiple_of(2) {
                        data_8.push(0);
                    }
                    data_8
                }
                SampleData::Empty => Vec::new(),
            };

            let mut final_bytes = vec![0u8; sample_bytes.len()];
            for (j, &val) in sample_bytes.iter().enumerate() {
                final_bytes[j] = val as u8;
            }

            if final_bytes.len() > MOD_MAX_SAMPLE_BYTES {
                return Err(ModWriteError::SampleTooLong {
                    sample_index,
                    byte_len: final_bytes.len(),
                    maximum: MOD_MAX_SAMPLE_BYTES,
                });
            }

            if !final_bytes.is_empty() {
                length_words = (final_bytes.len() / 2) as u16;
                finetune_nibble = finetune_to_nibble(sample.finetune);
                volume_64 = vol255_to_64(sample.volume);

                if sample.loop_kind != SampleLoopKind::None
                    && sample.loop_length > MOD_LOOP_ENABLED_MIN_BYTES
                {
                    let start = (sample.loop_start / MOD_SAMPLE_LENGTH_WORD_BYTES) as u16;
                    let len = (sample.loop_length / MOD_SAMPLE_LENGTH_WORD_BYTES) as u16;
                    loop_start_words = start;
                    loop_length_words = len;
                } else {
                    loop_start_words = MOD_EMPTY_LOOP_START_WORDS;
                    loop_length_words = MOD_EMPTY_LOOP_LENGTH_WORDS;
                }

                sample_data_to_write.push(final_bytes);
            } else {
                loop_start_words = MOD_EMPTY_LOOP_START_WORDS;
                loop_length_words = MOD_EMPTY_LOOP_LENGTH_WORDS;
                sample_data_to_write.push(Vec::new());
            }
        } else {
            loop_start_words = MOD_EMPTY_LOOP_START_WORDS;
            loop_length_words = MOD_EMPTY_LOOP_LENGTH_WORDS;
            sample_data_to_write.push(Vec::new());
        }

        bytes.extend_from_slice(&length_words.to_be_bytes());
        bytes.push(finetune_nibble);
        bytes.push(volume_64);
        bytes.extend_from_slice(&loop_start_words.to_be_bytes());
        bytes.extend_from_slice(&loop_length_words.to_be_bytes());
    }

    // 3. Song length and restart position
    bytes.push(module.orders.len() as u8);
    bytes.push((module.header.restart_position & MOD_RESTART_POSITION_MASK) as u8);

    // 4. Order List (128 bytes)
    let mut order_table = [0u8; MOD_ORDER_TABLE_LEN];
    for (j, &pat) in module.orders.iter().enumerate() {
        order_table[j] = pat;
    }
    bytes.extend_from_slice(&order_table);

    // 5. Signature (4 bytes)
    let sig = get_mod_signature(module.header.channel_count);
    bytes.extend_from_slice(&sig);

    // 6. Pattern Data
    for p in 0..num_patterns {
        let pattern = module
            .patterns
            .get(p)
            .ok_or(ModWriteError::MissingPattern { pattern_index: p })?;

        if pattern.rows() != MOD_PATTERN_ROWS || pattern.channels() < module.header.channel_count {
            return Err(ModWriteError::InvalidPatternShape {
                pattern_index: p,
                rows: pattern.rows(),
                channels: pattern.channels(),
                required_rows: MOD_PATTERN_ROWS,
                required_channels: module.header.channel_count,
            });
        }

        for r in 0..MOD_PATTERN_ROWS {
            for c in 0..module.header.channel_count {
                let cell = pattern
                    .cell(c, r)
                    .expect("writer validated pattern shape before walking cells");

                let note_val = cell.note.raw();
                let note_period = note_to_amiga_period(note_val);

                if cell.instrument > MOD_MAX_INSTRUMENT_NUMBER {
                    return Err(ModWriteError::UnsupportedInstrument {
                        pattern_index: p,
                        row: r,
                        channel: c,
                        instrument: cell.instrument,
                        maximum: MOD_MAX_INSTRUMENT_NUMBER,
                    });
                }
                let ins_num = cell.instrument;

                for (effect_slot, effect) in cell.effects.iter().enumerate().skip(1) {
                    if *effect != EffectCommand::default() {
                        return Err(ModWriteError::UnsupportedExtraEffect {
                            pattern_index: p,
                            row: r,
                            channel: c,
                            effect_slot,
                        });
                    }
                }

                let primary_effect = cell.effects.first().copied().unwrap_or_default();
                let (effect, operand) =
                    effect_to_mod(primary_effect.effect, primary_effect.operand);

                let b1 = (ins_num & 0x10) | (((note_period >> 8) & 0x0f) as u8);
                let b2 = (note_period & 0xff) as u8;
                let b3 = ((ins_num & 0x0f) << 4) | (effect & 0x0f);
                let b4 = operand;

                bytes.push(b1);
                bytes.push(b2);
                bytes.push(b3);
                bytes.push(b4);
            }
        }
    }

    // 7. Sample Data (signed 8-bit PCM)
    for sample_bytes in sample_data_to_write {
        bytes.extend_from_slice(&sample_bytes);
    }

    Ok(bytes)
}

fn mod_instrument_sample(
    module: &Module,
    instrument_index: usize,
) -> Result<Option<(usize, &Sample)>, ModWriteError> {
    let Some(instrument) = module.instruments.get(instrument_index) else {
        return Ok(None);
    };
    let Some(sample_index) = instrument.sample_slots.first().and_then(|slot| *slot) else {
        return Ok(None);
    };
    let Some(sample) = module.samples.get(sample_index) else {
        return Err(ModWriteError::MissingSample {
            instrument_index,
            sample_index,
        });
    };

    Ok(Some((sample_index, sample)))
}

fn pad_string(s: &str, len: usize) -> Vec<u8> {
    let mut bytes = s.as_bytes().to_vec();
    if bytes.len() > len {
        bytes.truncate(len);
    } else {
        bytes.resize(len, 0);
    }
    bytes
}

fn finetune_to_nibble(finetune: i8) -> u8 {
    let modfinetunes = [
        0, 16, 32, 48, 64, 80, 96, 112, -128, -112, -96, -80, -64, -48, -32, -16,
    ];
    let mut best_index = 0;
    let mut min_diff = i32::MAX;
    for (i, &val) in modfinetunes.iter().enumerate() {
        let diff = (finetune as i32 - val).abs();
        if diff < min_diff {
            min_diff = diff;
            best_index = i;
        }
    }
    best_index as u8
}

fn vol255_to_64(vol: u8) -> u8 {
    (((vol as u32 * 64 + 128) / 255) as u8).min(64)
}

fn note_to_amiga_period(note_num: u8) -> u16 {
    if note_num == 0 || note_num > 120 {
        return 0;
    }
    let periods = [
        1712, 1616, 1524, 1440, 1356, 1280, 1208, 1140, 1076, 1016, 960, 907,
    ];
    let y = (note_num - 1) as usize;
    let per = ((periods[y % 12] * 16) >> (y / 12)) >> 2;
    per as u16
}

fn effect_to_mod(effect: u8, operand: u8) -> (u8, u8) {
    if (0x30..=0x3f).contains(&effect) {
        let cmd = effect - 0x30;
        (0x0e, (cmd << 4) | (operand & 0x0f))
    } else if effect == 0x20 {
        (0x00, operand)
    } else if effect == 0x0c {
        (0x0c, vol255_to_64(operand))
    } else {
        (effect, operand)
    }
}

fn get_mod_signature(channel_count: u16) -> [u8; 4] {
    match channel_count {
        4 => *b"M.K.",
        6 => *b"6CHN",
        8 => *b"8CHN",
        c if (1..=9).contains(&c) => {
            let ch_char = b'0' + c as u8;
            [ch_char, b'C', b'H', b'N']
        }
        c if (10..=99).contains(&c) => {
            let tens = b'0' + (c / 10) as u8;
            let ones = b'0' + (c % 10) as u8;
            [tens, ones, b'C', b'H']
        }
        _ => *b"M.K.", // fallback
    }
}
