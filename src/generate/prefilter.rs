//! Lexical pre-filter: recognise tokens that are *not* verbs so they can be
//! kept out of the verb parser entirely.
//!
//! The reverse parser is verb-only and works by generate-and-test, so it
//! happily emits spurious "imperative/infinitive of a fabricated root" analyses
//! for closed-class words (pronouns, prepositions, particles) and proper nouns.
//! Those readings dominate the ambiguity review even though the token is never
//! a verb. We can't derive part-of-speech from the parser, so we recognise these
//! forms up front from two references:
//!
//! - a curated list of **closed-class function words** (their headwords are
//!   stored mangled in Strong's, so a hand list is more reliable there);
//! - **proper nouns** from the lexicon (`english.pos` `n-pr*` / `np`).
//!
//! Matching is by **exact pointed form** (cantillation-stripped via
//! [`normalize_surface`]): we exclude the precise vocalised non-verb form, so a
//! genuine verb that merely shares the consonants (e.g. שָׁאַל "he asked" vs the
//! name שָׁאוּל) is left untouched. Leading proclitics are peeled before matching
//! so וְלֹא, בְּמִצְרַיִם resolve to לֹא, מִצְרַיִם.

use std::collections::HashSet;
use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::Connection;

use super::hebrew_db::normalize_surface;

/// Single-consonant proclitics (conjunction ו, prepositions ב/כ/ל/מ, article ה,
/// relative ש) — the same set the verb parser peels.
const PROCLITICS: [char; 7] = [
    '\u{05D5}', '\u{05D1}', '\u{05DB}', '\u{05DC}', '\u{05DE}', '\u{05D4}', '\u{05E9}',
];

/// Dagesh point — a forte here marks the doubling the definite article induces.
const DAGESH: char = '\u{05BC}';

/// Curated closed-class function words (exact pointed forms). Pronouns,
/// demonstratives, interrogatives/relative, independent prepositions, and the
/// common particles/negatives/adverbs. Deliberately omits forms that are also
/// common verbs (e.g. עוֹד) so the filter never costs verb recall.
const FUNCTION_WORDS: &[&str] = &[
    // personal pronouns
    "אֲנִי",
    "אָנֹכִי",
    "אַתָּה",
    "אַתְּ",
    "אַתֶּם",
    "אַתֶּן",
    "הוּא",
    "הִיא",
    "הֵם",
    "הֵמָּה",
    "הֵן",
    "הֵנָּה",
    "אֲנַחְנוּ",
    "נַחְנוּ",
    // demonstratives
    "זֶה",
    "זֹאת",
    "זוֹ",
    "אֵלֶּה",
    // interrogatives / relative
    "מִי",
    "מָה",
    "מַה",
    "מֶה",
    "אֲשֶׁר",
    // independent prepositions
    "אֶל",
    "עַל",
    "עַד",
    "אַחַר",
    "אַחֲרֵי",
    "בֵּין",
    "תַּחַת",
    "נֶגֶד",
    "לִפְנֵי",
    "עִם",
    "מִן",
    "אֵת",
    "אֶת",
    "יַעַן",
    "לְמַעַן",
    // particles / negatives / adverbs
    "לֹא",
    "אַל",
    "אִם",
    "כִּי",
    "גַּם",
    "אַף",
    "רַק",
    "אַךְ",
    "הִנֵּה",
    "נָא",
    "פֶּן",
    "שָׁם",
    "פֹּה",
    "כֹּה",
    "כֵּן",
    "עַתָּה",
    "אָז",
    "מְאֹד",
    "אֵין",
    "יֵשׁ",
    "אוֹ",
    "כֹּל",
    "כָּל",
    // inflected prepositions / object marker / particles carrying a pronominal
    // suffix — closed-class paradigms, never verbs, so always safe to exclude.
    // ל "to/for"
    "לִי",
    "לְךָ",
    "לָךְ",
    "לוֹ",
    "לָהּ",
    "לָנוּ",
    "לָכֶם",
    "לָכֶן",
    "לָהֶם",
    "לָהֶן",
    // ב "in/with"
    "בִּי",
    "בְּךָ",
    "בָּךְ",
    "בּוֹ",
    "בָּהּ",
    "בָּנוּ",
    "בָּכֶם",
    "בָּם",
    "בָּהֶם",
    "בָּהֶן",
    // עִם "with"
    "עִמִּי",
    "עִמְּךָ",
    "עִמָּךְ",
    "עִמּוֹ",
    "עִמָּהּ",
    "עִמָּנוּ",
    "עִמָּכֶם",
    "עִמָּהֶם",
    // אֵת object marker
    "אֹתִי",
    "אֹתְךָ",
    "אֹתָךְ",
    "אֹתוֹ",
    "אֹתָהּ",
    "אֹתָנוּ",
    "אֶתְכֶם",
    "אֶתְכֶן",
    "אֹתָם",
    "אֹתָן",
    "אֶתְהֶם",
    "אֶתְהֶן",
    // אֵת/אִתּ "with (accompaniment)"
    "אִתִּי",
    "אִתְּךָ",
    "אִתָּךְ",
    "אִתּוֹ",
    "אִתָּהּ",
    "אִתָּנוּ",
    "אִתְּכֶם",
    "אִתָּם",
    "אִתָּן",
    // אֶל "to/toward"
    "אֵלַי",
    "אֵלֶיךָ",
    "אֵלַיִךְ",
    "אֵלָיו",
    "אֵלֶיהָ",
    "אֵלֵינוּ",
    "אֲלֵיכֶם",
    "אֲלֵיכֶן",
    "אֲלֵיהֶם",
    "אֲלֵיהֶן",
    // עַל "upon/against"
    "עָלַי",
    "עָלֶיךָ",
    "עָלַיִךְ",
    "עָלָיו",
    "עָלֶיהָ",
    "עָלֵינוּ",
    "עֲלֵיכֶם",
    "עֲלֵיכֶן",
    "עֲלֵיהֶם",
    "עֲלֵיהֶן",
    // מִן "from"
    "מִמֶּנִּי",
    "מִמְּךָ",
    "מִמֵּךְ",
    "מִמֶּנּוּ",
    "מִמֶּנָּה",
    "מִכֶּם",
    "מִכֶּן",
    "מֵהֶם",
    "מֵהֶן",
    "מֵהֵמָּה",
    // הִנֵּה "behold" + suffix
    "הִנְנִי",
    "הִנֶּנִּי",
    "הִנּוֹ",
    "הִנָּם",
    // Pentateuchal ketiv of the 3fs pronoun (written הוא, read הִיא), plus its
    // demonstrative use after the article. Closed-class, never a verb.
    "הִוא",
    "הַהִוא",
    "הַהוּא",
    // pausal / variant 2ms pronoun
    "אָתָּה",
    // poetic & defective suffixed prepositions the citation set above misses:
    // אֶל "to" (pausal/defective), מִן "from" (poetic לָמוֹ), עִם "with",
    // אַחַר/אַחֲרֵי "after", תַּחַת "under", כְּ "like".
    "אֵלָי",
    "אֲלֵהֶם",
    "אֲלֵיהֶן",
    "לָמוֹ",
    "בָּמוֹ",
    "כְּמוֹ",
    "כָּמוֹנִי",
    "כָּמוֹךָ",
    "כָּמֹהוּ",
    "עִמָּדִי",
    "עִמָּדוֹ",
    "אַחֲרַי",
    "אַחֲרֶיךָ",
    "אַחֲרָיו",
    "אַחֲרֶיהָ",
    "אַחֲרֵינוּ",
    "אַחֲרֵיכֶם",
    "אַחֲרֵיהֶם",
    "תַּחְתַּי",
    "תַּחְתֶּיךָ",
    "תַּחְתָּיו",
    "תַּחְתֵּינוּ",
    "תַּחְתֵּיהֶם",
    "תַּחְתָּם",
    // high-frequency adverbs with no verb homograph in this exact pointing
    "סָבִיב",
    "יַחְדָּו",
    "יַחַד",
    "אֵיךְ",
    "אֵיכָה",
    "מַדּוּעַ",
    "לָכֵן",
    "אוּלַי",
    "טֶרֶם",
    "אָמֵן",
    "סֶלָה",
    // Aramaic relative/genitive particle דִּי "which/of" (also the noun דַּי
    // "sufficiency"); closed-class, never a verb.
    "דִּי",
    // Dagesh-less surface variants of closed-class forms. The Masoretic DB
    // surfaces (and proclitic-peeled remainders like the בֵין of וּבֵין) often
    // drop the dagesh that the citation spelling carries, so list them so they
    // classify as function words too. אָנִי is the qamats-pointed variant of אֲנִי.
    "בוֹ",
    "בֵין",
    "אָנִי",
    // Dagesh-less surface variants of suffixed prepositions (the begedkefet drops
    // its lene after certain preceding words): בִי/בְךָ/בָהֶם/בָכֶם beside the
    // dotted בִּי/בְּךָ/בָּהֶם/בָּכֶם already listed. Plus further closed-class
    // preposition/particle + suffix paradigms attested in the text.
    "בִי",
    "בְךָ",
    "בָהֶם",
    "בָכֶם",
    "עָלָי",
    "עָלָיִךְ",
    "מֵעָלָי",
    "אַחֲרָי",
    "עֲלֵהֶם",
    "עָלֵימוֹ",
    "עִמָּם",
    "מִמֶּךָּ",
    "תַּחְתֶּיהָ",
    "נֶגְדּוֹ",
    "לְנֶגְדִּי",
    "בַּעֲדוֹ",
    "כָמוֹךָ",
    "אוֹתָךְ",
    // אֵין/אַיִן "there is not" + suffix; הִנֵּה "behold" + suffix.
    "אֵינֶנּוּ",
    "אֵינְךָ",
    "הִנֵּנִי",
    "אֵינֶנִּי",
    "אֵינָם",
    "אֵינֶנָּה",
    "אֵינְכֶם",
    "הִנְּךָ",
    // closed-class preposition/particle + suffix (defective & variant spellings)
    "כָּכֶם",
    "אַחֲרֵיהֶן",
    "אֲלֵכֶם",
    "נֶגְדֶּךָ",
    "כָמֹהוּ",
    "כָמוֹנִי",
    "אֵלָיִךְ",
    "תַּחְתָּי",
    "נֶגְדִּי",
    "בֵּינֵינוּ",
    "תַחְתָּיו",
    "אֶצְלוֹ",
    "לְנֶגְדָּם",
    "אֶצְלִי",
    "עֲלֵהֶן",
    "תַחְתָּם",
    "לְמַעַנְכֶם",
    "יֶשְׁנוֹ",
    "בֵּינֵיהֶם",
    // High-frequency adverbs/particles missing their dagesh-less or variant
    // surface: כֹה (כֹּה without lene), חִנָּם "freely", מַעְלָה "upward"
    // (covers וָמַעְלָה / לְמַעְלָה after proclitic-peeling).
    "כֹה",
    "חִנָּם",
    "מַעְלָה",
    // Frozen prepositions — lexicalised bound infinitives/nominals that never
    // function as live verbs: לִקְרַאת "to meet" (+suffix), בַּעֲבוּר "for the
    // sake of", and the adverbial הַרְבֵּה "much/many".
    "לִקְרַאת",
    "לִקְרָאתוֹ",
    "בַּעֲבוּר",
    "הַרְבֵּה",
    // Number + pronominal suffix — closed-class, never a verb: שְׁנֵיהֶם "the
    // two of them".
    "שְׁנֵיהֶם",
    // Aramaic closed-class particles: דְּנָה "this", אֱדַיִן/בֵּאדַיִן "then",
    // דִי "which/of" (dagesh-less variant of the דִּי already listed).
    "דְּנָה",
    "אֱדַיִן",
    "בֵּאדַיִן",
    "דִי",
    "קֳבֵל",
    "קֳדָם",
    // Adverb פֹה "here" — the dagesh-less variant of the פֹּה already listed.
    "פֹה",
    // Frozen adverbs/interjections — closed-class, never live verbs:
    // מִלְמָעְלָה "from above", פִּתְאֹם "suddenly", אָכֵן "surely/indeed",
    // חָלִילָה "far be it".
    "מִלְמָעְלָה",
    "פִּתְאֹם",
    "אָכֵן",
    "חָלִילָה",
    // Optative particle לוּ / לֻא "if only, would that" (Strong 3863) — closed-
    // class, never a verb. (לֹא the negative is already listed above.)
    "לוּ",
    "לֻא",
    // Aramaic closed-class preposition + pronominal suffix לֵהּ "to him", and the
    // Aramaic 3fs/3mp/2mp variants; never a Hebrew verb.
    "לֵהּ",
    "לַהּ",
    "לְהוֹן",
    "לְכוֹן",
    "לָנָא",
    // Poetic/relative particles and interjections, plus a pronoun spelling
    // variant — all closed-class, never verbs: זוּ "which/this" (relative,
    // Strong 2098), אֲהָהּ "alas", רֵיקָם "in vain/empty-handed" (frozen adverb),
    // אֲנָחְנוּ "we" (defective spelling of אֲנַחְנוּ already listed).
    "זוּ",
    "אֲהָהּ",
    "רֵיקָם",
    "אֲנָחְנוּ",
];

/// Closed-class surfaces matched by **exact** pointed form — no proclitic
/// peeling. Harvested from gold (every attested reading is a
/// particle/pronoun/preposition/adverb/conjunction + optional suffix — never a
/// verb/noun/adjective), these are the inflected-function-word tail of
/// review_missing (עוֹדֶנּוּ, בֵּינוֹ, אֵיפֹה, אֲחֹרַנִּית …). They are matched
/// exactly, *not* via [`deprefixed_forms`], because many are short suffixed
/// forms (לָה, אֱהִי) that a real verb could peel down to — e.g. מָשְׁלָה loses
/// mem+shin to "לָה" — and peel-matching them would wrongly silence that verb.
/// Exact match is collision-free: the listed surface is never itself a verb.
const FUNCTION_WORDS_EXACT: &[&str] = &[
    "אֱהִי",
    "אֱלֵי",
    "אֲבוֹי",
    "אֲחֹרַנִּית",
    "אֲל",
    "אֲלֵהֶן",
    "אִשׁ",
    "אֵיכָכָה",
    "אֵיכֹה",
    "אֵינֵמוֹ",
    "אֵיפֹה",
    "אֵלֵימוֹ",
    "אֵלָו",
    "אֵפוֹ",
    "אֵפוֹא",
    "אֶמֶשׁ",
    "אֶצְלָהּ",
    "אֶצְלָם",
    "אַחֲלֵי",
    "אַחֲלַי",
    "אַחֲרַיִךְ",
    "אַחַרֶיךָ",
    "אַיֶּכָּה",
    "אַלְלַי",
    "אַתֵּנָה",
    "אָיִן",
    "אָמֶשׁ",
    "אָנֶה",
    "אָנָּא",
    "אָנָּה",
    "אָתְּ",
    "אָתּ",
    "אֹתְכָה",
    "אֹתָכָה",
    "אֹתָנָה",
    "אוֹתְהֶם",
    "אוֹתְהֶן",
    "אוֹתָנָה",
    "אוֹתָנוּ",
    "בְּמוֹ",
    "בְּעוֹדֶנִּי",
    "בְּעוֹדֶנּוּ",
    "בְּפִתְאֹם",
    "בְמוֹ",
    "בִּלְעָדָי",
    "בִּלְתֶּךָ",
    "בֵּינְךָ",
    "בֵּינֵיכֶם",
    "בֵּינֵכֶם",
    "בֵּינֵנוּ",
    "בֵּינֹתֵינוּ",
    "בֵּינֹתָם",
    "בֵּינוֹ",
    "בֵּינוֹת",
    "בֵּינוֹתֵינוּ",
    "בֵינֵיהֶם",
    "בֵינֵינוּ",
    "בַּעֲדֵינוּ",
    "בַּעֲדֵךְ",
    "בַּעֲדֶךָ",
    "בַּעֲדָהּ",
    "בַעֲדָהּ",
    "בַעֲדוֹ",
    "הֲ",
    "הֲמִבַּלְעֲדֵי",
    "הִנֶּה",
    "הִנֶּךָּ",
    "הִנֶּנּוּ",
    "הַאִשׁ",
    "הַעוֹדֶנּוּ",
    "הָהּ",
    "הוֹ",
    "וְאִילוֹ",
    "וְאֵיכָכָה",
    "וְאֵינֵךְ",
    "וְאֵינֵמוֹ",
    "וְאֵיפֹה",
    "וְאֶתְמוּל",
    "וְאַחֲרַיִךְ",
    "וְאַתֵּנָה",
    "וְאוֹתָנוּ",
    "וְנֶגְדָּם",
    "וְעָדֵיכֶם",
    "וְעָדֶיךָ",
    "וְעֹדֶנּוּ",
    "וְעוֹדֶנּוּ",
    "וְתַחְתֶּיהָ",
    "וְתַחְתַּי",
    "וָאָיִן",
    "וּבִלְעָדֶיךָ",
    "וּבֵינְךָ",
    "וּבֵינֵיהֶם",
    "וּבֵינֵיכֶם",
    "וּבֵינֵךְ",
    "וּבֵינֶיךָ",
    "וּבֵינֶךָ",
    "וּבַעֲבוּר",
    "וּכְמוֹ",
    "וּלְמָטָּה",
    "וּמִבַּלְעָדַי",
    "וּמִלְמַעְלָה",
    "וּפִתְאֹם",
    "זֹּה",
    "זוּלָתֶךָ",
    "חָלִלָה",
    "יֶּשׁ",
    "יֶשְׁךָ",
    "יֶשְׁכֶם",
    "יַחְדָּיו",
    "יָּחַד",
    "כְמוֹ",
    "כָּמֹנוּ",
    "כָּמוֹהָ",
    "כָמֹנוּ",
    "כָמוֹהָ",
    "כָמוֹנוּ",
    "לְאָיִן",
    "לְבַעֲבוּר",
    "לְמָטָּה",
    "לְמוֹ",
    "לְמוֹאל",
    "לְנֶגְדְּכֶם",
    "לִפְנִימָה",
    "לַפַּלְמוֹנִי",
    "לָה",
    "לָכֶנָה",
    "לא",
    "מִבֵּינוֹת",
    "מִבַּלְעֲדֵי",
    "מִבַּלְעָדַי",
    "מִלְּמָטָּה",
    "מִמֶּךָ",
    "מִנְהֶם",
    "מִפְּנִימָה",
    "מֵאֶצְלָם",
    "מֵאַחֲרָיִךְ",
    "מֵהֵנָה",
    "מָּטָּה",
    "מָּעְלָה",
    "מָטָּה",
    "נִכְחוֹ",
    "נֶגְדְּךָ",
    "נֶגְדָּהּ",
    "נֶגְדָּם",
    "נֶגְדָה",
    "נָחְנוּ",
    "עֲדֶן",
    "עֲדֶנָה",
    "עֲלֵכֶם",
    "עֳלֶיהָ",
    "עָדֶיהָ",
    "עָדֶיךָ",
    "עָדָיו",
    "עָלָיְכִי",
    "עָתָּה",
    "עֹדֶנּוּ",
    "עוֹדֶנִּי",
    "עוֹדֶנָּה",
    "עוֹדֶנּוּ",
    "עוֹדָךְ",
    "פְּלֹנִי",
    "פְּנִימָה",
    "פְנִימָה",
    "פִּתְאוֹם",
    "פִתְאֹם",
    "קְדֹרַנִּית",
    "תַּחְתֵּיכֶם",
    "תַּחְתֵּנִי",
    "תַּחְתֶּנָּה",
    "תַחְתֵּיהֶם",
    "תַחְתֵּיהֶן",
    "תַחְתֵּיכֶם",
    "תַחְתֵּינוּ",
    "תַחְתֶּיהָ",
    "תַחְתֶּיךָ",
    "תַחְתָּי",
];

/// The Tetragrammaton and its surface variants. The lexicon carries the divine
/// name with a holem on the he (יְהֹוָה, Strong's 3068/3069), but the Masoretic
/// text writes the Qere-perpetuum pointing without it (יְהוָה / יְהוִה / יֱהוִה),
/// so it never matches the lexicon's proper-noun inventory. We add the attested
/// surface forms directly. The proclitic-peeled remainders (יהוָה / יהוִה, with no
/// shewa under the yod) let prefixed forms — לַיהוָה, בַּיהוָה, וַיהוָה — resolve too.
const DIVINE_NAMES: &[&str] = &["יְהוָה", "יְהוִה", "יֱהוִה", "יהוָה", "יהוִה"];

/// Recognises non-verb tokens by exact pointed form.
pub struct Prefilter {
    function: HashSet<String>,
    /// Exact-match (no proclitic peeling) closed-class forms — see
    /// [`FUNCTION_WORDS_EXACT`].
    function_exact: HashSet<String>,
    proper: HashSet<String>,
}

impl Prefilter {
    /// Build from the curated function words plus the lexicon's proper nouns.
    pub fn load(lexicon_db: &Path) -> Result<Self> {
        let function: HashSet<String> = FUNCTION_WORDS
            .iter()
            .map(|s| normalize_surface(s))
            .filter(|s| !s.is_empty())
            .collect();
        let function_exact: HashSet<String> = FUNCTION_WORDS_EXACT
            .iter()
            .map(|s| normalize_surface(s))
            .filter(|s| !s.is_empty())
            .collect();

        let db = Connection::open(lexicon_db)
            .with_context(|| format!("opening {}", lexicon_db.display()))?;
        let mut stmt =
            db.prepare("SELECT word FROM english WHERE pos LIKE 'n-pr%' OR pos = 'np'")?;
        let mut proper = HashSet::new();
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        for w in rows {
            let n = normalize_surface(&w?);
            if !n.is_empty() {
                proper.insert(n);
            }
        }
        // The divine name is absent from the lexicon's pointing (see DIVINE_NAMES);
        // add its attested surface forms so it is recognised as a proper noun.
        proper.extend(
            DIVINE_NAMES
                .iter()
                .map(|s| normalize_surface(s))
                .filter(|s| !s.is_empty()),
        );
        // Many names never match the lexicon by exact pointing (plene/defective,
        // pausal, or simply absent — see [`super::proper_names`]); add the
        // gold-harvested attested surface forms directly.
        proper.extend(
            super::proper_names::PROPER_NAMES
                .iter()
                .map(|s| normalize_surface(s))
                .filter(|s| !s.is_empty()),
        );
        Ok(Self {
            function,
            function_exact,
            proper,
        })
    }

    /// Classify a cantillation-normalised surface. Returns `"function"` or
    /// `"proper"` if the form — or a de-prefixed remainder — is a known
    /// non-verb, else `None` (parse it as a verb).
    pub fn classify(&self, surface: &str) -> Option<&'static str> {
        // Exact-match closed-class forms first (no proclitic peeling, so no
        // collision with a verb that could peel down to a short suffixed form).
        if self.function_exact.contains(surface) {
            return Some("function");
        }
        for form in deprefixed_forms(surface) {
            if self.function.contains(&form) {
                return Some("function");
            }
            if self.proper.contains(&form) {
                return Some("proper");
            }
        }
        None
    }

    /// Decide whether to exclude a token from verb parsing, given whether the
    /// parser found a plausible verb reading for it.
    ///
    /// Function words are always excluded — their headwords are not verbs.
    /// A proper-noun match, however, *yields* to the verb parser when the token
    /// also has a plausible verb reading: many names are homographs of genuine
    /// verb forms (e.g. שָׁאַל "he asked" vs שָׁאוּל), and excluding those costs
    /// recall. Names with no verb reading stay excluded.
    pub fn exclude(&self, surface: &str, has_plausible_verb: bool) -> Option<&'static str> {
        match self.classify(surface) {
            Some("proper") if has_plausible_verb => None,
            other => other,
        }
    }
}

/// Split a pointed string into clusters of `base letter + following points`.
fn clusters(s: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for c in s.chars() {
        let is_base = (0x05D0..=0x05EA).contains(&(c as u32));
        if is_base || out.is_empty() {
            out.push(c.to_string());
        } else {
            out.last_mut().unwrap().push(c);
        }
    }
    out
}

/// Remove a dagesh (forte) from the first cluster of `form`, returning the
/// normalised remainder if one was present. When the definite article attaches —
/// either written (הַ) or assimilated into a preposition (בַּ = בְּ+הַ) — it doubles
/// the following consonant with a dagesh forte that the bare lexical form lacks.
/// Stripping it lets a peeled remainder match its citation form (הַזֶּה→זֶה,
/// בַּיּוֹם→יוֹם). Operates on a [`normalize_surface`]-ordered string, where the
/// dagesh sorts after the vowel within the cluster.
fn strip_initial_dagesh(form: &str) -> Option<String> {
    let cl = clusters(form);
    let first = cl.first()?;
    if !first.contains(DAGESH) {
        return None;
    }
    let stripped: String = first.chars().filter(|&c| c != DAGESH).collect();
    Some(
        std::iter::once(stripped)
            .chain(cl[1..].iter().cloned())
            .collect(),
    )
}

/// The surface itself plus every remainder after peeling 1–2 leading proclitic
/// clusters (so prefixed function words/names still match their base form). For
/// each peeled remainder, the article-doubled variant is also offered with its
/// dagesh forte stripped (see [`strip_initial_dagesh`]).
fn deprefixed_forms(surface: &str) -> Vec<String> {
    let cl = clusters(surface);
    let mut forms = vec![surface.to_string()];
    // A function word can pick up a conjunctive dagesh forte on its first
    // consonant from the preceding word (dehiq / atthat mer'ahevin): לָּךְ, נָּא,
    // בִי, בְךָ all carry a forte the citation form lacks. Offer the bare surface
    // with that initial dagesh stripped so it still matches its headword.
    if let Some(bare) = strip_initial_dagesh(surface) {
        forms.push(bare);
    }
    let max = 2.min(cl.len().saturating_sub(1));
    for k in 1..=max {
        let all_proclitic = cl[..k]
            .iter()
            .all(|c| c.chars().next().is_some_and(|b| PROCLITICS.contains(&b)));
        if all_proclitic {
            let rem: String = cl[k..].concat();
            if let Some(bare) = strip_initial_dagesh(&rem) {
                forms.push(bare);
            }
            forms.push(rem);
        }
    }
    forms
}

#[cfg(test)]
mod tests {
    use super::*;

    fn func_only() -> Prefilter {
        Prefilter {
            function: FUNCTION_WORDS
                .iter()
                .map(|s| normalize_surface(s))
                .collect(),
            function_exact: FUNCTION_WORDS_EXACT
                .iter()
                .map(|s| normalize_surface(s))
                .collect(),
            proper: HashSet::new(),
        }
    }

    #[test]
    fn matches_bare_function_word() {
        let pf = func_only();
        assert_eq!(pf.classify(&normalize_surface("לֹא")), Some("function"));
        assert_eq!(pf.classify(&normalize_surface("הוּא")), Some("function"));
    }

    #[test]
    fn matches_prefixed_function_word() {
        let pf = func_only();
        // וְלֹא = conjunction ו + לֹא
        assert_eq!(pf.classify(&normalize_surface("וְלֹא")), Some("function"));
    }

    #[test]
    fn ignores_ordinary_verb() {
        let pf = func_only();
        assert_eq!(pf.classify(&normalize_surface("שָׁמַר")), None);
    }

    #[test]
    fn matches_suffixed_preposition() {
        let pf = func_only();
        // closed-class preposition + pronominal suffix — never a verb.
        assert_eq!(pf.classify(&normalize_surface("לוֹ")), Some("function"));
        assert_eq!(pf.classify(&normalize_surface("עָלָיו")), Some("function"));
        assert_eq!(pf.classify(&normalize_surface("אֹתוֹ")), Some("function"));
        assert_eq!(pf.classify(&normalize_surface("מִמֶּנּוּ")), Some("function"));
    }

    #[test]
    fn matches_divine_name_and_prefixed() {
        let pf = Prefilter {
            function: HashSet::new(),
            function_exact: HashSet::new(),
            proper: DIVINE_NAMES.iter().map(|s| normalize_surface(s)).collect(),
        };
        assert_eq!(pf.classify(&normalize_surface("יְהוָה")), Some("proper"));
        // לַיהוָה = proclitic לַ + the peeled remainder יהוָה (no shewa on yod).
        assert_eq!(pf.classify(&normalize_surface("לַיהוָה")), Some("proper"));
    }

    #[test]
    fn exact_function_word_matches_only_exact_form() {
        let pf = func_only();
        // עוֹדֶנּוּ (עוֹד + suffix) is in the exact set — matched verbatim.
        assert_eq!(pf.classify(&normalize_surface("עוֹדֶנּוּ")), Some("function"));
        // But "לָה" (exact-only) must NOT be reached by peeling a real verb:
        // מָשְׁלָה "she ruled" can peel mem+shin down to לָה, yet stays a verb.
        assert_eq!(pf.classify(&normalize_surface("מָשְׁלָה")), None);
        // וָאֱהִי "and I was" (היה wayyiqtol) must not match exact-only "אֱהִי".
        assert_eq!(pf.classify(&normalize_surface("וָאֱהִי")), None);
    }

    #[test]
    fn matches_article_doubled_function_word() {
        let pf = func_only();
        // הַזֶּה = article הַ + זֶה, with the article doubling the zayin (dagesh
        // forte) that the bare demonstrative lacks.
        assert_eq!(pf.classify(&normalize_surface("הַזֶּה")), Some("function"));
        // הַזֹּאת = article + זֹאת.
        assert_eq!(pf.classify(&normalize_surface("הַזֹּאת")), Some("function"));
    }
}
