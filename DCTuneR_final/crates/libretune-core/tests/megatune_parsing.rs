//! `[MegaTune]` section parsing robustness tests.
//!
//! Real-world INI files (especially hand-edited or legacy ones) deviate from
//! the clean key=value form. These tests confirm `EcuDefinition::from_str`
//! tolerates three common variations found in the wild:
//! - excess inline whitespace (padded by editors),
//! - trailing `;` comments (TunerStudio-style annotations),
//! - case-insensitive section headers (`[MEGATUNE]`).
//!
//! Each test uses a minimal `[MegaTune]` section and asserts both the parsed
//! `signature` and `queryCommand` so a regression in either field is caught.

#[cfg(test)]
mod tests {
    use libretune_core::ini::EcuDefinition;

    /// Leading/trailing whitespace around values must be trimmed — INI files
    /// are often auto-aligned by editors, producing padded `=` assignments.
    #[test]
    fn test_parse_megatune_with_whitespace() {
        let content = r#"
[MegaTune]
   signature      =   "test_signature"   
   queryCommand   =   "Q"

"#;
        let def = EcuDefinition::from_str(content).unwrap();
        assert_eq!(def.signature, "test_signature");
        assert_eq!(def.query_command, "Q");
    }

    /// Trailing `;` line comments (TunerStudio convention) must be stripped
    /// from the value, otherwise `signature` would silently include the
    /// comment text and break ECU signature matching.
    #[test]
    fn test_parse_megatune_with_comments() {
        let content = r#"
[MegaTune]
   signature = "test_signature" ; This is a comment
   queryCommand = "Q"
"#;
        let def = EcuDefinition::from_str(content).unwrap();
        assert_eq!(def.signature, "test_signature");
        assert_eq!(def.query_command, "Q");
    }

    /// Section names are matched case-insensitively. MegaSquirt-era INI files
    /// have been seen with `[MEGATUNE]`, `[megatune]`, etc. — a strict match
    /// would reject otherwise-valid files.
    #[test]
    fn test_parse_megatune_case_insensitive() {
        let content = r#"
[MEGATUNE]
   signature = "test_signature"
   queryCommand = "Q"
"#;
        let def = EcuDefinition::from_str(content).unwrap();
        assert_eq!(def.signature, "test_signature");
        assert_eq!(def.query_command, "Q");
    }
}
