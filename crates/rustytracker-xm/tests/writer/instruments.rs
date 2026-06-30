use crate::*;

#[test]
fn writes_empty_instrument_headers_without_sample_data() {
    let module = Module::empty();
    let bytes = write_header_patterns_and_instruments(&module);
    let header = parse_xm_header(&bytes).unwrap();
    let pattern_headers = parse_xm_pattern_headers(&bytes, &header).unwrap();
    let instrument_offset = pattern_headers.last().unwrap().next_offset;
    let section = parse_xm_instruments(&bytes, &header, instrument_offset).unwrap();

    assert_eq!(section.instruments.len(), module.instruments.len());
    assert_eq!(section.next_offset, bytes.len());
    assert!(section.instruments.iter().all(|instrument| {
        instrument.header_size == XM_WRITER_EMPTY_INSTRUMENT_HEADER_SIZE
            && instrument.sample_count == 0
            && instrument.sample_header_size.is_none()
            && instrument.note_sample_map.is_none()
    }));
}

#[test]
fn writes_extended_zero_sample_instruments_when_metadata_must_roundtrip() {
    let mut module = Module::empty();
    let mut instrument = Instrument::empty(XM_WRITER_FIRST_INSTRUMENT_INDEX);
    instrument.name = InstrumentName::new(XM_WRITER_TEST_INSTRUMENT_NAME);
    instrument.sample_slots = vec![None; SAMPLES_PER_INSTRUMENT];
    instrument.note_sample_map = vec![None; MAX_XM_NOTES as usize];
    instrument.volume_envelope = test_envelope();
    instrument.panning_envelope = test_envelope();
    instrument.vibrato = Vibrato {
        waveform: XM_WRITER_TEST_VIBRATO_WAVEFORM,
        sweep: XM_WRITER_TEST_VIBRATO_SWEEP,
        depth: XM_WRITER_TEST_VIBRATO_DEPTH,
        rate: XM_WRITER_TEST_VIBRATO_RATE,
    };
    instrument.volume_fadeout = XM_WRITER_TEST_VOLUME_FADEOUT;
    module.instruments = vec![instrument];

    let bytes = write_header_patterns_and_instruments(&module);
    let section = parse_written_instruments(&bytes);
    let instrument = &section.instruments[XM_WRITER_FIRST_INSTRUMENT_INDEX];

    assert_eq!(section.instruments.len(), XM_WRITER_SINGLE_INSTRUMENT_COUNT);
    assert_eq!(section.next_offset, bytes.len());
    assert_eq!(instrument.header_size, XM_WRITER_INSTRUMENT_HEADER_SIZE);
    assert_eq!(instrument.name, XM_WRITER_TEST_INSTRUMENT_NAME);
    assert_eq!(instrument.sample_count, XM_WRITER_EMPTY_SAMPLE_COUNT);
    assert_eq!(
        instrument.sample_header_size,
        Some(XM_WRITER_SAMPLE_HEADER_SIZE)
    );
    assert_eq!(
        instrument.volume_envelope.as_ref().unwrap().points[0],
        rustytracker_xm::XmEnvelopePoint {
            frame: XM_WRITER_TEST_ENVELOPE_FRAME,
            value: XM_WRITER_TEST_ENVELOPE_VALUE,
        }
    );
    assert_eq!(
        instrument.panning_envelope.as_ref().unwrap().points[0],
        rustytracker_xm::XmEnvelopePoint {
            frame: XM_WRITER_TEST_ENVELOPE_FRAME,
            value: XM_WRITER_TEST_ENVELOPE_VALUE,
        }
    );
    assert_eq!(
        instrument.vibrato_type,
        Some(XM_WRITER_TEST_VIBRATO_WAVEFORM)
    );
    assert_eq!(instrument.vibrato_sweep, Some(XM_WRITER_TEST_VIBRATO_SWEEP));
    assert_eq!(instrument.vibrato_depth, Some(XM_WRITER_TEST_VIBRATO_DEPTH));
    assert_eq!(instrument.vibrato_rate, Some(XM_WRITER_TEST_VIBRATO_RATE));
    assert_eq!(
        instrument.volume_fadeout,
        Some(XM_WRITER_TEST_VOLUME_FADEOUT)
    );
    assert!(instrument.samples.is_empty());
}

#[test]
fn writes_instrument_metadata_and_empty_sample_headers() {
    let module = module_with_one_named_empty_sample();
    let bytes = write_header_patterns_and_instruments(&module);
    let header = parse_xm_header(&bytes).unwrap();
    let pattern_headers = parse_xm_pattern_headers(&bytes, &header).unwrap();
    let instrument_offset = pattern_headers.last().unwrap().next_offset;
    let section = parse_xm_instruments(&bytes, &header, instrument_offset).unwrap();
    let instrument = &section.instruments[XM_WRITER_FIRST_INSTRUMENT_INDEX];
    let sample = &instrument.samples[XM_WRITER_FIRST_SAMPLE_INDEX];

    assert_eq!(section.instruments.len(), XM_WRITER_SINGLE_INSTRUMENT_COUNT);
    assert_eq!(section.next_offset, bytes.len());
    assert_eq!(instrument.header_size, XM_WRITER_INSTRUMENT_HEADER_SIZE);
    assert_eq!(instrument.name, XM_WRITER_TEST_INSTRUMENT_NAME);
    assert_eq!(instrument.sample_count, XM_WRITER_SINGLE_SAMPLE_COUNT);
    assert_eq!(
        instrument.sample_header_size,
        Some(XM_WRITER_SAMPLE_HEADER_SIZE)
    );
    assert_eq!(
        instrument.note_sample_map.as_ref().unwrap(),
        &vec![XM_WRITER_EMPTY_OPERAND; MAX_XM_NOTES as usize]
    );
    assert_eq!(
        instrument.volume_envelope.as_ref().unwrap().points[0],
        rustytracker_xm::XmEnvelopePoint {
            frame: XM_WRITER_TEST_ENVELOPE_FRAME,
            value: XM_WRITER_TEST_ENVELOPE_VALUE,
        }
    );
    assert_eq!(
        instrument.volume_envelope.as_ref().unwrap().point_count,
        XM_WRITER_TEST_ENVELOPE_POINT_COUNT
    );
    assert_eq!(
        instrument.volume_envelope.as_ref().unwrap().flags,
        XM_WRITER_TEST_ENVELOPE_FLAG
    );
    assert_eq!(
        instrument.panning_envelope.as_ref().unwrap().points[0],
        rustytracker_xm::XmEnvelopePoint {
            frame: XM_WRITER_TEST_ENVELOPE_FRAME,
            value: XM_WRITER_TEST_ENVELOPE_VALUE,
        }
    );
    assert_eq!(
        instrument.vibrato_type,
        Some(XM_WRITER_TEST_VIBRATO_WAVEFORM)
    );
    assert_eq!(instrument.vibrato_sweep, Some(XM_WRITER_TEST_VIBRATO_SWEEP));
    assert_eq!(instrument.vibrato_depth, Some(XM_WRITER_TEST_VIBRATO_DEPTH));
    assert_eq!(instrument.vibrato_rate, Some(XM_WRITER_TEST_VIBRATO_RATE));
    assert_eq!(
        instrument.volume_fadeout,
        Some(XM_WRITER_TEST_VOLUME_FADEOUT)
    );

    assert_eq!(sample.length, 0);
    assert_eq!(sample.name, XM_WRITER_TEST_SAMPLE_NAME);
    assert_eq!(sample.volume_64, XM_WRITER_TEST_SAMPLE_VOLUME_64);
    assert_eq!(sample.volume, XM_WRITER_TEST_SAMPLE_VOLUME_255);
    assert_eq!(sample.panning, XM_WRITER_TEST_SAMPLE_PANNING);
    assert_eq!(sample.finetune, XM_WRITER_TEST_SAMPLE_FINETUNE);
    assert_eq!(sample.relative_note, XM_WRITER_TEST_SAMPLE_RELATIVE_NOTE);
    assert_eq!(sample.sample_type, XM_WRITER_FORWARD_LOOP_SAMPLE_TYPE);
    assert_eq!(sample.loop_kind, SampleLoopKind::Forward);
}

#[test]
fn roundtrips_nonzero_core_sample_indexes_as_xm_local_slots() {
    let mut module = Module::empty();
    let mut instrument = Instrument::empty(XM_WRITER_FIRST_INSTRUMENT_INDEX);
    let mut sample_slots = vec![None; SAMPLES_PER_INSTRUMENT];
    sample_slots[XM_WRITER_FIRST_SAMPLE_INDEX] = Some(XM_WRITER_SECOND_SAMPLE_INDEX);
    instrument.sample_slots = sample_slots;
    instrument.note_sample_map = vec![Some(XM_WRITER_SECOND_SAMPLE_INDEX); MAX_XM_NOTES as usize];
    instrument.volume_fadeout = XM_WRITER_TEST_VOLUME_FADEOUT;

    let expected_local_sample = Sample {
        name: SampleName::new(XM_WRITER_TEST_SAMPLE_NAME),
        length: XM_WRITER_TEST_SAMPLE_FRAME_COUNT_8,
        volume: XM_WRITER_TEST_SAMPLE_VOLUME_255,
        panning: XM_WRITER_TEST_SAMPLE_PANNING,
        volume_fadeout: XM_WRITER_TEST_VOLUME_FADEOUT,
        data: SampleData::pcm8(XM_WRITER_TEST_SAMPLE_VALUES_8.to_vec()),
        ..Sample::default()
    };

    module.instruments = vec![instrument];
    module.samples = vec![Sample::default(), expected_local_sample.clone()];

    let written = write_xm_module(&module).unwrap();
    let reparsed = parse_xm_module(&written).unwrap();
    let instrument = &reparsed.instruments[XM_WRITER_FIRST_INSTRUMENT_INDEX];

    assert_eq!(
        instrument.sample_slots[XM_WRITER_FIRST_SAMPLE_INDEX],
        Some(XM_WRITER_FIRST_SAMPLE_INDEX)
    );
    assert!(instrument
        .sample_slots
        .iter()
        .skip(XM_WRITER_SECOND_SAMPLE_INDEX)
        .all(Option::is_none));
    assert!(instrument
        .note_sample_map
        .iter()
        .all(|sample_index| *sample_index == Some(XM_WRITER_FIRST_SAMPLE_INDEX)));
    assert_eq!(reparsed.samples.len(), SAMPLES_PER_INSTRUMENT);
    assert_eq!(
        sample_roundtrip_summary(&reparsed.samples[XM_WRITER_FIRST_SAMPLE_INDEX]),
        sample_roundtrip_summary(&expected_local_sample)
    );
    assert_eq!(
        reparsed.samples[XM_WRITER_SECOND_SAMPLE_INDEX],
        Sample::default()
    );
}
