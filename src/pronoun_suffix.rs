//! Pronominal endings on function words, for the tutor's suffix drill.
//!
//! Hebrew joins its pronouns onto the end of a preposition (or the object
//! marker) as a suffix: אֵלַי "to me", עָלָיו "on him". The tutor teaches the
//! concept once (the `prep-suffix` grammar card) and then drills each ending
//! as its own SRS card, shown highlighted on a rotating host word the learner
//! already knows — the same explain-once-then-drill pattern vowels use.
//!
//! [`PRONOUN_SUFFIXES`] is the ending inventory, keyed by person-gender-number
//! (the `suffix_srs` key). [`split_pronoun_suffix`] finds the longest ending a
//! surface carries and splits it into a stem + suffix span, which the app
//! renders with the suffix in red. Matching is on raw codepoints (dagesh
//! variants are listed explicitly), so the split is exact for the curated
//! suffixed function words that host the drill — eligibility itself is
//! [`crate::grammar::pronoun_suffix_host`], not this match: plenty of
//! ordinary words end in the same letters (אֲדֹנָי, מִי) without carrying a
//! pronoun.

/// One drillable pronominal ending: a stable person-gender-number `key`
/// (recorded in `progress.suffix_srs`), the pronoun it stands for, and every
/// pointed spelling it takes on the curated host words (pausal and
/// dagesh-bearing variants listed separately — matching is codepoint-exact).
#[derive(Debug, Clone, Copy)]
pub struct PronounSuffix {
    pub key: &'static str,
    /// The learner-facing meaning, phrased like the curated glosses ("me",
    /// "you (m.)") — also the quiz answer, so meanings are distinct per key.
    pub meaning: &'static str,
    pub endings: &'static [&'static str],
}

/// The ending inventory, in teaching order (most frequent first — the order
/// endings are introduced as drills once a host word has graduated).
#[rustfmt::skip]
pub const PRONOUN_SUFFIXES: &[PronounSuffix] = &[
    PronounSuffix { key: "3ms", meaning: "him", endings: &[
        "\u{05B8}\u{05D9}\u{05D5}",           // ־ָיו  (עָלָיו, אֵלָיו)
        "\u{05B4}\u{05D9}\u{05D5}",           // ־ִיו  (אָבִיו, פִּיו)
        "\u{05D4}\u{05D5}\u{05BC}",           // ־הוּ  (כָּמֹהוּ)
        "\u{05D5}\u{05B9}",                   // ־וֹ   (לוֹ, בּוֹ, אֹתוֹ)
    ]},
    PronounSuffix { key: "1cs", meaning: "me", endings: &[
        "\u{05E0}\u{05B4}\u{05BC}\u{05D9}",   // ־נִּי (מִמֶּנִּי)
        "\u{05B7}\u{05D9}",                   // ־ַי  (אֵלַי, עָלַי)
        "\u{05B8}\u{05D9}",                   // ־ָי  (pausal אֵלָי)
        "\u{05B4}\u{05D9}",                   // ־ִי  (לִי, בִּי, עִמִּי)
    ]},
    PronounSuffix { key: "3mp", meaning: "them", endings: &[
        "\u{05B5}\u{05D9}\u{05D4}\u{05B6}\u{05DD}", // ־ֵיהֶם (עֲלֵיהֶם)
        "\u{05D4}\u{05B6}\u{05DD}",           // ־הֶם  (לָהֶם, מֵהֶם)
        "\u{05DE}\u{05D5}\u{05B9}",           // ־מוֹ  (poetic לָמוֹ)
        "\u{05B8}\u{05DD}",                   // ־ָם  (אֹתָם, עִמָּם)
    ]},
    PronounSuffix { key: "2ms", meaning: "you (m.)", endings: &[
        "\u{05B6}\u{05D9}\u{05DA}\u{05B8}",   // ־ֶיךָ (אֵלֶיךָ, עָלֶיךָ)
        "\u{05DA}\u{05B8}\u{05BC}",           // ־ךָּ  (מִמֶּךָּ)
        "\u{05DA}\u{05B8}",                   // ־ךָ  (לְךָ, עִמְּךָ)
    ]},
    PronounSuffix { key: "3fs", meaning: "her", endings: &[
        "\u{05B6}\u{05D9}\u{05D4}\u{05B8}",   // ־ֶיהָ (עָלֶיהָ, אֵלֶיהָ)
        "\u{05E0}\u{05B8}\u{05BC}\u{05D4}",   // ־נָּה (מִמֶּנָּה)
        "\u{05B8}\u{05D4}\u{05BC}",           // ־ָהּ  (לָהּ, בָּהּ — mapiq)
    ]},
    PronounSuffix { key: "1cp", meaning: "us", endings: &[
        "\u{05B5}\u{05D9}\u{05E0}\u{05D5}\u{05BC}", // ־ֵינוּ (אֵלֵינוּ, עָלֵינוּ)
        "\u{05E0}\u{05BC}\u{05D5}\u{05BC}",   // ־נּוּ (מִמֶּנּוּ)
        "\u{05B8}\u{05E0}\u{05D5}\u{05BC}",   // ־ָנוּ (לָנוּ, עִמָּנוּ)
    ]},
    PronounSuffix { key: "2mp", meaning: "you (pl.)", endings: &[
        "\u{05B5}\u{05D9}\u{05DB}\u{05B6}\u{05DD}", // ־ֵיכֶם (עֲלֵיכֶם)
        "\u{05DB}\u{05BC}\u{05B6}\u{05DD}",   // ־כֶּם (מִכֶּם — assimilated נ doubles)
        "\u{05DB}\u{05B6}\u{05DD}",           // ־כֶם  (לָכֶם, אֶתְכֶם)
    ]},
    PronounSuffix { key: "2fs", meaning: "you (f.)", endings: &[
        "\u{05B7}\u{05D9}\u{05B4}\u{05DA}\u{05B0}", // ־ַיִךְ (עָלַיִךְ)
        "\u{05B8}\u{05D9}\u{05B4}\u{05DA}\u{05B0}", // ־ָיִךְ (pausal עָלָיִךְ)
        "\u{05B8}\u{05DA}\u{05B0}",           // ־ָךְ  (לָךְ, עִמָּךְ)
        "\u{05B5}\u{05DA}\u{05B0}",           // ־ֵךְ  (מִמֵּךְ)
    ]},
    PronounSuffix { key: "3fp", meaning: "them (f.)", endings: &[
        "\u{05B5}\u{05D9}\u{05D4}\u{05B6}\u{05DF}", // ־ֵיהֶן (עֲלֵיהֶן)
        "\u{05D4}\u{05B6}\u{05DF}",           // ־הֶן  (לָהֶן)
    ]},
];

/// The inventory entry with this key, if known.
pub fn pronoun_suffix(key: &str) -> Option<&'static PronounSuffix> {
    PRONOUN_SUFFIXES.iter().find(|p| p.key == key)
}

/// A surface split at its pronominal ending: `surface == stem + suffix`
/// (codepoint concatenation). The suffix span is what the app highlights.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuffixSplit {
    pub key: &'static str,
    pub meaning: &'static str,
    pub stem: String,
    pub suffix: String,
}

fn is_mark(c: char) -> bool {
    ('\u{0591}'..='\u{05C7}').contains(&c)
}

/// A base consonant with its combining marks, and the byte offset of each
/// codepoint in the source string.
struct Cluster {
    base: char,
    /// (byte offset, mark), in source order.
    marks: Vec<(usize, char)>,
    start: usize,
}

fn clusters(s: &str) -> Vec<Cluster> {
    let mut out: Vec<Cluster> = Vec::new();
    for (i, c) in s.char_indices() {
        if is_mark(c) {
            if let Some(last) = out.last_mut() {
                last.marks.push((i, c));
            }
        } else {
            out.push(Cluster {
                base: c,
                marks: Vec::new(),
                start: i,
            });
        }
    }
    out
}

fn mark_set(marks: &[(usize, char)]) -> Vec<char> {
    let mut m: Vec<char> = marks.iter().map(|&(_, c)| c).collect();
    m.sort_unstable();
    m
}

/// Where `surface` starts matching `ending`, as a byte offset — or `None`.
///
/// Matching is cluster-aware, not a raw suffix compare: attaching a pronoun
/// often doubles the stem's last consonant, so a dagesh sits between the
/// ending's opening vowel and its consonant (עִמִּי is מ + hiriq + dagesh +
/// yod). Full clusters of the ending must match exactly (base and mark set);
/// the ending's *leading* marks (its opening niqqud, which sits under the
/// stem's last consonant) need only be present there — any doubling dagesh
/// stays with the match, so the highlighted span keeps it.
fn match_ending(surface_clusters: &[Cluster], ending: &str) -> Option<usize> {
    let mut lead: Vec<char> = Vec::new();
    let mut full: Vec<(char, Vec<char>)> = Vec::new();
    for c in ending.chars() {
        if is_mark(c) {
            match full.last_mut() {
                Some((_, marks)) => marks.push(c),
                None => lead.push(c),
            }
        } else {
            full.push((c, Vec::new()));
        }
    }
    for (_, marks) in &mut full {
        marks.sort_unstable();
    }

    // The stem must keep at least one consonant of its own.
    let n = surface_clusters.len().checked_sub(full.len())?;
    if n == 0 {
        return None;
    }
    for (ec, sc) in full.iter().zip(&surface_clusters[n..]) {
        if ec.0 != sc.base || ec.1 != mark_set(&sc.marks) {
            return None;
        }
    }
    let host = &surface_clusters[n - 1];
    if lead.is_empty() {
        return Some(surface_clusters[n].start);
    }
    // Every leading mark must sit on the stem's last consonant; the span
    // starts at the first of them (a doubling dagesh after it is included).
    let host_set = mark_set(&host.marks);
    if !lead.iter().all(|m| host_set.contains(m)) {
        return None;
    }
    host.marks
        .iter()
        .find(|(_, c)| lead.contains(c))
        .map(|&(i, _)| i)
}

/// Split `surface` at the longest matching pronominal ending, if any. The
/// niqqud opening an ending often sits under the stem's last consonant
/// (אֵלַי splits to אֵל + ־ַי with the patach in the suffix span) — exactly
/// the span a grammar would underline. Callers must gate on
/// [`crate::grammar::pronoun_suffix_host`]; the match alone is meaningless on
/// arbitrary words.
pub fn split_pronoun_suffix(surface: &str) -> Option<SuffixSplit> {
    pronoun_suffix_splits(surface).into_iter().next()
}

/// Every pronominal-ending match on `surface`, longest ending first (ties in
/// inventory order, like [`split_pronoun_suffix`] which is simply the first of
/// these). Callers with their own plausibility anchor — e.g. requiring the
/// remainder to cover a known stem's consonants, which is how שְׁמוֹ avoids
/// the poetic 3mp ־מוֹ and splits at שֵׁם + ־וֹ — pick the first survivor.
pub fn pronoun_suffix_splits(surface: &str) -> Vec<SuffixSplit> {
    let sc = clusters(surface);
    let mut all: Vec<(&'static PronounSuffix, usize, usize)> = Vec::new();
    for p in PRONOUN_SUFFIXES {
        for e in p.endings {
            if let Some(at) = match_ending(&sc, e) {
                all.push((p, at, e.len()));
            }
        }
    }
    all.sort_by(|a, b| b.2.cmp(&a.2));
    all.into_iter()
        .map(|(p, at, _)| SuffixSplit {
            key: p.key,
            meaning: p.meaning,
            stem: surface[..at].to_string(),
            suffix: surface[at..].to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn split(s: &str) -> SuffixSplit {
        split_pronoun_suffix(s).unwrap_or_else(|| panic!("no split for {s}"))
    }

    #[test]
    fn splits_rejoin_to_the_surface() {
        for s in [
            "אֵלַי", "אֵלָי", "עָלָיו", "אֵלֶיךָ", "עָלֶיהָ", "לוֹ", "בָּהּ",
            "מִמֶּנּוּ", "מִמֶּנִּי", "מִמֶּנָּה", "מִמֶּךָּ", "לָהֶם", "מֵהֶם",
            "עֲלֵיהֶם", "אֶתְכֶם", "עֲלֵיכֶם", "לָנוּ", "אֵלֵינוּ", "עָלַיִךְ",
            "לָךְ", "אֹתָם", "לָמוֹ", "כָּמֹהוּ", "עֲלֵיהֶן", "לָהֶן", "עִמָּדִי",
        ] {
            let sp = split(s);
            assert_eq!(format!("{}{}", sp.stem, sp.suffix), s, "rejoin {s}");
        }
    }

    #[test]
    fn person_and_span_are_right() {
        // The pausal 1cs — the form that motivated the drill — splits like
        // its context twin, just with the long vowel in the suffix span.
        assert_eq!(split("אֵלַי").key, "1cs");
        assert_eq!(split("אֵלָי").key, "1cs");
        assert_eq!(split("אֵלָי").stem, "אֵל");
        assert_eq!(split("אֵלַי").meaning, "me");

        assert_eq!(split("עָלָיו").key, "3ms");
        assert_eq!(split("עָלָיו").suffix, "\u{05B8}\u{05D9}\u{05D5}");
        assert_eq!(split("לוֹ").key, "3ms");

        // Longest match wins: poetic לָמוֹ is "them", not a ־וֹ "him"; the
        // dagesh in עִמּוֹ keeps it off the ־מוֹ pattern.
        assert_eq!(split("לָמוֹ").key, "3mp");
        assert_eq!(split("עִמּוֹ").key, "3ms");
        assert_eq!(split("עֲלֵיהֶם").suffix.chars().count(), 5);

        // The doubled-nun מִן forms carry the nun in the highlighted span.
        assert_eq!(split("מִמֶּנּוּ").key, "1cp");
        assert_eq!(split("מִמֶּנִּי").key, "1cs");
        assert_eq!(split("מִמֶּנָּה").key, "3fs");
    }

    #[test]
    fn meanings_are_distinct_quiz_options() {
        let mut seen = std::collections::HashSet::new();
        for p in PRONOUN_SUFFIXES {
            assert!(seen.insert(p.meaning), "duplicate meaning {}", p.meaning);
            assert!(pronoun_suffix(p.key).is_some());
        }
    }

    #[test]
    fn unsuffixed_function_words_do_not_split() {
        // Host eligibility is gated elsewhere; but the bare function words in
        // the same curated families must not accidentally match an ending.
        for s in ["אֶת", "אֵת", "עַל", "אֶל", "מִן", "בְּעַד", "כָּל"] {
            assert!(split_pronoun_suffix(s).is_none(), "{s} should not split");
        }
    }
}
