//! Learner-facing romanization of pointed Hebrew — the tutor's "how it
//! sounds", ported from the app so cards arrive with their voicing attached
//! and the app stays presentation-only.
//!
//! This is deliberately *not* a scholarly transliteration. It makes pragmatic
//! choices so the output reads naturally for a learner: modern consonant
//! values (ח → "ch", צ → "ts"), aleph and ayin marked with an apostrophe,
//! vocal sheva voiced as "e" (silent sheva — word-final, or closing a
//! syllable after a short vowel — is dropped: לָךְ → "lakh", מִדְבָּר →
//! "midbar"), matres lectionis (a vowel-letter vav or yod) folded into the
//! vowel they lengthen, and furtive patah read before its guttural
//! (רוּחַ → "ruach"). Cantillation is ignored; spaces are kept and a maqaf
//! becomes a hyphen.
//!
//! Not to be confused with [`crate::transliterate`], the lossless SEDRA↔NT
//! Hebrew↔Syriac mapping used for storage.

/// Niqqud / dagesh / shin-sin dots we consume; cantillation (U+0591–U+05AF)
/// and meteg (U+05BD) are intentionally excluded so they're skipped as noise.
fn is_mark(c: char) -> bool {
    ('\u{05B0}'..='\u{05BC}').contains(&c) || c == '\u{05C1}' || c == '\u{05C2}' || c == '\u{05C7}'
}

fn is_base(c: char) -> bool {
    ('\u{05D0}'..='\u{05EA}').contains(&c)
}

/// A vowel point's collapsed word-level value (one of a/e/i/o/u), or "" for a
/// non-vowel mark.
fn vowel(c: char) -> &'static str {
    match c {
        '\u{05B0}' // sheva (voiced approximation)
        | '\u{05B1}' // hataf segol
        | '\u{05B5}' // tsere
        | '\u{05B6}' => "e", // segol
        '\u{05B2}' // hataf patah
        | '\u{05B7}' // patah
        | '\u{05B8}' => "a", // qamats
        '\u{05B4}' => "i", // hiriq
        '\u{05B3}' // hataf qamats
        | '\u{05B9}' // holam
        | '\u{05C7}' => "o", // qamats qatan
        '\u{05BB}' => "u", // qubuts
        _ => "",
    }
}

/// A vowel point's *distinguishing* respelling for a syllable quiz — a
/// friendly digraph for a long vowel (qamats "ah" vs patah "a", tsere "ey" vs
/// segol "e", holam "oh"), a superscript for the reduced vowels (sheva/hataf).
/// [`romanize`] collapses all of these to one of a/e/i/o/u, so syllable
/// voicing must keep them apart for the quiz options to differ.
pub fn vowel_vocalisation(c: char) -> &'static str {
    match c {
        '\u{05B7}' => "a",  // patah
        '\u{05B8}' => "ah", // qamats
        '\u{05B6}' => "e",  // segol
        '\u{05B5}' => "ey", // tsere
        '\u{05B4}' => "i",  // hiriq
        '\u{05B9}' => "oh", // holam
        '\u{05BB}' => "u",  // qubuts
        '\u{05B0}' => "ᵉ",  // sheva (vocal, superscript)
        '\u{05B1}' => "ᵉ",  // hataf segol
        '\u{05B2}' => "ᵃ",  // hataf patah
        '\u{05B3}' => "ᵒ",  // hataf qamats
        '\u{05C7}' => "o",  // qamats qatan
        _ => "",
    }
}

fn consonant(c: char, dagesh: bool, sin_dot: bool) -> &'static str {
    match c {
        '\u{05D0}' | '\u{05E2}' => "'", // alef / ayin — glottal stop
        '\u{05D1}' => {
            if dagesh {
                "b"
            } else {
                "v"
            }
        }
        '\u{05D2}' => "g",
        '\u{05D3}' => "d",
        '\u{05D4}' => "h",
        '\u{05D5}' => "v", // vav (consonantal; vowel use handled by caller)
        '\u{05D6}' => "z",
        '\u{05D7}' => "ch",
        '\u{05D8}' => "t",
        '\u{05D9}' => "y",
        // Final and medial kaf soften identically; the dagesh of a 2ms
        // suffix ־ךָּ keeps the hard value (מִמֶּךָּ → "mimeka").
        '\u{05DA}' | '\u{05DB}' => {
            if dagesh {
                "k"
            } else {
                "kh"
            }
        }
        '\u{05DC}' => "l",
        '\u{05DD}' | '\u{05DE}' => "m",
        '\u{05DF}' | '\u{05E0}' => "n",
        '\u{05E1}' => "s",
        '\u{05E3}' | '\u{05E4}' => {
            if dagesh {
                "p"
            } else {
                "f"
            }
        }
        '\u{05E5}' | '\u{05E6}' => "ts",
        '\u{05E7}' => "k",
        '\u{05E8}' => "r",
        '\u{05E9}' => {
            if sin_dot {
                "s"
            } else {
                "sh"
            }
        }
        '\u{05EA}' => "t",
        _ => "",
    }
}

/// Group the code points into clusters: each base consonant carries its
/// trailing marks; spaces and maqaf become their own one-element separators.
fn clusters(text: &str) -> Vec<Vec<char>> {
    let mut out: Vec<Vec<char>> = Vec::new();
    for c in text.chars() {
        if is_base(c) {
            out.push(vec![c]);
        } else if is_mark(c) {
            if let Some(last) = out.last_mut()
                && is_base(last[0])
            {
                last.push(c);
            }
        } else if c == ' ' {
            out.push(vec![' ']);
        } else if c == '\u{05BE}' {
            out.push(vec!['\u{05BE}']); // maqaf
        }
        // everything else (cantillation, sof pasuq, paseq, …) is dropped
    }
    out
}

/// True if no base-consonant cluster follows index `k` before a separator/end —
/// i.e. `k` is the last consonant of its word.
fn word_final(clusters: &[Vec<char>], k: usize) -> bool {
    match clusters.get(k + 1) {
        // A word's consonants are contiguous, so the next cluster decides it:
        // another base consonant means more letters follow; a separator means
        // this is the end.
        Some(next) => !is_base(next[0]),
        None => true,
    }
}

/// The onset sound of a single consonant glyph, for voicing a teaching
/// syllable (e.g. building `הֶ` → "he"). Unlike [`romanize`] this bypasses
/// the word-level heuristics (mater folding, furtive patah, vav-as-vowel),
/// which need neighbouring clusters and could otherwise swallow a lone host
/// consonant. A dagesh or sin dot carried in the glyph is honoured (בּ →
/// "b", שׂ → "s"); a bare begadkefat letter takes its soft value (ב → "v").
pub fn consonant_onset(glyph: &str) -> &'static str {
    let mut chars = glyph.chars();
    let Some(c) = chars.next() else { return "" };
    if !is_base(c) {
        return "";
    }
    let dagesh = glyph.chars().any(|m| m == '\u{05BC}');
    let sin_dot = glyph.chars().any(|m| m == '\u{05C2}');
    consonant(c, dagesh, sin_dot)
}

/// The voiced reading of a teaching syllable — `host`'s onset plus `vowel`'s
/// distinguishing respelling (בּ + qamats → "bah") — for a vowel quiz option.
pub fn voiced_syllable(host: &str, vowel: char) -> String {
    format!("{}{}", consonant_onset(host), vowel_vocalisation(vowel))
}

/// The voiced reading of a one-syllable string: a base consonant with its dot
/// marks (dagesh, sin/shin dot — e.g. בּ or שׂ) and a single vowel point, in
/// any combining order. The onset scan ignores the vowel and the vowel scan
/// ignores the dots, so no positional split is needed.
pub fn voiced_syllable_str(syllable: &str) -> String {
    match syllable
        .chars()
        .find(|&c| !vowel_vocalisation(c).is_empty())
    {
        Some(v) => voiced_syllable(syllable, v),
        None => consonant_onset(syllable).to_string(),
    }
}

/// Romanize pointed Hebrew text word-by-word (see the module docs for the
/// conventions). Cantillation is skipped; a maqaf becomes a hyphen.
/// Whether a vowel point is short — the kind that closes its syllable on the
/// next vowel-less consonant, making that consonant's sheva silent (the hiriq
/// of מִדְבָּר silences the dalet's sheva: "midbar", not "midebar"). Qamats
/// is treated as long: the corpus never marks qamats qatan, and a vocal "e"
/// is the safer guess under a long vowel.
fn is_short_vowel(m: char) -> bool {
    matches!(m, '\u{05B4}' | '\u{05B6}' | '\u{05B7}' | '\u{05BB}')
}

pub fn romanize(text: &str) -> String {
    let clusters = clusters(text);
    let mut out = String::new();
    let mut last_vowel = "";
    // Whether the previous cluster ended in a short vowel (no mater) — the
    // condition under which a following sheva closes the syllable silently.
    let mut last_short = false;

    for (k, cl) in clusters.iter().enumerate() {
        let base = cl[0];

        if base == ' ' {
            out.push(' ');
            last_vowel = "";
            last_short = false;
            continue;
        }
        if base == '\u{05BE}' {
            out.push('-');
            last_vowel = "";
            last_short = false;
            continue;
        }

        let marks = &cl[1..];
        let dagesh = marks.contains(&'\u{05BC}');
        let sin_dot = marks.contains(&'\u{05C2}');
        let is_last = word_final(&clusters, k);
        // A sheva is silent word-finally (לָךְ → "lakh") or when it closes a
        // syllable after a short vowel (מִדְבָּר → "midbar") — unless its
        // consonant carries dagesh forte, whose second half opens a new
        // syllable (הַמְּלָכִים keeps its "e"). Word-initially and after a
        // long vowel or another sheva it stays vocal.
        let silent_sheva = marks.contains(&'\u{05B0}') && (is_last || (last_short && !dagesh));
        let vowels: Vec<&str> = marks
            .iter()
            .filter(|&&m| !(silent_sheva && m == '\u{05B0}'))
            .map(|&m| vowel(m))
            .filter(|v| !v.is_empty())
            .collect();
        let vstr = vowels.concat();

        // Vav serving as a vowel letter: holam male (וֹ → o) or shuruq (וּ → u).
        if base == '\u{05D5}' {
            if marks.contains(&'\u{05B9}') {
                out.push('o');
                last_vowel = "o";
                last_short = false;
                continue;
            }
            if dagesh && vowels.is_empty() {
                out.push('u');
                last_vowel = "u";
                last_short = false;
                continue;
            }
            // A consonantal vav with no point at all mid-word is a
            // defectively written holam (עָון → "avon", מִצְות → "mitsvot") —
            // fully pointed text gives every consonant a vowel or sheva.
            if marks.is_empty() && !is_last {
                out.push_str("vo");
                last_vowel = "o";
                last_short = false;
                continue;
            }
        }

        if base == '\u{05D9}' && vowels.is_empty() && !dagesh {
            // Yod as a mater lengthening a preceding hiriq (…ִי): fold into
            // the "i" — and the vowel is long now, so a following sheva
            // stays vocal.
            if last_vowel == "i" {
                last_short = false;
                continue;
            }
            // The silent yod of the 3ms plural suffix ־ָיו (עָלָיו →
            // "'alav"): qamats, then yod, then a bare word-final vav.
            if last_vowel == "a"
                && clusters
                    .get(k + 1)
                    .is_some_and(|n| n.len() == 1 && n[0] == '\u{05D5}')
                && word_final(&clusters, k + 1)
            {
                continue;
            }
        }

        // Furtive patah under a final guttural is sounded *before* it
        // (רוּחַ → ruach).
        if is_last
            && marks.contains(&'\u{05B7}')
            && (base == '\u{05D7}' || base == '\u{05E2}' || base == '\u{05D4}')
        {
            out.push('a');
            out.push_str(consonant(base, dagesh, sin_dot));
            last_vowel = "";
            last_short = false;
            continue;
        }

        out.push_str(consonant(base, dagesh, sin_dot));
        out.push_str(&vstr);
        last_vowel = vowels.last().copied().unwrap_or("");
        last_short = marks
            .iter()
            .rev()
            .copied()
            .find(|&m| !vowel(m).is_empty() || m == '\u{05B0}')
            .is_some_and(is_short_vowel);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn common_words_read_naturally() {
        assert_eq!(romanize("שָׁלוֹם"), "shalom");
        assert_eq!(romanize("בַּיִת"), "bayit");
        assert_eq!(romanize("תּוֹרָה"), "torah"); // final he voiced
        assert_eq!(romanize("בְּרֵאשִׁית"), "bere'shit"); // aleph → '
    }

    #[test]
    fn dagesh_hardens_begadkefat() {
        assert_eq!(romanize("בּ"), "b");
        assert_eq!(romanize("ב"), "v");
        assert_eq!(romanize("כּ"), "k");
        assert_eq!(romanize("כ"), "kh");
    }

    #[test]
    fn sin_vs_shin_dot() {
        assert_eq!(romanize("שׁ"), "sh");
        assert_eq!(romanize("שׂ"), "s");
    }

    #[test]
    fn furtive_patah_is_read_before_the_guttural() {
        assert_eq!(romanize("רוּחַ"), "ruach");
    }

    #[test]
    fn maqaf_becomes_a_hyphen() {
        assert_eq!(romanize("עַל־כֵּן"), "'al-ken"); // ayin → '
    }

    #[test]
    fn silent_sheva_is_dropped() {
        // Word-final sheva (the 2fs suffix hosts of the pronoun drill).
        assert_eq!(romanize("לָךְ"), "lakh");
        assert_eq!(romanize("עִמָּךְ"), "'imakh");
        // A sheva closing a syllable after a short vowel is silent…
        assert_eq!(romanize("מִדְבָּר"), "midbar");
        // …but stays vocal word-initially, and under dagesh forte.
        assert_eq!(romanize("בְּרֵאשִׁית"), "bere'shit");
        assert_eq!(romanize("הַמְּלָכִים"), "hamelakhim");
    }

    #[test]
    fn silent_yod_of_the_3ms_plural_suffix() {
        assert_eq!(romanize("עָלָיו"), "'alav");
        assert_eq!(romanize("אֵלָיו"), "'elav");
        assert_eq!(romanize("דְּבָרָיו"), "devarav");
        // A word-final consonantal yod still sounds (גּוֹי "goy").
        assert_eq!(romanize("גּוֹי"), "goy");
    }

    #[test]
    fn defective_holam_on_consonantal_vav() {
        // The corpus writes these with a bare, vowel-less vav; the missing
        // holam must still be read.
        assert_eq!(romanize("עָון"), "'avon");
        assert_eq!(romanize("מִצְות"), "mitsvot");
        assert_eq!(romanize("מִצְותָיו"), "mitsvotav");
    }

    #[test]
    fn final_kaf_and_pe_honour_dagesh() {
        assert_eq!(romanize("מִמֶּךָּ"), "mimeka"); // ־ךָּ hard k
        assert_eq!(romanize("מִמְּךָ"), "mimekha"); // ־ךָ soft kh
    }

    #[test]
    fn spaces_between_words_are_kept() {
        assert_eq!(romanize("בַּיִת שָׁלוֹם"), "bayit shalom");
    }

    #[test]
    fn cantillation_is_ignored() {
        // Genesis 1:1 opener with an accent mark on the first word.
        assert_eq!(romanize("בְּרֵאשִׁ֖ית"), "bere'shit");
    }

    #[test]
    fn onset_voices_a_lone_consonant() {
        // Word-level heuristics must NOT apply to a syllable host.
        assert_eq!(consonant_onset("ה"), "h");
        assert_eq!(consonant_onset("מ"), "m");
        assert_eq!(consonant_onset("ח"), "ch");
        assert_eq!(consonant_onset("ב"), "v"); // bare host: soft begadkefat
        assert_eq!(consonant_onset("א"), "'"); // aleph
        assert_eq!(consonant_onset("ע"), "'"); // ayin
    }

    #[test]
    fn onset_honours_dot_marks_carried_by_the_glyph() {
        assert_eq!(consonant_onset("בּ"), "b"); // dagesh hardens begadkefat
        assert_eq!(consonant_onset("כּ"), "k");
        assert_eq!(consonant_onset("פּ"), "p");
        assert_eq!(consonant_onset("שׁ"), "sh"); // shin dot
        assert_eq!(consonant_onset("שׂ"), "s"); // sin dot
    }

    #[test]
    fn non_consonants_have_no_onset() {
        assert_eq!(consonant_onset(""), "");
        assert_eq!(consonant_onset("ֶ"), ""); // a bare vowel point
    }

    #[test]
    fn voiced_syllables_keep_vowel_lengths_apart() {
        assert_eq!(voiced_syllable("בּ", '\u{05B8}'), "bah"); // qamats
        assert_eq!(voiced_syllable("בּ", '\u{05B7}'), "ba"); // patah
        assert_eq!(voiced_syllable("ב", '\u{05B8}'), "vah"); // soft bet
        assert_eq!(voiced_syllable("שׂ", '\u{05B9}'), "soh"); // sin + holam
        assert_eq!(voiced_syllable("שׁ", '\u{05B0}'), "shᵉ"); // shin + sheva
        assert_eq!(voiced_syllable("ח", '\u{05B2}'), "chᵃ"); // hataf patah
    }

    #[test]
    fn voiced_syllable_str_splits_marks_from_the_vowel() {
        assert_eq!(voiced_syllable_str("בְּ"), "bᵉ"); // bet+dagesh+sheva
        assert_eq!(voiced_syllable_str("שֹׂ"), "soh"); // shin+sin-dot+holam
        assert_eq!(voiced_syllable_str("רֶ"), "re");
        assert_eq!(voiced_syllable_str("ב"), "v"); // bare consonant
        assert_eq!(voiced_syllable_str(""), "");
    }
}
