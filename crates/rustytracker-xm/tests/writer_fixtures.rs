use std::fs;
use std::path::PathBuf;

use rustytracker_core::{FrequencyTable, Module, ModuleTitle};
use rustytracker_xm::{parse_xm_header, parse_xm_module, write_xm_header, XmWriteError};

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

fn fixture_path(file_name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../MilkyTracker/resources/music")
        .join(file_name)
}
