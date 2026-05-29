//! SEDRA transliteration → pointed Hebrew letters.
//!
//! Ported from `bm_tools.sedra.db.from_transliteration` (alphabet = "hebrew").

/// Substitutions applied to the raw transliteration string before per-character
/// mapping. Order matters and mirrors the Python `HEBREW_REPLACEMENTS` dict.
const REPLACEMENTS: &[(&str, &str)] = &[
    (",", ""),
    ("_", ""),
    ("-", ""),
    ("*", ""),
    ("uO", "\u{FB35}"), // vav with dagesh (presentation form)
    ("iA", "Ai"),
    ("D'", "\u{FB33}"), // dalet with dagesh (presentation form)
    ("aD", "Da"),
];

/// Per-character map from SEDRA transliteration to Hebrew.
fn map_char(c: char) -> Option<&'static str> {
    Some(match c {
        'A' => "א",
        'B' => "ב",
        'G' => "ג",
        'D' => "ד",
        'H' => "ה",
        'O' => "ו",
        'Z' => "ז",
        'K' => "ח",
        'Y' => "ט",
        ';' => "י",
        'C' => "כ",
        'L' => "ל",
        'M' => "מ",
        'N' => "נ",
        'S' => "ס",
        'E' => "ע",
        'I' => "פ",
        '/' => "צ",
        'X' => "ק",
        'R' => "ר",
        'W' => "\u{FB2A}", // shin with shin dot (presentation form)
        'T' => "ת",
        '\'' => "ּ",
        'a' => "ַ",
        'e' => "ֵ",
        'i' => "ִ",
        'o' => "ָ",
        'u' => "ֻ",
        _ => return None,
    })
}

/// Final-form replacement applied to the last character of the result.
fn final_form(c: char) -> Option<char> {
    Some(match c {
        'צ' => 'ץ',
        'נ' => 'ן',
        'כ' => 'ך',
        'פ' => 'ף',
        'מ' => 'ם',
        _ => return None,
    })
}

/// Convert a SEDRA transliteration string into pointed Hebrew.
pub fn to_hebrew(word: &str) -> String {
    let mut s = word.to_owned();
    for (sub, rep) in REPLACEMENTS {
        s = s.replace(sub, rep);
    }

    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match map_char(c) {
            Some(mapped) => out.push_str(mapped),
            None => out.push(c),
        }
    }

    if let Some(last) = out.chars().next_back()
        && let Some(replacement) = final_form(last)
    {
        out.pop();
        out.push(replacement);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::to_hebrew;

    #[test]
    fn presentation_form_ligatures() {
        // The SEDRA→Hebrew map emits precomposed presentation forms, not
        // decomposed letter+mark sequences, for these cases.
        assert_eq!(to_hebrew("D'"), "\u{FB33}"); // dalet with dagesh
        assert_eq!(to_hebrew("uO"), "\u{FB35}"); // vav with dagesh
        assert_eq!(to_hebrew("W"), "\u{FB2A}"); // shin with shin dot
    }

    #[test]
    fn first_word_of_matthew() {
        // BFBS word "C'ToBoA" → כּתָבָא (kaf+dagesh stays decomposed).
        assert_eq!(to_hebrew("C'ToBoA"), "\u{05DB}\u{05BC}\u{05EA}\u{05B8}\u{05D1}\u{05B8}\u{05D0}");
    }

    #[test]
    fn final_form_applied_to_last_letter() {
        // Trailing mem becomes final mem.
        assert_eq!(to_hebrew("AABRoHoM"), "אאברָהָ\u{05DD}");
    }
}
