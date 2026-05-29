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
    ('_', '_', '_'),
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
