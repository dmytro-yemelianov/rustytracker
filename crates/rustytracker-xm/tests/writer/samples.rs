use crate::*;

#[test]
fn writes_8_bit_sample_payloads_with_delta_encoding() {
    let mut module = module_with_one_named_empty_sample();
    module.samples[XM_WRITER_FIRST_SAMPLE_INDEX].loop_start = XM_WRITER_TEST_SAMPLE_LOOP_START;
    module.samples[XM_WRITER_FIRST_SAMPLE_INDEX].loop_length = XM_WRITER_TEST_SAMPLE_LOOP_LENGTH;
    module.samples[XM_WRITER_FIRST_SAMPLE_INDEX].data =
        SampleData::pcm8(XM_WRITER_TEST_SAMPLE_VALUES_8.to_vec());

    let bytes = write_header_patterns_and_instruments(&module);
    let section = parse_written_instruments(&bytes);
    let sample = &section.instruments[XM_WRITER_FIRST_INSTRUMENT_INDEX].samples
        [XM_WRITER_FIRST_SAMPLE_INDEX];

    assert_eq!(
        &bytes[bytes.len() - XM_WRITER_TEST_SAMPLE_DELTAS_8.len()..],
        XM_WRITER_TEST_SAMPLE_DELTAS_8
    );
    assert_eq!(sample.length, XM_WRITER_TEST_SAMPLE_VALUES_8.len() as u32);
    assert_eq!(sample.loop_start, XM_WRITER_TEST_SAMPLE_LOOP_START);
    assert_eq!(sample.loop_length, XM_WRITER_TEST_SAMPLE_LOOP_LENGTH);
    assert_eq!(
        sample.decoded_data.as_i8().unwrap(),
        XM_WRITER_TEST_SAMPLE_VALUES_8
    );
}

#[test]
fn writes_16_bit_sample_payloads_with_delta_encoding() {
    let mut module = module_with_one_named_empty_sample();
    module.samples[XM_WRITER_FIRST_SAMPLE_INDEX].loop_start = XM_WRITER_TEST_SAMPLE_LOOP_START;
    module.samples[XM_WRITER_FIRST_SAMPLE_INDEX].loop_length = XM_WRITER_TEST_SAMPLE_LOOP_LENGTH;
    module.samples[XM_WRITER_FIRST_SAMPLE_INDEX].data =
        SampleData::pcm16(XM_WRITER_TEST_SAMPLE_VALUES_16.to_vec());

    let bytes = write_header_patterns_and_instruments(&module);
    let section = parse_written_instruments(&bytes);
    let sample = &section.instruments[XM_WRITER_FIRST_INSTRUMENT_INDEX].samples
        [XM_WRITER_FIRST_SAMPLE_INDEX];

    assert_eq!(
        &bytes[bytes.len() - XM_WRITER_TEST_SAMPLE_DELTAS_16.len()..],
        XM_WRITER_TEST_SAMPLE_DELTAS_16
    );
    assert_eq!(
        sample.length,
        (XM_WRITER_TEST_SAMPLE_VALUES_16.len() * XM_WRITER_BYTES_PER_16_BIT_SAMPLE) as u32
    );
    assert_eq!(sample.sample_type, XM_WRITER_FORWARD_16_BIT_SAMPLE_TYPE);
    assert_eq!(
        sample.loop_start,
        XM_WRITER_TEST_SAMPLE_LOOP_START * XM_WRITER_BYTES_PER_16_BIT_SAMPLE as u32
    );
    assert_eq!(
        sample.loop_length,
        XM_WRITER_TEST_SAMPLE_LOOP_LENGTH * XM_WRITER_BYTES_PER_16_BIT_SAMPLE as u32
    );
    assert_eq!(sample.loop_start_frames, XM_WRITER_TEST_SAMPLE_LOOP_START);
    assert_eq!(sample.loop_length_frames, XM_WRITER_TEST_SAMPLE_LOOP_LENGTH);
    assert_eq!(
        sample.decoded_data.as_i16().unwrap(),
        XM_WRITER_TEST_SAMPLE_VALUES_16
    );
}

#[test]
fn rejects_16_bit_sample_loop_offsets_that_do_not_fit_xm_u32_fields() {
    let mut module = module_with_one_named_empty_sample();
    module.samples[XM_WRITER_FIRST_SAMPLE_INDEX].loop_start = XM_WRITER_OVERLONG_SAMPLE_LOOP_START;
    module.samples[XM_WRITER_FIRST_SAMPLE_INDEX].data =
        SampleData::pcm16(XM_WRITER_TEST_SAMPLE_VALUES_16.to_vec());

    assert_eq!(
        write_xm_instruments(&module).unwrap_err(),
        XmWriteError::SampleFieldTooLarge {
            instrument_index: XM_WRITER_FIRST_INSTRUMENT_INDEX,
            sample_index: XM_WRITER_FIRST_SAMPLE_INDEX,
            field: XmSampleField::LoopStart,
            value: XM_WRITER_OVERLONG_16_BIT_LOOP_START_BYTE_LEN,
            maximum: XM_WRITER_U32_FIELD_MAX,
        }
    );
}
