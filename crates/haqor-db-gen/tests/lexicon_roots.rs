//! Which root does a BDB entry belong to?
//!
//! BDB groups entries into sections headed by a root, and a derivative inherits
//! that root — `מִשְׁפָּט` belongs to `שפט` even though its own consonants begin
//! with a mem, so inheritance cannot be checked against the headword's spelling.
//!
//! But BDB also parks pure *cross-references* in whatever section they sort
//! into. "רוּת v. רעה" sits in the רוש section because רוּת sorts there, not
//! because Ruth derives from "be in want" — and inheriting made Ruth a member of
//! that family, so a reader looking up וְלָרָשׁ "and the poor man" in 2 Sam 12:3
//! was offered twelve verses from the book of Ruth. 1,542 entries were filed
//! that way. A redirect now takes the root of the article it points at.
//!
//! Builds the lexicon from `src_texts/`, so it needs no generated database —
//! but BDB lives in the `src_texts/HebrewLexicon` submodule, so a checkout
//! without that submodule cannot run it.

use std::path::{Path, PathBuf};

use haqor_db_gen::generate_lexicon;

/// The source tree, once the BDB submodule is known to be there.
///
/// Without the check the failure is a bare "No such file or directory" from
/// deep inside generation, which reads as a bug in the lexicon builder.
fn src_texts() -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../src_texts");
    assert!(
        dir.join("HebrewLexicon/BrownDriverBriggs.xml").exists(),
        "src_texts/HebrewLexicon is empty — run \
         `git submodule update --init src_texts/HebrewLexicon`"
    );
    dir
}

/// `(entry id, expected root, why)`.
const EXPECTED: &[(&str, &str, &str)] = &[
    // The reported case: "רוּת v. רעה" — Ruth, filed in the רוש section.
    (
        "t.br.ac",
        "רעה",
        "רוּת redirects to רעה, and sorts under רוש",
    ),
    // Others from the same section, which must keep the root they derive from.
    ("t.br.aa", "רוש", "רוּשׁ heads the section"),
    (
        "t.br.ab",
        "רוש",
        "רִישׁ 'poverty' is a real derivative of רוש",
    ),
    // A redirect whose target is in a quite different part of the lexicon.
    ("a.ag.ac", "בטח", "אֲבַטִּיחִים 'watermelons' redirects to בטח"),
    ("a.ag.ai", "אבה", "אֶבְיוֹן 'needy' redirects to אָבָה"),
    ("a.am.ad", "בנט", "אַבְנֵט redirects to בנט"),
    // Redirects that say so in prose rather than with "see".
    ("h.cc.ac", "חיה", "חַי — 'חִיאֵל under חיה'"),
    ("p.cl.aa", "עופ", "עִיף — '= עוף q.v.'"),
    // Guards against over-reaching. Neither is a redirect: both simply lack a
    // bold definition and a pos, so their gloss comes from the sense fallback.
    // A first cut keyed those on their own consonant skeleton and took real
    // roots away from them.
    ("a.gj.ah", "אשר", "אֲשֻׁרִים 'steps' derives from אשר"),
    ("v.av.ae", "שבע", "שִׁבְעָ֫נָה derives from שבע"),
];

#[test]
fn cross_references_are_filed_under_the_article_they_point_at() {
    let output = std::env::temp_dir().join("haqor-lexicon-roots-check.db");
    generate_lexicon(&src_texts(), &output).expect("generating the lexicon");
    let db = rusqlite::Connection::open(&output).expect("opening the generated lexicon");

    for (id, want, why) in EXPECTED {
        let (root, word, gloss) = db
            .query_row(
                "SELECT root, word, COALESCE(gloss, '') FROM bdb WHERE bdb_id = ?1",
                [id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .unwrap_or_else(|e| panic!("{id} is missing from the lexicon: {e}"));
        assert_eq!(
            &root, want,
            "{id} ({word}, {gloss:?}) is filed under {root}, expected {want} — {why}"
        );
    }

    let _ = std::fs::remove_file(&output);
}

/// No whole-lexicon sweep accompanies the cases above, deliberately.
///
/// The tempting one — "the target named in the gloss must equal the root" — is
/// not a property BDB has. The text after "see" is as often a pointed headword
/// as a root (`see אַי`), finals are written as finals where roots are not
/// (`see גרף` against the root `גרפ`), and an article named by its own spelling
/// can legitimately sit under a different root (`see אחת` is filed under `אחח`).
/// A sweep on that basis reports hundreds of failures that are all correct, which
/// is worse than no sweep: it would be silenced rather than believed.
///
/// The end-to-end property *is* checkable, and is asserted where it can be —
/// `haqor_core::bible`'s `root_occurrences_exclude_unrelated_redirects`, which
/// requires the built corpus and states the reader-visible consequence: the root
/// רוש must not offer verses from the book of Ruth.
#[test]
fn every_entry_has_a_root_or_is_deliberately_rootless() {
    let output = std::env::temp_dir().join("haqor-lexicon-sweep-check.db");
    generate_lexicon(&src_texts(), &output).expect("generating the lexicon");
    let db = rusqlite::Connection::open(&output).expect("opening the generated lexicon");

    // Re-rooting redirects must not leave a hole. Some entries legitimately have
    // no root: a section whose head is a particle or a bare letter (א, אוֹ "or",
    // אוּלַי "perhaps") has no triliteral root to give, and its members inherit
    // that. Currently 228 — 96 such section heads and 132 entries under them.
    // This is a ceiling against a change that strips roots wholesale, not a
    // target to drive down.
    let rootless = db
        .query_row(
            "SELECT COUNT(*) FROM bdb WHERE root IS NULL OR root = ''",
            [],
            |row| row.get::<_, i64>(0),
        )
        .expect("counting");
    assert!(
        rootless <= 240,
        "{rootless} lexicon entries have no root, up from 228; re-rooting \
         redirects should not take roots away"
    );

    let _ = std::fs::remove_file(&output);
}
