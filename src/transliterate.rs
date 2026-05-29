//! SEDRA transliteration and lossless NT Hebrew ↔ Syriac conversion.
//!
//! The NT text originates from the SEDRA `strVocalised` transliteration. We
//! render it into **Hebrew** for storage in the `bible` table (the single
//! source of truth) and convert to **Syriac** on demand.
//!
//! To keep the round trip lossless, every SEDRA symbol maps to a *single*
//! Hebrew code point and a *single* Syriac code point (no dropped marks, no
//! reordering, no precomposed ligatures, no final-form folding). Because each
//! mapping column is a bijection, Hebrew ↔ Syriac is a pure per-character
//! substitution that reverses exactly.
//!
//! This differs from the legacy Python rendering (which dropped rukkakha and
//! seyame and omitted some vowels in Syriac) — that was deliberately lossy and
//! could not round-trip.

/// `(sedra, hebrew, syriac)` — each column holds distinct single code points,
/// so any two columns form a bijection over the symbols that occur in the NT.
///
/// Mark choices: dagesh ↔ qushshaya, rafe ↔ rukkakha (both mark
/// spirantisation), Hebrew niqqud ↔ Syriac below-vowels, combining diaeresis
/// for seyame in both scripts.
const MAP: &[(char, char, char)] = &[
    // Consonants
    ('A', '\u{05D0}', '\u{0710}'),
    ('B', '\u{05D1}', '\u{0712}'),
    ('G', '\u{05D2}', '\u{0713}'),
    ('D', '\u{05D3}', '\u{0715}'),
    ('H', '\u{05D4}', '\u{0717}'),
    ('O', '\u{05D5}', '\u{0718}'),
    ('Z', '\u{05D6}', '\u{0719}'),
    ('K', '\u{05D7}', '\u{071A}'),
    ('Y', '\u{05D8}', '\u{071B}'),
    (';', '\u{05D9}', '\u{071D}'),
    ('C', '\u{05DB}', '\u{071F}'),
    ('L', '\u{05DC}', '\u{0720}'),
    ('M', '\u{05DE}', '\u{0721}'),
    ('N', '\u{05E0}', '\u{0722}'),
    ('S', '\u{05E1}', '\u{0723}'),
    ('E', '\u{05E2}', '\u{0725}'),
    ('I', '\u{05E4}', '\u{0726}'),
    ('/', '\u{05E6}', '\u{0728}'),
    ('X', '\u{05E7}', '\u{0729}'),
    ('R', '\u{05E8}', '\u{072A}'),
    ('W', '\u{05E9}', '\u{072B}'),
    ('T', '\u{05EA}', '\u{072C}'),
    // Marks
    ('\'', '\u{05BC}', '\u{0741}'), // dagesh ↔ qushshaya
    (',', '\u{05BF}', '\u{0742}'),  // rafe ↔ rukkakha
    ('a', '\u{05B7}', '\u{0731}'),  // pathah ↔ pthaha
    ('e', '\u{05B5}', '\u{0737}'),  // tsere ↔ rbasa
    ('i', '\u{05B4}', '\u{073B}'),  // hiriq ↔ hbasa
    ('o', '\u{05B8}', '\u{0734}'),  // qamats ↔ zqapha
    ('u', '\u{05BB}', '\u{073E}'),  // qubuts ↔ esasa
    ('*', '\u{0308}', '\u{0308}'),  // seyame (combining diaeresis, both)
    ('_', '_', '\u{0748}'),         // linea occultans (silent letter): bare in Hebrew storage, oblique line below in Syriac
    ('-', '-', '-'),
];

/// Render one SEDRA transliteration word into NT Hebrew. Unknown characters
/// pass through unchanged.
pub fn sedra_to_hebrew(word: &str) -> String {
    word.chars()
        .map(|c| MAP.iter().find(|(s, _, _)| *s == c).map_or(c, |(_, h, _)| *h))
        .collect()
}

/// Convert NT Hebrew text to Syriac. Unmapped characters (e.g. spaces between
/// words) pass through unchanged.
pub fn hebrew_to_syriac(text: &str) -> String {
    text.chars()
        .map(|c| MAP.iter().find(|(_, h, _)| *h == c).map_or(c, |(_, _, s)| *s))
        .collect()
}

/// Convert Syriac text back to NT Hebrew. Exact inverse of [`hebrew_to_syriac`].
pub fn syriac_to_hebrew(text: &str) -> String {
    text.chars()
        .map(|c| MAP.iter().find(|(_, _, s)| *s == c).map_or(c, |(_, h, _)| *h))
        .collect()
}

/// Convert Hebrew text back to its SEDRA transliteration. Exact inverse of
/// [`sedra_to_hebrew`].
pub fn hebrew_to_sedra(text: &str) -> String {
    text.chars()
        .map(|c| MAP.iter().find(|(_, h, _)| *h == c).map_or(c, |(s, _, _)| *s))
        .collect()
}

/// Render stored NT Hebrew for *display*. The stored form is a lossless
/// bijection of the SEDRA transliteration (see [`sedra_to_hebrew`]); that form
/// round-trips to Syriac exactly but reads as non-idiomatic Hebrew. This makes
/// it look like ordinary pointed Hebrew **without** touching storage or the
/// Syriac round trip:
///   - drops rafe (U+05BF) — Syriac marks rukkakha, Hebrew leaves soft BGDKT bare;
///   - drops the linea occultans marker (`_`, silent letter) — no Hebrew equivalent;
///   - folds the last consonant of each word to its final form.
///
/// Not lossless and never fed back into Hebrew↔Syriac conversion.
pub fn hebrew_display(text: &str) -> String {
    let stripped: String = text.chars().filter(|&c| c != '\u{05BF}' && c != '_').collect();
    stripped
        .split_inclusive(' ')
        .map(final_form_word)
        .collect()
}

/// Fold the last Hebrew consonant of a single token (which may carry a trailing
/// space and combining marks) to its final form.
fn final_form_word(token: &str) -> String {
    let mut chars: Vec<char> = token.chars().collect();
    if let Some(i) = chars.iter().rposition(|&c| is_hebrew_consonant(c)) {
        chars[i] = final_form(chars[i]);
    }
    chars.into_iter().collect()
}

fn is_hebrew_consonant(c: char) -> bool {
    ('\u{05D0}'..='\u{05EA}').contains(&c)
}

/// Canonicalise an NT Hebrew word for lexicon matching against `strVocalised`.
/// A word tapped in the app may be in display form ([`hebrew_display`]: rafe
/// dropped, final letters folded) or in the raw stored form. Folding finals
/// back to medial and dropping rafe maps both to the same key; the stored side
/// is matched after `replace`-ing rafe out (it has no final forms). Final-form
/// folding is 1:1, so this reversal is exact.
pub fn lookup_key(word: &str) -> String {
    word.chars()
        .filter(|&c| c != '\u{05BF}' && c != '_')
        .map(medial_form)
        .collect()
}

fn medial_form(c: char) -> char {
    match c {
        '\u{05DA}' => '\u{05DB}', // final kaf → kaf
        '\u{05DD}' => '\u{05DE}', // final mem → mem
        '\u{05DF}' => '\u{05E0}', // final nun → nun
        '\u{05E3}' => '\u{05E4}', // final pe → pe
        '\u{05E5}' => '\u{05E6}', // final tsadi → tsadi
        other => other,
    }
}

fn final_form(c: char) -> char {
    match c {
        '\u{05DB}' => '\u{05DA}', // kaf → final kaf
        '\u{05DE}' => '\u{05DD}', // mem → final mem
        '\u{05E0}' => '\u{05DF}', // nun → final nun
        '\u{05E4}' => '\u{05E3}', // pe → final pe
        '\u{05E6}' => '\u{05E5}', // tsadi → final tsadi
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_are_bijective() {
        // No two rows share a Hebrew or a Syriac code point.
        for (i, a) in MAP.iter().enumerate() {
            for b in &MAP[i + 1..] {
                assert_ne!(a.1, b.1, "duplicate Hebrew code point {:?}", a.1);
                assert_ne!(a.2, b.2, "duplicate Syriac code point {:?}", a.2);
            }
        }
    }

    #[test]
    fn matthew_1_1_first_word_round_trips() {
        // SEDRA "C'ToBoA" → NT Hebrew → Syriac → back.
        let heb = sedra_to_hebrew("C'ToBoA");
        let syr = hebrew_to_syriac(&heb);
        assert_eq!(syriac_to_hebrew(&syr), heb);
        // Hebrew carries dagesh; Syriac carries qushshaya.
        assert!(heb.contains('\u{05BC}'));
        assert!(syr.contains('\u{0741}'));
    }

    #[test]
    fn sedra_hebrew_round_trips() {
        // Every SEDRA symbol that occurs in the SEDRA source tables must
        // survive sedra → Hebrew → sedra unchanged (lossless storage).
        for word in ["AoAaR", "B'oAAaR", "C'ToBoA", "B,A*", "BR-ABA", "A;XNON"] {
            assert_eq!(hebrew_to_sedra(&sedra_to_hebrew(word)), word);
        }
    }

    #[test]
    fn display_drops_rafe_and_folds_finals() {
        // SEDRA "AaB,oHoT,C,uON" → stored Hebrew has rafe and medial finals.
        let stored = sedra_to_hebrew("AaB,oHoT,C,uON");
        assert!(stored.contains('\u{05BF}'));
        let shown = hebrew_display(&stored);
        assert!(!shown.contains('\u{05BF}'), "rafe must be gone");
        assert!(shown.ends_with('\u{05DF}'), "trailing nun → final nun");
        // Matching key reconstructs the rafe-less stored form, so lexicon
        // lookup of the displayed word still resolves.
        let stored_key: String = stored.chars().filter(|&c| c != '\u{05BF}').collect();
        assert_eq!(lookup_key(&shown), stored_key);
    }

    #[test]
    fn linea_occultans_drops_in_hebrew_renders_in_syriac() {
        // SEDRA "OLaAKaOH_;" (Matt 1:2 "and his brothers") — the `_` marks a
        // silent he. Hebrew drops it; Syriac shows the oblique line below.
        let stored = sedra_to_hebrew("OLaAKaOH_;");
        assert!(stored.contains('_'));
        assert!(!hebrew_display(&stored).contains('_'), "underscore must be gone");
        let syr = hebrew_to_syriac(&stored);
        assert!(syr.contains('\u{0748}') && !syr.contains('_'));
        assert_eq!(syriac_to_hebrew(&syr), stored); // still round-trips
        // Displayed word still resolves: its key equals the rafe/underscore-
        // stripped stored form used by the lexicon query.
        let key: String = stored.chars().filter(|&c| c != '\u{05BF}' && c != '_').collect();
        assert_eq!(lookup_key(&hebrew_display(&stored)), key);
    }

    #[test]
    fn lookup_key_matches_raw_and_display() {
        let stored = sedra_to_hebrew("C'T,oB,oA");
        let shown = hebrew_display(&stored);
        let expected: String = stored.chars().filter(|&c| c != '\u{05BF}').collect();
        assert_eq!(lookup_key(&shown), expected);
        assert_eq!(lookup_key(&stored), expected);
    }

    #[test]
    fn rukkakha_and_seyame_survive() {
        // SEDRA word using rukkakha (,) and seyame (*) — dropped by the old
        // lossy rendering, preserved here.
        let heb = sedra_to_hebrew("B,A*");
        assert!(heb.contains('\u{05BF}')); // rafe (rukkakha)
        assert!(heb.contains('\u{0308}')); // seyame
        let syr = hebrew_to_syriac(&heb);
        assert!(syr.contains('\u{0742}')); // rukkakha
        assert_eq!(syriac_to_hebrew(&syr), heb);
    }
}
