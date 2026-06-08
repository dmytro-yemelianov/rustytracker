use std::fs;
use std::path::PathBuf;

use rustytracker_core::{FrequencyTable, Module, ModuleTitle, Note, Pattern, PatternCell};
use rustytracker_xm::{
    decode_xm_patterns, parse_xm_header, parse_xm_module, parse_xm_pattern_headers,
    write_xm_header, write_xm_patterns, XmWriteError,
};

const FIXTURES: &[&str] = &[
    "milky.xm",
    "slumberjack.xm",
    "sv_ttt.xm",
    "theday.xm",
    "universalnetwork2_real.xm",
];
const XM_WRITER_TEST_VERSION: u16 = 0x0104;
const XM_WRITER_TEST_HEADER_SIZE: u32 = 276;
const XM_WRITER_TEST_ORDERS: &[u8] = &[0, 0, 0];
const XM_WRITER_ORDER_TABLE_LEN: usize = 256;
const XM_WRITER_OVERLONG_ORDER_LEN: usize = XM_WRITER_ORDER_TABLE_LEN + 1;
const XM_WRITER_TEST_ROWS: u16 = 1;
const XM_WRITER_TEST_CHANNELS: u16 = 2;
const XM_WRITER_TEST_EFFECT_SLOTS: u8 = 2;
const XM_WRITER_TEST_NOTE: u8 = 49;
const XM_WRITER_TEST_INSTRUMENT: u8 = 3;
const XM_WRITER_PATTERN_HEADER_LEN: u32 = 9;
const XM_WRITER_PATTERN_PACKING_TYPE: u8 = 0;
const XM_WRITER_EMPTY_PATTERN_DATA_LEN: u16 = 0;

#[test]
fn writes_empty_module_header_and_order_table() {
    let mut module = Module::empty();
    module.header.title = ModuleTitle::new("empty writer test");
    module.orders = XM_WRITER_TEST_ORDERS.to_vec();

    let bytes = write_xm_header(&module).unwrap();
    let header = parse_xm_header(&bytes).unwrap();

    assert_eq!(header.title, "empty writer test");
    assert_eq!(header.version, XM_WRITER_TEST_VERSION);
    assert_eq!(header.header_size, XM_WRITER_TEST_HEADER_SIZE);
    assert_eq!(header.song_length, module.orders.len() as u16);
    assert_eq!(header.restart_position, module.header.restart_position);
    assert_eq!(header.channel_count, module.header.channel_count);
    assert_eq!(header.pattern_count, module.patterns.len() as u16);
    assert_eq!(header.instrument_count, module.instruments.len() as u16);
    assert_eq!(header.frequency_table, FrequencyTable::Linear);
    assert_eq!(header.default_tick_speed, module.header.tick_speed);
    assert_eq!(header.default_bpm, module.header.bpm);
    assert_eq!(header.orders, module.orders);
}

#[test]
fn roundtrips_bundled_fixture_headers_and_orders() {
    for fixture in FIXTURES {
        let bytes = fs::read(fixture_path(fixture)).unwrap();
        let module = parse_xm_module(&bytes).unwrap();

        let written = write_xm_header(&module).unwrap();
        let header = parse_xm_header(&written).unwrap();

        assert_eq!(header.title, module.header.title.as_str(), "{fixture}");
        assert_eq!(
            header.channel_count, module.header.channel_count,
            "{fixture}"
        );
        assert_eq!(
            header.frequency_table, module.header.frequency_table,
            "{fixture}"
        );
        assert_eq!(
            header.default_tick_speed, module.header.tick_speed,
            "{fixture}"
        );
        assert_eq!(header.default_bpm, module.header.bpm, "{fixture}");
        assert_eq!(
            header.restart_position, module.header.restart_position,
            "{fixture}"
        );
        assert_eq!(header.orders, module.orders, "{fixture}");
        assert_eq!(
            header.pattern_count,
            module.patterns.len() as u16,
            "{fixture}"
        );
        assert_eq!(
            header.instrument_count,
            module.instruments.len() as u16,
            "{fixture}"
        );
    }
}

#[test]
fn rejects_order_tables_that_do_not_fit_in_xm_header() {
    let mut module = Module::empty();
    module.orders = vec![0; XM_WRITER_OVERLONG_ORDER_LEN];

    assert_eq!(
        write_xm_header(&module).unwrap_err(),
        XmWriteError::TooManyOrders {
            requested: XM_WRITER_OVERLONG_ORDER_LEN,
            maximum: XM_WRITER_ORDER_TABLE_LEN,
        }
    );
}

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

fn fixture_path(file_name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../MilkyTracker/resources/music")
        .join(file_name)
}

fn write_header_and_patterns(module: &Module) -> Vec<u8> {
    let mut bytes = write_xm_header(module).unwrap();
    bytes.extend_from_slice(&write_xm_patterns(module).unwrap());
    bytes
}
