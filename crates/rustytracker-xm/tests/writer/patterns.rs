use crate::*;

#[test]
fn writes_empty_pattern_headers_without_payload_data() {
    let module = Module::empty();
    let bytes = write_header_and_patterns(&module);
    let header = parse_xm_header(&bytes).unwrap();
    let pattern_headers = parse_xm_pattern_headers(&bytes, &header).unwrap();
    let patterns = decode_xm_patterns(&bytes, &header).unwrap();

    assert_eq!(pattern_headers.len(), module.patterns.len());
    assert_eq!(
        pattern_headers[0].header_length,
        XM_WRITER_PATTERN_HEADER_LEN
    );
    assert_eq!(
        pattern_headers[0].packing_type,
        XM_WRITER_PATTERN_PACKING_TYPE
    );
    assert_eq!(
        pattern_headers[0].packed_data_len,
        XM_WRITER_EMPTY_PATTERN_DATA_LEN
    );
    assert_eq!(patterns[0].rows(), module.patterns[0].rows());
    assert_eq!(patterns[0].channels(), module.header.channel_count);
}

#[test]
fn writes_only_active_channels_from_editor_sized_patterns() {
    let mut module = Module::empty_with_channels(XM_WRITER_TEST_CHANNELS).unwrap();
    module.patterns[0]
        .set_cell(
            1,
            0,
            PatternCell {
                note: Note::Key(XM_WRITER_TEST_NOTE),
                instrument: XM_WRITER_TEST_INSTRUMENT,
                ..PatternCell::default()
            },
        )
        .unwrap();

    let bytes = write_header_and_patterns(&module);
    let header = parse_xm_header(&bytes).unwrap();
    let pattern_headers = parse_xm_pattern_headers(&bytes, &header).unwrap();
    let patterns = decode_xm_patterns(&bytes, &header).unwrap();

    assert_eq!(patterns[0].channels(), XM_WRITER_TEST_CHANNELS);
    assert_eq!(
        pattern_headers[0].packed_data_len as usize,
        module.patterns[0].rows() as usize * XM_WRITER_TEST_CHANNELS as usize * 5
    );
    assert_eq!(
        patterns[0].cell(1, 0).unwrap().note,
        Note::Key(XM_WRITER_TEST_NOTE)
    );
}

#[test]
fn rejects_patterns_with_fewer_channels_than_the_module_header() {
    let mut module = Module::empty_with_channels(XM_WRITER_TEST_CHANNELS).unwrap();
    module.patterns = vec![Pattern::new(
        XM_WRITER_TEST_ROWS,
        XM_WRITER_TEST_CHANNELS - 1,
        XM_WRITER_TEST_EFFECT_SLOTS,
    )];

    assert_eq!(
        write_xm_patterns(&module).unwrap_err(),
        XmWriteError::InvalidPatternShape {
            pattern_index: 0,
            channels: XM_WRITER_TEST_CHANNELS - 1,
            required_channels: XM_WRITER_TEST_CHANNELS,
        }
    );
}

#[test]
fn rejects_non_empty_cells_outside_active_channel_count() {
    let mut module = Module::empty_with_channels(XM_WRITER_TEST_CHANNELS).unwrap();
    module.patterns[0]
        .set_cell(
            XM_WRITER_TEST_CHANNELS,
            0,
            PatternCell {
                note: Note::Key(XM_WRITER_TEST_NOTE),
                instrument: XM_WRITER_TEST_INSTRUMENT,
                ..PatternCell::default()
            },
        )
        .unwrap();

    assert_eq!(
        write_xm_patterns(&module).unwrap_err(),
        XmWriteError::PatternDataOutsideChannelCount {
            pattern_index: 0,
            row: 0,
            channel: XM_WRITER_TEST_CHANNELS,
            channel_count: XM_WRITER_TEST_CHANNELS,
        }
    );
}

#[test]
fn writes_simple_unpacked_pattern_cells() {
    let mut module = Module::empty_with_channels(XM_WRITER_TEST_CHANNELS).unwrap();
    let mut pattern = Pattern::new(
        XM_WRITER_TEST_ROWS,
        XM_WRITER_TEST_CHANNELS,
        XM_WRITER_TEST_EFFECT_SLOTS,
    );
    pattern
        .set_cell(
            0,
            0,
            PatternCell {
                note: Note::Key(XM_WRITER_TEST_NOTE),
                instrument: XM_WRITER_TEST_INSTRUMENT,
                ..PatternCell::default()
            },
        )
        .unwrap();
    module.patterns = vec![pattern];

    let bytes = write_header_and_patterns(&module);
    let header = parse_xm_header(&bytes).unwrap();
    let patterns = decode_xm_patterns(&bytes, &header).unwrap();
    let cell = patterns[0].cell(0, 0).unwrap();

    assert_eq!(cell.note, Note::Key(XM_WRITER_TEST_NOTE));
    assert_eq!(cell.instrument, XM_WRITER_TEST_INSTRUMENT);
    assert_eq!(cell.effects, PatternCell::default().effects);
}
