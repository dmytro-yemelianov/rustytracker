use rustytracker_xm::{
    XmEnvelope, XmEnvelopePoint, XmInstrument, XmInstrumentSection, XmModuleHeader, XmParseError,
    XmPatternHeader, XmResult, XmSampleData, XmSampleField, XmSampleHeader, XmWriteError,
    XmWriteResult,
};

#[test]
fn crate_root_re_exports_xm_model_and_error_api() {
    let _ = core::mem::size_of::<XmModuleHeader>();
    let _ = core::mem::size_of::<XmPatternHeader>();
    let _ = core::mem::size_of::<XmInstrumentSection>();
    let _ = core::mem::size_of::<XmInstrument>();
    let _ = core::mem::size_of::<XmEnvelope>();
    let _ = core::mem::size_of::<XmEnvelopePoint>();
    let _ = core::mem::size_of::<XmSampleHeader>();
    let _ = core::mem::size_of::<XmSampleData>();

    let parse_result: XmResult<()> = Err(XmParseError::InvalidSignature);
    assert_eq!(parse_result, Err(XmParseError::InvalidSignature));

    let write_result: XmWriteResult<()> = Err(XmWriteError::SampleFieldTooLarge {
        instrument_index: 0,
        sample_index: 0,
        field: XmSampleField::Length,
        value: 1,
        maximum: 0,
    });
    assert_eq!(
        write_result,
        Err(XmWriteError::SampleFieldTooLarge {
            instrument_index: 0,
            sample_index: 0,
            field: XmSampleField::Length,
            value: 1,
            maximum: 0,
        })
    );
}
