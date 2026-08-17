//! Reset to defaults & CSV import/export Tauri commands.

use libretune_core::ini::DataType;
use libretune_core::tune::TuneValue;

use crate::commands::constant_values::read_constant_from_cache_or_tune;
use crate::state::AppState;

/// Reset all tune values to their INI-defined defaults
#[tauri::command]
pub async fn reset_tune_to_defaults(state: tauri::State<'_, AppState>) -> Result<u32, String> {
    let def_guard = state.definition.lock().await;
    let mut cache_guard = state.tune_cache.lock().await;
    let mut tune_guard = state.current_tune.lock().await;

    let def = def_guard.as_ref().ok_or("Definition not loaded")?;
    let cache = cache_guard.as_mut().ok_or("Tune cache not loaded")?;
    let tune = tune_guard.as_mut().ok_or("No tune loaded")?;

    let mut reset_count = 0u32;
    let mut skipped_no_default = 0usize;

    // Reset each constant to its default value
    for (name, constant) in &def.constants {
        // Skip arrays - they don't have simple defaults
        if !matches!(constant.shape, libretune_core::ini::Shape::Scalar) {
            continue;
        }

        // Only reset constants the INI actually declares a default for.
        // Falling back to `constant.min` turned "reset to defaults" into
        // "reset to minimum" for the ~68% of constants with no [Defaults]
        // entry — observed live: reqFuel 12.5 -> 0, nCylinders -> "INVALID",
        // reported as a successful reset of 730 constants.
        let Some(&default_value) = def.default_values.get(name) else {
            skipped_no_default += 1;
            continue;
        };

        // Update PC variable locally
        if constant.is_pc_variable {
            cache.local_values.insert(name.clone(), default_value);
            tune.constants
                .insert(name.clone(), TuneValue::Scalar(default_value));
            reset_count += 1;
            continue;
        }

        // Update ECU constant in cache and tune file
        // Convert display value to raw value for storage
        let raw_value = constant.display_to_raw(default_value);

        // Update tune file
        tune.constants
            .insert(name.clone(), TuneValue::Scalar(default_value));

        // Encode with the INI's declared endianness. encode_constant_value
        // hardcoded big-endian, byte-swapping every multi-byte scalar on
        // little-endian ECUs (Speeduino/rusEFI) — observed live: mapMax 260
        // written as 1025, boostSens 2000 as 53255.
        let mut bytes = vec![0u8; constant.data_type.size_bytes()];
        constant
            .data_type
            .write_to_bytes(&mut bytes, 0, raw_value, def.endianness);
        cache.write_bytes(constant.page, constant.offset, &bytes);
        reset_count += 1;
    }

    if skipped_no_default > 0 {
        tracing::info!(
            "reset_tune_to_defaults: {} constants reset; {} skipped (no [Defaults] entry)",
            reset_count,
            skipped_no_default
        );
    }
    Ok(reset_count)
}

/// Export tune data to CSV file
#[tauri::command]
pub async fn export_tune_as_csv(
    state: tauri::State<'_, AppState>,
    path: String,
) -> Result<u32, String> {
    let def_guard = state.definition.lock().await;
    let cache_guard = state.tune_cache.lock().await;
    let tune_guard = state.current_tune.lock().await;

    let def = def_guard.as_ref().ok_or("Definition not loaded")?;

    let mut csv_lines = Vec::new();
    csv_lines.push(
        "Name,Page,Offset,Shape,Value,Units,Min,Max,Scale,Translate,DataType,IsPcVariable"
            .to_string(),
    );

    let mut export_count = 0u32;

    // Export all constants
    for (name, constant) in &def.constants {
        // Get the current value(s)
        let value_str = if constant.data_type == DataType::String {
            // String constant — read raw bytes from cache/tune
            let str_len = constant.size_bytes();
            let raw = if let Some(cache) = cache_guard.as_ref() {
                cache
                    .read_bytes(constant.page, constant.offset, str_len as u16)
                    .map(|b| b.to_vec())
            } else {
                None
            };
            let raw = raw.or_else(|| {
                tune_guard.as_ref().and_then(|tune| {
                    tune.pages.get(&constant.page).and_then(|page_data| {
                        let start = constant.offset as usize;
                        let end = start + str_len;
                        if end <= page_data.len() {
                            Some(page_data[start..end].to_vec())
                        } else {
                            None
                        }
                    })
                })
            });
            if let Some(bytes) = raw {
                // Trim null padding
                let s = String::from_utf8_lossy(&bytes);
                let trimmed = s.trim_end_matches('\0');
                format!("\"{}\"", trimmed.replace('"', "\"\""))
            } else {
                "\"\"".to_string()
            }
        } else if matches!(constant.shape, libretune_core::ini::Shape::Scalar) {
            // Scalar constant
            let value = read_constant_from_cache_or_tune(
                name,
                constant,
                def.endianness,
                tune_guard.as_ref(),
                cache_guard.as_ref(),
            );
            format!("{}", value)
        } else {
            // Array constant — read all elements
            let elem_size = constant.data_type.size_bytes();
            let elem_count = constant.shape.element_count();
            let mut values = Vec::with_capacity(elem_count);

            for idx in 0..elem_count {
                let offset = constant.offset + (idx * elem_size) as u16;
                let raw_bytes = if let Some(cache) = cache_guard.as_ref() {
                    cache
                        .read_bytes(constant.page, offset, elem_size as u16)
                        .map(|b| b.to_vec())
                } else {
                    None
                };
                let raw_bytes = raw_bytes.or_else(|| {
                    tune_guard.as_ref().and_then(|tune| {
                        tune.pages.get(&constant.page).and_then(|page_data| {
                            let start = offset as usize;
                            let end = start + elem_size;
                            if end <= page_data.len() {
                                Some(page_data[start..end].to_vec())
                            } else {
                                None
                            }
                        })
                    })
                });
                let raw_val = if let Some(bytes) = raw_bytes {
                    constant
                        .data_type
                        .read_from_bytes(&bytes, 0, def.endianness)
                        .unwrap_or(0.0)
                } else {
                    0.0
                };
                let display_val = constant.raw_to_display(raw_val);
                values.push(format!("{}", display_val));
            }
            format!("\"[{}]\"", values.join(","))
        };

        let shape_str = match &constant.shape {
            libretune_core::ini::Shape::Scalar => "scalar".to_string(),
            libretune_core::ini::Shape::Array1D(n) => format!("[{}]", n),
            libretune_core::ini::Shape::Array2D { rows, cols } => format!("[{}x{}]", rows, cols),
        };

        // Escape name and units for CSV (in case they contain commas)
        let escaped_name = if name.contains(',') || name.contains('"') {
            format!("\"{}\"", name.replace('"', "\"\""))
        } else {
            name.clone()
        };
        let escaped_units = if constant.units.contains(',') || constant.units.contains('"') {
            format!("\"{}\"", constant.units.replace('"', "\"\""))
        } else {
            constant.units.clone()
        };

        let data_type_str = format!("{:?}", constant.data_type);

        csv_lines.push(format!(
            "{},{},{},{},{},{},{},{},{},{},{},{}",
            escaped_name,
            constant.page,
            constant.offset,
            shape_str,
            value_str,
            escaped_units,
            constant.min,
            constant.max,
            constant.scale,
            constant.translate,
            data_type_str,
            constant.is_pc_variable
        ));
        export_count += 1;
    }

    // Write to file
    let csv_content = csv_lines.join("\n");
    std::fs::write(&path, csv_content).map_err(|e| format!("Failed to write CSV file: {}", e))?;

    Ok(export_count)
}

/// Import tune data from CSV file
#[tauri::command]
pub async fn import_tune_from_csv(
    state: tauri::State<'_, AppState>,
    path: String,
) -> Result<u32, String> {
    let def_guard = state.definition.lock().await;
    let mut cache_guard = state.tune_cache.lock().await;
    let mut tune_guard = state.current_tune.lock().await;

    let def = def_guard.as_ref().ok_or("Definition not loaded")?;
    let cache = cache_guard.as_mut().ok_or("Tune cache not loaded")?;
    let tune = tune_guard.as_mut().ok_or("No tune loaded")?;

    // Read CSV file
    let csv_content =
        std::fs::read_to_string(&path).map_err(|e| format!("Failed to read CSV file: {}", e))?;

    let mut import_count = 0u32;
    let mut errors = Vec::new();

    for (line_num, line) in csv_content.lines().enumerate() {
        // Skip header
        if line_num == 0 && (line.starts_with("Name,") || line.starts_with("\"Name\"")) {
            continue;
        }

        // Skip empty lines
        if line.trim().is_empty() {
            continue;
        }

        // Parse CSV line (simple parser - handles basic quoting)
        let fields: Vec<&str> = parse_csv_line(line);

        // Support both old format (11 cols: Name,Page,Offset,Value,...)
        // and new format (12 cols: Name,Page,Offset,Shape,Value,...)
        let (name, value_field) = if fields.len() >= 12 {
            // New format with Shape column
            (fields[0].trim(), fields[4].trim())
        } else if fields.len() >= 4 {
            // Legacy format without Shape column
            (fields[0].trim(), fields[3].trim())
        } else {
            errors.push(format!("Line {}: too few fields", line_num + 1));
            continue;
        };

        // Find constant in definition
        let constant = match def.constants.get(name) {
            Some(c) => c,
            None => {
                // Constant not found - skip silently (might be from different INI)
                continue;
            }
        };

        // Handle string constants
        if constant.data_type == DataType::String {
            let str_val = value_field
                .trim_start_matches('"')
                .trim_end_matches('"')
                .replace("\"\"", "\"");
            let max_len = constant.size_bytes();
            let mut raw_data = vec![0u8; max_len];
            let copy_len = str_val.len().min(max_len);
            raw_data[..copy_len].copy_from_slice(&str_val.as_bytes()[..copy_len]);
            cache.write_bytes(constant.page, constant.offset, &raw_data);
            tune.constants
                .insert(name.to_string(), TuneValue::String(str_val));
            import_count += 1;
            continue;
        }

        // Handle array constants (value looks like "[1.0,2.0,3.0]")
        if !matches!(constant.shape, libretune_core::ini::Shape::Scalar) {
            let array_str = value_field
                .trim_start_matches('"')
                .trim_end_matches('"')
                .trim_start_matches('[')
                .trim_end_matches(']');

            let elem_size = constant.data_type.size_bytes();
            let elem_count = constant.shape.element_count();
            let values: Vec<f64> = array_str
                .split(',')
                .filter_map(|s| s.trim().parse::<f64>().ok())
                .collect();

            let parse_count = values.len().min(elem_count);
            for (idx, &display_val) in values.iter().take(parse_count).enumerate() {
                let clamped = display_val.clamp(constant.min, constant.max);
                let raw_val = constant.display_to_raw(clamped);
                let offset = constant.offset + (idx * elem_size) as u16;
                let mut bytes = vec![0u8; elem_size];
                constant
                    .data_type
                    .write_to_bytes(&mut bytes, 0, raw_val, def.endianness);
                cache.write_bytes(constant.page, offset, &bytes);
            }

            tune.constants.insert(
                name.to_string(),
                TuneValue::Array(truncate_imported_array(values, elem_count)),
            );
            import_count += 1;
            continue;
        }

        // Scalar constant
        let value: f64 = match value_field.parse() {
            Ok(v) => v,
            Err(_) => {
                errors.push(format!(
                    "Line {}: invalid value '{}'",
                    line_num + 1,
                    value_field
                ));
                continue;
            }
        };

        // Find constant in definition
        let constant = match def.constants.get(name) {
            Some(c) => c,
            None => {
                // Constant not found - skip silently (might be from different INI)
                continue;
            }
        };

        // Validate value is within bounds
        let clamped_value = value.clamp(constant.min, constant.max);
        if (clamped_value - value).abs() > 0.0001 {
            errors.push(format!(
                "Line {}: value {} clamped to {} (range {}-{})",
                line_num + 1,
                value,
                clamped_value,
                constant.min,
                constant.max
            ));
        }

        // Update PC variable locally
        if constant.is_pc_variable {
            cache.local_values.insert(name.to_string(), clamped_value);
            tune.constants
                .insert(name.to_string(), TuneValue::Scalar(clamped_value));
            import_count += 1;
            continue;
        }

        // Update ECU constant
        let raw_value = constant.display_to_raw(clamped_value);
        tune.constants
            .insert(name.to_string(), TuneValue::Scalar(clamped_value));

        // Encode with the INI's declared endianness. encode_constant_value
        // hardcoded big-endian, byte-swapping every multi-byte scalar on
        // little-endian ECUs (Speeduino/rusEFI) — observed live: mapMax 260
        // written as 1025, boostSens 2000 as 53255.
        let mut bytes = vec![0u8; constant.data_type.size_bytes()];
        constant
            .data_type
            .write_to_bytes(&mut bytes, 0, raw_value, def.endianness);
        cache.write_bytes(constant.page, constant.offset, &bytes);
        import_count += 1;
    }

    // Log errors if any
    if !errors.is_empty() {
        eprintln!("[CSV Import] {} warnings/errors:", errors.len());
        for err in errors.iter().take(10) {
            eprintln!("  {}", err);
        }
        if errors.len() > 10 {
            eprintln!("  ... and {} more", errors.len() - 10);
        }
    }

    Ok(import_count)
}
/// Simple CSV line parser that handles quoted fields
pub(crate) fn parse_csv_line(line: &str) -> Vec<&str> {
    let mut fields = Vec::new();
    let mut start = 0;
    let mut in_quotes = false;

    // Use char_indices() so `i` is a byte offset into `line` (matching what
    // `line[start..i]` needs) instead of a char-count index — indexing by
    // char position and then slicing the str by byte range panics as soon as
    // any multi-byte UTF-8 character (e.g. the "°" used in some INI units)
    // appears before a delimiter.
    for (i, ch) in line.char_indices() {
        if ch == '"' {
            in_quotes = !in_quotes;
        } else if ch == ',' && !in_quotes {
            let field = &line[start..i];
            // Strip surrounding quotes if present
            let trimmed = field.trim();
            if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
                fields.push(&trimmed[1..trimmed.len() - 1]);
            } else {
                fields.push(trimmed);
            }
            start = i + ch.len_utf8();
        }
    }

    // Add last field
    let field = &line[start..];
    let trimmed = field.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        fields.push(&trimmed[1..trimmed.len() - 1]);
    } else {
        fields.push(trimmed);
    }

    fields
}

/// Truncate an imported array-constant's values to `elem_count` so
/// `tune.constants` stays consistent with what actually gets written to
/// cache — a CSV containing more values than the constant's defined size
/// should not leave an oversized array in memory.
pub(crate) fn truncate_imported_array(mut values: Vec<f64>, elem_count: usize) -> Vec<f64> {
    values.truncate(values.len().min(elem_count));
    values
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_csv_line_splits_ascii_fields() {
        let fields = parse_csv_line("foo,1,2,bar");
        assert_eq!(fields, vec!["foo", "1", "2", "bar"]);
    }

    #[test]
    fn parse_csv_line_handles_quoted_fields_with_commas() {
        let fields = parse_csv_line(r#"name,"[1,2,3]",units"#);
        assert_eq!(fields, vec!["name", "[1,2,3]", "units"]);
    }

    #[test]
    fn parse_csv_line_does_not_panic_on_multibyte_utf8_before_delimiter() {
        // Regression test: demo.ini ships constants with units like "gear N°"
        // (e.g. torqueReductionCutGearBins). Exporting a tune with such a
        // constant, then re-importing that CSV, used to panic because the
        // old parser mixed a char-index loop counter with byte-index string
        // slicing — any multi-byte UTF-8 char before a comma corrupted the
        // byte offset used to slice the &str.
        let fields = parse_csv_line(
            "torqueReductionCutGearBins,21,58,[2],\"[1,2]\",gear N°,0,20,1,0,S08,false",
        );
        assert_eq!(fields[0], "torqueReductionCutGearBins");
        assert_eq!(fields[5], "gear N°");
        assert_eq!(fields[11], "false");
    }

    #[test]
    fn parse_csv_line_handles_multibyte_utf8_immediately_before_delimiter() {
        // Delimiter directly follows the multi-byte char (no ASCII buffer),
        // which is the tightest case for a char/byte index mismatch to panic.
        let fields = parse_csv_line("N°,next");
        assert_eq!(fields, vec!["N°", "next"]);
    }

    #[test]
    fn truncate_imported_array_leaves_undersized_arrays_untouched() {
        assert_eq!(truncate_imported_array(vec![1.0, 2.0], 4), vec![1.0, 2.0]);
    }

    #[test]
    fn truncate_imported_array_truncates_oversized_arrays_to_elem_count() {
        // A CSV hand-edited (or from a mismatched INI) to have more values
        // than the constant's defined shape must not leave tune.constants
        // holding an array longer than what was actually written to cache.
        assert_eq!(
            truncate_imported_array(vec![1.0, 2.0, 3.0, 4.0, 5.0], 3),
            vec![1.0, 2.0, 3.0]
        );
    }
}
