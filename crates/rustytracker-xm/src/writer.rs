use rustytracker_core::{
    EffectCommand, Envelope as CoreEnvelope, EnvelopePoint as CoreEnvelopePoint, FrequencyTable,
    Instrument, Module, Note, Pattern, PatternCell, Sample, SampleData as CoreSampleData,
    SampleLoopKind, Vibrato as CoreVibrato, EDITOR_PATTERN_CHANNELS, INTERNAL_EFFECT_EXTENDED_BASE,
    INTERNAL_EFFECT_EXTENDED_MAX, INTERNAL_EFFECT_EXTRA_FINE_PORTA_BASE,
    INTERNAL_EFFECT_EXTRA_FINE_PORTA_MAX, INTERNAL_EFFECT_EXTRA_FINE_PORTA_MIN,
    INTERNAL_EFFECT_NONZERO_ARPEGGIO, MAX_ACTIVE_ORDERS, MAX_INSTRUMENTS, MAX_PATTERNS,
    MIN_CHANNEL_COUNT, SAMPLES_PER_INSTRUMENT, SAMPLE_DEFAULT_VOLUME_FADEOUT, XM_EFFECT_EXTENDED,
};

use crate::*;

pub fn write_xm_header(module: &Module) -> XmWriteResult<Vec<u8>> {
    if module.orders.is_empty() {
        return Err(XmWriteError::EmptyOrderList);
    }

    if module.orders.len() > MAX_ACTIVE_ORDERS {
        return Err(XmWriteError::TooManyOrders {
            requested: module.orders.len(),
            maximum: MAX_ACTIVE_ORDERS,
        });
    }

    if !(MIN_CHANNEL_COUNT..=EDITOR_PATTERN_CHANNELS).contains(&module.header.channel_count) {
        return Err(XmWriteError::InvalidChannelCount {
            channel_count: module.header.channel_count,
            minimum: MIN_CHANNEL_COUNT,
            maximum: EDITOR_PATTERN_CHANNELS,
        });
    }

    if module.patterns.len() > MAX_PATTERNS {
        return Err(XmWriteError::TooManyPatterns {
            requested: module.patterns.len(),
            maximum: MAX_PATTERNS,
        });
    }

    if module.instruments.len() > MAX_INSTRUMENTS {
        return Err(XmWriteError::TooManyInstruments {
            requested: module.instruments.len(),
            maximum: MAX_INSTRUMENTS,
        });
    }

    let mut bytes = vec![ASCII_NUL; XM_MIN_HEADER_BYTES];

    bytes[..XM_HEADER_SIGNATURE_LENGTH].copy_from_slice(XM_HEADER_SIGNATURE);
    bytes[MARKER_OFFSET] = XM_MARKER;
    write_fixed_text(
        &mut bytes[TITLE_OFFSET..TITLE_OFFSET + TITLE_LEN],
        module.header.title.as_str(),
    );
    write_fixed_text(
        &mut bytes[TRACKER_OFFSET..TRACKER_OFFSET + TRACKER_LEN],
        XM_WRITER_TRACKER_NAME,
    );
    write_u16(&mut bytes, VERSION_OFFSET, XM_VERSION_1_04);
    write_u32(&mut bytes, HEADER_SIZE_OFFSET, XM_WRITER_HEADER_SIZE);
    write_u16(&mut bytes, HEADER_FIELDS_OFFSET, module.orders.len() as u16);
    write_u16(
        &mut bytes,
        XM_RESTART_FIELD_OFFSET,
        module.header.restart_position,
    );
    write_u16(
        &mut bytes,
        XM_CHANNELS_FIELD_OFFSET,
        module.header.channel_count,
    );
    write_u16(
        &mut bytes,
        XM_PATTERNS_FIELD_OFFSET,
        module.patterns.len() as u16,
    );
    write_u16(
        &mut bytes,
        XM_INSTRUMENTS_FIELD_OFFSET,
        module.instruments.len() as u16,
    );
    write_u16(
        &mut bytes,
        XM_FLAGS_FIELD_OFFSET,
        match module.header.frequency_table {
            FrequencyTable::Amiga => XM_WRITER_AMIGA_FLAGS,
            FrequencyTable::Linear => XM_LINEAR_FREQUENCY_FLAG,
        },
    );
    write_u16(
        &mut bytes,
        XM_TICK_SPEED_FIELD_OFFSET,
        module.header.tick_speed,
    );
    write_u16(&mut bytes, XM_BPM_FIELD_OFFSET, module.header.bpm);

    bytes[ORDER_TABLE_OFFSET..ORDER_TABLE_OFFSET + XM_ORDER_TABLE_LEN].fill(XM_WRITER_EMPTY_ORDER);
    bytes[ORDER_TABLE_OFFSET..ORDER_TABLE_OFFSET + module.orders.len()]
        .copy_from_slice(&module.orders);

    Ok(bytes)
}

pub fn write_xm_module(module: &Module) -> XmWriteResult<Vec<u8>> {
    let mut bytes = write_xm_header(module)?;
    bytes.extend_from_slice(&write_xm_patterns(module)?);
    bytes.extend_from_slice(&write_xm_instruments(module)?);
    Ok(bytes)
}

pub fn write_xm_patterns(module: &Module) -> XmWriteResult<Vec<u8>> {
    let mut bytes = Vec::new();

    for (pattern_index, pattern) in module.patterns.iter().enumerate() {
        validate_xm_pattern_shape(module, pattern_index, pattern)?;

        let data = if pattern_is_empty(pattern, module.header.channel_count) {
            Vec::new()
        } else {
            write_xm_pattern_data(pattern, module.header.channel_count)
        };

        if data.len() > U16_MAX_AS_USIZE {
            return Err(XmWriteError::PatternDataTooLong {
                pattern_index,
                byte_len: data.len(),
                maximum: U16_MAX_AS_USIZE,
            });
        }

        let header_offset = bytes.len();
        bytes.resize(header_offset + XM_PATTERN_HEADER_LEN, ASCII_NUL);
        write_u32(&mut bytes, header_offset, XM_WRITER_PATTERN_HEADER_LEN);
        bytes[header_offset + XM_PATTERN_TYPE_OFFSET] = XM_WRITER_PATTERN_PACKING_TYPE;
        write_u16(
            &mut bytes,
            header_offset + XM_PATTERN_ROWS_OFFSET,
            pattern.rows(),
        );
        write_u16(
            &mut bytes,
            header_offset + XM_PATTERN_DATA_LEN_OFFSET,
            data.len() as u16,
        );
        bytes.extend_from_slice(&data);
    }

    Ok(bytes)
}

fn validate_xm_pattern_shape(
    module: &Module,
    pattern_index: usize,
    pattern: &Pattern,
) -> XmWriteResult<()> {
    let channel_count = module.header.channel_count;
    if pattern.channels() < channel_count {
        return Err(XmWriteError::InvalidPatternShape {
            pattern_index,
            channels: pattern.channels(),
            required_channels: channel_count,
        });
    }

    for row in 0..pattern.rows() {
        for channel in channel_count..pattern.channels() {
            let cell = pattern
                .cell(channel, row)
                .expect("writer walks cells inside pattern bounds");
            if !cell_is_empty(cell) {
                return Err(XmWriteError::PatternDataOutsideChannelCount {
                    pattern_index,
                    row,
                    channel,
                    channel_count,
                });
            }
        }
    }

    Ok(())
}

pub fn write_xm_instruments(module: &Module) -> XmWriteResult<Vec<u8>> {
    let mut bytes = Vec::new();

    for (instrument_index, instrument) in module.instruments.iter().enumerate() {
        write_xm_instrument(&mut bytes, module, instrument_index, instrument)?;
    }

    Ok(bytes)
}

fn active_xm_sample_count(module: &Module, instrument: &Instrument) -> usize {
    instrument
        .sample_slots
        .iter()
        .enumerate()
        .rev()
        .find(|(_, sample_index)| {
            sample_index
                .and_then(|sample_index| module.samples.get(sample_index))
                .is_some_and(sample_is_active)
        })
        .map(|(sample_index, _)| sample_index + 1)
        .unwrap_or_default()
}

fn sample_is_active(sample: &Sample) -> bool {
    sample != &Sample::default()
}

fn instrument_needs_extension_header(sample_count: usize, instrument: &Instrument) -> bool {
    sample_count != XM_WRITER_EMPTY_INSTRUMENT_SAMPLE_COUNT
        || zero_sample_instrument_has_extension_metadata(instrument)
}

fn zero_sample_instrument_has_extension_metadata(instrument: &Instrument) -> bool {
    instrument.volume_envelope != CoreEnvelope::default()
        || instrument.panning_envelope != CoreEnvelope::default()
        || instrument.vibrato != CoreVibrato::default()
        || instrument.volume_fadeout != SAMPLE_DEFAULT_VOLUME_FADEOUT
}

fn write_xm_instrument(
    bytes: &mut Vec<u8>,
    module: &Module,
    instrument_index: usize,
    instrument: &Instrument,
) -> XmWriteResult<()> {
    let sample_count = active_xm_sample_count(module, instrument);
    if sample_count > SAMPLES_PER_INSTRUMENT {
        return Err(XmWriteError::TooManyInstrumentSamples {
            instrument_index,
            requested: sample_count,
            maximum: SAMPLES_PER_INSTRUMENT,
        });
    }

    let has_extension_header = instrument_needs_extension_header(sample_count, instrument);
    let header_size = if has_extension_header {
        XM_WRITER_INSTRUMENT_HEADER_SIZE
    } else {
        XM_WRITER_EMPTY_INSTRUMENT_HEADER_SIZE
    };
    let instrument_offset = bytes.len();
    bytes.resize(instrument_offset + header_size as usize, ASCII_NUL);

    write_u32(bytes, instrument_offset, header_size);
    write_fixed_text(
        &mut bytes[instrument_offset + XM_INSTRUMENT_SIZE_LEN
            ..instrument_offset + XM_INSTRUMENT_SIZE_LEN + XM_INSTRUMENT_NAME_LEN],
        instrument.name.as_str(),
    );
    bytes[instrument_offset + XM_INSTRUMENT_SIZE_LEN + XM_INSTRUMENT_TYPE_OFFSET] =
        XM_WRITER_INSTRUMENT_TYPE;
    write_u16(
        bytes,
        instrument_offset + XM_INSTRUMENT_SIZE_LEN + XM_INSTRUMENT_SAMPLE_COUNT_OFFSET,
        sample_count as u16,
    );

    if !has_extension_header {
        return Ok(());
    }

    let sample_header_size_offset =
        instrument_offset + XM_WRITER_EMPTY_INSTRUMENT_HEADER_SIZE as usize;
    write_u32(
        bytes,
        sample_header_size_offset,
        XM_WRITER_SAMPLE_HEADER_SIZE,
    );
    write_xm_instrument_extension(
        bytes,
        sample_header_size_offset + XM_SAMPLE_HEADER_SIZE_LEN,
        instrument,
        sample_count,
    );

    for sample_index in 0..sample_count {
        write_xm_sample_header(bytes, module, instrument_index, instrument, sample_index)?;
    }

    for sample_index in 0..sample_count {
        write_xm_sample_payload(bytes, module, instrument, sample_index);
    }

    Ok(())
}

fn write_xm_instrument_extension(
    bytes: &mut [u8],
    extension_offset: usize,
    instrument: &Instrument,
    sample_count: usize,
) {
    let mut offset = extension_offset;

    write_xm_note_sample_map(bytes, offset, instrument, sample_count);
    offset += XM_NOTE_SAMPLE_MAP_LEN;

    write_xm_envelope_points(bytes, offset, &instrument.volume_envelope);
    offset += XM_ENVELOPE_POINT_COUNT * XM_ENVELOPE_POINT_BYTES;
    write_xm_envelope_points(bytes, offset, &instrument.panning_envelope);
    offset += XM_ENVELOPE_POINT_COUNT * XM_ENVELOPE_POINT_BYTES;

    bytes[offset] = instrument
        .volume_envelope
        .point_count
        .min(XM_ENVELOPE_POINT_COUNT_MAX);
    offset += BYTE_1_OFFSET;
    bytes[offset] = instrument
        .panning_envelope
        .point_count
        .min(XM_ENVELOPE_POINT_COUNT_MAX);
    offset += BYTE_1_OFFSET;
    bytes[offset] = instrument.volume_envelope.sustain_point;
    offset += BYTE_1_OFFSET;
    bytes[offset] = instrument.volume_envelope.loop_start_point;
    offset += BYTE_1_OFFSET;
    bytes[offset] = instrument.volume_envelope.loop_end_point;
    offset += BYTE_1_OFFSET;
    bytes[offset] = instrument.panning_envelope.sustain_point;
    offset += BYTE_1_OFFSET;
    bytes[offset] = instrument.panning_envelope.loop_start_point;
    offset += BYTE_1_OFFSET;
    bytes[offset] = instrument.panning_envelope.loop_end_point;
    offset += BYTE_1_OFFSET;
    bytes[offset] = instrument.volume_envelope.flags;
    offset += BYTE_1_OFFSET;
    bytes[offset] = instrument.panning_envelope.flags;
    offset += BYTE_1_OFFSET;

    bytes[offset] = instrument.vibrato.waveform;
    offset += BYTE_1_OFFSET;
    bytes[offset] = instrument.vibrato.sweep;
    offset += BYTE_1_OFFSET;
    bytes[offset] = instrument.vibrato.depth >> XM_VIBRATO_DEPTH_SHIFT;
    offset += BYTE_1_OFFSET;
    bytes[offset] = instrument.vibrato.rate;
    offset += BYTE_1_OFFSET;
    write_u16(
        bytes,
        offset,
        instrument.volume_fadeout >> XM_VOLUME_FADEOUT_SHIFT,
    );
}

fn write_xm_note_sample_map(
    bytes: &mut [u8],
    offset: usize,
    instrument: &Instrument,
    sample_count: usize,
) {
    for note_index in 0..XM_NOTE_SAMPLE_MAP_LEN {
        bytes[offset + note_index] = instrument
            .note_sample_map
            .get(note_index)
            .and_then(|sample_index| *sample_index)
            .and_then(|sample_index| xm_sample_slot_for_core_sample(instrument, sample_index))
            .filter(|sample_index| *sample_index < sample_count)
            .map(|sample_index| sample_index as u8)
            .unwrap_or_default();
    }
}

fn xm_sample_slot_for_core_sample(
    instrument: &Instrument,
    core_sample_index: usize,
) -> Option<usize> {
    instrument
        .sample_slots
        .iter()
        .position(|sample_index| *sample_index == Some(core_sample_index))
}

fn write_xm_envelope_points(bytes: &mut [u8], offset: usize, envelope: &CoreEnvelope) {
    for point_index in 0..XM_ENVELOPE_POINT_COUNT {
        let point_offset = offset + point_index * XM_ENVELOPE_POINT_BYTES;
        let point = envelope
            .points
            .get(point_index)
            .copied()
            .unwrap_or(CoreEnvelopePoint {
                frame: XM_WRITER_EMPTY_ENVELOPE_FRAME,
                value: XM_WRITER_EMPTY_ENVELOPE_VALUE,
            });

        write_u16(bytes, point_offset + XM_ENVELOPE_X_OFFSET, point.frame);
        write_u16(
            bytes,
            point_offset + XM_ENVELOPE_Y_OFFSET,
            point.value >> XM_ENVELOPE_VALUE_SHIFT,
        );
    }
}

fn write_xm_sample_header(
    bytes: &mut Vec<u8>,
    module: &Module,
    instrument_index: usize,
    instrument: &Instrument,
    sample_index: usize,
) -> XmWriteResult<()> {
    let sample = xm_sample_for_slot(module, instrument, sample_index);
    let header_offset = bytes.len();
    bytes.resize(header_offset + XM_SAMPLE_HEADER_LEN, ASCII_NUL);

    if let Some(sample) = sample {
        let sample_byte_len =
            xm_sample_data_byte_len(&sample.data, instrument_index, sample_index)?;
        let loop_start_byte_len = xm_sample_frame_count_to_byte_len(
            sample.loop_start,
            &sample.data,
            instrument_index,
            sample_index,
            XmSampleField::LoopStart,
        )?;
        let loop_length_byte_len = xm_sample_frame_count_to_byte_len(
            sample.loop_length,
            &sample.data,
            instrument_index,
            sample_index,
            XmSampleField::LoopLength,
        )?;
        let mut cursor = header_offset;
        write_u32(bytes, cursor, sample_byte_len);
        cursor += XM_SAMPLE_LENGTH_LEN;
        write_u32(bytes, cursor, loop_start_byte_len);
        cursor += XM_SAMPLE_LOOP_START_LEN;
        write_u32(bytes, cursor, loop_length_byte_len);
        cursor += XM_SAMPLE_LOOP_LENGTH_LEN;
        bytes[cursor] = vol255_to_64(sample.volume);
        cursor += XM_SAMPLE_VOLUME_LEN;
        bytes[cursor] = sample.finetune as u8;
        cursor += XM_SAMPLE_FINETUNE_LEN;
        bytes[cursor] = xm_sample_type(sample);
        cursor += XM_SAMPLE_TYPE_LEN;
        bytes[cursor] = sample.panning;
        cursor += XM_SAMPLE_PANNING_LEN;
        bytes[cursor] = sample.relative_note as u8;
        cursor += XM_SAMPLE_RELATIVE_NOTE_LEN;
        bytes[cursor] = XM_WRITER_SAMPLE_RESERVED;
        cursor += XM_SAMPLE_RESERVED_LEN;
        write_fixed_text(
            &mut bytes[cursor..cursor + XM_SAMPLE_NAME_LEN],
            sample.name.as_str(),
        );
    }

    Ok(())
}

fn xm_sample_type(sample: &Sample) -> u8 {
    xm_sample_data_type(sample) | xm_sample_loop_kind(sample.loop_kind)
}

fn xm_sample_data_type(sample: &Sample) -> u8 {
    match &sample.data {
        CoreSampleData::Empty => sample.sample_type & XM_SAMPLE_NON_LOOP_TYPE_MASK,
        CoreSampleData::Pcm8(_) => XM_SAMPLE_8_BIT_FLAG,
        CoreSampleData::Pcm16(_) => XM_SAMPLE_16_BIT_FLAG,
    }
}

fn xm_sample_loop_kind(loop_kind: SampleLoopKind) -> u8 {
    match loop_kind {
        SampleLoopKind::None => XM_SAMPLE_LOOP_NONE,
        SampleLoopKind::Forward => XM_SAMPLE_LOOP_FORWARD,
        SampleLoopKind::PingPong => XM_SAMPLE_LOOP_PING_PONG,
    }
}

fn xm_sample_for_slot<'a>(
    module: &'a Module,
    instrument: &Instrument,
    sample_index: usize,
) -> Option<&'a Sample> {
    instrument
        .sample_slots
        .get(sample_index)
        .and_then(|sample_index| *sample_index)
        .and_then(|sample_index| module.samples.get(sample_index))
}

fn xm_sample_data_byte_len(
    data: &CoreSampleData,
    instrument_index: usize,
    sample_index: usize,
) -> XmWriteResult<u32> {
    if matches!(data, CoreSampleData::Empty) {
        return Ok(XM_WRITER_EMPTY_SAMPLE_BYTE_LEN);
    }

    let frame_count = data.frame_count() as u64;
    let byte_len = frame_count.saturating_mul(xm_sample_bytes_per_frame(data) as u64);

    xm_u32_sample_field(
        byte_len,
        instrument_index,
        sample_index,
        XmSampleField::Length,
    )
}

fn xm_sample_frame_count_to_byte_len(
    frame_count: u32,
    data: &CoreSampleData,
    instrument_index: usize,
    sample_index: usize,
    field: XmSampleField,
) -> XmWriteResult<u32> {
    if matches!(data, CoreSampleData::Empty) {
        return Ok(XM_WRITER_EMPTY_SAMPLE_BYTE_LEN);
    }

    let byte_len = u64::from(frame_count) * xm_sample_bytes_per_frame(data) as u64;

    xm_u32_sample_field(byte_len, instrument_index, sample_index, field)
}

fn xm_u32_sample_field(
    value: u64,
    instrument_index: usize,
    sample_index: usize,
    field: XmSampleField,
) -> XmWriteResult<u32> {
    if value > U32_FIELD_MAX {
        return Err(XmWriteError::SampleFieldTooLarge {
            instrument_index,
            sample_index,
            field,
            value,
            maximum: U32_FIELD_MAX,
        });
    }

    Ok(value as u32)
}

fn xm_sample_bytes_per_frame(data: &CoreSampleData) -> usize {
    match data {
        CoreSampleData::Empty | CoreSampleData::Pcm8(_) => BYTES_PER_8_BIT_SAMPLE,
        CoreSampleData::Pcm16(_) => BYTES_PER_16_BIT_SAMPLE,
    }
}

fn write_xm_sample_payload(
    bytes: &mut Vec<u8>,
    module: &Module,
    instrument: &Instrument,
    sample_index: usize,
) {
    if let Some(sample) = xm_sample_for_slot(module, instrument, sample_index) {
        match &sample.data {
            CoreSampleData::Empty => {}
            CoreSampleData::Pcm8(values) => write_xm_delta8(bytes, values),
            CoreSampleData::Pcm16(values) => write_xm_delta16(bytes, values),
        }
    }
}

fn write_xm_delta8(bytes: &mut Vec<u8>, values: &[i8]) {
    let mut previous = XM_WRITER_DELTA_INITIAL_8;

    for &value in values {
        let delta = value.wrapping_sub(previous);
        bytes.push(delta as u8);
        previous = value;
    }
}

fn write_xm_delta16(bytes: &mut Vec<u8>, values: &[i16]) {
    let mut previous = XM_WRITER_DELTA_INITIAL_16;

    for &value in values {
        let delta = value.wrapping_sub(previous);
        bytes.extend_from_slice(&delta.to_le_bytes());
        previous = value;
    }
}

fn pattern_is_empty(pattern: &Pattern, channel_count: u16) -> bool {
    for row in 0..pattern.rows() {
        for channel in 0..channel_count {
            let cell = pattern
                .cell(channel, row)
                .expect("writer walks cells inside pattern bounds");
            if !cell_is_empty(cell) {
                return false;
            }
        }
    }

    true
}

fn cell_is_empty(cell: &PatternCell) -> bool {
    cell.note == Note::Empty
        && cell.instrument == EMPTY_OPERAND
        && cell
            .effects
            .iter()
            .all(|effect| *effect == EffectCommand::default())
}

fn write_xm_pattern_data(pattern: &Pattern, channel_count: u16) -> Vec<u8> {
    let mut bytes = Vec::new();

    for row in 0..pattern.rows() {
        for channel in 0..channel_count {
            let cell = pattern
                .cell(channel, row)
                .expect("writer walks cells inside pattern bounds");
            write_xm_cell(&mut bytes, cell);
        }
    }

    bytes
}

fn write_xm_cell(bytes: &mut Vec<u8>, cell: &PatternCell) {
    let (volume, effect) = xm_columns_from_core_effects(&cell.effects);

    bytes.push(core_note_to_xm(cell.note));
    bytes.push(cell.instrument);
    bytes.push(volume);
    bytes.push(effect.effect);
    bytes.push(effect.operand);
}

fn xm_columns_from_core_effects(effects: &[EffectCommand]) -> (u8, EffectCommand) {
    if effects.len() <= XM_WRITER_SINGLE_EFFECT_SLOT_COUNT {
        let effect = effects
            .iter()
            .rev()
            .find(|effect| **effect != EffectCommand::default())
            .copied()
            .map(core_effect_to_xm)
            .unwrap_or_default();

        return (XM_WRITER_EMPTY_VOLUME_COLUMN, effect);
    }

    let mut volume = XM_WRITER_EMPTY_VOLUME_COLUMN;
    let mut effect_column = EffectCommand::default();

    for (index, effect) in effects.iter().copied().enumerate() {
        if effect == EffectCommand::default() {
            continue;
        }

        let xm_effect = core_effect_to_xm(effect);

        if index == 0 {
            if !note_portamento_requires_effect_column(xm_effect) {
                if let Some(volume_column) = xm_effect_to_volume_column(xm_effect, true) {
                    volume = volume_column;
                    continue;
                }
            }

            if effect_column == EffectCommand::default() {
                effect_column = xm_effect;
                continue;
            }
        }

        if effect_column == EffectCommand::default() {
            effect_column = xm_effect;
        } else if volume == XM_WRITER_EMPTY_VOLUME_COLUMN {
            if let Some(volume_column) = xm_effect_to_volume_column(xm_effect, false) {
                volume = volume_column;
            }
        }
    }

    (volume, effect_column)
}

fn core_effect_to_xm(effect: EffectCommand) -> EffectCommand {
    match effect.effect {
        INTERNAL_EFFECT_NONZERO_ARPEGGIO => EffectCommand {
            effect: EMPTY_EFFECT,
            operand: effect.operand,
        },
        INTERNAL_EFFECT_EXTENDED_BASE..=INTERNAL_EFFECT_EXTENDED_MAX => EffectCommand {
            effect: XM_EFFECT_EXTENDED,
            operand: ((effect.effect - INTERNAL_EFFECT_EXTENDED_BASE) << XM_NIBBLE_SHIFT)
                | (effect.operand & XM_NIBBLE_MASK),
        },
        INTERNAL_EFFECT_EXTRA_FINE_PORTA_MIN..=INTERNAL_EFFECT_EXTRA_FINE_PORTA_MAX => {
            EffectCommand {
                effect: XM_EFFECT_EXTRA_FINE_PORTA,
                operand: ((effect.effect - INTERNAL_EFFECT_EXTRA_FINE_PORTA_BASE)
                    << XM_NIBBLE_SHIFT)
                    | effect.operand.min(XM_NIBBLE_MASK),
            }
        }
        XM_EFFECT_PROTRACKER_MIN..=XM_EFFECT_PROTRACKER_MAX => {
            let operand =
                if effect.effect == XM_EFFECT_VOLUME || effect.effect == XM_EFFECT_GLOBAL_VOLUME {
                    vol255_to_64(effect.operand)
                } else {
                    effect.operand
                };

            EffectCommand {
                effect: effect.effect,
                operand,
            }
        }
        _ => effect,
    }
}

fn xm_effect_to_volume_column(
    effect: EffectCommand,
    allow_fine_volume_slide_relocation: bool,
) -> Option<u8> {
    match effect.effect {
        XM_EFFECT_VOLUME => Some(XM_VOLUME_SET_MIN + effect.operand.min(XM_VOLUME_MAX)),
        XM_EFFECT_EXTENDED if allow_fine_volume_slide_relocation => {
            xm_extended_fine_volume_slide_column(effect.operand)
        }
        XM_EFFECT_EXTENDED => None,
        INTERNAL_EFFECT_VOLUME_SLIDE => xm_volume_slide_column(effect.operand),
        INTERNAL_EFFECT_VIBRATO_COMPAT => xm_vibrato_column(effect.operand),
        INTERNAL_EFFECT_PANNING => Some(volume_command(
            XM_VOLUME_SET_PANNING,
            effect.operand >> XM_NIBBLE_SHIFT,
        )),
        INTERNAL_EFFECT_PANNING_SLIDE => xm_panning_slide_column(effect.operand),
        INTERNAL_EFFECT_TONE_PORTAMENTO => {
            if note_portamento_requires_effect_column(effect) {
                None
            } else {
                Some(volume_command(
                    XM_VOLUME_TONE_PORTAMENTO,
                    effect.operand >> XM_NIBBLE_SHIFT,
                ))
            }
        }
        _ => None,
    }
}

fn xm_volume_slide_column(operand: u8) -> Option<u8> {
    let low = operand & XM_NIBBLE_MASK;
    let high = operand >> XM_NIBBLE_SHIFT;

    if low != EMPTY_OPERAND && high == EMPTY_OPERAND {
        Some(volume_command(XM_VOLUME_SLIDE_DOWN, low))
    } else if high != EMPTY_OPERAND && low == EMPTY_OPERAND {
        Some(volume_command(XM_VOLUME_SLIDE_UP, high))
    } else {
        None
    }
}

fn xm_extended_fine_volume_slide_column(operand: u8) -> Option<u8> {
    let command = operand >> XM_NIBBLE_SHIFT;
    let amount = operand & XM_NIBBLE_MASK;

    if amount == EMPTY_OPERAND {
        return None;
    }

    match command {
        XM_EXTENDED_FINE_VOLUME_SLIDE_DOWN_COMMAND => {
            Some(volume_command(XM_VOLUME_FINE_DOWN, amount))
        }
        XM_EXTENDED_FINE_VOLUME_SLIDE_UP_COMMAND => Some(volume_command(XM_VOLUME_FINE_UP, amount)),
        _ => None,
    }
}

fn xm_vibrato_column(operand: u8) -> Option<u8> {
    let low = operand & XM_NIBBLE_MASK;
    let high = operand >> XM_NIBBLE_SHIFT;

    if high != EMPTY_OPERAND && low == EMPTY_OPERAND {
        Some(volume_command(XM_VOLUME_SET_VIBRATO_SPEED, high))
    } else if high == EMPTY_OPERAND {
        Some(volume_command(XM_VOLUME_VIBRATO, low))
    } else {
        None
    }
}

fn xm_panning_slide_column(operand: u8) -> Option<u8> {
    let low = operand & XM_NIBBLE_MASK;
    let high = operand >> XM_NIBBLE_SHIFT;

    if low != EMPTY_OPERAND {
        Some(volume_command(XM_VOLUME_PANNING_SLIDE_LEFT, low))
    } else if high != EMPTY_OPERAND {
        Some(volume_command(XM_VOLUME_PANNING_SLIDE_RIGHT, high))
    } else {
        None
    }
}

fn note_portamento_requires_effect_column(effect: EffectCommand) -> bool {
    effect.effect == INTERNAL_EFFECT_TONE_PORTAMENTO
        && effect.operand & XM_NIBBLE_MASK != EMPTY_OPERAND
}

fn volume_command(command: u8, operand: u8) -> u8 {
    (command << XM_NIBBLE_SHIFT) | (operand & XM_NIBBLE_MASK)
}

fn core_note_to_xm(note: Note) -> u8 {
    match note {
        Note::Empty => XM_NOTE_EMPTY,
        Note::Key(value) => value,
        Note::Off => XM_NOTE_OFF,
    }
}

fn vol255_to_64(volume: u8) -> u8 {
    ((u16::from(volume) * u16::from(XM_VOLUME_MAX)) / CORE_VOLUME_MAX) as u8
}

fn write_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + BYTE_2_OFFSET].copy_from_slice(&value.to_le_bytes());
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + BYTE_3_OFFSET + BYTE_1_OFFSET].copy_from_slice(&value.to_le_bytes());
}

fn write_fixed_text(bytes: &mut [u8], value: &str) {
    bytes.fill(ASCII_NUL);

    for (target, source) in bytes.iter_mut().zip(value.as_bytes()) {
        *target = *source;
    }
}
