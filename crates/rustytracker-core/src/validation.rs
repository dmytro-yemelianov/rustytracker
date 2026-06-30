use crate::{Module, SampleData, SAMPLE_DEFAULT_PANNING, SAMPLE_DEFAULT_VOLUME_FADEOUT};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportValidation {
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

impl ExportValidation {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Validates the export file path for issues like lossiness (non-UTF8) or incompleteness (missing extension/filename).
pub fn validate_export_path(path: &Path, expected_format: &str) -> ExportValidation {
    let mut validation = ExportValidation {
        warnings: Vec::new(),
        errors: Vec::new(),
    };

    // 1. Path UTF-8 check
    let Some(path_str) = path.to_str() else {
        validation
            .errors
            .push("Path contains invalid UTF-8/lossy characters.".to_string());
        return validation;
    };

    if path_str.trim().is_empty() {
        validation.errors.push("Path is empty.".to_string());
        return validation;
    };

    // 2. Incomplete path checks (filename / extension)
    if path.file_name().is_none() {
        validation
            .errors
            .push("Path does not contain a valid file name.".to_string());
        return validation;
    }

    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        validation
            .errors
            .push("Path is missing a file extension.".to_string());
        return validation;
    };

    let ext_lower = ext.to_lowercase();
    let expected_lower = expected_format.to_lowercase();
    if ext_lower != expected_lower {
        validation.errors.push(format!(
            "Unsupported or mismatched extension: expected '.{}', got '.{}'",
            expected_lower, ext_lower
        ));
    }

    validation
}

/// Validates the module configuration against target format limits and identifies lossy features or incomplete metadata.
pub fn validate_module_for_export(module: &Module, format: &str) -> ExportValidation {
    let mut validation = ExportValidation {
        warnings: Vec::new(),
        errors: Vec::new(),
    };

    let format_lower = format.to_lowercase();

    // 1. Check for incomplete metadata: Title is empty
    if module.header.title.as_str().trim().is_empty() {
        validation
            .warnings
            .push("Module title is empty.".to_string());
    }

    // Identify active instruments and samples
    let mut active_instruments = vec![false; module.instruments.len()];
    let mut active_samples = vec![false; module.samples.len()];

    for (i, inst) in module.instruments.iter().enumerate() {
        let has_name = !inst.name.as_str().trim().is_empty();
        let has_vol_envelope =
            inst.volume_envelope.point_count > 0 || inst.volume_envelope.flags != 0;
        let has_pan_envelope =
            inst.panning_envelope.point_count > 0 || inst.panning_envelope.flags != 0;
        let has_vibrato = inst.vibrato != crate::Vibrato::default();
        let has_custom_fadeout = inst.volume_fadeout != SAMPLE_DEFAULT_VOLUME_FADEOUT;

        if has_name || has_vol_envelope || has_pan_envelope || has_vibrato || has_custom_fadeout {
            active_instruments[i] = true;
            for &slot in &inst.sample_slots {
                if let Some(s_idx) = slot {
                    if s_idx < module.samples.len() {
                        active_samples[s_idx] = true;
                    }
                }
            }
        }
    }

    // Identify active samples by data presence
    for (s_idx, sample) in module.samples.iter().enumerate() {
        if !matches!(sample.data, SampleData::Empty) {
            active_samples[s_idx] = true;
        }
    }

    // 2. Check for incomplete metadata: Active instrument or sample has empty name
    for (i, inst) in module.instruments.iter().enumerate() {
        if active_instruments[i] && inst.name.as_str().trim().is_empty() {
            validation
                .warnings
                .push(format!("Active instrument {} has an empty name.", i));
        }
    }
    for (i, sample) in module.samples.iter().enumerate() {
        if active_samples[i] && sample.name.as_str().trim().is_empty() {
            validation
                .warnings
                .push(format!("Active sample {} has an empty name.", i));
        }
    }

    if format_lower == "xm" {
        // XM-specific validation
        if module.header.channel_count > 32 {
            validation.errors.push(format!(
                "XM format supports a maximum of 32 channels. Module has {} channels.",
                module.header.channel_count
            ));
        }
    } else if format_lower == "mod" {
        // MOD-specific validation
        // 1. Channel count constraints
        if module.header.channel_count > 32 {
            validation.errors.push(format!(
                "MOD format supports a maximum of 32 channels. Module has {} channels.",
                module.header.channel_count
            ));
        } else if module.header.channel_count != 4
            && module.header.channel_count != 6
            && module.header.channel_count != 8
        {
            validation.warnings.push(format!(
                "Module has {} channels. Standard MOD format typically supports only 4, 6, or 8 channels.",
                module.header.channel_count
            ));
        }

        // 2. Instrument constraints (MOD only supports up to 31 instruments)
        let mut too_many_instruments = false;
        for i in 31..module.instruments.len() {
            if active_instruments[i] {
                too_many_instruments = true;
                break;
            }
        }
        if too_many_instruments {
            validation.errors.push("MOD format only supports up to 31 instruments. Active instruments beyond index 31 are present.".to_string());
        }

        // 3. Envelope constraints (MOD does not support envelopes)
        for (i, inst) in module.instruments.iter().enumerate() {
            if active_instruments[i] {
                if inst.volume_envelope.point_count > 0 || inst.volume_envelope.flags != 0 {
                    validation.warnings.push(format!(
                        "Instrument {} has volume envelope configured, which MOD format does not support.",
                        i
                    ));
                }
                if inst.panning_envelope.point_count > 0 || inst.panning_envelope.flags != 0 {
                    validation.warnings.push(format!(
                        "Instrument {} has panning envelope configured, which MOD format does not support.",
                        i
                    ));
                }
            }
        }

        // 4. Sample constraints (16-bit samples, custom panning, sample length)
        for (i, sample) in module.samples.iter().enumerate() {
            if active_samples[i] {
                if matches!(sample.data, SampleData::Pcm16(_)) {
                    validation.warnings.push(format!(
                        "Sample {} is 16-bit. MOD format only supports 8-bit samples; it will be converted lossily.",
                        i
                    ));
                }
                if sample.panning != SAMPLE_DEFAULT_PANNING {
                    validation.warnings.push(format!(
                        "Sample {} has custom panning configuration, which MOD format does not support.",
                        i
                    ));
                }
                if sample.length > 131070 {
                    validation.errors.push(format!(
                        "Sample {} length ({} bytes) exceeds maximum supported by MOD format (131070 bytes).",
                        i, sample.length
                    ));
                }
            }
        }
    }

    validation
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FixedText, Instrument, Sample, SampleData};
    use std::path::Path;

    #[test]
    fn test_path_validations() {
        let valid_path = Path::new("test.xm");
        let path_val = validate_export_path(valid_path, "xm");
        assert!(path_val.is_valid());
        assert!(path_val.warnings.is_empty());

        let mismatched_path = Path::new("test.mod");
        let path_val = validate_export_path(mismatched_path, "xm");
        assert!(!path_val.is_valid());
        assert_eq!(
            path_val.errors[0],
            "Unsupported or mismatched extension: expected '.xm', got '.mod'"
        );

        let no_ext_path = Path::new("test");
        let path_val = validate_export_path(no_ext_path, "xm");
        assert!(!path_val.is_valid());
        assert_eq!(path_val.errors[0], "Path is missing a file extension.");

        let empty_path = Path::new("");
        let path_val = validate_export_path(empty_path, "xm");
        assert!(!path_val.is_valid());
        assert_eq!(path_val.errors[0], "Path is empty.");
    }

    #[test]
    fn test_module_metadata_warnings() {
        let mut module = Module::empty();
        module.header.title = FixedText::new("");
        let val = validate_module_for_export(&module, "xm");
        assert!(val.is_valid());
        assert_eq!(val.warnings[0], "Module title is empty.");

        // Mark instrument active, but give it an empty name
        module.instruments[0] = Instrument::empty(0);
        module.instruments[0].volume_envelope.point_count = 1; // makes it active
        let val = validate_module_for_export(&module, "xm");
        assert!(val
            .warnings
            .contains(&"Active instrument 0 has an empty name.".to_string()));
    }

    #[test]
    fn test_xm_channel_count() {
        let mut module = Module::empty();
        module.header.channel_count = 33; // exceeding 32
        let val = validate_module_for_export(&module, "xm");
        assert!(!val.is_valid());
        assert_eq!(
            val.errors[0],
            "XM format supports a maximum of 32 channels. Module has 33 channels."
        );
    }

    #[test]
    fn test_mod_export_constraints() {
        let mut module = Module::empty();
        module.header.channel_count = 5;
        let val = validate_module_for_export(&module, "mod");
        assert!(val.warnings.contains(&"Module has 5 channels. Standard MOD format typically supports only 4, 6, or 8 channels.".to_string()));

        // Envelope warning
        module.instruments[2] = Instrument::empty(2);
        module.instruments[2].volume_envelope.point_count = 2; // active & has envelope
        module.instruments[2].name = FixedText::new("Lead");
        let val = validate_module_for_export(&module, "mod");
        assert!(val.warnings.contains(
            &"Instrument 2 has volume envelope configured, which MOD format does not support."
                .to_string()
        ));

        // Sample warnings
        module.samples[0] = Sample {
            length: 100,
            data: SampleData::pcm16(vec![0; 100]),
            panning: 200,
            ..Sample::default()
        };
        let val = validate_module_for_export(&module, "mod");
        assert!(val.warnings.contains(&"Sample 0 is 16-bit. MOD format only supports 8-bit samples; it will be converted lossily.".to_string()));
        assert!(val.warnings.contains(
            &"Sample 0 has custom panning configuration, which MOD format does not support."
                .to_string()
        ));

        // Instrument limits (exceeding index 31)
        module.instruments[32] = Instrument::empty(32);
        module.instruments[32].name = FixedText::new("Ins 32"); // active
        let val = validate_module_for_export(&module, "mod");
        assert!(!val.is_valid());
        assert_eq!(val.errors[0], "MOD format only supports up to 31 instruments. Active instruments beyond index 31 are present.");
    }
}
