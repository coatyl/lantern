//! Text helpers shared by the title and folder-name treatments.

/// Trim and collapse every run of whitespace to a single ASCII space.
pub(super) fn normalize_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Decode the named and numeric HTML entities that browsers write into
/// bookmark exports.  Anything that is not a well-formed, known entity is
/// left exactly as it was.
pub(super) fn decode_html_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let tail = &rest[amp + 1..];
        let name_len = tail
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '#'))
            .unwrap_or(tail.len());
        let decoded = tail[name_len..]
            .starts_with(';')
            .then(|| decode_entity(&tail[..name_len]))
            .flatten();
        match decoded {
            Some(ch) => {
                out.push(ch);
                rest = &tail[name_len + 1..];
            }
            None => {
                // Not an entity: keep the `&` and rescan what follows it.
                out.push('&');
                rest = tail;
            }
        }
    }
    out.push_str(rest);
    out
}

/// The character for entity `name` (the text between `&` and `;`).
fn decode_entity(name: &str) -> Option<char> {
    let ch = match name {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "nbsp" => ' ',
        "copy" => '©',
        "reg" => '®',
        "trade" => '™',
        "mdash" => '—',
        "ndash" => '–',
        "lsquo" => '\u{2018}',
        "rsquo" => '\u{2019}',
        "ldquo" => '\u{201C}',
        "rdquo" => '\u{201D}',
        _ => {
            let number = name.strip_prefix('#')?;
            let code = match number.strip_prefix(|c| c == 'x' || c == 'X') {
                Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                None => number.parse().ok()?,
            };
            return char::from_u32(code);
        }
    };
    Some(ch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_known_entities_and_keeps_everything_else() {
        let cases = [
            ("Tom &amp; Jerry", "Tom & Jerry"),
            ("A &lt; B &gt; C", "A < B > C"),
            (
                "Say &quot;hi&quot; &amp; it&apos;s done",
                "Say \"hi\" & it's done",
            ),
            ("Rust &mdash; fast &ndash; safe", "Rust — fast – safe"),
            ("&#169; 2024", "© 2024"),
            ("A &#x26; B &#X26; C", "A & B & C"),
            (
                "&unknown; &AMP; &#xZZ; &#; &#99999999;",
                "&unknown; &AMP; &#xZZ; &#; &#99999999;",
            ),
            ("Tom & Jerry &amp", "Tom & Jerry &amp"),
            // A stray `&` must not swallow the entity right after it.
            ("&&amp;", "&&"),
            ("R&D&amp;Co", "R&D&Co"),
            ("no entities", "no entities"),
        ];
        for (input, expected) in cases {
            assert_eq!(decode_html_entities(input), expected, "input: {input:?}");
        }
    }

    #[test]
    fn whitespace_runs_collapse_and_edges_trim() {
        assert_eq!(normalize_whitespace("  Hello \t\n  World  "), "Hello World");
        assert_eq!(normalize_whitespace("clean"), "clean");
        assert_eq!(normalize_whitespace("   "), "");
    }
}
