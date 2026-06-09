#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XmParseError {
    Truncated {
        expected: usize,
        actual: usize,
    },
    InvalidSignature,
    InvalidMarker(u8),
    UnsupportedVersion(u16),
    OrderTableTooShort {
        song_length: usize,
        available: usize,
    },
    InvalidOrderCount {
        order_count: usize,
        minimum: usize,
        maximum: usize,
    },
    InvalidChannelCount {
        channel_count: u16,
        minimum: u16,
        maximum: u16,
    },
    TooManyPatterns {
        pattern_count: u16,
        maximum: usize,
    },
    TooManyInstruments {
        instrument_count: u16,
        maximum: usize,
    },
    PatternHeaderTooShort {
        pattern_index: usize,
        expected: usize,
        actual: usize,
    },
    InvalidPatternHeaderLength {
        pattern_index: usize,
        header_length: u32,
        minimum: usize,
    },
    PatternDataTooShort {
        pattern_index: usize,
        expected: usize,
        actual: usize,
    },
    PackedPatternCellTooShort {
        pattern_index: usize,
        row: u16,
        channel: u16,
        expected: usize,
        actual: usize,
    },
    PackedPatternDataLengthMismatch {
        pattern_index: usize,
        consumed: usize,
        declared: usize,
    },
    InstrumentHeaderTooShort {
        instrument_index: usize,
        expected: usize,
        actual: usize,
    },
    InstrumentBodyTooShort {
        instrument_index: usize,
        expected: usize,
        actual: usize,
    },
    InvalidInstrumentSize {
        instrument_index: usize,
        size: u32,
    },
    InstrumentExtensionTooLong {
        instrument_index: usize,
        extension_len: usize,
        maximum: usize,
    },
    TooManyInstrumentSamples {
        instrument_index: usize,
        sample_count: u16,
        maximum: usize,
    },
    SampleHeaderTooShort {
        instrument_index: usize,
        sample_index: usize,
        expected: usize,
        actual: usize,
    },
    SampleDataTooShort {
        instrument_index: usize,
        sample_index: usize,
        expected: usize,
        actual: usize,
    },
    UnsupportedAdpcmSample {
        instrument_index: usize,
        sample_index: usize,
    },
}

pub type XmResult<T> = Result<T, XmParseError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XmWriteError {
    TooManyOrders {
        requested: usize,
        maximum: usize,
    },
    EmptyOrderList,
    InvalidChannelCount {
        channel_count: u16,
        minimum: u16,
        maximum: u16,
    },
    TooManyPatterns {
        requested: usize,
        maximum: usize,
    },
    TooManyInstruments {
        requested: usize,
        maximum: usize,
    },
    PatternDataTooLong {
        pattern_index: usize,
        byte_len: usize,
        maximum: usize,
    },
    InvalidPatternShape {
        pattern_index: usize,
        channels: u16,
        required_channels: u16,
    },
    PatternDataOutsideChannelCount {
        pattern_index: usize,
        row: u16,
        channel: u16,
        channel_count: u16,
    },
    TooManyInstrumentSamples {
        instrument_index: usize,
        requested: usize,
        maximum: usize,
    },
    SampleFieldTooLarge {
        instrument_index: usize,
        sample_index: usize,
        field: XmSampleField,
        value: u64,
        maximum: u64,
    },
}

pub type XmWriteResult<T> = Result<T, XmWriteError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XmSampleField {
    Length,
    LoopStart,
    LoopLength,
}
