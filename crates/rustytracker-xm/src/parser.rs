use rustytracker_core::{
    EffectCommand, Envelope as CoreEnvelope, EnvelopePoint as CoreEnvelopePoint, FrequencyTable,
    Instrument, InstrumentName, Module, ModuleHeader, ModuleTitle, Note, Pattern, PatternCell,
    Sample, SampleData as CoreSampleData, SampleLoopKind, SampleName, Vibrato as CoreVibrato,
    EDITOR_PATTERN_CHANNELS, INTERNAL_EFFECT_EXTENDED_BASE, INTERNAL_EFFECT_EXTRA_FINE_PORTA_BASE,
    MAX_ACTIVE_ORDERS, MAX_INSTRUMENTS, MAX_PATTERNS, MIN_ACTIVE_ORDERS, MIN_CHANNEL_COUNT,
    SAMPLES_PER_INSTRUMENT, SAMPLE_DEFAULT_FLAGS, SAMPLE_DEFAULT_VOLUME_FADEOUT,
    INTERNAL_EFFECT_NONZERO_ARPEGGIO, XM_EFFECT_EXTENDED,
};

use crate::*;

pub fn parse_xm_header(bytes: &[u8]) -> XmResult<XmModuleHeader> {
    if bytes.len() < XM_MIN_HEADER_BYTES {
        return Err(XmParseError::Truncated {
            expected: XM_MIN_HEADER_BYTES,
            actual: bytes.len(),
        });
    }

    if &bytes[..XM_HEADER_SIGNATURE_LENGTH] != XM_HEADER_SIGNATURE {
        return Err(XmParseError::InvalidSignature);
    }

    if bytes[MARKER_OFFSET] != XM_MARKER {
        return Err(XmParseError::InvalidMarker(bytes[MARKER_OFFSET]));
    }

    let version = read_u16(bytes, VERSION_OFFSET);
    if !matches!(version, XM_VERSION_1_02 | XM_VERSION_1_03 | XM_VERSION_1_04) {
        return Err(XmParseError::UnsupportedVersion(version));
    }

    let header_size = read_u32(bytes, HEADER_SIZE_OFFSET);
    let song_length = read_u16(bytes, HEADER_FIELDS_OFFSET);
    let restart_position = read_u16(bytes, XM_RESTART_FIELD_OFFSET);
    let channel_count = read_u16(bytes, XM_CHANNELS_FIELD_OFFSET);
    let pattern_count = read_u16(bytes, XM_PATTERNS_FIELD_OFFSET);
    let instrument_count = read_u16(bytes, XM_INSTRUMENTS_FIELD_OFFSET);
    let flags = read_u16(bytes, XM_FLAGS_FIELD_OFFSET);
    let default_tick_speed = read_u16(bytes, XM_TICK_SPEED_FIELD_OFFSET);
    let default_bpm = read_u16(bytes, XM_BPM_FIELD_OFFSET);

    if !(MIN_ACTIVE_ORDERS..=MAX_ACTIVE_ORDERS).contains(&(song_length as usize)) {
        return Err(XmParseError::InvalidOrderCount {
            order_count: song_length as usize,
            minimum: MIN_ACTIVE_ORDERS,
            maximum: MAX_ACTIVE_ORDERS,
        });
    }

    if !(MIN_CHANNEL_COUNT..=EDITOR_PATTERN_CHANNELS).contains(&channel_count) {
        return Err(XmParseError::InvalidChannelCount {
            channel_count,
            minimum: MIN_CHANNEL_COUNT,
            maximum: EDITOR_PATTERN_CHANNELS,
        });
    }

    if pattern_count as usize > MAX_PATTERNS {
        return Err(XmParseError::TooManyPatterns {
            pattern_count,
            maximum: MAX_PATTERNS,
        });
    }

    if instrument_count as usize > MAX_INSTRUMENTS {
        return Err(XmParseError::TooManyInstruments {
            instrument_count,
            maximum: MAX_INSTRUMENTS,
        });
    }

    let order_end = ORDER_TABLE_OFFSET + song_length as usize;
    if order_end > bytes.len() {
        return Err(XmParseError::OrderTableTooShort {
            song_length: song_length as usize,
            available: bytes.len().saturating_sub(ORDER_TABLE_OFFSET),
        });
    }

    Ok(XmModuleHeader {
        title: decode_fixed_text(&bytes[TITLE_OFFSET..TITLE_OFFSET + TITLE_LEN]),
        tracker_name: decode_fixed_text(&bytes[TRACKER_OFFSET..TRACKER_OFFSET + TRACKER_LEN]),
        version,
        header_size,
        song_length,
        restart_position,
        channel_count,
        pattern_count,
        instrument_count,
        flags,
        frequency_table: if flags & XM_LINEAR_FREQUENCY_FLAG == XM_LINEAR_FREQUENCY_FLAG {
            FrequencyTable::Linear
        } else {
            FrequencyTable::Amiga
        },
        default_tick_speed,
        default_bpm,
        orders: bytes[ORDER_TABLE_OFFSET..order_end].to_vec(),
    })
}

pub fn parse_xm_pattern_headers(
    bytes: &[u8],
    header: &XmModuleHeader,
) -> XmResult<Vec<XmPatternHeader>> {
    let fixed_pattern_header_len = if header.version == XM_VERSION_1_02 {
        XM_1_02_PATTERN_HEADER_LEN
    } else {
        XM_PATTERN_HEADER_LEN
    };
    let mut offset = HEADER_SIZE_OFFSET + header.header_size as usize;
    let mut patterns = Vec::with_capacity(header.pattern_count as usize);

    for pattern_index in 0..header.pattern_count as usize {
        let header_end = offset + fixed_pattern_header_len;
        if header_end > bytes.len() {
            return Err(XmParseError::PatternHeaderTooShort {
                pattern_index,
                expected: header_end,
                actual: bytes.len(),
            });
        }

        let header_length = read_u32(bytes, offset);
        if header_length < fixed_pattern_header_len as u32 {
            return Err(XmParseError::InvalidPatternHeaderLength {
                pattern_index,
                header_length,
                minimum: fixed_pattern_header_len,
            });
        }

        let declared_header_end = offset + header_length as usize;
        if declared_header_end > bytes.len() {
            return Err(XmParseError::PatternHeaderTooShort {
                pattern_index,
                expected: declared_header_end,
                actual: bytes.len(),
            });
        }

        let packing_type = bytes[offset + XM_PATTERN_TYPE_OFFSET];
        let (row_count, packed_data_len) = if header.version == XM_VERSION_1_02 {
            (
                bytes[offset + XM_1_02_PATTERN_ROWS_OFFSET] as u16 + XM_1_02_ROW_COUNT_BASE,
                read_u16(bytes, offset + XM_1_02_PATTERN_DATA_LEN_OFFSET),
            )
        } else {
            (
                read_u16(bytes, offset + XM_PATTERN_ROWS_OFFSET),
                read_u16(bytes, offset + XM_PATTERN_DATA_LEN_OFFSET),
            )
        };

        let packed_data_offset = declared_header_end;
        let next_offset = packed_data_offset + packed_data_len as usize;
        if next_offset > bytes.len() {
            return Err(XmParseError::PatternDataTooShort {
                pattern_index,
                expected: next_offset,
                actual: bytes.len(),
            });
        }

        patterns.push(XmPatternHeader {
            index: pattern_index,
            header_length,
            packing_type,
            row_count,
            packed_data_len,
            packed_data_offset,
            next_offset,
        });
        offset = next_offset;
    }

    Ok(patterns)
}

pub fn decode_xm_patterns(bytes: &[u8], header: &XmModuleHeader) -> XmResult<Vec<Pattern>> {
    parse_xm_pattern_headers(bytes, header)?
        .iter()
        .map(|pattern_header| decode_xm_pattern(bytes, header, pattern_header))
        .collect()
}

pub fn decode_xm_pattern(
    bytes: &[u8],
    header: &XmModuleHeader,
    pattern_header: &XmPatternHeader,
) -> XmResult<Pattern> {
    if pattern_header.next_offset > bytes.len() {
        return Err(XmParseError::PatternDataTooShort {
            pattern_index: pattern_header.index,
            expected: pattern_header.next_offset,
            actual: bytes.len(),
        });
    }

    let data = &bytes[pattern_header.packed_data_offset..pattern_header.next_offset];
    let mut data_cursor = 0;
    let mut pattern = Pattern::new(
        pattern_header.row_count,
        header.channel_count,
        XM_EXPANDED_EFFECT_SLOTS,
    );

    if data.is_empty() {
        return Ok(pattern);
    }

    for row in 0..pattern_header.row_count {
        for channel in 0..header.channel_count {
            let slot = read_xm_slot(data, &mut data_cursor, pattern_header.index, row, channel)?;
            let cell = normalize_xm_slot(slot);
            pattern
                .set_cell(channel, row, cell)
                .expect("decoder writes cells inside the allocated pattern shape");
        }
    }

    if data_cursor != data.len() {
        return Err(XmParseError::PackedPatternDataLengthMismatch {
            pattern_index: pattern_header.index,
            consumed: data_cursor,
            declared: data.len(),
        });
    }

    Ok(pattern)
}

pub fn parse_xm_module(bytes: &[u8]) -> XmResult<Module> {
    let header = parse_xm_header(bytes)?;
    let pattern_headers = parse_xm_pattern_headers(bytes, &header)?;
    let mut patterns = pattern_headers
        .iter()
        .map(|pattern_header| decode_xm_pattern(bytes, &header, pattern_header))
        .collect::<XmResult<Vec<_>>>()?;
    extend_patterns_for_order_references(&mut patterns, &header);
    let instrument_offset = pattern_headers
        .last()
        .map(|pattern_header| pattern_header.next_offset)
        .unwrap_or(HEADER_SIZE_OFFSET + header.header_size as usize);
    let instrument_section = parse_xm_instruments(bytes, &header, instrument_offset)?;

    Ok(Module {
        header: ModuleHeader {
            title: ModuleTitle::new(&header.title),
            channel_count: header.channel_count,
            frequency_table: header.frequency_table,
            bpm: header.default_bpm,
            tick_speed: header.default_tick_speed,
            main_volume: rustytracker_core::DEFAULT_MAIN_VOLUME,
            restart_position: header.restart_position,
            is_mod: false,
        },
        orders: header.orders,
        patterns,
        instruments: instrument_section
            .instruments
            .iter()
            .map(convert_instrument_to_core)
            .collect(),
        samples: convert_samples_to_core(&instrument_section.instruments),
    })
}

fn extend_patterns_for_order_references(patterns: &mut Vec<Pattern>, header: &XmModuleHeader) {
    let required_pattern_count = header
        .orders
        .iter()
        .map(|&pattern_index| pattern_index as usize + BYTE_1_OFFSET)
        .max()
        .unwrap_or(patterns.len());

    while patterns.len() < required_pattern_count {
        patterns.push(Pattern::new(
            XM_ORDER_REFERENCE_PATTERN_ROWS,
            header.channel_count,
            XM_EXPANDED_EFFECT_SLOTS,
        ));
    }
}

pub fn parse_xm_instruments(
    bytes: &[u8],
    header: &XmModuleHeader,
    start_offset: usize,
) -> XmResult<XmInstrumentSection> {
    let mut offset = start_offset;
    let mut instruments = Vec::with_capacity(header.instrument_count as usize);

    for instrument_index in 0..header.instrument_count as usize {
        let parsed = parse_xm_instrument(bytes, instrument_index, offset)?;
        offset = parsed.next_offset;
        instruments.push(parsed);
    }

    Ok(XmInstrumentSection {
        instruments,
        next_offset: offset,
    })
}

fn parse_xm_instrument(
    bytes: &[u8],
    instrument_index: usize,
    start_offset: usize,
) -> XmResult<XmInstrument> {
    ensure_instrument_range(
        bytes,
        start_offset,
        XM_INSTRUMENT_SIZE_LEN,
        instrument_index,
        true,
    )?;

    let header_size = read_u32(bytes, start_offset);
    let mut offset = start_offset + XM_INSTRUMENT_SIZE_LEN;
    let (name, instrument_type, sample_count) =
        read_instrument_identity(bytes, instrument_index, header_size, &mut offset)?;

    if sample_count as usize > XM_NOTE_SAMPLE_MAP_LEN {
        return Err(XmParseError::TooManyInstrumentSamples {
            instrument_index,
            sample_count,
            maximum: XM_NOTE_SAMPLE_MAP_LEN,
        });
    }

    let mut instrument = XmInstrument {
        index: instrument_index,
        header_size,
        name,
        instrument_type,
        sample_count,
        sample_header_size: None,
        note_sample_map: None,
        volume_envelope: None,
        panning_envelope: None,
        vibrato_type: None,
        vibrato_sweep: None,
        vibrato_depth: None,
        vibrato_rate: None,
        volume_fadeout: None,
        samples: Vec::with_capacity(sample_count as usize),
        next_offset: offset,
    };

    if header_size <= XM_INSTRUMENT_NO_EXTENSION_MAX_SIZE {
        return Ok(instrument);
    }

    ensure_instrument_range(
        bytes,
        offset,
        XM_SAMPLE_HEADER_SIZE_LEN,
        instrument_index,
        false,
    )?;
    instrument.sample_header_size = Some(read_u32(bytes, offset));
    offset += XM_SAMPLE_HEADER_SIZE_LEN;

    let extension_len = header_size
        .checked_sub(XM_INSTRUMENT_BASE_WITH_SAMPLE_HEADER_SIZE)
        .ok_or(XmParseError::InvalidInstrumentSize {
            instrument_index,
            size: header_size,
        })? as usize;

    if extension_len > XM_INSTRUMENT_EXTENSION_MAX_LEN {
        return Err(XmParseError::InstrumentExtensionTooLong {
            instrument_index,
            extension_len,
            maximum: XM_INSTRUMENT_EXTENSION_MAX_LEN,
        });
    }

    ensure_instrument_range(bytes, offset, extension_len, instrument_index, false)?;
    let mut extension = [ASCII_NUL; XM_INSTRUMENT_EXTENSION_MAX_LEN];
    extension[..extension_len].copy_from_slice(&bytes[offset..offset + extension_len]);
    offset += extension_len;

    let extension_data = parse_instrument_extension(&extension);
    instrument.note_sample_map = Some(extension_data.note_sample_map);
    instrument.volume_envelope = Some(extension_data.volume_envelope);
    instrument.panning_envelope = Some(extension_data.panning_envelope);
    instrument.vibrato_type = Some(extension_data.vibrato_type);
    instrument.vibrato_sweep = Some(extension_data.vibrato_sweep);
    instrument.vibrato_depth = Some(extension_data.vibrato_depth);
    instrument.vibrato_rate = Some(extension_data.vibrato_rate);
    instrument.volume_fadeout = Some(extension_data.volume_fadeout);

    let mut samples = Vec::with_capacity(sample_count as usize);
    for sample_index in 0..sample_count as usize {
        let sample = read_sample_header(bytes, instrument_index, sample_index, offset)?;
        offset += XM_SAMPLE_HEADER_LEN;
        samples.push(sample);
    }

    let mut sample_data_offset = offset;
    for sample in &mut samples {
        sample.data_offset = sample_data_offset;
        sample.data_end = sample_data_offset + sample.length as usize;
        if sample.data_end > bytes.len() {
            return Err(XmParseError::SampleDataTooShort {
                instrument_index,
                sample_index: sample.index,
                expected: sample.data_end,
                actual: bytes.len(),
            });
        }
        if is_adpcm_sample(sample.reserved) {
            return Err(XmParseError::UnsupportedAdpcmSample {
                instrument_index,
                sample_index: sample.index,
            });
        }
        sample.decoded_data = decode_sample_data(
            &bytes[sample.data_offset..sample.data_end],
            sample.sample_type,
        );
        sample_data_offset = sample.data_end;
    }

    instrument.samples = samples;
    instrument.next_offset = sample_data_offset;
    Ok(instrument)
}

fn convert_instrument_to_core(instrument: &XmInstrument) -> Instrument {
    let base_sample = instrument.index * SAMPLES_PER_INSTRUMENT;
    let mut sample_slots = vec![None; SAMPLES_PER_INSTRUMENT];
    for sample in &instrument.samples {
        if sample.index < SAMPLES_PER_INSTRUMENT {
            sample_slots[sample.index] = Some(base_sample + sample.index);
        }
    }

    Instrument {
        name: InstrumentName::new(&instrument.name),
        sample_slots,
        note_sample_map: instrument
            .note_sample_map
            .as_ref()
            .map(|note_map| {
                note_map
                    .iter()
                    .map(|&sample_index| {
                        let sample_index = sample_index as usize;
                        if sample_index < instrument.sample_count as usize
                            && sample_index < SAMPLES_PER_INSTRUMENT
                        {
                            Some(base_sample + sample_index)
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_else(|| vec![None; XM_NOTE_SAMPLE_MAP_LEN]),
        volume_envelope: instrument
            .volume_envelope
            .as_ref()
            .map(convert_envelope_to_core)
            .unwrap_or_default(),
        panning_envelope: instrument
            .panning_envelope
            .as_ref()
            .map(convert_envelope_to_core)
            .unwrap_or_default(),
        vibrato: CoreVibrato {
            waveform: instrument.vibrato_type.unwrap_or_default(),
            sweep: instrument.vibrato_sweep.unwrap_or_default(),
            depth: instrument.vibrato_depth.unwrap_or_default(),
            rate: instrument.vibrato_rate.unwrap_or_default(),
        },
        volume_fadeout: instrument
            .volume_fadeout
            .unwrap_or(SAMPLE_DEFAULT_VOLUME_FADEOUT),
    }
}

fn convert_envelope_to_core(envelope: &XmEnvelope) -> CoreEnvelope {
    CoreEnvelope {
        points: envelope
            .points
            .iter()
            .map(|point| CoreEnvelopePoint {
                frame: point.frame,
                value: point.value,
            })
            .collect(),
        point_count: envelope.point_count,
        sustain_point: envelope.sustain_point,
        loop_start_point: envelope.loop_start_point,
        loop_end_point: envelope.loop_end_point,
        flags: envelope.flags,
    }
}

fn convert_samples_to_core(instruments: &[XmInstrument]) -> Vec<Sample> {
    let mut samples = vec![Sample::default(); instruments.len() * SAMPLES_PER_INSTRUMENT];

    for instrument in instruments {
        let base_sample = instrument.index * SAMPLES_PER_INSTRUMENT;
        let volume_fadeout = instrument
            .volume_fadeout
            .unwrap_or(SAMPLE_DEFAULT_VOLUME_FADEOUT);

        for sample in &instrument.samples {
            if sample.index >= SAMPLES_PER_INSTRUMENT {
                continue;
            }

            samples[base_sample + sample.index] = Sample {
                name: SampleName::new(&sample.name),
                length: sample.frame_count,
                loop_start: sample.loop_start_frames,
                loop_length: sample.loop_length_frames,
                loop_kind: sample.loop_kind,
                volume: sample.volume,
                panning: sample.panning,
                flags: SAMPLE_DEFAULT_FLAGS,
                volume_fadeout,
                sample_type: sample.sample_type,
                finetune: sample.finetune,
                relative_note: sample.relative_note,
                data: match &sample.decoded_data {
                    XmSampleData::Pcm8(values) => CoreSampleData::pcm8(values.clone()),
                    XmSampleData::Pcm16(values) => CoreSampleData::pcm16(values.clone()),
                },
            };
        }
    }

    samples
}

fn read_instrument_identity(
    bytes: &[u8],
    instrument_index: usize,
    header_size: u32,
    offset: &mut usize,
) -> XmResult<(String, u8, u16)> {
    if (XM_INSTRUMENT_SHORT_SIZE_MIN..XM_INSTRUMENT_NO_EXTENSION_MAX_SIZE).contains(&header_size) {
        let payload_len = (header_size - XM_INSTRUMENT_SIZE_LEN as u32) as usize;
        ensure_instrument_range(bytes, *offset, payload_len, instrument_index, false)?;

        let mut buffer = [ASCII_NUL; XM_INSTRUMENT_SHORT_BUFFER_LEN];
        buffer[..payload_len].copy_from_slice(&bytes[*offset..*offset + payload_len]);
        *offset += payload_len;

        return Ok((
            decode_fixed_text(&buffer[..XM_INSTRUMENT_NAME_LEN]),
            buffer[XM_INSTRUMENT_TYPE_OFFSET],
            read_u16(&buffer, XM_INSTRUMENT_SAMPLE_COUNT_OFFSET),
        ));
    }

    ensure_instrument_range(
        bytes,
        *offset,
        XM_INSTRUMENT_FIXED_FIELDS_LEN,
        instrument_index,
        false,
    )?;

    let name = decode_fixed_text(&bytes[*offset..*offset + XM_INSTRUMENT_NAME_LEN]);
    *offset += XM_INSTRUMENT_NAME_LEN;
    let instrument_type = bytes[*offset];
    *offset += BYTE_1_OFFSET;
    let sample_count = read_u16(bytes, *offset);
    *offset += BYTE_2_OFFSET;

    Ok((name, instrument_type, sample_count))
}

struct ParsedInstrumentExtension {
    note_sample_map: Vec<u8>,
    volume_envelope: XmEnvelope,
    panning_envelope: XmEnvelope,
    vibrato_type: u8,
    vibrato_sweep: u8,
    vibrato_depth: u8,
    vibrato_rate: u8,
    volume_fadeout: u16,
}

fn parse_instrument_extension(
    extension: &[u8; XM_INSTRUMENT_EXTENSION_MAX_LEN],
) -> ParsedInstrumentExtension {
    let mut offset = 0;
    let note_sample_map = extension[offset..offset + XM_NOTE_SAMPLE_MAP_LEN].to_vec();
    offset += XM_NOTE_SAMPLE_MAP_LEN;

    let volume_points = read_envelope_points(extension, offset);
    offset += XM_ENVELOPE_POINT_COUNT * XM_ENVELOPE_POINT_BYTES;
    let panning_points = read_envelope_points(extension, offset);
    offset += XM_ENVELOPE_POINT_COUNT * XM_ENVELOPE_POINT_BYTES;

    let volume_point_count = extension[offset].min(XM_ENVELOPE_POINT_COUNT_MAX);
    offset += BYTE_1_OFFSET;
    let panning_point_count = extension[offset].min(XM_ENVELOPE_POINT_COUNT_MAX);
    offset += BYTE_1_OFFSET;
    let volume_sustain_point = extension[offset];
    offset += BYTE_1_OFFSET;
    let volume_loop_start_point = extension[offset];
    offset += BYTE_1_OFFSET;
    let volume_loop_end_point = extension[offset];
    offset += BYTE_1_OFFSET;
    let panning_sustain_point = extension[offset];
    offset += BYTE_1_OFFSET;
    let panning_loop_start_point = extension[offset];
    offset += BYTE_1_OFFSET;
    let panning_loop_end_point = extension[offset];
    offset += BYTE_1_OFFSET;
    let volume_flags = extension[offset];
    offset += BYTE_1_OFFSET;
    let panning_flags = extension[offset];
    offset += BYTE_1_OFFSET;

    let vibrato_type = extension[offset];
    offset += BYTE_1_OFFSET;
    let vibrato_sweep = extension[offset];
    offset += BYTE_1_OFFSET;
    let vibrato_depth = extension[offset] << XM_VIBRATO_DEPTH_SHIFT;
    offset += BYTE_1_OFFSET;
    let vibrato_rate = extension[offset];
    offset += BYTE_1_OFFSET;
    let volume_fadeout = read_u16(extension, offset) << XM_VOLUME_FADEOUT_SHIFT;

    ParsedInstrumentExtension {
        note_sample_map,
        volume_envelope: XmEnvelope {
            points: volume_points,
            point_count: volume_point_count,
            sustain_point: volume_sustain_point,
            loop_start_point: volume_loop_start_point,
            loop_end_point: volume_loop_end_point,
            flags: volume_flags,
        },
        panning_envelope: XmEnvelope {
            points: panning_points,
            point_count: panning_point_count,
            sustain_point: panning_sustain_point,
            loop_start_point: panning_loop_start_point,
            loop_end_point: panning_loop_end_point,
            flags: panning_flags,
        },
        vibrato_type,
        vibrato_sweep,
        vibrato_depth,
        vibrato_rate,
        volume_fadeout,
    }
}

fn read_envelope_points(bytes: &[u8], offset: usize) -> Vec<XmEnvelopePoint> {
    (0..XM_ENVELOPE_POINT_COUNT)
        .map(|point_index| {
            let point_offset = offset + point_index * XM_ENVELOPE_POINT_BYTES;
            XmEnvelopePoint {
                frame: read_u16(bytes, point_offset + XM_ENVELOPE_X_OFFSET),
                value: read_u16(bytes, point_offset + XM_ENVELOPE_Y_OFFSET)
                    << XM_ENVELOPE_VALUE_SHIFT,
            }
        })
        .collect()
}

fn read_sample_header(
    bytes: &[u8],
    instrument_index: usize,
    sample_index: usize,
    offset: usize,
) -> XmResult<XmSampleHeader> {
    if offset + XM_SAMPLE_HEADER_LEN > bytes.len() {
        return Err(XmParseError::SampleHeaderTooShort {
            instrument_index,
            sample_index,
            expected: offset + XM_SAMPLE_HEADER_LEN,
            actual: bytes.len(),
        });
    }

    let mut cursor = offset;
    let length = read_u32(bytes, cursor);
    cursor += XM_SAMPLE_LENGTH_LEN;
    let loop_start = read_u32(bytes, cursor);
    cursor += XM_SAMPLE_LOOP_START_LEN;
    let loop_length = read_u32(bytes, cursor);
    cursor += XM_SAMPLE_LOOP_LENGTH_LEN;
    let volume_64 = bytes[cursor];
    cursor += XM_SAMPLE_VOLUME_LEN;
    let finetune = bytes[cursor] as i8;
    cursor += XM_SAMPLE_FINETUNE_LEN;
    let sample_type = bytes[cursor];
    cursor += XM_SAMPLE_TYPE_LEN;
    let panning = bytes[cursor];
    cursor += XM_SAMPLE_PANNING_LEN;
    let relative_note = bytes[cursor] as i8;
    cursor += XM_SAMPLE_RELATIVE_NOTE_LEN;
    let reserved = bytes[cursor];
    cursor += XM_SAMPLE_RESERVED_LEN;
    let name = decode_fixed_text(&bytes[cursor..cursor + XM_SAMPLE_NAME_LEN]);
    let frame_count = sample_frame_count(length, sample_type);
    let loop_start_frames = sample_frame_count(loop_start, sample_type);
    let loop_length_frames = sample_frame_count(loop_length, sample_type);
    let decoded_data = empty_sample_data(sample_type);

    Ok(XmSampleHeader {
        index: sample_index,
        length,
        frame_count,
        loop_start,
        loop_start_frames,
        loop_length,
        loop_length_frames,
        volume_64,
        volume: vol64_to_255(volume_64),
        finetune,
        sample_type,
        loop_kind: sample_loop_kind(sample_type),
        panning,
        relative_note,
        reserved,
        name,
        data_offset: XM_EMPTY_SAMPLE_DATA_LEN as usize,
        data_end: XM_EMPTY_SAMPLE_DATA_LEN as usize,
        decoded_data,
    })
}

fn sample_frame_count(byte_len: u32, sample_type: u8) -> u32 {
    let sample_count = if is_16_bit_sample(sample_type) {
        byte_len / BYTES_PER_16_BIT_SAMPLE as u32
    } else {
        byte_len
    };

    if is_stereo_sample(sample_type) {
        sample_count / STEREO_CHANNEL_COUNT_U32
    } else {
        sample_count
    }
}

fn empty_sample_data(sample_type: u8) -> XmSampleData {
    if is_16_bit_sample(sample_type) {
        XmSampleData::Pcm16(Vec::new())
    } else {
        XmSampleData::Pcm8(Vec::new())
    }
}

fn decode_sample_data(bytes: &[u8], sample_type: u8) -> XmSampleData {
    if is_16_bit_sample(sample_type) {
        let values = decode_delta16(bytes);
        XmSampleData::Pcm16(if is_stereo_sample(sample_type) {
            mix_stereo_i16_to_mono(values)
        } else {
            values
        })
    } else {
        let values = decode_delta8(bytes);
        XmSampleData::Pcm8(if is_stereo_sample(sample_type) {
            mix_stereo_i8_to_mono(values)
        } else {
            values
        })
    }
}

fn decode_delta8(bytes: &[u8]) -> Vec<i8> {
    let mut accumulator = 0_i8;
    bytes
        .iter()
        .map(|&byte| {
            accumulator = accumulator.wrapping_add(byte as i8);
            accumulator
        })
        .collect()
}

fn decode_delta16(bytes: &[u8]) -> Vec<i16> {
    let mut accumulator = 0_i16;
    bytes
        .chunks_exact(BYTES_PER_16_BIT_SAMPLE)
        .map(|chunk| {
            let delta = i16::from_le_bytes([chunk[0], chunk[BYTE_1_OFFSET]]);
            accumulator = accumulator.wrapping_add(delta);
            accumulator
        })
        .collect()
}

fn is_16_bit_sample(sample_type: u8) -> bool {
    sample_type & XM_SAMPLE_16_BIT_FLAG == XM_SAMPLE_16_BIT_FLAG
}

fn is_stereo_sample(sample_type: u8) -> bool {
    sample_type & XM_SAMPLE_STEREO_FLAG == XM_SAMPLE_STEREO_FLAG
}

fn is_adpcm_sample(reserved: u8) -> bool {
    reserved == XM_SAMPLE_ADPCM_RESERVED
}

fn sample_loop_kind(sample_type: u8) -> SampleLoopKind {
    match sample_type & XM_SAMPLE_LOOP_MASK {
        XM_SAMPLE_LOOP_NONE => SampleLoopKind::None,
        XM_SAMPLE_LOOP_FORWARD => SampleLoopKind::Forward,
        XM_SAMPLE_LOOP_PING_PONG | XM_SAMPLE_LOOP_UNDEFINED => SampleLoopKind::PingPong,
        _ => unreachable!("loop-kind mask can only produce XM loop values"),
    }
}

fn mix_stereo_i8_to_mono(values: Vec<i8>) -> Vec<i8> {
    let frame_count = values.len() / STEREO_CHANNEL_COUNT;

    (0..frame_count)
        .map(|frame| {
            average_stereo_sample(values[frame] as i32, values[frame + frame_count] as i32)
                .clamp(i8::MIN as i32, i8::MAX as i32) as i8
        })
        .collect()
}

fn mix_stereo_i16_to_mono(values: Vec<i16>) -> Vec<i16> {
    let frame_count = values.len() / STEREO_CHANNEL_COUNT;

    (0..frame_count)
        .map(|frame| {
            average_stereo_sample(values[frame] as i32, values[frame + frame_count] as i32)
                .clamp(i16::MIN as i32, i16::MAX as i32) as i16
        })
        .collect()
}

fn average_stereo_sample(left: i32, right: i32) -> i32 {
    (left + right) >> STEREO_AVERAGE_SHIFT
}

fn ensure_instrument_range(
    bytes: &[u8],
    offset: usize,
    len: usize,
    instrument_index: usize,
    header: bool,
) -> XmResult<()> {
    let expected = offset + len;
    if expected <= bytes.len() {
        return Ok(());
    }

    if header {
        Err(XmParseError::InstrumentHeaderTooShort {
            instrument_index,
            expected,
            actual: bytes.len(),
        })
    } else {
        Err(XmParseError::InstrumentBodyTooShort {
            instrument_index,
            expected,
            actual: bytes.len(),
        })
    }
}

fn read_xm_slot(
    data: &[u8],
    data_cursor: &mut usize,
    pattern_index: usize,
    row: u16,
    channel: u16,
) -> XmResult<[u8; XM_CELL_FIELD_COUNT]> {
    let first = read_xm_slot_byte(data, data_cursor, pattern_index, row, channel)?;
    let mut slot = [EMPTY_OPERAND; XM_CELL_FIELD_COUNT];

    if first & XM_CELL_PACKED_FLAG != EMPTY_OPERAND {
        for (field, val) in slot.iter_mut().enumerate() {
            if first & (XM_FIELD_PRESENT_BIT_BASE << field) != EMPTY_OPERAND {
                *val = read_xm_slot_byte(data, data_cursor, pattern_index, row, channel)?;
            }
        }
    } else {
        slot[XM_NOTE_FIELD_INDEX] = first;
        for field in slot.iter_mut().skip(FIRST_UNPACKED_CELL_FIELD) {
            *field = read_xm_slot_byte(data, data_cursor, pattern_index, row, channel)?;
        }
    }

    Ok(slot)
}

fn read_xm_slot_byte(
    data: &[u8],
    data_cursor: &mut usize,
    pattern_index: usize,
    row: u16,
    channel: u16,
) -> XmResult<u8> {
    if *data_cursor >= data.len() {
        return Err(XmParseError::PackedPatternCellTooShort {
            pattern_index,
            row,
            channel,
            expected: *data_cursor + BYTE_1_OFFSET,
            actual: data.len(),
        });
    }

    let byte = data[*data_cursor];
    *data_cursor += BYTE_1_OFFSET;
    Ok(byte)
}

fn normalize_xm_slot(mut slot: [u8; XM_CELL_FIELD_COUNT]) -> PatternCell {
    if !VALID_XM_EFFECTS.contains(&slot[XM_EFFECT_FIELD_INDEX]) {
        slot[XM_EFFECT_FIELD_INDEX] = EMPTY_EFFECT;
        slot[XM_OPERAND_FIELD_INDEX] = EMPTY_OPERAND;
    }

    if slot[XM_EFFECT_FIELD_INDEX] == XM_EFFECT_VOLUME
        || slot[XM_EFFECT_FIELD_INDEX] == XM_EFFECT_GLOBAL_VOLUME
    {
        slot[XM_OPERAND_FIELD_INDEX] = vol64_to_255(slot[XM_OPERAND_FIELD_INDEX]);
    }

    if slot[XM_EFFECT_FIELD_INDEX] == EMPTY_EFFECT && slot[XM_OPERAND_FIELD_INDEX] != EMPTY_OPERAND
    {
        slot[XM_EFFECT_FIELD_INDEX] = INTERNAL_EFFECT_NONZERO_ARPEGGIO;
    }

    if slot[XM_EFFECT_FIELD_INDEX] == XM_EFFECT_EXTENDED {
        slot[XM_EFFECT_FIELD_INDEX] =
            (slot[XM_OPERAND_FIELD_INDEX] >> XM_NIBBLE_SHIFT) + INTERNAL_EFFECT_EXTENDED_BASE;
        slot[XM_OPERAND_FIELD_INDEX] &= XM_NIBBLE_MASK;
    }

    if slot[XM_EFFECT_FIELD_INDEX] == XM_EFFECT_EXTRA_FINE_PORTA {
        slot[XM_EFFECT_FIELD_INDEX] = (slot[XM_OPERAND_FIELD_INDEX] >> XM_NIBBLE_SHIFT)
            + INTERNAL_EFFECT_EXTRA_FINE_PORTA_BASE;
        slot[XM_OPERAND_FIELD_INDEX] &= XM_NIBBLE_MASK;
    }

    PatternCell {
        note: xm_note_to_core(slot[XM_NOTE_FIELD_INDEX]),
        instrument: slot[XM_INSTRUMENT_FIELD_INDEX],
        effects: vec![
            convert_xm_volume_effect(slot[XM_VOLUME_FIELD_INDEX]),
            EffectCommand {
                effect: slot[XM_EFFECT_FIELD_INDEX],
                operand: slot[XM_OPERAND_FIELD_INDEX],
            },
        ],
    }
}

fn xm_note_to_core(note: u8) -> Note {
    match note {
        XM_NOTE_EMPTY => Note::Empty,
        XM_NOTE_OFF => Note::Off,
        value => Note::Key(value),
    }
}

fn convert_xm_volume_effect(volume: u8) -> EffectCommand {
    let mut effect = EMPTY_EFFECT;
    let mut operand = EMPTY_OPERAND;

    if (XM_VOLUME_SET_MIN..=XM_VOLUME_SET_MAX).contains(&volume) {
        effect = XM_EFFECT_VOLUME;
        operand = vol64_to_255(volume - XM_VOLUME_SET_MIN);
    }

    if volume >= XM_VOLUME_COMMAND_MIN {
        let xm_effect = volume >> XM_NIBBLE_SHIFT;
        let xm_operand = volume & XM_NIBBLE_MASK;

        if xm_operand != EMPTY_OPERAND {
            match xm_effect {
                XM_VOLUME_SLIDE_DOWN => {
                    effect = INTERNAL_EFFECT_VOLUME_SLIDE;
                    operand = xm_operand;
                }
                XM_VOLUME_SLIDE_UP => {
                    effect = INTERNAL_EFFECT_VOLUME_SLIDE;
                    operand = xm_operand << XM_NIBBLE_SHIFT;
                }
                XM_VOLUME_FINE_DOWN => {
                    effect = INTERNAL_EFFECT_FINE_VOLUME_SLIDE_DOWN;
                    operand = xm_operand;
                }
                XM_VOLUME_FINE_UP => {
                    effect = INTERNAL_EFFECT_FINE_VOLUME_SLIDE_UP;
                    operand = xm_operand;
                }
                XM_VOLUME_SET_VIBRATO_SPEED => {
                    effect = INTERNAL_EFFECT_VIBRATO_COMPAT;
                    operand = xm_operand << XM_NIBBLE_SHIFT;
                }
                XM_VOLUME_VIBRATO => {
                    effect = INTERNAL_EFFECT_VIBRATO_COMPAT;
                    operand = xm_operand;
                }
                XM_VOLUME_SET_PANNING => {
                    effect = INTERNAL_EFFECT_PANNING;
                    operand = pan15_to_255(xm_operand);
                }
                XM_VOLUME_PANNING_SLIDE_LEFT => {
                    effect = INTERNAL_EFFECT_PANNING_SLIDE;
                    operand = xm_operand;
                }
                XM_VOLUME_PANNING_SLIDE_RIGHT => {
                    effect = INTERNAL_EFFECT_PANNING_SLIDE;
                    operand = xm_operand << XM_NIBBLE_SHIFT;
                }
                XM_VOLUME_TONE_PORTAMENTO => {
                    effect = INTERNAL_EFFECT_TONE_PORTAMENTO;
                    operand = xm_operand << XM_NIBBLE_SHIFT;
                }
                _ => {}
            }
        } else {
            match xm_effect {
                XM_VOLUME_VIBRATO => {
                    effect = INTERNAL_EFFECT_VIBRATO_COMPAT;
                    operand = xm_operand;
                }
                XM_VOLUME_SET_PANNING => {
                    effect = INTERNAL_EFFECT_PANNING;
                    operand = pan15_to_255(xm_operand);
                }
                XM_VOLUME_TONE_PORTAMENTO => {
                    effect = INTERNAL_EFFECT_TONE_PORTAMENTO;
                    operand = xm_operand;
                }
                _ => {}
            }
        }
    }

    EffectCommand { effect, operand }
}

fn vol64_to_255(volume: u8) -> u8 {
    (((volume.min(XM_VOLUME_MAX) as u32 * VOL64_TO_255_SCALE + VOL64_TO_255_ROUNDING)
        >> VOL64_TO_255_SHIFT)
        & BYTE_MASK) as u8
}

fn pan15_to_255(panning: u8) -> u8 {
    if panning >= XM_PAN_COLUMN_MAX {
        FULL_PANNING
    } else {
        panning << XM_NIBBLE_SHIFT
    }
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + BYTE_1_OFFSET]])
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + BYTE_1_OFFSET],
        bytes[offset + BYTE_2_OFFSET],
        bytes[offset + BYTE_3_OFFSET],
    ])
}

fn decode_fixed_text(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .rposition(|&byte| byte > ASCII_CONTROL_MAX)
        .map(|index| index + TEXT_INDEX_TO_LEN_OFFSET)
        .unwrap_or(ASCII_NUL as usize);

    bytes[..end]
        .iter()
        .map(|&byte| {
            if byte == ASCII_NUL || !(ASCII_CONTROL_MAX..=ASCII_DELETE).contains(&byte) {
                ' '
            } else {
                byte as char
            }
        })
        .collect()
}
