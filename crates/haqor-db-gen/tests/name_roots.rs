//! A name is often built from two roots, not one.
//!
//! BDB prints each lexeme in a single root section, and for a compound name
//! that section can only be one of its elements — whichever the alphabet put
//! first. אֱלִיעֶ֫זֶר "God is help" is filed under אלה, so עזר never listed it;
//! יְהוֹנָתָן sits under הוה and lost נתן altogether. Strong's records the
//! composition (`from 410 and 5828`), and `entry_root` carries the result: every
//! root an entry belongs to, the BDB section first.
//!
//! What this pins down is that the extra membership reaches the two places a
//! reader meets a root — its lexeme tree and its concordance — and that the
//! word-info sheet is told there is a choice to make.
//!
//! Reads the built `data/haqor.db` rather than generating one, so it is a check
//! on the shipped artifact.

use std::path::{Path, PathBuf};

use haqor_core::bible::Bible;

fn data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

/// Skip when the runtime database has not been built, so a fresh checkout's
/// `cargo test` still passes — but fail when `HAQOR_REQUIRE_DATA` is set, which
/// CI does after generating it.
macro_rules! require_runtime {
    () => {
        if !data_dir().join("haqor.db").exists() {
            assert!(
                std::env::var_os("HAQOR_REQUIRE_DATA").is_none(),
                "HAQOR_REQUIRE_DATA is set but {} has no haqor.db",
                data_dir().display()
            );
            eprintln!("skipping: data/haqor.db not generated in this checkout");
            return;
        }
    };
}

#[test]
fn a_compound_name_offers_both_of_its_roots() {
    require_runtime!();
    let bible = Bible::open(data_dir()).expect("opening haqor.db");

    let options = bible
        .hebrew_root_options("אֱלִיעֶזֶר", "אלה")
        .expect("root options for Eliezer");
    let roots: Vec<&str> = options.iter().map(|o| o.root.as_str()).collect();
    assert_eq!(
        roots,
        vec!["אלה", "עזר"],
        "Eliezer is God (אל) + help (עזר); BDB prints it under אלה"
    );
    // The section root leads, and the labels say what each root means so the
    // choice reads as "god" against "help" rather than as two spellings.
    assert!(options[0].is_primary && !options[1].is_primary);
    // The label is the element's own gloss, which is what says which sense of a
    // shared section is meant: אלה heads the demonstrative אֵלֶּה "these", but
    // the element Eliezer is built from is אֵל "god".
    assert_eq!(options[0].gloss, "god");
    assert_eq!(options[1].gloss, "help");

    // The frequent names never reach the noun parser — the prefilter classifies
    // them — so they are reached as headwords in their own right instead. Israel
    // is the archetype of the compound: "God (אל) persists (שׂרה)" — a sin root,
    // keyed apart from שרה "let loose".
    let options = bible
        .hebrew_root_options("יִשְׂרָאֵל", "שׂרה")
        .expect("root options for Israel");
    let roots: Vec<&str> = options.iter().map(|o| o.root.as_str()).collect();
    assert_eq!(roots, vec!["שׂרה", "אלה"]);

    // The two lexicons rarely point a name alike: the corpus writes Jedidiah
    // with a mappiq (יְדִידְיָהּ) where BDB's headword has a plain he, and Joshua
    // defective (יְהוֹשֻׁעַ) where Strong's writes it plene. Neither is reachable
    // by pointing, so both come through the token's own tagging.
    for (word, root, expected) in [
        ("יְדִידְיָהּ", "ידד", ["ידד", "הוה"]),
        ("יְהוֹשֻׁעַ", "הוה", ["הוה", "ישע"]),
    ] {
        let options = bible
            .hebrew_root_options(word, root)
            .expect("root options for a compound name");
        let roots: Vec<&str> = options.iter().map(|o| o.root.as_str()).collect();
        assert_eq!(roots, expected.to_vec(), "{word} is built from two roots");
    }

    // A name whose root the parser invented — מִיכָאֵל resolves to the skeleton
    // מיכ, which is no lexeme's root — still offers the element BDB knows.
    let options = bible
        .hebrew_root_options("מִיכָאֵל", "מיכ")
        .expect("root options for Michael");
    assert!(
        options.iter().any(|o| o.root == "אלה"),
        "Michael is \"who is like God\", got {options:?}"
    );

    // An ordinary word has one root, so the sheet has no choice to offer.
    let options = bible
        .hebrew_root_options("דָּבָר", "דבר")
        .expect("root options for דָּבָר");
    assert_eq!(options.len(), 1, "expected a single root, got {options:?}");
}

#[test]
fn a_name_stands_in_the_lists_of_every_root_it_is_made_of() {
    require_runtime!();
    let bible = Bible::open(data_dir()).expect("opening haqor.db");

    // Genesis 15:2, Abraham's steward Eliezer of Damascus — the first token of
    // the name in the canon, and previously in no root's concordance at all:
    // BDB points its headword with a stress accent (אֱלִיעֶ֫זֶר), which the
    // concordance join did not see through.
    for root in ["אלה", "עזר"] {
        let verses = bible
            .hebrew_root_occurrences(root)
            .expect("root occurrences");
        assert!(
            verses
                .iter()
                .any(|v| (v.book, v.chapter, v.verse) == (1, 15, 2)),
            "{root} should list Gen 15:2, where אֱלִיעֶזֶר stands"
        );

        let tokens = bible
            .hebrew_root_occurrences_detailed(root)
            .expect("detailed root occurrences");
        assert!(
            tokens.iter().any(|t| t.surface == "אֱלִיעֶזֶר"),
            "{root}'s token list should include the name itself"
        );

        let tree = bible.hebrew_bdb_by_root(root).expect("root tree");
        assert!(
            tree.iter().any(|entry| entry.headword.contains("אֱלִיעֶ")),
            "{root}'s lexeme tree should include Eliezer"
        );
    }

    // Joshua's tokens reach the root he is named for, spelling differences and
    // proclitics included — the tagging says which article the word is, so
    // וִיהוֹשֻׁעַ counts as much as the bare form.
    let saved = bible
        .hebrew_root_occurrences_detailed("ישע")
        .expect("occurrences of ישע");
    assert!(
        saved.iter().filter(|t| t.surface.contains("הוֹשֻׁעַ")).count() > 100,
        "ישע should list Joshua's tokens ({} found)",
        saved.iter().filter(|t| t.surface.contains("הוֹשֻׁעַ")).count()
    );
}

#[test]
fn a_name_filed_under_another_names_article_keeps_its_own_roots() {
    require_runtime!();
    let bible = Bible::open(data_dir()).expect("opening haqor.db");

    // BDB takes מְפִיבֹשֶׁת Mephibosheth for an alteration of Merib-baal and
    // prints it as a cross-reference in ריב, and the lexical index sent H4648
    // to Merib-baal's article. The name then read "Baal is; advocate" and
    // offered בעל; its own elements are פאה and בֹּשֶׁת "shame". 2 Sam 4:4.
    let info = bible
        .hebrew_word_info_at("מְפִיבֹֽשֶׁת", 9, 4, 4, 24)
        .expect("Mephibosheth in 2 Sam 4:4");
    assert!(
        !info.gloss.contains("Baal"),
        "Mephibosheth is not glossed as Merib-baal: {:?}",
        info.gloss
    );
    let options = bible
        .hebrew_root_options(&info.word, &info.root)
        .expect("root options for Mephibosheth");
    let roots: Vec<&str> = options.iter().map(|o| o.root.as_str()).collect();
    assert_eq!(roots, vec!["ריב", "פאה", "בוש"]);

    // The family BDB files it in holds ריב's lexemes, not the Klein and Jastrow
    // articles that only share a short spelling with a stub of it (רִב "see
    // ריב" meeting רַב "much") or with a truncated alternative (Jastrow's רִיבּ׳
    // for רִיבּוֹא "myriad").
    let tree = bible.hebrew_bdb_by_root("ריב").expect("root tree");
    let lexemes = bible
        .root_lexemes("ריב", tree, Vec::new())
        .expect("root lexemes");
    let headwords: Vec<&str> = lexemes.iter().map(|l| l.headword.as_str()).collect();
    for stray in ["רַב", "רֹב", "רוֹב", "רִבּוֹא", "רִבּוּנָא"] {
        assert!(!headwords.contains(&stray), "{stray} in ריב: {headwords:?}");
    }
    assert!(headwords.contains(&"רִיב"));
}
