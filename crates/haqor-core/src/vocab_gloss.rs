//! Curated learner glosses for the most frequent OT surface forms.
//!
//! The automatic BDB bridge ([`crate::bible::Bible::hebrew_word_info`]) keys on
//! citation form and often mis-glosses closed-class particles, pronominal-
//! suffixed forms and construct chains (and cannot see homograph context). This
//! table pins a concise learner gloss — and an optional composition note — for
//! the highest-frequency such words, so the tutor shows the right meaning
//! regardless of how the parser resolved the surface.
//!
//! This data lives here, in the core, rather than in the Flutter layer: the app
//! is presentation only. Lookups normalise both the stored surface and the
//! table's keys through [`vocab_key`](crate::vocab_gloss::vocab_key), so combining-mark order and dagesh
//! variants (בֶּן/בֶן) collapse to one key.

use rusqlite::Connection;

/// A curated gloss for a surface: the learner-facing meaning and an optional
/// composition/teaching note ("לְ (to) + ־וֹ (him)").
#[derive(Debug, Clone)]
pub struct CuratedGloss {
    pub gloss: String,
    pub note: Option<String>,
}

/// Reduce a pointed surface form to a stable lookup key: dagesh, meteg and
/// cantillation dropped, the remaining marks sorted within each letter's
/// cluster. Mirrors the app's former `vocabKey` so the same literals match.
pub fn vocab_key(surface: &str) -> String {
    let mut out = String::new();
    let mut marks: Vec<u32> = Vec::new();
    for c in surface.chars() {
        let u = c as u32;
        let is_mark = (0x0591..=0x05C7).contains(&u);
        if !is_mark {
            flush_marks(&mut out, &mut marks);
            out.push(c);
        } else if u != 0x05BC && u != 0x05BD && !(0x0591..=0x05AF).contains(&u) {
            // Keep vowel points and shin/sin dots; drop dagesh, meteg, accents.
            marks.push(u);
        }
    }
    flush_marks(&mut out, &mut marks);
    out
}

fn flush_marks(out: &mut String, marks: &mut Vec<u32>) {
    marks.sort_unstable();
    for &m in marks.iter() {
        if let Some(ch) = char::from_u32(m) {
            out.push(ch);
        }
    }
    marks.clear();
}

/// The curated gloss for `surface`, if one is registered. Normalises the input
/// through [`vocab_key`] before lookup.
pub fn curated_gloss(db: &Connection, surface: &str) -> Option<CuratedGloss> {
    query_curated_gloss(db, surface, false)
}

/// An explicit reader override for `surface`, if one is registered.
///
/// Unlike an ordinary curated gloss, this is intended to replace an imported
/// occurrence-level gloss rather than merely provide a lexical fallback.
pub fn curated_reader_gloss(db: &Connection, surface: &str) -> Option<CuratedGloss> {
    query_curated_gloss(db, surface, true)
}

fn query_curated_gloss(
    db: &Connection,
    surface: &str,
    reader_override_only: bool,
) -> Option<CuratedGloss> {
    let key = vocab_key(surface);
    let query = if reader_override_only {
        "SELECT surface, gloss, note FROM word_gloss WHERE reader_override = 1"
    } else {
        "SELECT surface, gloss, note FROM word_gloss"
    };
    let mut stmt = db.prepare(query).ok()?;
    stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
        ))
    })
    .ok()?
    .flatten()
    .find_map(|(stored, gloss, note)| {
        (vocab_key(&stored) == key && !gloss.is_empty()).then_some(CuratedGloss { gloss, note })
    })
}

/// Whether `surface` is one of the curated proper names — names whose BDB
/// entry is missing or oddly glossed, so the automatic `n.pr` detection
/// (`crate::bible::is_name_gloss`) can't see them. Complements that check
/// wherever the tutor classifies names.
pub fn curated_name(db: &Connection, surface: &str) -> bool {
    let key = vocab_key(surface);
    let Ok(mut stmt) = db.prepare("SELECT surface FROM word_gloss WHERE is_name = 1") else {
        return false;
    };
    stmt.query_map([], |row| row.get::<_, String>(0))
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .any(|stored| vocab_key(&stored) == key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vocab_key_drops_dagesh_and_sorts_marks() {
        // בֶּן (bet + dagesh + segol) and בֶן (bet + segol) collapse.
        assert_eq!(vocab_key("בֶּן"), vocab_key("בֶן"));
        assert_eq!(vocab_key("כָּל"), vocab_key("כָל"));
        // Cantillation and meteg are dropped.
        assert_eq!(vocab_key("דָּבָר"), vocab_key("דָּבָר\u{0591}"));
        // Combining-mark order is normalised: mem + segol + dagesh (database NFC
        // order, vowel before dagesh) matches mem + dagesh + segol (traditional).
        let nfc: String = ['\u{05DE}', '\u{05B6}', '\u{05BC}'].iter().collect();
        let traditional: String = ['\u{05DE}', '\u{05BC}', '\u{05B6}'].iter().collect();
        assert_eq!(vocab_key(&nfc), vocab_key(&traditional));
        // Meaningful distinctions are kept: different vowels, and the shin/sin dot.
        assert_ne!(vocab_key("עַם"), vocab_key("עִם"));
        assert_ne!(vocab_key("שׁ"), vocab_key("שׂ"));
    }

    #[test]
    fn curated_gloss_matches_dagesh_variants() {
        // Cargo runs this from the package root, so the data dir has to be
        // named relative to the manifest, not to the cwd.
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/haqor.db");
        if !data.exists() {
            return;
        }
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(&format!(
            "ATTACH DATABASE '{}' AS data",
            data.to_str().unwrap()
        ))
        .unwrap();
        // "the word" is registered under הַדָּבָר; a dagesh-stripped spelling
        // still resolves.
        assert!(curated_gloss(&db, "הַדָּבָר").is_some());
        assert_eq!(curated_gloss(&db, "אֶת").unwrap().gloss, "←");
        assert_eq!(curated_gloss(&db, "אֵת").unwrap().gloss, "←");
        assert_eq!(curated_gloss(&db, "וְאֶת").unwrap().gloss, "and ←");
        assert_eq!(curated_gloss(&db, "וְאֵת").unwrap().gloss, "and ←");
        assert!(curated_gloss(&db, "כִּי").unwrap().note.is_none());
        assert_eq!(
            curated_reader_gloss(&db, "אֱלֹהִים").unwrap().gloss,
            "Mighty-ones"
        );
        assert!(curated_reader_gloss(&db, "עַל").is_none());
        // An ordinary content word is not curated.
        assert!(curated_gloss(&db, "בָּרָא").is_none());
    }
}
