//! Bounded text-only entity repair for documented legacy source encodings.

/// Decode a single layer of numeric character references, leaving named, invalid
/// and oversized references unchanged. This is not an HTML/XML parser and never
/// interprets the resulting characters as markup. Call only for sources whose
/// encoding requires it; literal entity-looking text in other works is preserved.
pub fn decode_numeric_text_entities(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("&#") {
        output.push_str(&rest[..start]);
        rest = &rest[start..];
        let token = rest.as_bytes().iter().take(12).position(|b| *b == b';');
        let decoded = token.and_then(|end| {
            let number = &rest[2..end];
            let value = if let Some(hex) = number
                .strip_prefix('x')
                .or_else(|| number.strip_prefix('X'))
            {
                if hex.is_empty() || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return None;
                }
                u32::from_str_radix(hex, 16).ok()?
            } else {
                if number.is_empty() || !number.bytes().all(|b| b.is_ascii_digit()) {
                    return None;
                }
                number.parse().ok()?
            };
            let character = char::from_u32(value)?;
            (!character.is_control() || matches!(character, '\n' | '\t' | '\r'))
                .then_some((end, character))
        });
        if let Some((end, character)) = decoded {
            output.push(character);
            rest = &rest[end + 1..];
        } else {
            output.push_str("&#");
            rest = &rest[2..];
        }
    }
    output.push_str(rest);
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn numeric_source_repair_is_one_layer_and_does_not_parse_markup() {
        assert_eq!(
            decode_numeric_text_entities("Roma &#224; · tant&#226; · &#x3b1; &#X3B2;"),
            "Roma à · tantâ · α β"
        );
        assert_eq!(
            decode_numeric_text_entities("&#60;b&#62; &amp; &#38;#224;"),
            "<b> &amp; &#224;"
        );
        let invalid = "&#0; &#xD800; &#1114112; &#-1; &#x; &#999999999999999; &#224";
        assert_eq!(decode_numeric_text_entities(invalid), invalid);
    }
}
