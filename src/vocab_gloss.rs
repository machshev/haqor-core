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
//! table's keys through [`vocab_key`], so combining-mark order and dagesh
//! variants (בֶּן/בֶן) collapse to one key.

use std::collections::HashMap;
use std::sync::OnceLock;

/// A curated gloss for a surface: the learner-facing meaning and an optional
/// composition/teaching note ("לְ (to) + ־וֹ (him)").
#[derive(Debug, Clone, Copy)]
pub struct CuratedGloss {
    pub gloss: &'static str,
    pub note: Option<&'static str>,
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
pub fn curated_gloss(surface: &str) -> Option<CuratedGloss> {
    index().get(&vocab_key(surface)).copied()
}

/// Whether `surface` is one of the curated proper names — names whose BDB
/// entry is missing or oddly glossed, so the automatic `n.pr` detection
/// ([`crate::bible::is_name_gloss`]) can't see them. Complements that check
/// wherever the tutor classifies names.
pub fn curated_name(surface: &str) -> bool {
    static NAMES: OnceLock<std::collections::HashSet<String>> = OnceLock::new();
    NAMES
        .get_or_init(|| CURATED_NAMES.iter().map(|s| vocab_key(s)).collect())
        .contains(&vocab_key(surface))
}

fn index() -> &'static HashMap<String, CuratedGloss> {
    static INDEX: OnceLock<HashMap<String, CuratedGloss>> = OnceLock::new();
    INDEX.get_or_init(|| {
        CURATED
            .iter()
            .map(|&(surface, gloss, note)| (vocab_key(surface), CuratedGloss { gloss, note }))
            .collect()
    })
}

/// `(surface, gloss, note)` — the curated learner glosses.
#[rustfmt::skip]
const CURATED: &[(&str, &str, Option<&str>)] = &[
    // Object marker and its forms.
    ("אֶת", "(marks the direct object)", Some("Untranslated particle pointing to the object of the verb — the most common word in the Bible.")),
    ("אֵת", "(marks the direct object); with", None),
    ("וְאֶת", "and — (object marker)", Some("וְ (and) + אֶת")),
    ("וְאֵת", "and — (object marker)", Some("וְ (and) + אֵת")),
    ("אֹתוֹ", "him, it", Some("אֵת (object marker) + ־וֹ (him)")),
    ("אוֹתוֹ", "him, it", Some("אֵת (object marker) + ־וֹ (him)")),
    ("אֹתָם", "them", Some("אֵת (object marker) + ־ָם (them)")),
    ("אוֹתָם", "them", Some("אֵת (object marker) + ־ָם (them)")),
    ("אֹתָהּ", "her, it", Some("אֵת (object marker) + ־ָהּ (her)")),
    ("אֹתִי", "me", Some("אֵת (object marker) + ־ִי (me)")),
    ("אֶתְכֶם", "you (plural)", Some("אֵת (object marker) + ־כֶם")),

    // The divine name.
    ("יְהוָה", "the LORD (YHWH)", Some("The divine name, traditionally read aloud as אֲדֹנָי (Adonai).")),
    ("יְהוִה", "the LORD (YHWH)", Some("Pointed to be read as אֱלֹהִים when it follows אֲדֹנָי.")),
    ("לַיהוָה", "to the LORD", Some("לְ (to) + the divine name")),
    ("בַּיהוָה", "in the LORD", Some("בְּ (in) + the divine name")),

    // Core particles.
    // Slashes, not commas: the card headlines the leading sense only, and the
    // relative word is one word covering all three English renderings — the
    // context, not the Hebrew, picks between them.
    ("אֲשֶׁר", "who / which / that", Some("The relative word: links a clause to the word before it. \"Who\" for people, \"which\" or \"that\" for things — context decides.")),
    ("כַּאֲשֶׁר", "as / when", Some("כְּ (as) + אֲשֶׁר (which) — the pair fuses to \"as\" or \"when\".")),
    ("כַאֲשֶׁר", "as / when", Some("כְּ (as) + אֲשֶׁר (which) — the pair fuses to \"as\" or \"when\".")),
    ("כִּי", "for, because, that, when", None),
    ("עַד", "until, as far as", None),
    ("וְעַד", "and until", Some("וְ (and) + עַד")),
    ("אִם", "if", None),
    ("וְאִם", "and if", Some("וְ (and) + אִם")),
    ("אַל", "not, do not", Some("Negative used with commands and wishes.")),
    ("נָא", "please, now", Some("Particle of entreaty.")),
    ("אֵין", "there is not, without", None),
    ("וְאֵין", "and there is not", Some("וְ (and) + אֵין")),
    ("אַף", "also, even; anger, nose", None),
    ("גַּם", "also, even", None),
    ("רַק", "only, surely", None),
    ("לְמַעַן", "for the sake of, so that", None),
    ("הֲלֹא", "is it not?", Some("הֲ (question) + לֹא (not)")),
    ("יַעַן", "because", None),
    ("חִנָּם", "for nothing, in vain", None),
    ("פִּתְאֹם", "suddenly", None),
    ("לָמָּה", "why?", Some("לְ (to) + מָה (what)")),
    ("סֶלָה", "Selah", Some("A pause or musical rest, mostly in Psalms.")),
    ("שָׁמָּה", "there, to there", Some("שָׁם (there) + ־ָה (toward)")),
    ("לִקְרַאת", "to meet, toward", None),
    ("לָכֵן", "therefore", None),
    ("אַחֲרֵי", "after, behind", None),
    ("עַתָּה", "now", None),
    ("וְעַתָּה", "and now", Some("וְ (and) + עַתָּה (now)")),
    ("עוֹד", "still, yet, again", None),
    ("וְגַם", "and also", Some("וְ (and) + גַּם (also)")),
    ("בֵּין", "between", None),
    ("זֶה", "this (m.)", None),
    ("זֹאת", "this (f.)", None),
    ("הַזֹּאת", "this (f.)", Some("הַ (the) + זֹאת")),

    // Pronouns.
    ("אַתָּה", "you (m.)", None),
    ("אַתֶּם", "you (plural)", None),
    ("הוּא", "he, it", None),
    ("הַהוּא", "that (m.)", Some("הַ (the) + הוּא (he)")),
    ("הִיא", "she, it", None),
    ("הִוא", "she, it", Some("Ketiv spelling of הִיא (she), common in the Torah.")),
    ("הַהִיא", "that (f.)", Some("הַ (the) + הִיא (she)")),
    ("הֵם", "they", None),
    ("הֵמָּה", "they", None),
    ("אֲנַחְנוּ", "we", None),

    // לְ + suffixes.
    ("לוֹ", "to him, for him", Some("לְ (to) + ־וֹ (him)")),
    ("לִי", "to me, for me", Some("לְ (to) + ־ִי (me)")),
    ("לְךָ", "to you (m.)", Some("לְ (to) + ־ךָ (you)")),
    ("לָךְ", "to you (f.)", Some("לְ (to) + ־ךְ (you)")),
    ("לָהּ", "to her", Some("לְ (to) + ־ָהּ (her)")),
    ("לָהֶם", "to them", Some("לְ (to) + ־הֶם (them)")),
    ("לָכֶם", "to you (plural)", Some("לְ (to) + ־כֶם (you)")),
    ("לָנוּ", "to us", Some("לְ (to) + ־נוּ (us)")),

    // בְּ + suffixes.
    ("בּוֹ", "in him, in it", Some("בְּ (in) + ־וֹ")),
    ("בָּהּ", "in her, in it", Some("בְּ (in) + ־ָהּ")),
    ("בָּהֶם", "in them", Some("בְּ (in) + ־הֶם")),

    // אֶל / עַל / מִן + suffixes.
    ("אֶל", "to, toward", None),
    ("עַל", "on, over, against", None),
    ("וְעַל", "and on", Some("וְ (and) + עַל")),
    ("מֵעַל", "from upon, from over", Some("מֵ (from) + עַל (on)")),
    ("אֵלָיו", "to him", Some("אֶל (to) + ־ָיו (him)")),
    ("אֵלַי", "to me", Some("אֶל (to) + ־ַי (me)")),
    ("אֵלָי", "to me", Some("Pausal form of אֵלַי — אֶל (to) + ־ַי (me).")),
    ("אֵלֶיךָ", "to you", Some("אֶל (to) + ־ֶיךָ (you)")),
    ("אֲלֵיהֶם", "to them", Some("אֶל (to) + ־ֵיהֶם (them)")),
    ("עָלָיו", "on him, on it", Some("עַל (on) + ־ָיו")),
    ("עָלֶיהָ", "on her, on it", Some("עַל (on) + ־ֶיהָ")),
    ("עֲלֵיהֶם", "on them", Some("עַל (on) + ־ֵיהֶם")),
    ("עָלַי", "on me", Some("עַל (on) + ־ַי")),
    ("מִמֶּנּוּ", "from him, from it", Some("From מִן (from).")),
    ("מִמֶּנִּי", "from me", Some("מִן (from) + ־נִּי (me)")),
    ("עִמּוֹ", "with him", Some("עִם (with) + ־וֹ")),
    ("עִמָּדִי", "with me", Some("עִם (with) + ־ָדִי (me)")),
    ("לָמוֹ", "to them (poetic)", None),
    // The remaining frequent preposition/particle + suffix spellings the
    // lexicon bridge can't reach (no analysis row at all — they used to card
    // with a blank gloss).
    ("אֲלֵהֶם", "to them", Some("אֶל (to) + ־ֵהֶם (them), defective spelling")),
    ("אֵלֵינוּ", "to us", Some("אֶל (to) + ־ֵינוּ (us)")),
    ("אֵלַיִךְ", "to you (f.)", Some("אֶל (to) + ־ַיִךְ (you)")),
    ("אֲלֵיכֶם", "to you (plural)", Some("אֶל (to) + ־ֵיכֶם (you)")),
    ("עָלֶיךָ", "on you (m.)", Some("עַל (on) + ־ֶיךָ (you)")),
    ("עָלַיִךְ", "on you (f.)", Some("עַל (on) + ־ַיִךְ (you)")),
    ("עֲלֵיכֶם", "on you (plural)", Some("עַל (on) + ־ֵיכֶם (you)")),
    ("עָלֵינוּ", "on us", Some("עַל (on) + ־ֵינוּ (us)")),
    ("מֵעָלָיו", "from upon him", Some("מֵ (from) + עַל (on) + ־ָיו (him)")),
    ("מֵהֶם", "from them", Some("מִן (from) + ־הֶם (them)")),
    ("מִמְּךָ", "from you (m.)", Some("מִן (from) + ־ךָ (you)")),
    ("מִמֶּךָּ", "from you (m.)", Some("מִן (from) + ־ךָּ (you)")),
    ("מִמֵּךְ", "from you (f.)", Some("מִן (from) + ־ךְ (you)")),
    ("מִכֶּם", "from you (plural)", Some("מִן (from) + ־כֶם (you)")),
    ("עִמִּי", "with me", Some("עִם (with) + ־ִי (me)")),
    ("עִמְּךָ", "with you (m.)", Some("עִם (with) + ־ךָ (you)")),
    ("עִמָּךְ", "with you", Some("עִם (with) + ־ךְ (you)")),
    ("עִמָּנוּ", "with us", Some("עִם (with) + ־ָנוּ (us)")),
    ("עִמָּכֶם", "with you (plural)", Some("עִם (with) + ־ָכֶם (you)")),
    ("וְעִמּוֹ", "and with him", Some("וְ (and) + עִם (with) + ־וֹ (him)")),
    ("אִתּוֹ", "with him", Some("אֵת (with) + ־וֹ (him)")),
    ("אִתִּי", "with me", Some("אֵת (with) + ־ִי (me)")),
    ("אִתְּךָ", "with you (m.)", Some("אֵת (with) + ־ךָ (you)")),
    ("אִתָּךְ", "with you", Some("אֵת (with) + ־ךְ (you)")),
    ("אִתְּכֶם", "with you (plural)", Some("אֵת (with) + ־כֶם (you)")),
    ("אִתָּנוּ", "with us", Some("אֵת (with) + ־ָנוּ (us)")),
    ("בְּךָ", "in you (m.)", Some("בְּ (in) + ־ךָ (you)")),
    ("בָּךְ", "in you (f.)", Some("בְּ (in) + ־ךְ (you)")),
    ("בָּם", "in them", Some("בְּ (in) + ־ָם (them)")),
    ("בָּכֶם", "in you (plural)", Some("בְּ (in) + ־כֶם (you)")),
    ("תַּחְתָּיו", "under him; in his place", Some("תַּחַת (under) + ־ָיו (him)")),
    ("תַּחְתֶּיהָ", "under her; in its place", Some("תַּחַת (under) + ־ֶיהָ (her)")),
    ("אַחֲרָיו", "after him", Some("אַחֲרֵי (after) + ־ָיו (him)")),
    ("וְאַחֲרָיו", "and after him", Some("וְ (and) + אַחֲרֵי (after) + ־ָיו (him)")),
    ("אַחֲרֶיךָ", "after you (m.)", Some("אַחֲרֵי (after) + ־ֶיךָ (you)")),
    ("אַחֲרֵיהֶם", "after them", Some("אַחֲרֵי (after) + ־ֵיהֶם (them)")),
    ("כָּמוֹךָ", "like you", Some("כְּמוֹ (like) + ־ךָ (you)")),
    ("כָּמֹהוּ", "like him", Some("כְּמוֹ (like) + ־הוּ (him)")),
    ("אֹתְךָ", "you (m., object)", Some("אֵת (object marker) + ־ךָ (you)")),
    ("אֹתָךְ", "you (f., object)", Some("אֵת (object marker) + ־ךְ (you)")),
    ("אוֹתָךְ", "you (f., object)", Some("אֵת (object marker) + ־ךְ (you)")),
    ("אֹתָנוּ", "us (object)", Some("אֵת (object marker) + ־ָנוּ (us)")),
    ("הִנְנִי", "here I am; behold, I", Some("הִנֵּה (behold) + ־נִי (me)")),
    ("אֵינֶנּוּ", "he is not; it is gone", Some("אֵין (there is not) + ־נּוּ (he)")),
    ("עוֹדֶנּוּ", "he is still", Some("עוֹד (still) + ־נּוּ (he)")),

    // Frequent particle/pronoun compounds with no analysis row.
    ("הַכֹּל", "everything, the whole", Some("הַ (the) + כֹּל (all)")),
    ("מִכֹּל", "from all, more than all", Some("מִ (from) + כֹּל (all)")),
    ("הָהֵם", "those (m.)", Some("הָ (the) + הֵם (they)")),
    ("וְהֵם", "and they", Some("וְ (and) + הֵם (they)")),
    ("וּמַה", "and what?", Some("וּ (and) + מַה (what)")),
    ("וְכֹה", "and thus", Some("וְ (and) + כֹּה (thus)")),
    ("כָּזֹאת", "like this", Some("כָּ (like) + זֹאת (this)")),
    ("הֲיֵשׁ", "is there…?", Some("הֲ (question) + יֵשׁ (there is)")),
    ("לוֹא", "not", Some("Plene spelling of לֹא (not).")),
    ("וָמַעְלָה", "and upward", Some("וָ (and) + מַעְלָה (upward)")),
    ("מִלְמָעְלָה", "from above, on top", Some("מִ (from) + לְ (to) + מָעְלָה (above)")),
    ("הַלְלוּ", "praise!", Some("Imperative plural of הִלֵּל (praise) — as in הַלְלוּ־יָהּ.")),
    ("לִקְרָאתוֹ", "to meet him", Some("לִקְרַאת (to meet) + ־וֹ (him)")),
    ("בְּבֹאוֹ", "when he came", Some("בְּ (in) + בֹּא (coming) + ־וֹ (his)")),
    ("וְלַאֲשֶׁר", "and to those who", Some("וְ (and) + לְ (to) + אֲשֶׁר (who)")),

    // Suffixed nouns with no analysis row (the לֵב/לֵבָב family).
    ("לִבּוֹ", "his heart", Some("לֵב (heart) + ־וֹ (his)")),
    ("לִבָּם", "their heart", Some("לֵב (heart) + ־ָם (their)")),
    ("לִבְּךָ", "your heart", Some("לֵב (heart) + ־ךָ (your)")),
    ("בְּלִבּוֹ", "in his heart", Some("בְּ (in) + לֵב (heart) + ־וֹ (his)")),
    ("לְבָבוֹ", "his heart", Some("לֵבָב (heart) + ־וֹ (his)")),
    ("לְבָבְךָ", "your heart", Some("לֵבָב (heart) + ־ךָ (your)")),
    ("שְׁנֵיהֶם", "both of them", Some("שְׁנֵי (two of) + ־הֶם (them)")),

    // כֹּל family.
    ("כֹּל", "all, everything", None),
    ("כָּל", "all, every, the whole", Some("Construct form of כֹּל.")),
    ("וְכָל", "and all", Some("וְ (and) + כָּל")),
    ("בְּכָל", "in all, with all", Some("בְּ (in) + כָּל")),
    ("לְכָל", "to all", Some("לְ (to) + כָּל")),
    ("מִכָּל", "from all", Some("מִ (from) + כָּל")),

    // הָיָה (to be) forms the parser misses.
    ("וַיְהִי", "and it was, and it came to pass", Some("Narrative form of הָיָה (to be) — opens countless episodes.")),
    ("הָיוּ", "they were", Some("Perfect plural of הָיָה (to be).")),
    ("לֵאמֹר", "saying", Some("לְ (to) + אָמַר (say); introduces quoted speech.")),

    // Construct chains and suffixed nouns.
    ("בְּנֵי", "sons of", Some("Construct plural of בֵּן (son).")),
    ("וּבְנֵי", "and the sons of", Some("וּ (and) + בְּנֵי")),
    ("לִבְנֵי", "to the sons of", Some("לְ (to) + בְּנֵי")),
    ("בַּת", "daughter", None),
    ("בֵּית", "house of", Some("Construct of בַּיִת (house).")),
    ("לְבֵית", "to the house of", Some("לְ (to) + בֵּית")),
    ("הַבַּיִת", "the house", Some("הַ (the) + בַּיִת")),
    ("פָּנִים", "face, faces", Some("Plural in form, usually singular in meaning.")),
    ("פְּנֵי", "face of", Some("Construct of פָּנִים (face).")),
    ("מִפְּנֵי", "from before, because of", Some("מִ (from) + פְּנֵי")),
    ("לִפְנֵי", "before, in front of", Some("לְ (to) + פְּנֵי (face of)")),
    ("דִּבְרֵי", "words of", Some("Construct plural of דָּבָר (word).")),
    ("הַדָּבָר", "the word, the matter", Some("הַ (the) + דָּבָר")),
    ("דָּבָר", "word, thing, matter", None),
    ("אַנְשֵׁי", "men of", Some("Construct plural of אִישׁ (man).")),
    ("אָבִיו", "his father", Some("אָב (father) + ־ִיו (his)")),
    ("עַמִּי", "my people", Some("עַם (people) + ־ִי (my)")),
    ("נַפְשִׁי", "my soul, my life", Some("נֶפֶשׁ (soul) + ־ִי (my)")),
    ("בְּיַד", "in the hand of, by", Some("בְּ (in) + יַד (hand)")),
    ("יְמֵי", "days of", Some("Construct plural of יוֹם (day).")),
    ("שְׁנֵי", "two of", Some("Construct of שְׁנַיִם (two).")),
    ("שְׁתֵּים", "two (f.)", Some("Construct of שְׁתַּיִם (two).")),
    ("שָׁנָה", "year", None),
    ("יָמִים", "days", Some("Plural of יוֹם (day).")),
    ("מֵאוֹת", "hundreds", Some("Plural of מֵאָה (hundred).")),
    ("מֵאָה", "hundred", None),
    ("מַיִם", "water", None),
    ("מָיִם", "water", Some("Pausal form of מַיִם.")),
    ("הַמַּיִם", "the water", Some("הַ (the) + מַיִם")),
    ("הַמָּיִם", "the water", Some("הַ (the) + מַיִם, pausal form.")),
    ("רַבִּים", "many", Some("Plural of רַב (much, many).")),
    // The bridge resolves בְּרִית to BDB's n.pr entry (Baal-berith, the god
    // of Shechem) and carded the ordinary noun as "(a name)"; its suffixed
    // forms fell to a verb homograph instead (בְּרִיתוֹ "you ate him").
    ("בְּרִית", "covenant", None),
    ("בְרִית", "covenant", None),
    ("הַבְּרִית", "the covenant", Some("הַ (the) + בְּרִית")),
    ("בְּרִיתִי", "my covenant", Some("בְּרִית + ־ִי (my)")),
    ("בְרִיתִי", "my covenant", Some("בְּרִית + ־ִי (my)")),
    ("בְּרִיתוֹ", "his covenant", Some("בְּרִית + ־וֹ (his)")),
    ("בְרִיתוֹ", "his covenant", Some("בְּרִית + ־וֹ (his)")),
    ("בְּרִיתְךָ", "your covenant", Some("בְּרִית + ־ְךָ (your)")),
    ("בְרִיתְךָ", "your covenant", Some("בְּרִית + ־ְךָ (your)")),
    ("בְרִיתֶךָ", "your covenant", Some("בְּרִית + ־ֶךָ (your), pausal form.")),
    ("בְּרִיתֶךָ", "your covenant", Some("בְּרִית + ־ֶךָ (your), pausal form.")),
    ("בְּרִיתֵךְ", "your covenant", Some("בְּרִית + ־ֵךְ (your, f.)")),
    ("בְּרִיתְכֶם", "your covenant", Some("בְּרִית + ־ְכֶם (your, pl.)")),

    // אֱלֹהִים family.
    ("אֱלֹהִים", "God; gods", Some("Plural in form, usually singular in meaning when naming God.")),
    ("הָאֱלֹהִים", "the God, God", Some("הָ (the) + אֱלֹהִים")),
    ("אֱלֹהֵי", "God of", Some("Construct of אֱלֹהִים.")),
    ("אֱלֹהֶיךָ", "your God", Some("אֱלֹהִים + ־ֶיךָ (your)")),
    ("אֱלֹהֵינוּ", "our God", Some("אֱלֹהִים + ־ֵינוּ (our)")),
    ("אֵל", "God, god", None),

    // Article + common noun forms the parser misreads.
    ("מֶלֶךְ", "king", None),
    ("הַמֶּלֶךְ", "the king", Some("הַ (the) + מֶלֶךְ")),
    ("הָעָם", "the people", Some("הָ (the) + עַם")),
    ("הָעִיר", "the city", Some("הָ (the) + עִיר")),
    ("הַשָּׁמַיִם", "the heavens", Some("הַ (the) + שָׁמַיִם")),
    ("הַשָּׁמָיִם", "the heavens", Some("הַ (the) + שָׁמַיִם, pausal form.")),
    ("שָׁמַיִם", "heavens, sky", None),
    ("שָׁמָיִם", "heavens, sky", Some("Pausal form of שָׁמַיִם.")),
    ("הַכֹּהֲנִים", "the priests", Some("הַ (the) + plural of כֹּהֵן")),
    ("הַכֹּהֵן", "the priest", Some("הַ (the) + כֹּהֵן")),
    ("הַגּוֹיִם", "the nations", Some("הַ (the) + plural of גּוֹי")),
    ("צְבָאוֹת", "hosts, armies", Some("Plural of צָבָא; in יְהוָה צְבָאוֹת, “the LORD of hosts”.")),
    ("עוֹלָם", "forever; eternity, long ago", None),
    ("לַעֲשׂוֹת", "to do, to make", Some("לְ (to) + infinitive of עָשָׂה.")),
    ("אֶלֶף", "thousand", None),

    // Names the lexicon glosses oddly.
    ("יִשְׂרָאֵל", "Israel", None),
    ("שָׁאוּל", "Saul", Some("Means “asked (of God)”.")),
    ("אַבְשָׁלוֹם", "Absalom", Some("Means “my father is peace”.")),
    ("יוֹסֵף", "Joseph", Some("Means “he adds”.")),
    ("דָּוִיד", "David", Some("Later spelling of דָּוִד.")),
    ("יְהוֹשֻׁעַ", "Joshua", None),
    // Names BDB shelters under another headword — or whose consonant skeleton
    // collides with an ordinary lexeme, so the bridge serves the homograph's
    // sense (מֹשֶׁה carded "draw", שְׁלֹמֹה "garment", נֹחַ "rest").
    ("אַבְרָהָם", "Abraham", Some("Means “father of a multitude”; the patriarch, renamed from אַבְרָם.")),
    ("אַבְרָם", "Abram", Some("Means “exalted father”; the patriarch's earlier name.")),
    ("מֹשֶׁה", "Moses", Some("Perhaps “drawn out (of the water)”.")),
    ("דָּוִד", "David", Some("Means “beloved”.")),
    ("יַעֲקֹב", "Jacob", Some("Means “he grasps the heel”.")),
    ("שָׂרָה", "Sarah", Some("Means “princess”.")),
    ("שְׁלֹמֹה", "Solomon", Some("From שָׁלוֹם (peace).")),
    ("יְהוּדָה", "Judah", None),
    ("בִּנְיָמִן", "Benjamin", Some("Means “son of the right hand”.")),
    ("בִּנְיָמִין", "Benjamin", Some("Means “son of the right hand”.")),
    ("בָּבֶל", "Babylon", None),
    ("אֵלִיָּהוּ", "Elijah", Some("Means “my God is the LORD”.")),
    ("חִזְקִיָּהוּ", "Hezekiah", Some("Means “the LORD strengthens”.")),
    ("יִרְמְיָהוּ", "Jeremiah", None),
    ("יְחֶזְקֵאל", "Ezekiel", Some("Means “God strengthens”.")),
    ("יְשַׁעְיָהוּ", "Isaiah", Some("Means “the LORD is salvation”.")),
    ("נֹחַ", "Noah", Some("Means “rest”.")),
    ("רָחֵל", "Rachel", Some("Means “ewe”.")),
    ("לֵאָה", "Leah", None),
    ("רִבְקָה", "Rebekah", None),
    // Name homographs of ordinary words — glossed as both, not flagged as
    // names (the ordinary sense is the one to learn).
    ("אָדָם", "man, mankind; Adam", None),
    ("לָבָן", "white; Laban", None),
];

/// Curated surfaces that are proper names (see [`curated_name`]): the names of
/// [`CURATED`] plus famous names whose BDB gloss is already usable ("Esau",
/// "Jerusalem") but carries no `n.pr` marker for the automatic detection.
#[rustfmt::skip]
const CURATED_NAMES: &[&str] = &[
    "יִשְׂרָאֵל", "שָׁאוּל", "אַבְשָׁלוֹם", "יוֹסֵף", "דָּוִיד", "יְהוֹשֻׁעַ",
    "אַבְרָהָם", "אַבְרָם", "מֹשֶׁה", "דָּוִד", "יַעֲקֹב", "יִצְחָק", "שָׂרָה",
    "אַהֲרֹן", "שְׁלֹמֹה", "יְהוּדָה", "בִּנְיָמִן", "בִּנְיָמִין", "בָּבֶל",
    "אֵלִיָּהוּ", "חִזְקִיָּהוּ", "יִרְמְיָהוּ", "יְחֶזְקֵאל", "יְשַׁעְיָהוּ",
    "נֹחַ", "רָחֵל", "לֵאָה", "רִבְקָה", "עֵשָׂו", "אֶפְרַיִם", "יְרוּשָׁלִַם",
    "מִצְרַיִם", "פַּרְעֹה", "שְׁמוּאֵל", "שִׁמְשׁוֹן",
];

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
        // "the word" is registered under הַדָּבָר; a dagesh-stripped spelling
        // still resolves.
        assert!(curated_gloss("הַדָּבָר").is_some());
        assert_eq!(
            curated_gloss("אֶת").unwrap().gloss,
            "(marks the direct object)"
        );
        assert!(curated_gloss("כִּי").unwrap().note.is_none());
        // An ordinary content word is not curated.
        assert!(curated_gloss("בָּרָא").is_none());
    }
}
