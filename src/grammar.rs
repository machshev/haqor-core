//! Grammar concepts taught by the tutor.
//!
//! Reading needs more than vocabulary: prefixes, verb conjugations, binyanim,
//! construct chains and suffixes all change what a form means. The tutor
//! introduces each concept once, as a short gradeless card, the first time a
//! word about to be learnt exercises it (see [`crate::tutor::StudyItem::
//! ExplainGrammar`]). [`concepts_for`] maps a parsed word to the concepts it
//! uses; the teaching content lives here in the core (the app is presentation
//! only) and travels with the card.

use crate::bible::HebrewWord;

/// A teachable grammar concept: a short explanation plus an optional formula and
/// worked examples. Keyed by a stable `key` recorded in `progress.concepts_seen`
/// so it is shown at most once.
#[derive(Debug, Clone, Copy)]
pub struct GrammarConcept {
    pub key: &'static str,
    pub title: &'static str,
    pub explanation: &'static str,
    pub formula: Option<&'static str>,
    pub examples: &'static [&'static str],
}

/// The concept with this key, if known.
pub fn concept(key: &str) -> Option<&'static GrammarConcept> {
    CONCEPTS.iter().find(|c| c.key == key)
}

/// The total number of teachable grammar concepts — the top of the
/// [`concept_rank`] scale and the cap for the tutor's unlock frontier.
pub fn concept_count() -> usize {
    CONCEPTS.len()
}

/// A word's grammatical *complexity rank*: the highest [`CONCEPTS`] index among
/// the concepts it exercises (`CONCEPTS` is ordered by teaching difficulty), or
/// `-1` when it exercises none (a proper noun, function word, plain absolute
/// noun, or bare Qal verb). The tutor gates introduction on this so grammar
/// rules unlock one at a time: a word is only introducible once every concept it
/// uses — i.e. its rank — is below the current unlock frontier.
pub fn concept_rank(w: &HebrewWord) -> i64 {
    concepts_for(w)
        .iter()
        .filter_map(|k| CONCEPTS.iter().position(|c| &c.key == k))
        .map(|i| i as i64)
        .max()
        .unwrap_or(-1)
}

/// The grammar concepts a parsed word exercises, in teaching order (attached
/// proclitic first, then stem, then conjugation/number, then suffix). Only
/// concepts with teaching content are returned.
pub fn concepts_for(w: &HebrewWord) -> Vec<&'static str> {
    let mut keys: Vec<&'static str> = Vec::new();

    // Attached proclitic (article / conjunction / preposition), by its leading
    // letter. A cluster like וְהַ contributes its first element.
    if let Some(prefix) = w.prefix.as_deref()
        && let Some(c) = prefix.chars().next()
    {
        let k = match c {
            '\u{05D4}' => Some("article"),  // he
            '\u{05D5}' => Some("conj-ve"),  // vav
            '\u{05D1}' => Some("prep-be"),  // bet
            '\u{05DC}' => Some("prep-le"),  // lamed
            '\u{05DB}' => Some("prep-ke"),  // kaf
            '\u{05DE}' => Some("prep-min"), // mem
            _ => None,
        };
        keys.extend(k);
    }

    let vav_prefix = w.prefix.as_deref().and_then(|p| p.chars().next()) == Some('\u{05D5}');

    if let Some(binyan) = w.form.as_deref() {
        // Verb: derived stem (Qal is the plain baseline, no card), then
        // conjugation, then a vav-consecutive / object-suffix note.
        keys.extend(match binyan {
            "Niphal" => Some("binyan-niphal"),
            "Piel" => Some("binyan-piel"),
            "Pual" => Some("binyan-pual"),
            "Hiphil" => Some("binyan-hiphil"),
            "Hophal" => Some("binyan-hophal"),
            "Hithpael" => Some("binyan-hithpael"),
            _ => None,
        });
        if w.vav_con {
            keys.push("wayyiqtol");
        }
        keys.extend(match w.tense.as_deref() {
            // The weqatal card already covers the vav-consecutive perfect.
            Some("Perfect") if vav_prefix => Some("weqatal"),
            Some("Perfect") => Some("perfect"),
            // The wayyiqtol card already covers the narrative imperfect.
            Some("Imperfect") if !w.vav_con => Some("imperfect"),
            Some("Imperative") => Some("imperative"),
            Some("Inf. Construct") | Some("Inf. Absolute") => Some("infinitive"),
            Some("Participle (act.)")
            | Some("Participle (pas.)")
            | Some("Participle (pass.)")
            | Some("Participle") => Some("participle"),
            Some("Jussive") | Some("Cohortative") => Some("jussive-cohortative"),
            _ => None,
        });
        if w.obj_suffix.is_some() {
            keys.push("object-suffix");
        }
    } else if w.tense.is_none() {
        // Noun / adjective.
        let state = w.state.as_deref().unwrap_or("");
        if matches!(w.number.as_deref(), Some("Plural") | Some("Dual")) || state.starts_with("Pl") {
            keys.push("noun-plural");
        }
        if state == "Construct" {
            keys.push("construct");
        }
        if state.contains('+') {
            keys.push("suffix-possessive");
        }
    }

    keys
}

/// The teaching content, in a rough introduction order.
#[rustfmt::skip]
const CONCEPTS: &[GrammarConcept] = &[
    GrammarConcept {
        key: "article",
        title: "The definite article",
        explanation: "A הַ joined to the front of a word means \"the\". It normally \
            doubles the first letter of the word (a dot in the letter).",
        formula: Some("הַ + noun → \"the …\""),
        examples: &["הַמֶּלֶךְ — the king", "הָאָרֶץ — the land"],
    },
    GrammarConcept {
        key: "conj-ve",
        title: "The conjunction \"and\"",
        explanation: "A וְ joined to the front of a word means \"and\". It is the most \
            common way Hebrew links words and clauses.",
        formula: Some("וְ + word → \"and …\""),
        examples: &["וְהָאָרֶץ — and the earth", "וְאֶת — and (object marker)"],
    },
    GrammarConcept {
        key: "prep-be",
        title: "The preposition בְּ",
        explanation: "A בְּ joined to the front of a word means \"in\", \"with\" or \"by\".",
        formula: Some("בְּ + noun → \"in / with …\""),
        examples: &["בְּיַד — in the hand of", "בְּרֵאשִׁית — in the beginning"],
    },
    GrammarConcept {
        key: "prep-le",
        title: "The preposition לְ",
        explanation: "A לְ joined to the front of a word means \"to\" or \"for\". It also \
            marks the infinitive (\"to do\").",
        formula: Some("לְ + noun → \"to / for …\""),
        examples: &["לְךָ — to you", "לַיהוָה — to the LORD"],
    },
    GrammarConcept {
        key: "prep-ke",
        title: "The preposition כְּ",
        explanation: "A כְּ joined to the front of a word means \"like\" or \"as\".",
        formula: Some("כְּ + noun → \"like / as …\""),
        examples: &["כְּאִישׁ — like a man"],
    },
    GrammarConcept {
        key: "prep-min",
        title: "The preposition מִן",
        explanation: "מִן means \"from\" or \"out of\". Joined to a word it often appears \
            as מִ with the next letter doubled.",
        formula: Some("מִ + noun → \"from …\""),
        examples: &["מִכָּל — from all", "מִצְרַיִם — Egypt (\"from …\" prefix elsewhere)"],
    },
    GrammarConcept {
        key: "perfect",
        title: "The perfect (completed action)",
        explanation: "The Hebrew perfect describes an action viewed as complete. It is \
            usually translated as English past tense.",
        formula: Some("perfect → \"he did …\""),
        examples: &["אָמַר — he said", "שָׁמַר — he kept"],
    },
    GrammarConcept {
        key: "imperfect",
        title: "The imperfect (incomplete action)",
        explanation: "The imperfect describes action not yet complete — future, habitual \
            or ongoing. Often translated with \"will\".",
        formula: Some("imperfect → \"he will do …\""),
        examples: &["יִשְׁמֹר — he will keep", "יֹאמַר — he will say"],
    },
    GrammarConcept {
        key: "wayyiqtol",
        title: "The narrative past (וַ + verb)",
        explanation: "Hebrew narrative is carried by a וַ joined to an imperfect verb (with \
            the next letter doubled). It reads as simple past — \"and he …\" — and drives \
            almost every story in the Bible.",
        formula: Some("וַ + imperfect → \"and he did …\""),
        examples: &["וַיֹּאמֶר — and he said", "וַיְהִי — and it came to pass"],
    },
    GrammarConcept {
        key: "weqatal",
        title: "The vav-consecutive perfect (וְ + perfect)",
        explanation: "A וְ joined to a perfect verb often carries a future, command or \
            sequence of instructions forward, rather than simply meaning \"and he did\" — \
            it reads more like \"and he will do\" or \"and you shall do\".",
        formula: Some("וְ + perfect → \"and (then) he will / shall do …\""),
        examples: &["וְשָׁמַרְתָּ — and you shall keep", "וְהָיָה — and it will come to pass"],
    },
    GrammarConcept {
        key: "imperative",
        title: "The imperative (commands)",
        explanation: "The imperative gives a command addressed to \"you\".",
        formula: Some("imperative → \"do …!\""),
        examples: &["שְׁמַע — hear!", "לֵךְ — go!"],
    },
    GrammarConcept {
        key: "infinitive",
        title: "The infinitive",
        explanation: "The infinitive names the action itself — \"to keep\", \"keeping\". \
            It very often follows לְ (\"to do …\").",
        formula: Some("(לְ +) infinitive → \"to do …\""),
        examples: &["לֵאמֹר — saying", "לַעֲשׂוֹת — to do"],
    },
    GrammarConcept {
        key: "participle",
        title: "The participle",
        explanation: "The participle describes ongoing action or the one doing it — \
            \"keeping\", \"one who keeps\".",
        formula: Some("participle → \"doing / one who does\""),
        examples: &["שֹׁמֵר — keeping / a keeper", "יֹשֵׁב — sitting / dweller"],
    },
    GrammarConcept {
        key: "jussive-cohortative",
        title: "Wishes and exhortations",
        explanation: "Short volitional forms express a wish or exhortation: the jussive \
            (\"let him …\") and the cohortative (\"let me / let us …\").",
        formula: Some("→ \"let him …\" / \"let me …\""),
        examples: &["יְהִי — let there be", "נֵלְכָה — let us go"],
    },
    GrammarConcept {
        key: "binyan-niphal",
        title: "The Niphal stem",
        explanation: "The Niphal is usually the passive or reflexive counterpart of the \
            plain (Qal) verb — \"be done\" or \"do to oneself\".",
        formula: Some("Niphal → passive / reflexive of Qal"),
        examples: &["נִשְׁמַר — he was kept", "נִלְחַם — he fought"],
    },
    GrammarConcept {
        key: "binyan-piel",
        title: "The Piel stem",
        explanation: "The Piel often intensifies the plain verb or makes it factitive \
            (\"bring about\"). The middle letter is doubled.",
        formula: Some("Piel → intensive / factitive"),
        examples: &["דִּבֶּר — he spoke", "קִדֵּשׁ — he sanctified"],
    },
    GrammarConcept {
        key: "binyan-pual",
        title: "The Pual stem",
        explanation: "The Pual is the passive of the Piel — the intensive action done to \
            the subject.",
        formula: Some("Pual → passive of Piel"),
        examples: &["גֻּנַּב — it was stolen"],
    },
    GrammarConcept {
        key: "binyan-hiphil",
        title: "The Hiphil stem",
        explanation: "The Hiphil is causative — making someone or something do the action \
            (\"cause to …\"). It usually shows a ה prefix or an i-vowel.",
        formula: Some("Hiphil → \"cause to …\""),
        examples: &["הִשְׁמִיד — he destroyed (caused to be ruined)", "הִמְלִיךְ — he made king"],
    },
    GrammarConcept {
        key: "binyan-hophal",
        title: "The Hophal stem",
        explanation: "The Hophal is the passive of the Hiphil — \"be caused to …\".",
        formula: Some("Hophal → passive of Hiphil"),
        examples: &["הָמְלַךְ — he was made king"],
    },
    GrammarConcept {
        key: "binyan-hithpael",
        title: "The Hithpael stem",
        explanation: "The Hithpael is reflexive or reciprocal — doing the action to or \
            among oneselves. It shows a תְ infix.",
        formula: Some("Hithpael → reflexive / reciprocal"),
        examples: &["הִתְהַלֵּךְ — he walked about", "הִתְקַדֵּשׁ — he consecrated himself"],
    },
    GrammarConcept {
        key: "noun-plural",
        title: "Plural nouns",
        explanation: "Masculine plurals end in ־ִים and feminine plurals in ־וֹת.",
        formula: Some("־ִים (m.) / ־וֹת (f.)"),
        examples: &["מְלָכִים — kings", "תּוֹרוֹת — laws"],
    },
    GrammarConcept {
        key: "construct",
        title: "The construct chain (\"X of Y\")",
        explanation: "To say \"the X of Y\", the first noun takes a shortened \"construct\" \
            form and is read together with the next: \"word of the king\".",
        formula: Some("construct + noun → \"X of Y\""),
        examples: &["דְּבַר יְהוָה — the word of the LORD", "בֵּית הַמֶּלֶךְ — the house of the king"],
    },
    GrammarConcept {
        key: "suffix-possessive",
        title: "Possessive suffixes on nouns",
        explanation: "A pronoun can be joined to the end of a noun to show possession — \
            \"his word\", \"my people\".",
        formula: Some("noun + suffix → \"his / my / their …\""),
        examples: &["דְּבָרוֹ — his word", "עַמִּי — my people"],
    },
    GrammarConcept {
        key: "object-suffix",
        title: "Object suffixes on verbs",
        explanation: "A pronoun can be joined to the end of a verb to mark its object — \
            \"he kept him\", \"I will send them\".",
        formula: Some("verb + suffix → \"… him / them\""),
        examples: &["שְׁמָרוֹ — he kept him", "בְּרָכוֹ — he blessed him"],
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    fn verb(binyan: &str, tense: &str, vav_con: bool) -> HebrewWord {
        HebrewWord {
            form: Some(binyan.to_string()),
            tense: Some(tense.to_string()),
            vav_con,
            ..Default::default()
        }
    }

    fn verb_with_prefix(binyan: &str, tense: &str, prefix: &str) -> HebrewWord {
        HebrewWord {
            form: Some(binyan.to_string()),
            tense: Some(tense.to_string()),
            prefix: Some(prefix.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn every_returned_concept_has_content() {
        // Any key concepts_for can emit must resolve to a card.
        let mut samples = vec![
            verb("Qal", "Perfect", false),
            verb("Qal", "Imperfect", true),
            verb("Piel", "Imperfect", false),
            verb("Hiphil", "Imperative", false),
            verb("Niphal", "Participle (act.)", false),
        ];
        let mut noun = HebrewWord {
            number: Some("Plural".to_string()),
            state: Some("Construct".to_string()),
            ..Default::default()
        };
        noun.prefix = Some("הַ".to_string());
        samples.push(noun);
        let mut suffixed = HebrewWord {
            state: Some("Sg + 3ms".to_string()),
            ..Default::default()
        };
        suffixed.obj_suffix = None;
        samples.push(suffixed);

        for w in &samples {
            for key in concepts_for(w) {
                assert!(concept(key).is_some(), "no content for concept {key}");
            }
        }
    }

    #[test]
    fn wayyiqtol_supersedes_plain_imperfect_card() {
        let keys = concepts_for(&verb("Qal", "Imperfect", true));
        assert!(keys.contains(&"wayyiqtol"));
        assert!(!keys.contains(&"imperfect"), "narrative imperfect uses the wayyiqtol card");
    }

    #[test]
    fn weqatal_supersedes_plain_perfect_card() {
        let keys = concepts_for(&verb_with_prefix("Qal", "Perfect", "\u{05D5}\u{05B0}"));
        assert!(keys.contains(&"weqatal"));
        assert!(!keys.contains(&"perfect"), "vav-consecutive perfect uses the weqatal card");
        assert!(keys.contains(&"conj-ve"), "still notes the attached vav");
    }

    #[test]
    fn plain_perfect_unaffected_without_vav_prefix() {
        assert!(concepts_for(&verb("Qal", "Perfect", false)).contains(&"perfect"));
    }

    #[test]
    fn qal_has_no_binyan_card_but_derived_stems_do() {
        assert!(!concepts_for(&verb("Qal", "Perfect", false)).contains(&"binyan-piel"));
        assert!(concepts_for(&verb("Piel", "Perfect", false)).contains(&"binyan-piel"));
    }
}
