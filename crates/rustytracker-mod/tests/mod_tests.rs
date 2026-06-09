use rustytracker_core::{Note, SampleData, SampleLoopKind};
use rustytracker_mod::{parse_mod_module, ModParseError};

/// Helper to generate a valid, minimal MOD buffer.
/// If `sig` is provided, we write it to bytes 1080..1084 and make a 31-instrument header.
/// Otherwise, we generate a 15-instrument header (length 600 + patterns + samples).
#[allow(clippy::too_many_arguments)]
fn create_mock_mod(
    sig: Option<&[u8; 4]>,
    song_length: u8,
    orders: &[u8],
    patterns_data: &[Vec<u8>],   // each pattern is channels * 64 * 4 bytes
    sample_lengths: &[u16],      // sample lengths in words
    sample_loop_starts: &[u16],  // loop starts in words
    sample_loop_lengths: &[u16], // loop lengths in words
    sample_payloads: &[Vec<u8>], // sample byte data
) -> Vec<u8> {
    let instrument_count = if sig.is_some() { 31 } else { 15 };

    let header_len = 20 + instrument_count * 30 + 2 + 128 + if sig.is_some() { 4 } else { 0 };

    let mut bytes = vec![0u8; header_len];

    // 1. Title (20 bytes)
    let title = b"Test Module";
    bytes[0..title.len()].copy_from_slice(title);

    // 2. Instruments (30 bytes each)
    let mut cursor = 20;
    for i in 0..instrument_count {
        // Name (22 bytes)
        let name = format!("Sample {i}");
        let name_bytes = name.as_bytes();
        bytes[cursor..cursor + name_bytes.len()].copy_from_slice(name_bytes);
        cursor += 22;

        // Sample length (2 bytes in words)
        let len_val = sample_lengths.get(i).copied().unwrap_or(0);
        bytes[cursor..cursor + 2].copy_from_slice(&len_val.to_be_bytes());
        cursor += 2;

        // Finetune (1 byte)
        bytes[cursor] = if i == 1 { 8 } else { 0 }; // Instrument 1 has finetune 8 -> -128
        cursor += 1;

        // Volume (1 byte, 0..64)
        bytes[cursor] = if i == 2 { 32 } else { 64 }; // Instrument 2 has volume 32 -> 128, others 64 -> 255
        cursor += 1;

        // Loop start (2 bytes in words)
        let loop_start_val = sample_loop_starts.get(i).copied().unwrap_or(0);
        bytes[cursor..cursor + 2].copy_from_slice(&loop_start_val.to_be_bytes());
        cursor += 2;

        // Loop length (2 bytes in words)
        let loop_len_val = sample_loop_lengths.get(i).copied().unwrap_or(0);
        bytes[cursor..cursor + 2].copy_from_slice(&loop_len_val.to_be_bytes());
        cursor += 2;
    }

    // 3. Song length and restart (2 bytes)
    bytes[cursor] = song_length;
    bytes[cursor + 1] = 0; // restart position
    cursor += 2;

    // 4. Order List (128 bytes)
    for (i, &order) in orders.iter().enumerate() {
        if i < 128 {
            bytes[cursor + i] = order;
        }
    }
    cursor += 128;

    // 5. Signature (4 bytes)
    if let Some(s) = sig {
        bytes[cursor..cursor + 4].copy_from_slice(s);
        cursor += 4;
    }

    assert_eq!(cursor, header_len);

    // 6. Pattern Data
    for pat in patterns_data {
        bytes.extend_from_slice(pat);
    }

    // 7. Sample Data (signed 8-bit PCM)
    for payload in sample_payloads {
        bytes.extend_from_slice(payload);
    }

    bytes
}

#[test]
fn test_truncated_mod() {
    let bytes = vec![0u8; 100];
    let result = parse_mod_module(&bytes);
    assert!(matches!(result, Err(ModParseError::Truncated { .. })));
}

#[test]
fn test_15_instrument_parsing() {
    // 15 instruments, 4 channels
    // 1 pattern of 4 channels * 64 rows * 4 bytes = 1024 bytes
    // No samples
    let pattern_size = 4 * 64 * 4;
    let pat_bytes = vec![0u8; pattern_size];

    let bytes = create_mock_mod(
        None, // no signature -> 15 instrument mode
        1,
        &[0],
        &[pat_bytes],
        &[],
        &[],
        &[],
        &[],
    );

    let module = parse_mod_module(&bytes).unwrap();
    assert_eq!(module.header.title.as_str(), "Test Module");
    assert_eq!(module.header.channel_count, 4);
    assert_eq!(module.orders, vec![0]);
    assert_eq!(module.patterns.len(), 1);
    assert_eq!(module.instruments[0].name.as_str(), "Sample 0");
    assert_eq!(module.instruments[14].name.as_str(), "Sample 14");
    // Beyond 15 should be empty default instruments
    assert_eq!(module.instruments[15].name.as_str(), "");
}

#[test]
fn test_31_instrument_4_channel_parsing() {
    // 31 instruments, 4 channels (M.K. signature)
    let pattern_size = 4 * 64 * 4;
    // Row 0, Channel 0: Note period 856 (C-3), Instrument 1, Effect 0xC (set volume), operand 0x40 (64)
    // Row 0, Channel 1: Note period 0, Instrument 0, Effect 0x01 (portamento up), operand 0x00 (fine portamento -> mapped to 0)
    // Row 0, Channel 2: Note period 0, Instrument 0, Effect 0x0F (set speed), operand 0x06
    let mut pat_bytes = vec![0u8; pattern_size];

    // Channel 0: b1 = 0x00 (ins_num hi | period hi), b2 = 0x00, b3 = 0x00, b4 = 0x00
    // We want period = 856 (0x358) and instrument = 1 (0x01)
    // b1 = (1 & 0x10) | (856 >> 8) = 0x03
    // b2 = 856 & 0xff = 0x58
    // b3 = (1 << 4) | 0x0c = 0x1c
    // b4 = 64 = 0x40
    pat_bytes[0] = 0x03;
    pat_bytes[1] = 0x58;
    pat_bytes[2] = 0x1c;
    pat_bytes[3] = 0x40;

    // Channel 1: period 0, Instrument 0, Effect 0x01, operand 0x00
    // Mapped: effect 0x01, operand 0x00 -> effect = 0, operand = 0
    pat_bytes[4] = 0x00;
    pat_bytes[5] = 0x00;
    pat_bytes[6] = 0x01;
    pat_bytes[7] = 0x00;

    // Channel 2: period 0, Instrument 0, Effect 0x0F, operand 0x06
    // Mapped: effect = 0x0F (15), operand = 6
    pat_bytes[8] = 0x00;
    pat_bytes[9] = 0x00;
    pat_bytes[10] = 0x0f;
    pat_bytes[11] = 0x06;

    let sample_lengths = vec![4; 31]; // 4 words = 8 bytes
    let sample_payloads = vec![vec![1, 2, 3, 4, 252, 253, 254, 255]; 31];

    let bytes = create_mock_mod(
        Some(b"M.K."),
        1,
        &[0],
        &[pat_bytes],
        &sample_lengths,
        &[],
        &[],
        &sample_payloads,
    );

    let module = parse_mod_module(&bytes).unwrap();
    assert_eq!(module.header.channel_count, 4);
    assert_eq!(module.instruments[0].name.as_str(), "Sample 0");
    assert_eq!(module.instruments[30].name.as_str(), "Sample 30");

    // Test note translation
    let cell_0_0 = module.patterns[0].cell(0, 0).unwrap();
    assert_eq!(cell_0_0.note, Note::Key(37)); // C-3 (note value 37)
    assert_eq!(cell_0_0.instrument, 1);
    assert_eq!(cell_0_0.effects[0].effect, 0x0c);
    assert_eq!(cell_0_0.effects[0].operand, 255); // vol64_to_255(64) -> 255

    // Test effect mapping: Effect 1 with operand 0 should be mapped to 0 (ignored/empty)
    let cell_0_1 = module.patterns[0].cell(1, 0).unwrap();
    assert_eq!(cell_0_1.effects[0].effect, 0);
    assert_eq!(cell_0_1.effects[0].operand, 0);

    // Test effect 15: speed
    let cell_0_2 = module.patterns[0].cell(2, 0).unwrap();
    assert_eq!(cell_0_2.effects[0].effect, 0x0f);
    assert_eq!(cell_0_2.effects[0].operand, 0x06);

    // Check sample volume scaling and finetunes
    assert_eq!(module.samples[0].volume, 255);
    assert_eq!(module.samples[1].finetune, -128); // Finetune 8
    assert_eq!(module.samples[2].volume, 128); // Volume 32 -> 128

    // Check signed 8-bit PCM data loading
    match &module.samples[0].data {
        SampleData::Pcm8(data) => {
            assert_eq!(data, &[1, 2, 3, 4, -4, -3, -2, -1]);
        }
        other => panic!("expected PCM8, got {other:?}"),
    }
}

#[test]
fn test_31_instrument_8_channel_parsing() {
    // 31 instruments, 8 channels (FLT8 signature)
    let pattern_size = 8 * 64 * 4;
    let pat_bytes = vec![0u8; pattern_size];

    let bytes = create_mock_mod(Some(b"FLT8"), 1, &[0], &[pat_bytes], &[], &[], &[], &[]);

    let module = parse_mod_module(&bytes).unwrap();
    assert_eq!(module.header.channel_count, 8);
}

#[test]
fn test_loop_correction() {
    // Test that loop start & length correction works:
    // length = 10 bytes (5 words)
    // loop start = 3 words (6 bytes)
    // loop length = 3 words (6 bytes) -> total = 12 bytes (> 10)
    // Corrected loop start: 6 - (12 - 10) = 4 bytes.
    let sample_lengths = vec![5]; // 5 words = 10 bytes
    let sample_loop_starts = vec![3]; // 3 words = 6 bytes
    let sample_loop_lengths = vec![3]; // 3 words = 6 bytes
    let sample_payloads = vec![vec![0; 10]];

    let bytes = create_mock_mod(
        Some(b"M.K."),
        1,
        &[0],
        &[vec![0; 1024]],
        &sample_lengths,
        &sample_loop_starts,
        &sample_loop_lengths,
        &sample_payloads,
    );

    let module = parse_mod_module(&bytes).unwrap();
    let sample = &module.samples[0];
    assert_eq!(sample.length, 10);
    assert_eq!(sample.loop_start, 4); // Adjusted from 6 to 4 to fit in length 10
    assert_eq!(sample.loop_length, 6);
    assert_eq!(sample.loop_kind, SampleLoopKind::Forward);
}

#[test]
fn test_mod_writer_roundtrip() {
    // 31 instruments, 4 channels
    let pattern_size = 4 * 64 * 4;
    let mut pat_bytes = vec![0u8; pattern_size];

    // C-3 (note 37, period 856), instrument 1, effect 0xC, volume 64
    pat_bytes[0] = 0x03;
    pat_bytes[1] = 0x58;
    pat_bytes[2] = 0x1c;
    pat_bytes[3] = 0x40;

    let sample_lengths = vec![4; 31];
    let sample_payloads = vec![vec![1, 2, 3, 4, 252, 253, 254, 255]; 31];

    let original_bytes = create_mock_mod(
        Some(b"M.K."),
        1,
        &[0],
        &[pat_bytes],
        &sample_lengths,
        &[],
        &[],
        &sample_payloads,
    );

    let parsed_original = parse_mod_module(&original_bytes).unwrap();

    // Export using writer
    let exported_bytes = rustytracker_mod::write_mod_module(&parsed_original).unwrap();

    // Re-parse exported bytes
    let parsed_roundtrip = parse_mod_module(&exported_bytes).unwrap();

    assert_eq!(parsed_roundtrip.header.title.as_str(), parsed_original.header.title.as_str());
    assert_eq!(parsed_roundtrip.header.channel_count, parsed_original.header.channel_count);
    assert_eq!(parsed_roundtrip.orders, parsed_original.orders);
    assert_eq!(parsed_roundtrip.patterns.len(), parsed_original.patterns.len());

    // Compare first cell
    let cell_orig = parsed_original.patterns[0].cell(0, 0).unwrap();
    let cell_round = parsed_roundtrip.patterns[0].cell(0, 0).unwrap();
    assert_eq!(cell_round.note, cell_orig.note);
    assert_eq!(cell_round.instrument, cell_orig.instrument);
    assert_eq!(cell_round.effects[0].effect, cell_orig.effects[0].effect);
    assert_eq!(cell_round.effects[0].operand, cell_orig.effects[0].operand);

    // Compare instrument and sample values
    for i in 0..31 {
        assert_eq!(parsed_roundtrip.instruments[i].name.as_str(), parsed_original.instruments[i].name.as_str());
        assert_eq!(parsed_roundtrip.samples[i].volume, parsed_original.samples[i].volume);
        assert_eq!(parsed_roundtrip.samples[i].finetune, parsed_original.samples[i].finetune);
        assert_eq!(parsed_roundtrip.samples[i].length, parsed_original.samples[i].length);

        match (&parsed_roundtrip.samples[i].data, &parsed_original.samples[i].data) {
            (SampleData::Pcm8(r_data), SampleData::Pcm8(o_data)) => {
                assert_eq!(r_data, o_data);
            }
            (SampleData::Empty, SampleData::Empty) => {}
            _ => panic!("Sample data mismatch for index {i}"),
        }
    }
}

