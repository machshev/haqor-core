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
use crate::vocab_gloss::vocab_key;
use std::collections::HashMap;
use std::sync::OnceLock;

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

/// Every teachable concept, in teaching order — the ordering used by the
/// tutor's reference view of already-unlocked concepts.
pub fn concepts() -> &'static [GrammarConcept] {
    CONCEPTS
}

/// The total number of teachable grammar concepts — the top of the
/// [`concept_rank`] scale and the cap for the tutor's unlock frontier.
pub fn concept_count() -> usize {
    CONCEPTS.len()
}

/// The teaching-order index of a concept key — the value a surface exercising
/// this concept as its hardest rule carries as its [`concept_rank`]. `None`
/// for an unknown key.
pub fn concept_index(key: &str) -> Option<i64> {
    CONCEPTS.iter().position(|c| c.key == key).map(|i| i as i64)
}

/// A word's grammatical *complexity rank*: the highest [`CONCEPTS`] index among
/// the concepts it exercises (`CONCEPTS` is ordered by teaching difficulty), or
/// `-1` when it exercises none (a proper noun, function word, plain absolute
/// noun, or bare Qal verb). The tutor gates introduction on this so grammar
/// rules unlock one at a time: a word is only introducible once every concept it
/// uses — i.e. its rank — is below the current unlock frontier.
pub fn concept_rank(w: &HebrewWord) -> i64 {
    rank_of(&concepts_for(w))
}

/// [`concept_rank`] keyed by the surface form: the rank of
/// [`concepts_for_surface`], so curated function words and misparsed construct
/// forms gate like everything else.
pub fn concept_rank_for_surface(surface: &str, w: Option<&HebrewWord>) -> i64 {
    rank_of(&concepts_for_surface(surface, w))
}

fn rank_of(keys: &[&'static str]) -> i64 {
    keys.iter()
        .filter_map(|k| CONCEPTS.iter().position(|c| &c.key == k))
        .map(|i| i as i64)
        .max()
        .unwrap_or(-1)
}

/// The grammar concepts `surface` exercises. The curated [`SURFACE_CONCEPTS`]
/// table wins where present — it covers closed-class words the reverse-parser
/// never sees (standalone and suffixed prepositions) and frequent construct
/// forms it misreads (דְּבַר surfaces as a Qal imperative) — otherwise the
/// parse-derived [`concepts_for`] applies. `None` for `w` (no parse at all)
/// classifies from the table alone.
pub fn concepts_for_surface(surface: &str, w: Option<&HebrewWord>) -> Vec<&'static str> {
    if let Some(keys) = surface_concepts(surface) {
        return keys.to_vec();
    }
    let keys = w.map(concepts_for).unwrap_or_default();
    // A surface with no classification at all still betrays a conjunctive vav
    // in its spelling: essentially no Hebrew word begins with a pointed vav
    // that isn't the proclitic "and" (וָמַעְלָה "and upward" reaches here with
    // no parse and used to gate as grammar-free). Only the vav is recoverable
    // this way, so the fallback stays minimal.
    if keys.is_empty() {
        let mut cs = surface.chars();
        if cs.next() == Some('\u{05D5}')
            && cs.next().is_some_and(|c| ('\u{05B0}'..='\u{05BC}').contains(&c))
        {
            return vec!["conj-ve"];
        }
    }
    keys
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

/// Whether `surface` is a curated suffixed function word — an eligible host
/// for the pronoun-ending drill (see `progress.suffix_srs` in
/// [`crate::tutor`]). True for the suffixed prepositions and the suffixed
/// object-marker forms; the bare words in those families are excluded by the
/// caller's [`crate::pronoun_suffix::split_pronoun_suffix`] finding no ending.
pub fn pronoun_suffix_host(surface: &str) -> bool {
    surface_concepts(surface).is_some_and(|keys| {
        keys.contains(&"prep-suffix") || keys.contains(&"object-marker")
    })
}

/// The curated concepts for `surface`, if listed, matched through
/// [`vocab_key`] so dagesh and mark-order variants collapse to one entry.
fn surface_concepts(surface: &str) -> Option<&'static [&'static str]> {
    static INDEX: OnceLock<HashMap<String, &'static [&'static str]>> = OnceLock::new();
    INDEX
        .get_or_init(|| {
            SURFACE_CONCEPTS
                .iter()
                .map(|&(s, keys)| (vocab_key(s), keys))
                .collect()
        })
        .get(&vocab_key(surface))
        .copied()
}

/// Concepts for high-frequency surfaces the parse can't classify: closed-class
/// function words never reach the reverse-parser (the object marker, standalone
/// prepositions and every suffixed preposition), and some frequent construct
/// forms parse to a spurious verb or absolute reading. Each entry lists all
/// concepts the surface exercises, proclitic first. Pronouns and adverbs stay
/// unlisted — they exercise no concept and remain ungated.
#[rustfmt::skip]
const SURFACE_CONCEPTS: &[(&str, &[&str])] = &[
    // The definite direct-object marker, its suffixed forms ("him", "them",
    // "you"…) in both defective (אֹתוֹ) and plene (אוֹתוֹ) spellings, and the
    // frequent vav-prefixed combinations. The similar אִתּ־ forms below are the
    // preposition אֵת "with", a different lexeme.
    ("אֶת", &["object-marker"]),
    ("אֵת", &["object-marker"]),
    ("וְאֶת", &["conj-ve", "object-marker"]),
    ("וְאֵת", &["conj-ve", "object-marker"]),
    ("אֹתוֹ", &["object-marker"]),
    ("אוֹתוֹ", &["object-marker"]),
    ("אֹתָהּ", &["object-marker"]),
    ("אוֹתָהּ", &["object-marker"]),
    ("אֹתִי", &["object-marker"]),
    ("אוֹתִי", &["object-marker"]),
    ("אֹתְךָ", &["object-marker"]),
    ("אוֹתְךָ", &["object-marker"]),
    ("אֹתָךְ", &["object-marker"]),
    ("אוֹתָךְ", &["object-marker"]),
    ("אֹתָם", &["object-marker"]),
    ("אוֹתָם", &["object-marker"]),
    ("אֹתָנוּ", &["object-marker"]),
    ("אֶתְכֶם", &["object-marker"]),
    ("אֶתְהֶם", &["object-marker"]),
    ("אֶתְהֶן", &["object-marker"]),
    ("וְאֹתוֹ", &["conj-ve", "object-marker"]),
    ("וְאֹתִי", &["conj-ve", "object-marker"]),
    // Standalone prepositions.
    ("עַל", &["preposition"]),
    ("אֶל", &["preposition"]),
    ("עַד", &["preposition"]),
    ("עִם", &["preposition"]),
    ("אַחַר", &["preposition"]),
    ("אַחֲרֵי", &["preposition"]),
    ("תַּחַת", &["preposition"]),
    ("בֵּין", &["preposition"]),
    ("סָבִיב", &["preposition"]),
    ("לְמַעַן", &["preposition"]),
    ("כְּמוֹ", &["preposition"]),
    ("לִפְנֵי", &["preposition"]),
    ("מִפְּנֵי", &["preposition"]),
    ("מֵעַל", &["prep-min", "preposition"]),
    ("מֵאֵת", &["prep-min", "preposition"]),
    ("וְעַל", &["conj-ve", "preposition"]),
    ("וְאֶל", &["conj-ve", "preposition"]),
    ("וְעַד", &["conj-ve", "preposition"]),
    ("וּבֵין", &["conj-ve", "preposition"]),
    // Suffixed standalone prepositions ("on him", "to me", "with us", …) —
    // every form gates behind the pronoun-ending card as well as the
    // preposition card. Pausal twins (אֵלָי, עָלָי, אַחֲרָי) are separate
    // surfaces: vocab_key keeps vowel points, so they need their own entries.
    ("אֵלָיו", &["preposition", "prep-suffix"]),
    ("אֵלַי", &["preposition", "prep-suffix"]),
    ("אֵלָי", &["preposition", "prep-suffix"]),
    ("אֵלַיִךְ", &["preposition", "prep-suffix"]),
    ("אֵלֶיךָ", &["preposition", "prep-suffix"]),
    ("אֵלֶיהָ", &["preposition", "prep-suffix"]),
    ("אֵלֵינוּ", &["preposition", "prep-suffix"]),
    ("אֲלֵיהֶם", &["preposition", "prep-suffix"]),
    ("אֲלֵהֶם", &["preposition", "prep-suffix"]),
    ("אֲלֵיכֶם", &["preposition", "prep-suffix"]),
    ("עָלָיו", &["preposition", "prep-suffix"]),
    ("עָלֶיהָ", &["preposition", "prep-suffix"]),
    ("עָלַי", &["preposition", "prep-suffix"]),
    ("עָלָי", &["preposition", "prep-suffix"]),
    ("עָלַיִךְ", &["preposition", "prep-suffix"]),
    ("עָלָיִךְ", &["preposition", "prep-suffix"]),
    ("עָלֶיךָ", &["preposition", "prep-suffix"]),
    ("עָלֵינוּ", &["preposition", "prep-suffix"]),
    ("עֲלֵיהֶם", &["preposition", "prep-suffix"]),
    ("עֲלֵהֶם", &["preposition", "prep-suffix"]),
    ("עֲלֵיהֶן", &["preposition", "prep-suffix"]),
    ("עֲלֵיכֶם", &["preposition", "prep-suffix"]),
    ("עָלֵימוֹ", &["preposition", "prep-suffix"]),
    ("עִמּוֹ", &["preposition", "prep-suffix"]),
    ("עִמִּי", &["preposition", "prep-suffix"]),
    ("עִמָּדִי", &["preposition", "prep-suffix"]),
    ("עִמָּךְ", &["preposition", "prep-suffix"]),
    ("עִמְּךָ", &["preposition", "prep-suffix"]),
    ("עִמָּהּ", &["preposition", "prep-suffix"]),
    ("עִמָּם", &["preposition", "prep-suffix"]),
    ("עִמָּכֶם", &["preposition", "prep-suffix"]),
    ("עִמָּנוּ", &["preposition", "prep-suffix"]),
    ("אִתּוֹ", &["preposition", "prep-suffix"]),
    ("אִתִּי", &["preposition", "prep-suffix"]),
    ("אִתָּם", &["preposition", "prep-suffix"]),
    ("אִתָּנוּ", &["preposition", "prep-suffix"]),
    ("אִתָּךְ", &["preposition", "prep-suffix"]),
    ("אִתְּךָ", &["preposition", "prep-suffix"]),
    ("אִתְּכֶם", &["preposition", "prep-suffix"]),
    ("אַחֲרָיו", &["preposition", "prep-suffix"]),
    ("אַחֲרַי", &["preposition", "prep-suffix"]),
    ("אַחֲרָי", &["preposition", "prep-suffix"]),
    ("אַחֲרֶיךָ", &["preposition", "prep-suffix"]),
    ("אַחֲרֶיהָ", &["preposition", "prep-suffix"]),
    ("אַחֲרֵיהֶם", &["preposition", "prep-suffix"]),
    ("תַּחְתָּיו", &["preposition", "prep-suffix"]),
    ("תַּחְתֶּיהָ", &["preposition", "prep-suffix"]),
    ("בֵּינִי", &["preposition", "prep-suffix"]),
    ("כָּמוֹךָ", &["preposition", "prep-suffix"]),
    ("כָּמֹהוּ", &["preposition", "prep-suffix"]),
    ("בַּעֲדוֹ", &["preposition", "prep-suffix"]),
    ("בְּעַד", &["preposition"]),
    // Inseparable לְ — suffixed forms and opaque fusions.
    ("לוֹ", &["prep-le", "prep-suffix"]),
    ("לִי", &["prep-le", "prep-suffix"]),
    ("לְךָ", &["prep-le", "prep-suffix"]),
    ("לָךְ", &["prep-le", "prep-suffix"]),
    ("לָהּ", &["prep-le", "prep-suffix"]),
    ("לָהֶם", &["prep-le", "prep-suffix"]),
    ("לָהֶן", &["prep-le", "prep-suffix"]),
    ("לָכֶם", &["prep-le", "prep-suffix"]),
    ("לָנוּ", &["prep-le", "prep-suffix"]),
    ("לָמוֹ", &["prep-le", "prep-suffix"]),
    ("לַיַהְוֶה", &["prep-le"]),
    ("לֵאמֹר", &["prep-le", "infinitive"]),
    ("לַעֲשׂוֹת", &["prep-le", "infinitive"]),
    // Inseparable בְּ — suffixed forms.
    ("בּוֹ", &["prep-be", "prep-suffix"]),
    ("בָּהּ", &["prep-be", "prep-suffix"]),
    ("בָּהֶם", &["prep-be", "prep-suffix"]),
    ("בִּי", &["prep-be", "prep-suffix"]),
    ("בְּךָ", &["prep-be", "prep-suffix"]),
    ("בָּךְ", &["prep-be", "prep-suffix"]),
    ("בָּם", &["prep-be", "prep-suffix"]),
    ("בָּנוּ", &["prep-be", "prep-suffix"]),
    ("בָּכֶם", &["prep-be", "prep-suffix"]),
    ("בַּיַהְוֶה", &["prep-be"]),
    // Inseparable כְּ and מִן.
    ("כַּאֲשֶׁר", &["prep-ke"]),
    ("מִן", &["prep-min"]),
    ("מִמֶּנּוּ", &["prep-min", "prep-suffix"]),
    ("מִמֶּנִּי", &["prep-min", "prep-suffix"]),
    ("מִמֶּנָּה", &["prep-min", "prep-suffix"]),
    ("מִמְּךָ", &["prep-min", "prep-suffix"]),
    ("מִמֶּךָּ", &["prep-min", "prep-suffix"]),
    ("מֵהֶם", &["prep-min", "prep-suffix"]),
    ("מִשָּׁם", &["prep-min"]),
    ("וּמִן", &["conj-ve", "prep-min"]),
    // Construct forms the parser misses or misreads.
    ("כָּל", &["construct"]),
    ("בְּנֵי", &["construct"]),
    ("בֵּית", &["construct"]),
    ("פְּנֵי", &["construct"]),
    ("דִּבְרֵי", &["construct"]),
    ("דְּבַר", &["construct"]),
    ("אַנְשֵׁי", &["construct"]),
    ("יְמֵי", &["construct"]),
    ("שְׁנֵי", &["construct"]),
    ("אֱלֹהֵי", &["construct"]),
    ("וְכָל", &["conj-ve", "construct"]),
    ("וּבְנֵי", &["conj-ve", "construct"]),
    ("בְּכָל", &["prep-be", "construct"]),
    ("בְּיַד", &["prep-be", "construct"]),
    ("לְכָל", &["prep-le", "construct"]),
    ("לִבְנֵי", &["prep-le", "construct"]),
    ("לְבֵית", &["prep-le", "construct"]),
    ("מִכָּל", &["prep-min", "construct"]),
    ("מִכֹּל", &["prep-min", "construct"]),
    ("מִבֵּית", &["prep-min", "construct"]),
    ("כְּכָל", &["prep-ke", "construct"]),
];

/// The teaching content, in a rough introduction order.
#[rustfmt::skip]
const CONCEPTS: &[GrammarConcept] = &[
    GrammarConcept {
        key: "object-marker",
        title: "The object marker אֶת",
        explanation: "אֶת — the most common word in the Bible — has no meaning of its \
            own: it points at the definite direct object, the person or thing the verb \
            acts on, and is left untranslated. With a pronoun ending it is the object \
            itself: אֹתוֹ (him), אֹתָם (them).",
        formula: Some("verb + אֶת + definite noun → (marks the object)"),
        examples: &["וַיַּרְא אֶת הָאוֹר — and he saw the light", "אֹתוֹ — him"],
    },
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
        key: "preposition",
        title: "Prepositions",
        explanation: "Small words placed before another word tie it into the sentence — \
            עַל (on), אֶל (to), עִם (with), תַּחַת (under).",
        formula: Some("preposition + noun → \"on / to / with …\""),
        examples: &["עַל הָאָרֶץ — on the earth", "אֶל מֹשֶׁה — to Moses"],
    },
    GrammarConcept {
        key: "prep-suffix",
        title: "Pronoun endings on prepositions",
        explanation: "Hebrew has no separate word for \"me\" or \"him\" after a \
            preposition — the pronoun is joined onto its end as a suffix: אֵלַי (to me), \
            עָלָיו (on him), עִמָּנוּ (with us). At a pause in the verse the vowel often \
            lengthens: אֵלַי becomes אֵלָי.",
        formula: Some("preposition + ־ִי / ־וֹ / ־ָם → \"… me / him / them\""),
        examples: &["אֵלַי — to me", "עָלָיו — on him", "עִמָּנוּ — with us"],
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
        assert!(
            !keys.contains(&"imperfect"),
            "narrative imperfect uses the wayyiqtol card"
        );
    }

    #[test]
    fn weqatal_supersedes_plain_perfect_card() {
        let keys = concepts_for(&verb_with_prefix("Qal", "Perfect", "\u{05D5}\u{05B0}"));
        assert!(keys.contains(&"weqatal"));
        assert!(
            !keys.contains(&"perfect"),
            "vav-consecutive perfect uses the weqatal card"
        );
        assert!(keys.contains(&"conj-ve"), "still notes the attached vav");
    }

    #[test]
    fn plain_perfect_unaffected_without_vav_prefix() {
        assert!(concepts_for(&verb("Qal", "Perfect", false)).contains(&"perfect"));
    }

    #[test]
    fn every_surface_concept_has_content() {
        for (surface, keys) in SURFACE_CONCEPTS {
            for key in *keys {
                assert!(concept(key).is_some(), "no content for {key} ({surface})");
            }
        }
    }

    #[test]
    fn function_words_classify_without_a_parse() {
        // Standalone and suffixed prepositions never reach the parser.
        assert_eq!(concepts_for_surface("עַל", None), vec!["preposition"]);
        assert_eq!(
            concepts_for_surface("לוֹ", None),
            vec!["prep-le", "prep-suffix"]
        );
        // Dagesh variants collapse through vocab_key.
        assert_eq!(
            concepts_for_surface("בוֹ", None),
            vec!["prep-be", "prep-suffix"]
        );
        // Pronouns exercise no concept and stay ungated.
        assert!(concepts_for_surface("הוּא", None).is_empty());
        assert_eq!(concept_rank_for_surface("הוּא", None), -1);
    }

    #[test]
    fn every_prep_suffix_surface_splits_for_the_drill() {
        // Each curated suffixed preposition must be a drill host: eligible
        // via pronoun_suffix_host and splittable into stem + ending.
        for (surface, keys) in SURFACE_CONCEPTS {
            if !keys.contains(&"prep-suffix") {
                continue;
            }
            assert!(pronoun_suffix_host(surface), "{surface} not host-eligible");
            assert!(
                crate::pronoun_suffix::split_pronoun_suffix(surface).is_some(),
                "{surface} carries prep-suffix but no ending splits"
            );
        }
        // The suffixed object-marker forms host the same drill.
        for s in ["אֹתוֹ", "אֹתָם", "אֶתְכֶם"] {
            assert!(pronoun_suffix_host(s), "{s} not host-eligible");
            assert!(crate::pronoun_suffix::split_pronoun_suffix(s).is_some());
        }
        // The bare words in those families never split, so they never host.
        assert!(crate::pronoun_suffix::split_pronoun_suffix("אֶת").is_none());
    }

    #[test]
    fn suffixed_prepositions_gate_behind_the_pronoun_ending_card() {
        // Every suffixed form — including the pausal twins, which are distinct
        // vocab_keys (vowel points are kept) — carries prep-suffix, so none is
        // introducible before the pronoun-ending card unlocks.
        for s in ["אֵלַי", "אֵלָי", "עָלַי", "עָלָי", "אַחֲרָיו", "מֵהֶם"] {
            let keys = concepts_for_surface(s, None);
            assert!(keys.contains(&"prep-suffix"), "{s} should carry prep-suffix");
            assert!(
                concept_rank_for_surface(s, None) > concept_rank_for_surface("אֶל", None),
                "{s} gates later than the bare preposition"
            );
        }
        // The bare prepositions themselves don't mention the suffix card.
        for s in ["אֶל", "עַל", "מִן", "בְּעַד"] {
            assert!(!concepts_for_surface(s, None).contains(&"prep-suffix"));
        }
    }

    #[test]
    fn surface_table_overrides_a_wrong_parse() {
        // דְּבַר ("word of") misparses as a Qal imperative; the curated entry
        // pins the construct reading for gating and the concept card.
        let w = verb("Qal", "Imperative", false);
        assert_eq!(concepts_for_surface("דְּבַר", Some(&w)), vec!["construct"]);
        assert!(
            concept_rank_for_surface("דְּבַר", Some(&w)) > concept_rank_for_surface("עַל", None),
            "construct gates later than the preposition card"
        );
    }

    #[test]
    fn unlisted_surface_falls_back_to_the_parse() {
        let w = verb("Piel", "Perfect", false);
        assert_eq!(concepts_for_surface("קִדֵּשׁ", Some(&w)), concepts_for(&w));
    }

    #[test]
    fn qal_has_no_binyan_card_but_derived_stems_do() {
        assert!(!concepts_for(&verb("Qal", "Perfect", false)).contains(&"binyan-piel"));
        assert!(concepts_for(&verb("Piel", "Perfect", false)).contains(&"binyan-piel"));
    }
}
