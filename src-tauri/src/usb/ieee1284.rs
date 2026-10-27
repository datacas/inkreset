//! IEEE 1284 device ID parsing.
//!
//! The printer class returns a semicolon-separated key/value string such as:
//! `MFG:Epson;MDL:ET-8550 Series;CMD:...;SN:X4YF...;`
//! Keys are case-insensitive and MDL can repeat; the first value wins.

pub fn parse_id(raw: &str) -> (Option<String>, Option<String>, Option<String>) {
    let mut manufacturer = None;
    let mut model = None;
    let mut serial = None;

    for part in raw.split(';') {
        let Some((key, value)) = part.split_once(':') else {
            continue;
        };
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        match key.trim().to_ascii_uppercase().as_str() {
            "MFG" => {
                manufacturer.get_or_insert_with(|| value.to_string());
            }
            "MDL" => {
                model.get_or_insert_with(|| value.to_string());
            }
            "SN" | "S/N" | "SERIAL" | "SERIALNUMBER" => {
                serial.get_or_insert_with(|| value.to_string());
            }
            _ => {}
        }
    }

    (manufacturer, model, serial)
}

#[cfg(test)]
mod tests {
    use super::parse_id;

    #[test]
    fn parses_epson_id() {
        let (mfg, mdl, sn) = parse_id(
            "MFG:Epson;MDL:ET-8550 Series;CMD:ESCPL2,BDC,D4,D4PX,ESCPR7,END4;SN:X4YF09217C;",
        );
        assert_eq!(mfg.as_deref(), Some("Epson"));
        assert_eq!(mdl.as_deref(), Some("ET-8550 Series"));
        assert_eq!(sn.as_deref(), Some("X4YF09217C"));
    }

    #[test]
    fn first_model_wins_and_keys_are_case_insensitive() {
        let (_, mdl, _) = parse_id("mfg:Brother;MDL:DCP-L2550DN;mdl:Ignored;");
        assert_eq!(mdl.as_deref(), Some("DCP-L2550DN"));
    }

    #[test]
    fn tolerates_garbage() {
        let (mfg, mdl, sn) = parse_id("noise;MDL:;SN:ABC;==;");
        assert!(mfg.is_none());
        assert!(mdl.is_none());
        assert_eq!(sn.as_deref(), Some("ABC"));
    }
}

/// Decode the buffer returned by IOCTL_USBPRINT_GET_1284_ID.
///
/// usbprint returns ASCII bytes, but WOW64 stacks and some drivers hand back
/// UTF-16LE. Sniff the shape instead of truncating at the first zero byte.
pub fn decode_id(bytes: &[u8]) -> String {
    if bytes.len() >= 4 {
        // Decisive shape test: an ASCII ID has a printable second byte,
        // UTF-16LE ASCII has zeroes in the high byte of its first characters.
        let looks_utf16 = bytes[0] != 0
            && bytes[1] == 0
            && bytes[3] == 0
            && (bytes.len() < 6 || bytes[5] == 0);
        if looks_utf16 {
            let utf16: Vec<u16> = bytes
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .take_while(|&c| c != 0)
                .collect();
            return String::from_utf16_lossy(&utf16).trim().to_string();
        }
    }

    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).trim().to_string()
}

#[cfg(test)]
mod decode_tests {
    use super::decode_id;

    #[test]
    fn decodes_ascii_with_nul_padding() {
        let mut buf = b"MFG:Epson;MDL:XP-530;".to_vec();
        buf.resize(64, 0);
        assert_eq!(decode_id(&buf), "MFG:Epson;MDL:XP-530;");
    }

    #[test]
    fn decodes_utf16le() {
        let text = "MFG:Epson;MDL:ET-8550;";
        let mut buf: Vec<u8> = text.encode_utf16().flat_map(|c| c.to_le_bytes()).collect();
        buf.extend_from_slice(&[0, 0, 0, 0]);
        assert_eq!(decode_id(&buf), text);
    }

    #[test]
    fn handles_empty_and_no_nul() {
        assert_eq!(decode_id(&[]), "");
        assert_eq!(decode_id(b"MDL:L3250"), "MDL:L3250");
    }
}
