//! Resolving a surface — and a corpus token — to the word info the reader
//! shows, by searching the generation databases' candidate analyses.
//!
//! This is build-time work. `haqor db gen-runtime` runs it once per distinct
//! rendering and stores the answer in `haqor.db`, so the runtime looks word
//! info up rather than searching for it; see
//! `doc/adr/0006-single-runtime-database.md`. The queries here name the
//! generation schemas (`hebrewdb`, `lexdb`) and only work against a connection
//! with those attached.
//!
//! It lives in this crate rather than in `haqor-db-gen` because the pure
//! helpers it leans on — gloss inflection, name sniffing, consonant folding,
//! the OSHB morphology decoders — are the runtime's too, and duplicating them
//! is how the stored answer would drift from the live one.

use rusqlite::{Connection, OptionalExtension};

use crate::bible::{
    HebrewWord, OshbAnalysis, apply_oshb_analysis, bdb_rows, cross_reference_gloss, curated_gloss,
    decode_noun_label, decode_pgn, fold_consonants, has_plural_tail, name_pos,
    normalize_hebrew_combining, root_stub_gloss, strip_accents, unfinalize,
};

pub fn strong_lexeme(db: &Connection, strong: i64) -> Option<(String, String, bool)> {
    let mut stmt = db
        .prepare(
            "SELECT b.root, b.gloss, b.pos FROM lexdb.lexical_index i \
             JOIN lexdb.bdb b ON b.bdb_id = i.bdb_id \
             WHERE i.strong = ?1 ORDER BY b.bdb_id",
        )
        .ok()?;
    let rows: Vec<(String, String, String)> = stmt
        .query_map([strong], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            ))
        })
        .ok()?
        .collect::<rusqlite::Result<_>>()
        .ok()?;
    let root = rows
        .iter()
        .map(|row| row.0.as_str())
        .find(|root| !root.is_empty())?;
    let gloss = rows
        .iter()
        .map(|row| row.1.as_str())
        .find(|gloss| !gloss.is_empty() && !cross_reference_gloss(gloss) && !root_stub_gloss(gloss))
        .unwrap_or_default();
    let is_name = rows.iter().any(|row| name_pos(&row.2));
    Some((root.to_string(), gloss.to_string(), is_name))
}

/// One OSHB token tagging, as `hebrew.db.oshb_primary` stores it.
pub struct TokenTagging<'a> {
    pub source_word: &'a str,
    pub lemma: &'a str,
    pub morph: &'a str,
}

/// Resolve the word info the reader would show for `surface_id`, with a
/// concrete token's tagging applied when there is one. This is what
/// `gen-runtime` stores in `word_info`.
pub fn resolve(
    db: &Connection,
    surface_id: i64,
    norm: &str,
    tagging: Option<TokenTagging<'_>>,
) -> Option<HebrewWord> {
    let analysis = tagging.map(|t| OshbAnalysis {
        source_word: crate::bible::normalize_oshb_word(t.source_word),
        lemma: t.lemma.to_string(),
        morph: t.morph.to_string(),
    });
    word_info(db, surface_id, norm.to_string(), analysis.as_ref())
}

pub(crate) fn word_info(
    db: &Connection,
    surface_id: i64,
    norm: String,
    analysis: Option<&OshbAnalysis>,
) -> Option<HebrewWord> {
    let generated = generated_word(db, surface_id, norm.clone());
    let info = match analysis {
        Some(analysis) => {
            let seed = generated.unwrap_or_else(|| HebrewWord {
                word: norm.clone(),
                ..Default::default()
            });
            let (mut sourced, strong) = apply_oshb_analysis(seed, analysis);
            if let Some((root, gloss, is_name)) = strong.and_then(|s| strong_lexeme(db, s)) {
                sourced.root = root;
                if sourced.gloss.is_empty() {
                    sourced.gloss = gloss;
                }
                sourced.is_name |= is_name;
            }
            sourced
        }
        None => generated?,
    };
    // The live path applies the device-local `lexicon_entries` correction on
    // top of this. It deliberately has no counterpart here: that layer lives in
    // the writable progress database and is the one part of word info that
    // cannot be precomputed.
    Some(info)
}

pub fn generated_word(db: &Connection, surface_id: i64, norm: String) -> Option<HebrewWord> {
    // Top verb analysis by stored rank (attestation, then generator order).
    // `analysis_id` is unique, so it alone determines the pick; `has_bdb` is
    // still selected for the verb-vs-noun decision below.
    let verb = db
        .query_row(
            "SELECT a.root, a.binyan, a.form, a.pgn, a.prefix, a.vav_consecutive, \
                    a.obj_suffix, a.attested, \
                    EXISTS(SELECT 1 FROM lexdb.bdb b WHERE b.root = a.root) AS has_bdb \
             FROM hebrewdb.analyses a \
             WHERE a.surface_id = ?1 \
             ORDER BY EXISTS(SELECT 1 FROM lexdb.primary_analysis_overrides p \
                WHERE p.surface = ?2 AND p.analysis_type = 'verb' \
                  AND p.root = a.root AND p.binyan = a.binyan \
                  AND p.form = a.form AND p.pgn = a.pgn AND p.prefix = a.prefix \
                  AND p.vav_consecutive = a.vav_consecutive \
                  AND p.obj_suffix = a.obj_suffix) DESC, a.analysis_id ASC \
             LIMIT 1",
            rusqlite::params![surface_id, norm],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)? != 0,
                    row.get::<_, String>(6)?,
                    row.get::<_, i64>(7)? != 0,
                    row.get::<_, i64>(8)? != 0,
                ))
            },
        )
        .optional()
        .ok()?;

    // Candidate noun analyses, resolved to a BDB root by folding the stem to
    // bare medial consonants and matching `bdb.cons`; the curated overrides
    // are consulted first so a homograph collision (סוּס the horse vs BDB's
    // preceding "swallow; swift" bird entry) picks the intended lexeme. The
    // first stem that resolves wins; otherwise the first candidate is kept
    // unresolved so the morphology still shows even without a lexicon
    // bridge.
    let noun_rows = {
        let mut stmt = db
            .prepare(
                "SELECT n.kind, n.label, n.prefix, n.stem, \
                        EXISTS(SELECT 1 FROM lexdb.primary_analysis_overrides p \
                          WHERE p.surface = ?2 AND p.analysis_type = 'noun' \
                            AND p.stem = n.stem AND p.kind = n.kind \
                            AND p.label = n.label AND p.prefix = n.prefix) AS forced \
                 FROM hebrewdb.noun_analyses n \
                 WHERE n.surface_id = ?1 ORDER BY forced DESC, n.analysis_id ASC",
            )
            .ok()?;
        stmt.query_map(rusqlite::params![surface_id, norm], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)? != 0,
            ))
        })
        .ok()?
        .collect::<rusqlite::Result<Vec<_>>>()
        .ok()?
    };

    // Root/gloss empty if unresolved. `resolved` is tracked as a flag
    // rather than inferred from a non-empty root, because a curated lexeme
    // can pin a gloss while (following BDB) carrying no root — מַיִם
    // "water". `curated` marks a curated-table hit, which outranks any verb
    // reading below: the table exists to pin exactly the high-frequency
    // words whose skeleton collides with an unrelated lexeme, and such
    // words also attract junk verb parses (מַיִם as a jussive of יממ).
    // `is_name` follows the resolved BDB lexeme's `pos` (curated lexemes
    // are real vocabulary, never names).
    struct NounReading {
        kind: String,
        label: String,
        prefix: String,
        stem: String,
        root: String,
        gloss: String,
        resolved: bool,
        curated: bool,
        is_name: bool,
        forced: bool,
    }
    let noun: Option<NounReading> = {
        let mut chosen: Option<NounReading> = None;
        for (kind, label, prefix, stem, forced) in noun_rows {
            let curated = curated_gloss(db, &stem);
            let is_curated = curated.is_some();
            let resolved = curated
                .map(|(root, gloss)| (root, gloss, false))
                .or_else(|| cons_root(db, &stem));
            let resolves = resolved.is_some();
            let (root, gloss, is_name) = resolved.unwrap_or_default();
            let reading = NounReading {
                kind,
                label,
                prefix: unfinalize(&prefix),
                stem,
                root,
                gloss,
                resolved: resolves,
                curated: is_curated,
                is_name,
                forced,
            };
            if forced || resolves {
                chosen = Some(reading);
                break;
            }
            chosen.get_or_insert(reading);
        }
        chosen
    };

    let noun_resolves = noun.as_ref().is_some_and(|n| n.resolved);
    let noun_curated = noun.as_ref().is_some_and(|n| n.curated);
    let noun_forced = noun.as_ref().is_some_and(|n| n.forced);
    let verb_forced = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM lexdb.primary_analysis_overrides \
             WHERE surface = ?1 AND analysis_type = 'verb')",
            [&norm],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0)
        != 0;
    // A resolvable noun reading led by the definite article beats a verb
    // reading that only exists by mistreating that article: a he-peeled
    // non-participle (the article never prefixes a finite verb — הָעָם is
    // not an imperative of עמה) or a strong-verb fallback that buries the
    // article inside the root (הַיּוֹם as *הימ). The he must carry real
    // article pointing (patah/qamats/segol): a hataf-patah he is the
    // interrogative, whose verb reading is genuine (הֲתֵלֵךְ "will you
    // go?" is not "your mound"). Article + participle is real Hebrew, so
    // participles keep their verb reading.
    let article_pointed = |prefix: &str| {
        let mut cs = prefix.chars();
        cs.next() == Some('\u{05D4}')
            && matches!(cs.next(), Some('\u{05B7}' | '\u{05B8}' | '\u{05B6}'))
    };
    let article_noun = noun_resolves && noun.as_ref().is_some_and(|n| article_pointed(&n.prefix));
    let verb_shadows_article = verb.as_ref().is_some_and(|v| {
        let participle = v.2.contains("Participle");
        (!participle && v.4.starts_with('\u{05D4}')) || (!v.7 && v.0.starts_with('\u{05D4}'))
    });
    let verb_resolves = verb.as_ref().is_some_and(|v| v.8)
        && !(article_noun && verb_shadows_article)
        && !noun_curated;

    // Prefer a BDB-resolvable verb; else a resolvable noun; else whatever
    // analysis exists (verb before noun).
    if let Some((root, binyan, tense, pgn, prefix, vav_con, obj_suffix, _, _)) = verb
        .as_ref()
        .filter(|_| verb_forced || (!noun_forced && (verb_resolves || !noun_resolves)))
    {
        let (person, gender, number) = decode_pgn(pgn);
        let gloss = root_gloss(db, root);
        return Some(HebrewWord {
            word: norm,
            root: root.clone(),
            gloss,
            part_of_speech: Some("Verb".to_string()),
            form: (!binyan.is_empty()).then(|| binyan.clone()),
            tense: (!tense.is_empty()).then(|| tense.clone()),
            person,
            gender,
            number,
            state: None,
            prefix: (!prefix.is_empty()).then(|| prefix.clone()),
            vav_con: *vav_con,
            obj_suffix: (!obj_suffix.is_empty()).then(|| obj_suffix.clone()),
            is_name: false,
        });
    }

    if let Some(n) = noun {
        let (mut number, mut state) = decode_noun_label(&n.label);
        // A curated irregular / gold-harvested form carries only an opaque
        // label ("Irregular (father)", "Noun (heel)") — the inventory lists
        // attested surfaces per lemma with no per-form cell, so a possessive
        // suffix or plural ending is invisible to morphology display and
        // grammar gating (אֲבֹתָם taught as a plain grammar-free noun).
        // Recover the cell structurally from the surface's tail: a
        // pronominal ending upgrades the state to the "label + 3ms" shape
        // [`inflect_noun`] and [`crate::grammar::concepts_for`] already
        // understand; a plural/dual ending sets the number. Guarded by the
        // consonantal skeleton: only a form whose consonants go beyond
        // prefix + stem carries extra morphology — the lemma חַי must not
        // sniff its own ־ַי as "my …", and plural-tantum מַיִם (whose
        // curated lemma *is* the surface, dual tail and all) stays the
        // plain "water" even when prefixed (הַמַּיִם).
        let opaque = number.is_none()
            && state
                .as_deref()
                .is_some_and(|s| s.starts_with("Irregular (") || s.starts_with("Noun ("));
        let stem_cons = fold_consonants(&n.stem);
        let inflected =
            fold_consonants(&norm) != format!("{}{stem_cons}", fold_consonants(&n.prefix));
        if opaque && inflected {
            // Longest pronominal ending whose remainder still holds every
            // stem consonant — without the anchor, שְׁמוֹ would split at
            // the poetic 3mp ־מוֹ instead of שֵׁם + ־וֹ. A remainder
            // running past the stem is a plural stem (אֲבֹתָם "their
            // fathers", חַיֶּיךָ "your life"), so the number follows.
            let splits: Vec<(&str, String)> = crate::pronoun_suffix::pronoun_suffix_splits(&norm)
                .into_iter()
                .map(|sp| (sp.key, fold_consonants(&sp.stem)))
                .collect();
            // A feminine stem trades its final ה for ת before a suffix
            // (נְבֵלָה → נִבְלָתוֹ), and פֶּה drops its ה outright
            // (פִּיו, פִּיהֶם bind on פִּי), so anchor on those shapes too.
            let fem_cons = stem_cons
                .strip_suffix('\u{05D4}')
                .map(|s| format!("{s}\u{05EA}"));
            let fem_plural_cons = stem_cons
                .strip_suffix('\u{05D4}')
                .map(|s| format!("{s}\u{05D5}\u{05EA}"));
            let he_dropped = stem_cons.strip_suffix('\u{05D4}').filter(|s| !s.is_empty());
            let anchored = |rest: &str| {
                rest.ends_with(&stem_cons)
                    || rest.starts_with(&stem_cons)
                    || fem_cons
                        .as_deref()
                        .is_some_and(|f| rest.ends_with(f) || rest.starts_with(f))
                    || fem_plural_cons
                        .as_deref()
                        .is_some_and(|f| rest.ends_with(f) || rest.starts_with(f))
                    || he_dropped.is_some_and(|d| rest.ends_with(d))
            };
            let split = splits
                .iter()
                .find(|(_, rest)| anchored(rest))
                // A plural-tantum stem truncates before its suffix (פָּנָיו
                // keeps only פנ of פנים) — tolerate a remainder that is a
                // leading piece of the stem, but only when no full-stem
                // match exists, so שְׁמוֹ still prefers שֵׁם + ־וֹ.
                .or_else(|| {
                    splits
                        .iter()
                        .find(|(_, rest)| rest.len() >= 4 && stem_cons.starts_with(rest.as_str()))
                });
            if let Some((key, rest)) = split {
                state = state.map(|s| format!("{s} + {key}"));
                if rest.len() > stem_cons.len() + fold_consonants(&n.prefix).len() {
                    number = Some("Plural".to_string());
                }
            } else if has_plural_tail(&norm) {
                number = Some("Plural".to_string());
            }
        }
        return Some(HebrewWord {
            word: norm,
            root: n.root,
            gloss: n.gloss,
            part_of_speech: Some(if n.is_name { "Proper noun" } else { "Noun" }.to_string()),
            form: None,
            tense: None,
            person: None,
            gender: (!n.kind.is_empty()).then_some(n.kind),
            number,
            state,
            prefix: (!n.prefix.is_empty()).then_some(n.prefix),
            vav_con: false,
            obj_suffix: None,
            is_name: n.is_name,
        });
    }

    // Closed-class function words (and proper nouns) carry a surface row but
    // no generated verb/noun analysis — the prefilter strips their spurious
    // verb readings and they are not nouns. The gen-hebrew build precomputes
    // a lexicon bridge for them into `lexical_analyses`; read it back so the
    // app shows a gloss instead of "no OT parse". A missing table (an older
    // db) just yields `None`, the previous behaviour.
    //
    // A curated override wins over the baked bridge row: the build-time
    // bridge consults the lexical overlay too, but entries added since the
    // shipped hebrew.db was generated would otherwise be shadowed by the
    // stale row (אוּלַי stayed bridged to the river Ulai), and surfaces
    // with no row at all (עֲלֵי) would return no word info despite being
    // curated.
    // A curated lexicon entry carries a root (so the word links into its
    // BDB root tree); it must be consulted before the rootless learner
    // gloss below, or a word curated in both (לִקְרַאת) loses its root —
    // which the tutor's family gating relies on.
    if let Some((root, gloss)) = curated_gloss(db, &norm) {
        return Some(HebrewWord {
            word: norm,
            root,
            gloss,
            part_of_speech: None,
            form: None,
            tense: None,
            person: None,
            gender: None,
            number: None,
            state: None,
            prefix: None,
            vav_con: false,
            obj_suffix: None,
            is_name: false,
        });
    }
    // Learner-facing surface glosses also make analysis-less function words
    // resolvable. They intentionally do not carry a lexicon root (unlike
    // `curated_gloss` above), but a word such as מִכֹּל still needs to open
    // in word info rather than falling through as an unknown OT parse.
    if let Some(curated) = crate::vocab_gloss::curated_gloss(db, &norm) {
        return Some(HebrewWord {
            word: norm,
            root: String::new(),
            gloss: curated.gloss.to_string(),
            part_of_speech: None,
            form: None,
            tense: None,
            person: None,
            gender: None,
            number: None,
            state: None,
            prefix: None,
            vav_con: false,
            obj_suffix: None,
            is_name: false,
        });
    }
    let bridge = db
        .query_row(
            "SELECT la.root, la.gloss, la.prefix \
             FROM hebrewdb.lexical_analyses la \
             WHERE la.surface_id = ?1",
            [surface_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .ok()
        .flatten();
    if let Some((root, gloss, prefix)) = bridge {
        // The build-time bridge table carries no BDB `pos`; name detection
        // for these falls back to the gloss-text sniff (`is_name_gloss`)
        // in the tutor.
        return Some(HebrewWord {
            word: norm,
            root,
            gloss,
            part_of_speech: None,
            form: None,
            tense: None,
            person: None,
            gender: None,
            number: None,
            state: None,
            prefix: (!prefix.is_empty()).then(|| unfinalize(&prefix)),
            vav_con: false,
            obj_suffix: None,
            is_name: false,
        });
    }

    None
}

pub fn cons_root(db: &Connection, stem: &str) -> Option<(String, String, bool)> {
    if let Some(rows) = bdb_rows(db, stem) {
        let canonical = normalize_hebrew_combining(&strip_accents(stem));
        let exact = |(word, ..): &&(String, String, String, String)| {
            normalize_hebrew_combining(&strip_accents(word)) == canonical
        };
        if let Some((_, root, gloss, pos)) = rows
            .iter()
            .find(|row| exact(row) && !row.3.starts_with("vb") && !name_pos(&row.3))
            .or_else(|| {
                rows.iter()
                    .find(|row| exact(row) && !row.3.starts_with("vb"))
            })
            .or_else(|| rows.iter().find(exact))
            .or_else(|| rows.first())
        {
            return Some((root.clone(), gloss.clone(), name_pos(pos)));
        }
    }
    let cons = fold_consonants(stem);
    if cons.is_empty() {
        return None;
    }
    db.query_row(
        "SELECT root, pos FROM lexdb.bdb \
             WHERE cons = ?1 AND (gloss IS NULL OR gloss = '' OR gloss LIKE '(%') \
             ORDER BY bdb_id LIMIT 1",
        [cons],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                String::new(),
                name_pos(&row.get::<_, Option<String>>(1)?.unwrap_or_default()),
            ))
        },
    )
    .optional()
    .ok()
    .flatten()
}

pub fn root_gloss(db: &Connection, root: &str) -> String {
    let Ok(mut stmt) = db.prepare(
        "SELECT word, gloss FROM lexdb.bdb \
         WHERE root = ?1 AND gloss IS NOT NULL AND gloss <> '' \
         ORDER BY bdb_id",
    ) else {
        return String::new();
    };
    let Ok(rows) = stmt
        .query_map([root], |row| {
            Ok((
                row.get::<_, Option<String>>(0)?.unwrap_or_default(),
                row.get::<_, String>(1)?,
            ))
        })
        .and_then(|rows| rows.collect::<rusqlite::Result<Vec<_>>>())
    else {
        return String::new();
    };
    let mut glosses: Vec<String> = rows
        .into_iter()
        .map(|(word, imported)| {
            curated_gloss(db, &word)
                .filter(|(curated_root, _)| curated_root == root)
                .map(|(_, gloss)| gloss)
                .unwrap_or(imported)
        })
        .filter(|gloss| !cross_reference_gloss(gloss) && !root_stub_gloss(gloss))
        .collect();
    glosses.sort_by_key(|gloss| {
        gloss
            .chars()
            .next()
            .is_some_and(|c| matches!(c as u32, 0x0590..=0x05FF))
    });
    glosses.into_iter().next().unwrap_or_default()
}
