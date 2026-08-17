//! Text encoding helpers for INI files.
//!
//! Many ECU INI files in the wild — especially translated ones — use Windows-1252
//! (a superset of ISO-8859-1) rather than UTF-8. Reading them with
//! `String::from_utf8_lossy` corrupts every non-ASCII byte to `U+FFFD`,
//! so for example `Configurações` becomes `Configura��es`.
//!
//! [`decode_ini_bytes`] tries UTF-8 first (the modern, lossless path) and
//! transparently falls back to Windows-1252 when the bytes are not valid UTF-8.
//! UTF-8 byte-order marks are stripped if present.

const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];

/// Decode raw INI file bytes into a `String`, preferring UTF-8 and falling
/// back to Windows-1252 for legacy / translated definition files.
///
/// This never produces `U+FFFD` replacement characters: every byte sequence
/// is representable in Windows-1252, so the fallback is lossless.
pub fn decode_ini_bytes(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(UTF8_BOM).unwrap_or(bytes);

    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_owned(),
        Err(_) => {
            let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(bytes);
            decoded.into_owned()
        }
    }
}

/// Read a file from disk and decode its bytes via [`decode_ini_bytes`].
pub fn read_ini_file(path: &std::path::Path) -> std::io::Result<String> {
    let bytes = std::fs::read(path)?;
    Ok(decode_ini_bytes(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pure_ascii_round_trips() {
        assert_eq!(decode_ini_bytes(b"hello world"), "hello world");
    }

    #[test]
    fn utf8_portuguese_decoded_losslessly() {
        let s = "Configurações de Ignição — 90°C";
        assert_eq!(decode_ini_bytes(s.as_bytes()), s);
    }

    #[test]
    fn utf8_bom_is_stripped() {
        let mut bytes = Vec::from(UTF8_BOM);
        bytes.extend_from_slice("Mapa de Combustível".as_bytes());
        assert_eq!(decode_ini_bytes(&bytes), "Mapa de Combustível");
    }

    #[test]
    fn windows_1252_fallback_recovers_portuguese_accents() {
        // "Configurações" in Windows-1252:
        //   C  o  n  f  i  g  u  r  a  ç(0xE7)  õ(0xF5) e  s
        let bytes = b"Configura\xE7\xF5es";
        assert_eq!(decode_ini_bytes(bytes), "Configurações");
    }

    #[test]
    fn windows_1252_fallback_recovers_degree_symbol() {
        // "90°C" in Windows-1252: 0xB0 = ° in both Win-1252 and ISO-8859-1.
        let bytes = b"90\xB0C";
        assert_eq!(decode_ini_bytes(bytes), "90°C");
    }

    #[test]
    fn windows_1252_fallback_recovers_iso_8859_1_letters() {
        // ã (0xE3), ñ (0xF1), ü (0xFC) — common across Western European INIs.
        let bytes = b"acent\xE3o ma\xF1ana \xFCber";
        assert_eq!(decode_ini_bytes(bytes), "acentão mañana über");
    }

    #[test]
    fn never_emits_replacement_character() {
        // Any byte sequence the fallback handles must produce no U+FFFD.
        for hi in 0x80u8..=0xFFu8 {
            let bytes = [hi];
            let decoded = decode_ini_bytes(&bytes);
            assert!(
                !decoded.contains('\u{FFFD}'),
                "byte 0x{hi:02X} produced replacement char: {decoded:?}"
            );
        }
    }
}
