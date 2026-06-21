//! Curated irregular-verb surfaces harvested from the OSHB/morphhb gold tagging
//! (CC BY 4.0, Strong-keyed), restricted to the verb **stems the algorithmic
//! generator does not model** — Polel, Polal, Hithpolel, Poel, Pilpel,
//! Hithpalpel, Hishtaphel and the other rare/geminate-base binyanim
//! ([`super::verb::Binyan`] only covers the seven productive stems). These
//! forms cannot be produced by reversing a triliteral paradigm, so — exactly as
//! [`super::irregular_noun`] does for suppletive nouns — we list every attested
//! surface and match it exactly. Matching is gold-precise, so these only ever
//! add the correct reading; they never displace a generated analysis.
//!
//! Each entry is `(surface, root, binyan, form, pgn)` where `surface` is the
//! cantillation-normalised full token (proclitics included, as it appears in
//! the text) and `binyan`/`form`/`pgn` are rendered the same way the generator
//! labels its own analyses, so downstream consumers treat them uniformly.

use std::collections::HashMap;

/// One harvested irregular-verb reading. A surface may have several (homographs
/// / form ambiguity), so the lookup keys a surface to a list of these.
#[derive(Debug, Clone, Copy)]
pub struct IrregularVerb {
    pub surface: &'static str,
    pub root: &'static str,
    pub binyan: &'static str,
    pub form: &'static str,
    pub pgn: &'static str,
}

/// Build a surface → readings lookup over [`IRREGULAR_VERBS`] and
/// [`IRREGULAR_VOCALIZATIONS`].
pub fn lookup() -> HashMap<&'static str, Vec<&'static IrregularVerb>> {
    let mut m: HashMap<&'static str, Vec<&'static IrregularVerb>> = HashMap::new();
    for v in IRREGULAR_VERBS.iter().chain(IRREGULAR_VOCALIZATIONS) {
        m.entry(v.surface).or_default().push(v);
    }
    m
}

/// Attested forms of the *modeled* stems (Qal/Niphal/Piel/…) whose vocalization
/// the algorithmic generator does not produce — hapax / anomalous / doubly-weak
/// spellings (euphonic dagesh חָדֵלּוּ, the doubly-weak צוה contraction לְצַוּת,
/// the segol שְׁאֶלְתֶּם, etc.). Like [`IRREGULAR_VERBS`] these are gold-precise
/// full-surface matches that only ever add the correct reading; they exist for
/// the downstream product (the accuracy harness measures the generator and
/// excludes these gizra='Irregular' rows, so they never inflate it).
pub const IRREGULAR_VOCALIZATIONS: &[IrregularVerb] = &[
    IrregularVerb {
        // Euphonic dagesh forte in C3 (Judg 5:6-7); generator builds חָדֵלוּ.
        surface: "חָדֵלּוּ",
        root: "חדל",
        binyan: "Qal",
        form: "Perfect",
        pgn: "3cp",
    },
    IrregularVerb {
        // I-guttural Qal cohortative 1cs with the "loud" a-grade — patah prefix
        // and a full patah on the guttural C1 (ʾahargâ) where the generator
        // builds the segol/hataf grades (אֶהֶרְגָה). Gen 27:41 (Esau, "and I
        // will slay my brother Jacob"). The bible.db token carries the וְ.
        surface: "וְאַהַרְגָה",
        root: "הרג",
        binyan: "Qal",
        form: "Cohortative",
        pgn: "1cs",
    },
    IrregularVerb {
        // לקח (irregular pe-lamed) Qal imperative 2ms + 3mp suffix — qāḥem
        // (קָחֶם, Gen 48:9 "take them"); the generator builds the imperfect
        // hosts but not the bare imperative qaḥ + object suffix.
        surface: "קָחֶם",
        root: "לקח",
        binyan: "Qal",
        form: "Imperative",
        pgn: "2ms",
    },
    IrregularVerb {
        // צוה (doubly-weak) Pual perfect 2ms, tsere theme + paragogic he —
        // ṣuwwêṯâ (צֻוֵּיתָה, Gen 45:19 "you are commanded") — the generator's
        // tsere-theme alternant (צֻוֵּיתָ) doesn't compose with its paragogic-he
        // twin, so only צֻוֵּיתָ and צֻוִּיתָה are built.
        surface: "צֻוֵּיתָה",
        root: "צוה",
        binyan: "Pual",
        form: "Perfect",
        pgn: "2ms",
    },
    IrregularVerb {
        // III-aleph Qal perfect (weqatal) 3ms + 3ms suffix with the -āhû link —
        // qᵊrāʾāhû (וּקְרָאָהוּ, קרא "befall", Gen 42:38 "and harm befalls him")
        // — where the generator builds only the -ô 3ms suffix (קְרָאוֹ).
        surface: "וּקְרָאָהוּ",
        root: "קרא",
        binyan: "Qal",
        form: "Perfect",
        pgn: "3ms",
    },
    IrregularVerb {
        // III-He Piel perfect 3ms + 1cs suffix of נשה "forget" — naššanî
        // (נַשַּׁנִי, Gen 41:51, the Manasseh etymology "God has made me forget")
        // — a III-He derived-perfect + object suffix the generator doesn't build.
        surface: "נַשַּׁנִי",
        root: "נשה",
        binyan: "Piel",
        form: "Perfect",
        pgn: "3ms",
    },
    IrregularVerb {
        // I-guttural Qal wayyiqtol with a SILENT sheva closing the prefix
        // syllable, where apply_guttural writes the vocal hataf-patah —
        // wayyaʿḇōr (וַיַּעְבֹר, עבר, Gen 41:46 "and he passed through") for
        // וַיַּעֲבֹר.
        surface: "וַיַּעְבֹר",
        root: "עבר",
        binyan: "Qal",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        // Same silent-guttural wayyiqtol, wattaʿgaḇ (וַתַּעְגַּב, עגב "lust",
        // Ezek 23:5) for וַתַּעֲגַּב.
        surface: "וַתַּעְגַּב",
        root: "עגב",
        binyan: "Qal",
        form: "Wayyiqtol",
        pgn: "3fs",
    },
    IrregularVerb {
        // נתן Qal infinitive construct, un-assimilated qamats-theme variant
        // nᵊṯān (נְתָן, Gen 38:9 / Num 20:21 "to give seed") — beside the
        // generator's נְתֹן (holam) and assimilated תֵּת.
        surface: "נְתָן",
        root: "נתן",
        binyan: "Qal",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        // Geminate חנן "be gracious" Qal perfect 3ms + 1cs suffix, contracted —
        // the two nuns collapse to one dageshed radical: ḥannanî (חַנַּנִי,
        // Gen 33:11 "God has been gracious to me") — where the generator spells
        // the uncontracted חֲנָנַנִי.
        surface: "חַנַּנִי",
        root: "חנן",
        binyan: "Qal",
        form: "Perfect",
        pgn: "3ms",
    },
    IrregularVerb {
        // גנב Qal passive participle fs construct with the archaic
        // hireq-compaginis -î ending — gᵊnuḇtî (גְּנֻבְתִי / וּגְנֻבְתִי, Gen
        // 31:39 "stolen of day ... stolen of night") — where the generator
        // builds the ordinary -aṯ construct (גְּנֻבַת).
        surface: "גְּנֻבְתִי",
        root: "גנב",
        binyan: "Qal",
        form: "Participle (pas.)",
        pgn: "fs",
    },
    IrregularVerb {
        surface: "וּגְנֻבְתִי",
        root: "גנב",
        binyan: "Qal",
        form: "Participle (pas.)",
        pgn: "fs",
    },
    IrregularVerb {
        // עשה Qal infinitive absolute with the guttural C1 reduced to a
        // hataf-patah — ʿăśô (עֲשׂוֹ, Gen 31:28 "you have done foolishly in
        // doing") — where the generator (and WLC) build the full qamats grade
        // (עָשׂוֹ).
        surface: "עֲשׂוֹ",
        root: "עשה",
        binyan: "Qal",
        form: "Inf. Absolute",
        pgn: "",
    },
    IrregularVerb {
        // חבא "hide" (I-guttural + III-aleph) Niphal perfect 2ms with the
        // a-grade silent-sheva guttural — naḥbēʾtā (נַחְבֵּאתָ, 1 Sam 19:2,
        // weqatal "and hide yourself") — where the generator builds the e-grade
        // hataf prefix (נֶחֱבֵאתָ).
        surface: "נַחְבֵּאתָ",
        root: "חבא",
        binyan: "Niphal",
        form: "Perfect",
        pgn: "2ms",
    },
    IrregularVerb {
        // Pe-yod יחם "be in heat/conceive" (Jacob's flocks, Gen 30): the Qal
        // wayyiqtol 3fp takes the archaic YOD prefix (וַיֵּחַמְנָה) where the
        // generator builds the standard tav-prefix (וַתֵּחַמְנָה).
        surface: "וַיֵּחַמְנָה",
        root: "יחם",
        binyan: "Qal",
        form: "Wayyiqtol",
        pgn: "3fp",
    },
    IrregularVerb {
        // יחם Piel infinitive construct + 3fs energic suffix לְיַחְמֵנָּה
        // ("to conceive [at the sight of them]", Gen 30:41) — pe-yod Piel
        // inf+suffix the generator doesn't build.
        surface: "לְיַחְמֵנָּה",
        root: "יחם",
        binyan: "Piel",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        // מול "circumcise" inflects its Niphal geminate-style — a doubled C1
        // mem with holam after the hiriq prefix (nimmōl) — where the hollow
        // generator builds נָמֹל. Perfect 3cp נִמֹּלוּ (Gen 17:27).
        surface: "נִמֹּלוּ",
        root: "מול",
        binyan: "Niphal",
        form: "Perfect",
        pgn: "3cp",
    },
    IrregularVerb {
        // Same nimmōl pattern, Niphal participle mp נִמֹּלִים (Josh 5:8).
        surface: "נִמֹּלִים",
        root: "מול",
        binyan: "Niphal",
        form: "Participle (act.)",
        pgn: "mp",
    },
    IrregularVerb {
        // Geminate המם "rout" Qal wayyiqtol + 3mp suffix with anomalous u-theme
        // host yəhummēm (וַיְהֻמֵּם, Exod 14:24 etc.); generator reads it as Pual.
        surface: "וַיְהֻמֵּם",
        root: "הממ",
        binyan: "Qal",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        // C2-aleph segol theme before the heavy afformative; generator: שְׁאַלְתֶּם.
        surface: "שְׁאֶלְתֶּם",
        root: "שאל",
        binyan: "Qal",
        form: "Perfect",
        pgn: "2mp",
    },
    IrregularVerb {
        // Archaic/Aramaic-flavoured imperative (Isa 21:12,14).
        surface: "אֵתָיוּ",
        root: "אתה",
        binyan: "Qal",
        form: "Imperative",
        pgn: "2mp",
    },
    IrregularVerb {
        // Anomalous patah-retaining + dagesh construct participle (Isa 23:8-9).
        surface: "נִכְבַּדֵּי",
        root: "כבד",
        binyan: "Niphal",
        form: "Participle (act.)",
        pgn: "mp",
    },
    IrregularVerb {
        // Niphal perfect with o-theme (גאל II "defile", Lam 4:14).
        surface: "נְגֹאֲלוּ",
        root: "גאל",
        binyan: "Niphal",
        form: "Perfect",
        pgn: "3cp",
    },
    IrregularVerb {
        // Doubly-weak III-he + C2-vav Piel infinitive construct (with ל), צוה.
        surface: "לְצַוּת",
        root: "צוה",
        binyan: "Piel",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        // Doubly-weak Piel participle + 2ms suffix -ekkā, צוה.
        surface: "מְצַוֶּךָּ",
        root: "צוה",
        binyan: "Piel",
        form: "Participle (act.)",
        pgn: "ms",
    },
    IrregularVerb {
        // III-he/aleph Piel imperfect 2mp (תאה "mark out", Num 34:7-8).
        surface: "תְּתָאוּ",
        root: "תאה",
        binyan: "Piel",
        form: "Imperfect",
        pgn: "2mp",
    },
    IrregularVerb {
        // Qal vocalization of ערמ "be crafty" (1 Sam 23:22); generator reads it
        // only as Hiphil.
        surface: "יַעְרִם",
        root: "ערמ",
        binyan: "Qal",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        // Wayyiqtol of חלק "divide" + 3mp suffix: segol preformative with a
        // qamats (qamats-hatuf) under the C1 guttural (1 Chr 23-24); generator
        // builds the patah-prefix וַיַּחַלְקֵם.
        surface: "וַיֶּחָלְקֵם",
        root: "חלק",
        binyan: "Qal",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        // Qal imperfect of רדף "pursue" + 2ms suffix: anomalous dagesh + hataf-
        // patah under C2 dalet (יִרְדֲּפֶךָ, Ezek 35:6).
        surface: "יִרְדֲּפֶךָ",
        root: "רדף",
        binyan: "Qal",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        // Hollow שית "set" Qal perfect 2ms + 1cs suffix: patah + dageshed tav
        // host šatt- (שַׁתַּנִי, Ps 88:7).
        surface: "שַׁתַּנִי",
        root: "שית",
        binyan: "Qal",
        form: "Perfect",
        pgn: "2ms",
    },
    IrregularVerb {
        // Stative I-guttural imperative 2mp, pausal qamats theme (ʾehāḇû אֱהָבוּ,
        // Zech 8:17,19); generator builds the segol אֶהֱבוּ.
        surface: "אֱהָבוּ",
        root: "אהב",
        binyan: "Qal",
        form: "Imperative",
        pgn: "2mp",
    },
    IrregularVerb {
        // Qal imperfect 1cs of בחר, secondary hataf-patah under the non-guttural
        // C1 (אֶבֲחַר, Job 34:4,33); generator builds the silent-sheva אֶבְחַר.
        surface: "אֶבֲחַר",
        root: "בחר",
        binyan: "Qal",
        form: "Imperfect",
        pgn: "1cs",
    },
    IrregularVerb {
        // Geminate Qal imperfect 1cs of תמם with a tsere-yod preformative
        // (אֵיתָם, Hos 5:9 etc.); not produced by the geminate builder.
        surface: "אֵיתָם",
        root: "תמם",
        binyan: "Qal",
        form: "Imperfect",
        pgn: "1cs",
    },
    IrregularVerb {
        // Piel cohortative 1cs of לקט with a qamats-qatan theme (אֲלַקֳטָה,
        // Ruth 2:2,7); generator builds the sheva/tsere אֲלַקְּטָה.
        surface: "אֲלַקֳטָה",
        root: "לקט",
        binyan: "Piel",
        form: "Cohortative",
        pgn: "1cs",
    },
    IrregularVerb {
        // III-He Hophal perfect 3fs of עלה, holam preformative + hataf-patah
        // guttural (הֹעֲלָתָה, Nah 2:8); generator builds the qamats-hatuf הׇעְלָתָה.
        surface: "הֹעֲלָתָה",
        root: "עלה",
        binyan: "Hophal",
        form: "Perfect",
        pgn: "3fs",
    },
    IrregularVerb {
        // III-He Hophal perfect 2ms of ראה (הָרְאֵתָ, Deut 4:35).
        surface: "הָרְאֵתָ",
        root: "ראה",
        binyan: "Hophal",
        form: "Perfect",
        pgn: "2ms",
    },
    IrregularVerb {
        // III-ayin Hiphil perfect 2ms of ידע, patah under the guttural + plain
        // tav (הוֹדַעַתָ, 1 Sam 28:15); generator builds הוֹדַעְתָּ.
        surface: "הוֹדַעַתָ",
        root: "ידע",
        binyan: "Hiphil",
        form: "Perfect",
        pgn: "2ms",
    },
    IrregularVerb {
        // Stative Qal imperative 2fs of חרב, qamats-hatuf theme (חֳרָבִי,
        // Jer 50:21 etc.); generator builds חִרְבִי.
        surface: "חֳרָבִי",
        root: "חרב",
        binyan: "Qal",
        form: "Imperative",
        pgn: "2fs",
    },
    IrregularVerb {
        // Hollow רום Niphal imperfect 3mp with euphonic dagesh in C3 mem
        // (יֵרוֹמּוּ, Ps 89:17 etc.); generator builds the single-mem יֵרוֹמוּ.
        surface: "יֵרוֹמּוּ",
        root: "רום",
        binyan: "Niphal",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        // Denominal ימן "go to the right" Hiphil participle mp, yod kept
        // consonantal (מַיְמִינִים, 1 Chr 12:2); generator contracts it.
        surface: "מַיְמִינִים",
        root: "ימן",
        binyan: "Hiphil",
        form: "Participle (act.)",
        pgn: "mp",
    },
    IrregularVerb {
        // Hithpolel of הלל "praise/boast" (OSHB-tagged Hithpael), o-stem
        // reduplicated base the generator's Hithpael doesn't model (Ps 5:6 etc.).
        surface: "יִתְהֹלְלוּ",
        root: "הלל",
        binyan: "Hithpael",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        // Qamats-theme spelling of the same Hithpolel (יִתְהֹלָלוּ, Jer 46:9 etc.).
        surface: "יִתְהֹלָלוּ",
        root: "הלל",
        binyan: "Hithpael",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        // Hithpolel of מלל "languish" (OSHB-tagged Hithpael), o-stem base
        // (יִתְמֹלָלוּ, Ps 58:8).
        surface: "יִתְמֹלָלוּ",
        root: "מלל",
        binyan: "Hithpael",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        // Noun-like Qal infinitive construct of יכל "be able" (Num 14:16).
        surface: "יְכֹלֶת",
        root: "יכל",
        binyan: "Qal",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        // III-guttural Hithpael imperfect, theme qamats before furtive ʿayin
        // (Prov 18:1); generator builds the regular patah יִתְגַּלַּע.
        surface: "יִתְגַּלָּע",
        root: "גלע",
        binyan: "Hithpael",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        // II-guttural Hithpael imperfect compensatory variant (נחם); generator
        // builds the regular virtual-doubling יִתְנַחֵם.
        surface: "יִתְנֶחָם",
        root: "נחם",
        binyan: "Hithpael",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        // III-he/aleph Qal participle of נשא II "lend on interest" (creditor),
        // segol theme written with final aleph (Deut 24:11 etc.).
        surface: "נֹשֶׁא",
        root: "נשא",
        binyan: "Qal",
        form: "Participle (act.)",
        pgn: "ms",
    },
    IrregularVerb {
        // Geminate Qal imperfect 3fs, poetic energic spelling (Prov 1:20, 8:3).
        surface: "תָּרֹנָּה",
        root: "רננ",
        binyan: "Qal",
        form: "Imperfect",
        pgn: "3fs",
    },
    IrregularVerb {
        // Hothpaal-style census form, qamats + no doubling (Num 1:47 etc.);
        // generator builds the regular doubled הִתְפַּקְּדוּ.
        surface: "הִתְפָּקְדוּ",
        root: "פקד",
        binyan: "Hithpael",
        form: "Perfect",
        pgn: "3cp",
    },
    IrregularVerb {
        // Hollow/II-aleph Qal participle "those who despise" (Ezek 28:24,26).
        surface: "הַשָּׁאטִים",
        root: "שאט",
        binyan: "Qal",
        form: "Participle (act.)",
        pgn: "mp",
    },
    IrregularVerb {
        // Qal participle fp of אתה "the things to come" (Isa 41:23, 44:7).
        surface: "הָאֹתִיּוֹת",
        root: "אתה",
        binyan: "Qal",
        form: "Participle (act.)",
        pgn: "fp",
    },
    IrregularVerb {
        // Paragogic-he on a 3fs wayyiqtol of עגב (Ezek 23:3).
        surface: "וַתַּעְגְּבָה",
        root: "עגב",
        binyan: "Qal",
        form: "Wayyiqtol",
        pgn: "3fs",
    },
    IrregularVerb {
        // Doubly-weak pe-nun + consonantal-he Hiphil imperfect, mappiq
        // (נגה "illumine", 2 Sam 22:29).
        surface: "יַגִּיהַּ",
        root: "נגה",
        binyan: "Hiphil",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        // Doubly-weak pe-yod + III-he Qal infinitive construct (ירה "shoot",
        // with ל; 1 Sam 20:36, 2 Chr 35:23).
        surface: "לִירוֹת",
        root: "ירה",
        binyan: "Qal",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        // Apocopated III-he Qal jussive "may he have dominion" (Ps 72:8), final
        // sheva + dagesh.
        surface: "וְיֵרְדְּ",
        root: "רדה",
        binyan: "Qal",
        form: "Jussive",
        pgn: "3ms",
    },
    IrregularVerb {
        // Pe-yod Qal perfect 2fs with holam theme, וְיֹלַדְתְּ (Gen 16:11, Judg 13:5,7).
        surface: "וְיֹלַדְתְּ",
        root: "ילד",
        binyan: "Qal",
        form: "Perfect",
        pgn: "2fs",
    },
    IrregularVerb {
        // Apocopated III-he Qal wayyiqtol "and he took captive" (Num 21:1).
        surface: "וַיִּשְׁבְּ",
        root: "שבה",
        binyan: "Qal",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        // Defectively-pointed וַיֹּאמְרוּ (medial sheva omitted in the text).
        surface: "וַיֹּאמרוּ",
        root: "אמר",
        binyan: "Qal",
        form: "Wayyiqtol",
        pgn: "3mp",
    },
    IrregularVerb {
        // III-he Qal imperative 2ms with a-vowels, אָרָה "pluck" (Ps 80:13, Song 5:1).
        surface: "אָרָה",
        root: "ארה",
        binyan: "Qal",
        form: "Imperative",
        pgn: "2ms",
    },
    IrregularVerb {
        // Geminate Hiphil imperfect 1cs "I will do harm" (רעע); generator reads
        // it otherwise.
        surface: "אָרַע",
        root: "רעע",
        binyan: "Hiphil",
        form: "Imperfect",
        pgn: "1cs",
    },
    IrregularVerb {
        // Contracted I-aleph Hiphil imperfect 1cs "I will gather" (אסף, the two
        // alephs merging; Mic 4:6, Zeph 1:2).
        surface: "אֹסֵף",
        root: "אסף",
        binyan: "Hiphil",
        form: "Imperfect",
        pgn: "1cs",
    },
    IrregularVerb {
        // Pual fp participle construct with irregular de-gemination (קצע
        // "corner", with ל; Ezek 46:22) — not the regular doubled מְקֻצְּעֹת.
        surface: "לִמְקֻצְעֹת",
        root: "קצע",
        binyan: "Pual",
        form: "Participle (act.)",
        pgn: "fp",
    },
    IrregularVerb {
        // Lengthened (paragogic-he) Hiphil imperative, î-grade (יטב; Ps 36:11,
        // Isa 23:16) — the rare emphatic imperative the generator omits.
        surface: "הֵיטִיבָה",
        root: "יטב",
        binyan: "Hiphil",
        form: "Imperative",
        pgn: "2ms",
    },
    IrregularVerb {
        // Aramaic-flavoured Hiphil imperative 2mp of אתה "bring" (Isa 21:14).
        surface: "הֵתָיוּ",
        root: "אתה",
        binyan: "Hiphil",
        form: "Imperative",
        pgn: "2mp",
    },
    IrregularVerb {
        // Geminate Hiphil perfect 3cp "they blossomed" (נצץ; Song 6:11, 7:13).
        surface: "הֵנֵצוּ",
        root: "נצצ",
        binyan: "Hiphil",
        form: "Perfect",
        pgn: "3cp",
    },
    IrregularVerb {
        // Apocopated III-he Qal wayyiqtol 1cp "and we were" (היה; Num 13:33).
        surface: "וַנְּהִי",
        root: "היה",
        binyan: "Qal",
        form: "Wayyiqtol",
        pgn: "1cp",
    },
    IrregularVerb {
        // III-he Piel perfect 3ms "he rebuked / grew dim" (כהה; 1 Sam 3:13).
        surface: "כֵּהָה",
        root: "כהה",
        binyan: "Piel",
        form: "Perfect",
        pgn: "3ms",
    },
    IrregularVerb {
        // Pual participle ms "polished/burnished" (מרט, with ו; 1 Kgs 7:45).
        surface: "וּמוֹרָט",
        root: "מרט",
        binyan: "Pual",
        form: "Participle (act.)",
        pgn: "ms",
    },
    IrregularVerb {
        // Hollow Niphal participle ms "discerning" (בין, with ו; Gen 41:33,39).
        surface: "וּנְבוֹן",
        root: "בינ",
        binyan: "Niphal",
        form: "Participle (act.)",
        pgn: "ms",
    },
    IrregularVerb {
        // Qamats-grade Hiphil infinitive absolute "abundantly" (רבה; Amos 4:9)
        // beside the usual הַרְבֵּה.
        surface: "הַרְבָּה",
        root: "רבה",
        binyan: "Hiphil",
        form: "Inf. Absolute",
        pgn: "",
    },
    IrregularVerb {
        // להה "languish" (C2=C3=he, treated as III-He) Qal wayyiqtol 3fs — the
        // anomalous hapax wattēlah (וַתֵּלַהּ, Gen 47:13 "and the land of Egypt
        // languished"), with a tsere prefix + mappiq-he stem the generator does
        // not produce (BDB 3856a flags the vocalization as irregular).
        surface: "וַתֵּלַהּ",
        root: "להה",
        binyan: "Qal",
        form: "Wayyiqtol",
        pgn: "3fs",
    },
    IrregularVerb {
        // ילד Piel infinitive construct + 2fp suffix — bᵊyalleḏḵen (בְּיַלֶּדְכֶן,
        // Exod 1:16 "when you help give birth"), the proclitic-בְּ temporal
        // infinitive; the generator does not build the Piel inf-construct host
        // with the 2fp possessive suffix.
        surface: "בְּיַלֶּדְכֶן",
        root: "ילד",
        binyan: "Piel",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        // יטב (pe-yod) Hiphil wayyiqtol 3ms with the yod-grade short stem —
        // wayyêṭeḇ (וַיֵּיטֶב, Exod 1:20 "and God dealt well with"); the generator
        // builds the yod-grade long imperfect יֵיטִיב but only the vav-grade short
        // form וַיּוֹטֶב for the wayyiqtol.
        surface: "וַיֵּיטֶב",
        root: "יטב",
        binyan: "Hiphil",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        // יצב Hithpael wayyiqtol 3fs, contracted — the yod assimilates so
        // hiṯyaṣṣēḇ surfaces as wattēṣaṣṣaḇ (וַתֵּתַצַּב, Exod 2:4 "and she
        // stationed herself") beside the uncontracted וַתִּתְיַצֵּב.
        surface: "וַתֵּתַצַּב",
        root: "יצב",
        binyan: "Hithpael",
        form: "Wayyiqtol",
        pgn: "3fs",
    },
    IrregularVerb {
        // III-aleph מלא Piel wayyiqtol 3fp — wattᵊmalleʾnâ (וַתְּמַלֶּאנָה, Exod
        // 2:16 "and they filled"), segol theme + silent aleph before the -nâ
        // afformative, where the generator over-vocalizes the aleph
        // (תְּמַלֵּאֲנָה).
        surface: "וַתְּמַלֶּאנָה",
        root: "מלא",
        binyan: "Piel",
        form: "Wayyiqtol",
        pgn: "3fp",
    },
    IrregularVerb {
        // III-aleph קרא Qal imperative 2fp, anomalous -en ending — qirʾen
        // (קִרְאֶן, Exod 2:20 "call him") for the expected קְרֶאןָ / קְרֶאנָה.
        surface: "קִרְאֶן",
        root: "קרא",
        binyan: "Qal",
        form: "Imperative",
        pgn: "2fp",
    },
    IrregularVerb {
        // יסף Hiphil imperfect 2mp + paragogic nun, defective aleph spelling —
        // tōʾsip̄ûn (תֹאסִפוּן, Exod 5:7 "you shall not continue to give") where
        // the generator builds the plene vav grade תּוֹסִיפוּן.
        surface: "תֹאסִפוּן",
        root: "יסף",
        binyan: "Hiphil",
        form: "Imperfect",
        pgn: "2mp",
    },
    IrregularVerb {
        // עשה Qal wayyiqtol 3mp with full patah on the guttural C1 where the
        // generator writes hataf-patah — wayyaʿaśû (וַיַּעַשׂוּ, Exod 7:10 "and
        // they did so"); BHS preserves this Leningrad form beside וַיַּעֲשׂוּ.
        surface: "וַיַּעַשׂוּ",
        root: "עשה",
        binyan: "Qal",
        form: "Wayyiqtol",
        pgn: "3mp",
    },
    IrregularVerb {
        // הלך Qal wayyiqtol 3fs treated as a strong I-guttural-style verb —
        // wattihălaḵ (וַתִּהֲלַךְ, Exod 9:23 "and fire ran along"), the full
        // hireq-prefix form beside the usual pe-vav-style וַתֵּלֶךְ.
        surface: "וַתִּהֲלַךְ",
        root: "הלך",
        binyan: "Qal",
        form: "Wayyiqtol",
        pgn: "3fs",
    },
    IrregularVerb {
        // Geminate חגג Qal imperfect 2mp + 3ms suffix — tᵊḥāgguhû (תְּחָגֻּהוּ,
        // Exod 12:14 "you shall keep it as a feast"), qamats theme + dageshed
        // gimel + qubuts host before the -hû suffix, which the generator's
        // geminate object-suffix hosts do not produce.
        surface: "תְּחָגֻּהוּ",
        root: "חגג",
        binyan: "Qal",
        form: "Imperfect",
        pgn: "2mp",
    },
    IrregularVerb {
        // נשׂג Hiphil wayyiqtol 3mp "and they overtook" (Exod 14:9, the Egyptians
        // overtaking Israel). The Leningrad/bible.db token spells the sin without
        // the doubling dagesh (וַיַּשִׂיגוּ for וַיַּשִּׂיגוּ).
        surface: "וַיַּשִׂיגוּ",
        root: "נשג",
        binyan: "Hiphil",
        form: "Wayyiqtol",
        pgn: "3mp",
    },
    IrregularVerb {
        // נוה Hiphil imperfect 1cs + 3ms suffix "and I will glorify him"
        // (Exod 15:2, Song of the Sea) — wᵉʾanwēhû, an archaic poetic form the
        // generator does not build.
        surface: "וְאַנְוֵהוּ",
        root: "נוה",
        binyan: "Hiphil",
        form: "Imperfect",
        pgn: "1cs",
    },
    IrregularVerb {
        // כסה Piel imperfect 3mp + archaic 3mp suffix -ēmô "the deeps cover them"
        // (Exod 15:5) — yᵉḵasyumô, the rare yod-retaining III-He poetic stem with
        // the -mô suffix.
        surface: "יְכַסְיֻמוּ",
        root: "כסה",
        binyan: "Piel",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        // אדר Niphal participle ms construct + hireq compaginis "glorious in
        // power" (Exod 15:6) — neʾdārî, the archaic construct -î the generator's
        // participle paradigm does not carry.
        surface: "נֶאְדָּרִי",
        root: "אדר",
        binyan: "Niphal",
        form: "Participle (act.)",
        pgn: "ms",
    },
    IrregularVerb {
        // מלא Qal imperfect 3fs + archaic 3mp suffix -ēmô "my desire shall be
        // full of them" (Exod 15:9) — timlāʾēmô, a poetic suffix the generator
        // does not produce.
        surface: "תִּמְלָאֵמוֹ",
        root: "מלא",
        binyan: "Qal",
        form: "Imperfect",
        pgn: "3fs",
    },
    IrregularVerb {
        // נהל Piel perfect 2ms "you have guided" (Exod 15:13) — nēhaltā, tsere
        // theme where the generator builds the hiriq/patah grade.
        surface: "נֵהַלְתָּ",
        root: "נהל",
        binyan: "Piel",
        form: "Perfect",
        pgn: "2ms",
    },
    IrregularVerb {
        // יצא Hiphil perfect 2mp "you have brought us out" (Exod 16:3) —
        // hôṣēʾṯem, a III-aleph Hiphil perfect 2mp the generator does not build.
        surface: "הוֹצֵאתֶם",
        root: "יצא",
        binyan: "Hiphil",
        form: "Perfect",
        pgn: "2mp",
    },
    IrregularVerb {
        // אמר Qal wayyiqtol 3ms, defective spelling without the doubling dagesh
        // in the yod — wayyōʾmer (וַיֹאמֶר for וַיֹּאמֶר, Exod 16:9 "and Moses
        // said"); the bible.db token preserves this Leningrad defective form.
        surface: "וַיֹאמֶר",
        root: "אמר",
        binyan: "Qal",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        // Quadriliteral חספס Pual participle ms "fine, flake-like" (Exod 16:14,
        // the manna) — mᵉḥuspās; a four-radical root outside the triliteral
        // paradigm generator.
        surface: "מְחֻסְפָּס",
        root: "חספס",
        binyan: "Pual",
        form: "Participle (act.)",
        pgn: "ms",
    },
    IrregularVerb {
        // III-He חדה Qal wayyiqtol 3ms, apocopated with a bare dagesh closing the
        // final dalet — wayyiḥad (וַיִּחַדְּ, Exod 18:9 "and Jethro rejoiced") —
        // beside the generator's וַיִּחַד.
        surface: "וַיִּחַדְּ",
        root: "חדה",
        binyan: "Qal",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        // III-He עשה Qal infinitive construct + 3ms suffix "to do it" (Exod
        // 18:18) — ʿăśōhû, the holam-host inf-construct the generator does not
        // build with the -hû suffix.
        surface: "עֲשֹׂהוּ",
        root: "עשה",
        binyan: "Qal",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        // שפט Qal imperfect 3mp, plene-vav (defectiva-reversed) spelling —
        // yišpûṭû (יִשְׁפּוּטוּ, Exod 18:26 "they would judge") for יִשְׁפְּטוּ.
        surface: "יִשְׁפּוּטוּ",
        root: "שפט",
        binyan: "Qal",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        // גרש Piel perfect (weqatal) 2ms + archaic 3mp suffix -āmô "and you shall
        // drive them out" (Exod 23:31) — wᵉḡēraštāmô.
        surface: "וְגֵרַשְׁתָּמוֹ",
        root: "גרש",
        binyan: "Piel",
        form: "Perfect",
        pgn: "2ms",
    },
    IrregularVerb {
        // היה Qal imperfect 3fp with a doubling dagesh in the yod — tihyeynâ
        // (תִּהְיֶיּןָ for תִּהְיֶיןָ, e.g. Exod 25:27); the bible.db token carries
        // the dagesh the generator does not.
        surface: "תִּהְיֶיּןָ",
        root: "היה",
        binyan: "Qal",
        form: "Imperfect",
        pgn: "3fp",
    },
    IrregularVerb {
        // לבש Qal imperfect 3ms + 3mp suffix "the priest shall wear them" (Exod
        // 29:30) — yilbāšām, a qamats-host + -ām suffix the generator doesn't
        // build.
        surface: "יִלְבָּשָׁם",
        root: "לבש",
        binyan: "Qal",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        // זכר Niphal imperfect 3fs "shall be remembered / counted male" (Exod
        // 34:19) — tizzāḵār, with the a-theme the generator's Niphal imperfect
        // does not produce.
        surface: "תִּזָּכָר",
        root: "זכר",
        binyan: "Niphal",
        form: "Imperfect",
        pgn: "3fs",
    },
    IrregularVerb {
        // III-He כלה Qal wayyiqtol 3fs "and the work was finished" (Exod 39:32) —
        // wattēḵel, tsere prefix + segol theme beside the Piel וַתְּכַל.
        surface: "וַתֵּכֶל",
        root: "כלה",
        binyan: "Qal",
        form: "Wayyiqtol",
        pgn: "3fs",
    },
    IrregularVerb {
        // III-He כלה Piel imperfect 1cs + 2ms suffix "lest I consume you" (Exod
        // 33:3) — ʾăḵelḵā, the segol-host + -ḵā suffix the generator doesn't
        // build.
        surface: "אֲכֶלְךָ",
        root: "כלה",
        binyan: "Piel",
        form: "Imperfect",
        pgn: "1cs",
    },
    IrregularVerb {
        // III-He ראה Qal imperfect 3ms + 1cs suffix "no man shall see me (and
        // live)" (Exod 33:20) — yirʾanî, an apocopated III-He host + -anî suffix.
        surface: "יִרְאַנִי",
        root: "ראה",
        binyan: "Qal",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        // III-He כבה Qal imperfect 3fs "(the fire) shall not be put out" (Lev
        // 6:6) — tiḵbeh, a III-He segol-host imperfect the generator does not
        // build.
        surface: "תִכְבֶה",
        root: "כבה",
        binyan: "Qal",
        form: "Imperfect",
        pgn: "3fs",
    },
    IrregularVerb {
        // III-aleph טמא Piel infinitive construct + 2mp suffix "when you defile
        // (the land)" (Lev 18:28) — bᵉṭammaʾăḵem, with the proclitic בְּ.
        surface: "בְּטַמַּאֲכֶם",
        root: "טמא",
        binyan: "Piel",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        // נחל Hithpael perfect (weqatal) 2mp "you shall take them as an
        // inheritance" (Lev 25:46) — wᵉhiṯnaḥaltem.
        surface: "וְהִתְנַחֲלְתֶּם",
        root: "נחל",
        binyan: "Hithpael",
        form: "Perfect",
        pgn: "2mp",
    },
    IrregularVerb {
        // פרר Hiphil infinitive construct + 2mp suffix "your breaking (my
        // covenant)" (Lev 26:15) — lᵉhap̄rᵉḵem, with the proclitic לְ.
        surface: "לְהַפְרְכֶם",
        root: "פרר",
        binyan: "Hiphil",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        // קרב Qal infinitive construct + 3mp suffix "and when they came near (to
        // the altar)" (Exod 40:32) — ûḇᵉqorḇāṯām, with the proclitics וּבְ.
        surface: "וּבְקָרְבָתָם",
        root: "קרב",
        binyan: "Qal",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        // קרב Qal infinitive construct + 3mp suffix "when they drew near (before
        // the LORD)" (Lev 16:1) — bᵉqorḇāṯām, with the proclitic בְּ.
        surface: "בְּקָרְבָתָם",
        root: "קרב",
        binyan: "Qal",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        // III-He/weak דוה Qal infinitive construct + 3fs suffix "in the days of
        // her menstruation" (Lev 12:2) — dᵉwōṯāh.
        surface: "דְּותָהּ",
        root: "דוה",
        binyan: "Qal",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        // Geminate מקק Niphal imperfect 3mp "they shall rot away (in their
        // iniquity)" (Lev 26:39) — yimmaqqû.
        surface: "יִמָּקּוּ",
        root: "מקק",
        binyan: "Niphal",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        // נפל Hiphil infinitive construct "to make (the thigh) fall away" (Num
        // 5:22), proclitics וְ + לַ — wᵉlanpil.
        surface: "וְלַנְפִּל",
        root: "נפל",
        binyan: "Hiphil",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        // טהר Hithpael perfect (weqatal) 3cp "and let them purify themselves"
        // (Num 8:7) — wᵉhiṭṭehārû, segol-theme variant of וְהִטַּהֲרוּ.
        surface: "וְהִטֶּהָרוּ",
        root: "טהר",
        binyan: "Hithpael",
        form: "Perfect",
        pgn: "3cp",
    },
    IrregularVerb {
        // אצל Qal wayyiqtol 3ms "and he reserved/withdrew (of the spirit)" (Num
        // 11:25) — wayyāʾṣel.
        surface: "וַיָּאצֶל",
        root: "אצל",
        binyan: "Qal",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        // נאץ Piel imperfect 3mp + 1cs suffix "how long will they spurn me" (Num
        // 14:11) — yᵉnaʾăṣunî.
        surface: "יְנַאֲצֻנִי",
        root: "נאץ",
        binyan: "Piel",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        // III-He כלה Piel 1cs "that I may consume them" (Num 16:21 / 17:10) —
        // wāʾăḵalleh, patah-aleph variant; the consonantal form is a homograph
        // between the imperfect and the cohortative, so both are listed.
        surface: "וַאַכַלֶּה",
        root: "כלה",
        binyan: "Piel",
        form: "Imperfect",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "וַאַכַלֶּה",
        root: "כלה",
        binyan: "Piel",
        form: "Cohortative",
        pgn: "1cs",
    },
    IrregularVerb {
        // Hollow גוע Qal infinitive construct "to perish" (Num 17:28), proclitic
        // לְ — liḡwōaʿ.
        surface: "לִגְועַ",
        root: "גוע",
        binyan: "Qal",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        // III-He פדה Qal passive participle ms construct + 3ms suffix "those of it
        // to be redeemed" (Num 18:16) — pᵉḏûyāw.
        surface: "וּפְדוּיָו",
        root: "פדה",
        binyan: "Qal",
        form: "Participle (pass.)",
        pgn: "ms",
    },
    IrregularVerb {
        // ראה Qal wayyiqtol 3fs + 1cs suffix "and she (the donkey) saw me" (Num
        // 22:33) — wattirʾanî.
        surface: "וַתִּרְאַנִי",
        root: "ראה",
        binyan: "Qal",
        form: "Wayyiqtol",
        pgn: "3fs",
    },
    IrregularVerb {
        // III-aleph נשא Hithpael imperfect 3fs "and his kingdom shall be exalted"
        // (Num 24:7) — wᵉṯinnaśśēʾ.
        surface: "וְתִנַּשֵּׂא",
        root: "נשא",
        binyan: "Hithpael",
        form: "Imperfect",
        pgn: "3fs",
    },
    IrregularVerb {
        // Geminate פרר Hiphil imperfect 3ms + 3ms suffix "if her husband annuls
        // it" (Num 30:14) — yᵉp̄ērennû.
        surface: "יְפֵרֶנּוּ",
        root: "פרר",
        binyan: "Hiphil",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        // Hollow בוא Hiphil perfect 1cp + 3mp suffix "until we have brought them"
        // (Num 32:17) — hăḇîʾōnum.
        surface: "הֲבִיאֹנֻם",
        root: "בוא",
        binyan: "Hiphil",
        form: "Perfect",
        pgn: "1cp",
    },
    IrregularVerb {
        // אחז Niphal perfect (weqatal) 3cp "and they shall take possession" (Num
        // 32:30) — wᵉnōʾăḥăzû.
        surface: "וְנֹאחֲזוּ",
        root: "אחז",
        binyan: "Niphal",
        form: "Perfect",
        pgn: "3cp",
    },
    IrregularVerb {
        // נחל Hithpael imperfect 2mp "you shall divide it as inheritance" (Num
        // 33:54) — tiṯneḥālû, segol-theme variant of תִּתְנַחֲלוּ.
        surface: "תִּתְנֶחָלוּ",
        root: "נחל",
        binyan: "Hithpael",
        form: "Imperfect",
        pgn: "2mp",
    },
    IrregularVerb {
        // ראה Hiphil perfect 3ms + 2ms suffix "he showed you (his great fire)"
        // (Deut 4:36) — herʾăḵā.
        surface: "הֶרְאֲךָ",
        root: "ראה",
        binyan: "Hiphil",
        form: "Perfect",
        pgn: "3ms",
    },
    IrregularVerb {
        // שמר Qal infinitive construct + 3ms suffix "and because of his keeping
        // (the oath)" (Deut 7:8), proclitics וּ + מִ — ûmiššomrô.
        surface: "וּמִשָּׁמְרו",
        root: "שמר",
        binyan: "Qal",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        // Hollow שׂים Qal imperfect 3ms + 3mp suffix "he will lay them (on those
        // who hate you)" (Deut 7:15) — yᵉśîmām.
        surface: "יְשִׂימָם",
        root: "שים",
        binyan: "Qal",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        // אבד Hiphil perfect (weqatal) 2ms + 3mp suffix "and you shall destroy
        // them" (Deut 9:3) — wᵉhaʾăḇaḏtām.
        surface: "וְהַאַבַדְתָּם",
        root: "אבד",
        binyan: "Hiphil",
        form: "Perfect",
        pgn: "2ms",
    },
    IrregularVerb {
        // קרב Qal infinitive construct + 2mp suffix "when you draw near to the
        // battle" (Deut 20:2), proclitic כְּ — kᵉqorbᵉḵem.
        surface: "כְּקָרָבְכֶם",
        root: "קרב",
        binyan: "Qal",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        // יסר Piel perfect (weqatal) 3cp "and they chastise him" (Deut 21:18) —
        // wᵉyissᵉrû, reduced-sheva variant of וְיִסְּרוּ.
        surface: "וְיסְּרוּ",
        root: "יסר",
        binyan: "Piel",
        form: "Perfect",
        pgn: "3cp",
    },
    IrregularVerb {
        // III-aleph יצא Hiphil perfect (weqatal) 2mp "and you shall bring them
        // out" (Deut 22:24) — wᵉhôṣēʾṯem.
        surface: "וְהוֹצֵאתֶם",
        root: "יצא",
        binyan: "Hiphil",
        form: "Perfect",
        pgn: "2mp",
    },
    IrregularVerb {
        // פאר Piel imperfect 2ms "you shall not go over the boughs again" (Deut
        // 24:20) — tᵉp̄āʾēr.
        surface: "תְפַאֵר",
        root: "פאר",
        binyan: "Piel",
        form: "Imperfect",
        pgn: "2ms",
    },
    IrregularVerb {
        // Hollow דושׁ Qal infinitive construct + 3ms suffix "while it treads out
        // the grain" (Deut 25:4), proclitic בְּ — bᵉḏîšô.
        surface: "בְּדִישׁוֹ",
        root: "דוש",
        binyan: "Qal",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        // נצר Qal imperfect 3ms + 3ms energic suffix "he kept him (as the apple
        // of his eye)" (Deut 32:10) — yiṣṣᵉrenhû.
        surface: "יִצְּרֶנְהוּ",
        root: "נצר",
        binyan: "Qal",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        // III-He שׁיה Qal imperfect 2ms, apocopated "you forgot (the God who bore
        // you)" (Deut 32:18) — teši.
        surface: "תֶּשִׁי",
        root: "שיה",
        binyan: "Qal",
        form: "Imperfect",
        pgn: "2ms",
    },
    IrregularVerb {
        // III-He פאה Hiphil cohortative 1cs + 3mp suffix "I would have scattered
        // them (into corners)" (Deut 32:26) — ʾap̄ʾêhem.
        surface: "אַפְאֵיהֶם",
        root: "פאה",
        binyan: "Hiphil",
        form: "Cohortative",
        pgn: "1cs",
    },
    IrregularVerb {
        // עזר Qal imperfect 3mp + 2mp suffix "let them help you" (Deut 32:38) —
        // wᵉyaʿzᵉruḵem.
        surface: "וְיַעְזְרֻכֶם",
        root: "עזר",
        binyan: "Qal",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        // Hollow בוא Qal imperfect 3fs + paragogic he "let (the good will) come"
        // (Deut 33:16) — tāḇôʾtâ.
        surface: "תָּבוֹאתָה",
        root: "בוא",
        binyan: "Qal",
        form: "Imperfect",
        pgn: "3fs",
    },
    IrregularVerb {
        // צפן Qal wayyiqtol 3fs + 3ms suffix "and she hid him" (Josh 2:4) —
        // wattiṣpᵉnô.
        surface: "וַתִּצְפְּנוֹ",
        root: "צפן",
        binyan: "Qal",
        form: "Wayyiqtol",
        pgn: "3fs",
    },
    IrregularVerb {
        // III-He עלה Hiphil perfect 3fs + 3mp suffix "she had brought them up (to
        // the roof)" (Josh 2:6) — heʿĕlāṯam.
        surface: "הֶעֱלָתַם",
        root: "עלה",
        binyan: "Hiphil",
        form: "Perfect",
        pgn: "3fs",
    },
    IrregularVerb {
        // III-aleph חבא Hiphil perfect 3fs "(Rahab) hid (the messengers)" (Josh
        // 6:17) — heḥbᵉʾaṯâ.
        surface: "הֶחְבְּאַתָה",
        root: "חבא",
        binyan: "Hiphil",
        form: "Perfect",
        pgn: "3fs",
    },
    IrregularVerb {
        // עבר Hiphil perfect 2ms "why have you brought this people over (the
        // Jordan)" (Josh 7:7) — hēʿăḇartā, the I-guttural tsere-prefix variant.
        surface: "הֵעֲבַרְתָּ",
        root: "עבר",
        binyan: "Hiphil",
        form: "Perfect",
        pgn: "2ms",
    },
    IrregularVerb {
        // אחז Niphal perfect 3cp "they had taken possession" (Josh 22:9) —
        // nōʾăḥăzû (cf. the weqatal וְנֹאחֲזוּ of Num 32:30).
        surface: "נֹאחֲזוּ",
        root: "אחז",
        binyan: "Niphal",
        form: "Perfect",
        pgn: "3cp",
    },
    IrregularVerb {
        // I-yod ירא Qal infinitive construct "to fear (the LORD)" (Josh 22:25) —
        // yᵉrōʾ, holam-host variant of the usual יִרְאָה / יְרֹא.
        surface: "יְרֹא",
        root: "ירא",
        binyan: "Qal",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        // I-guttural אסף Qal wayyiqtol 3ms "and he gathered (all the tribes)"
        // (Josh 24:1) — wayyeʾesōp̄, plain-segol variant of וַיֶּאֱסֹף.
        surface: "וַיֶּאֶסֹף",
        root: "אסף",
        binyan: "Qal",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        // נגש Hiphil wayyiqtol 3ms "and he brought (it) near" (Judg 6:19) —
        // wayyaggaš, patah-theme short form beside the generator's וַיַּגֵּשׁ.
        surface: "וַיַּגַּשׁ",
        root: "נגש",
        binyan: "Hiphil",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        // III-He חנה Qal wayyiqtol 3mp + paragogic nun "and they camped (in
        // Arnon)" (Judg 11:18) — wayyaḥănûn.
        surface: "וַיַּחֲנון",
        root: "חנה",
        binyan: "Qal",
        form: "Wayyiqtol",
        pgn: "3mp",
    },
    IrregularVerb {
        // כרע Hiphil perfect 2fs + 1cs suffix "you have brought me very low"
        // (Judg 11:35, Jephthah's daughter) — hiḵraʿtinî.
        surface: "הִכְרַעְתִּנִי",
        root: "כרע",
        binyan: "Hiphil",
        form: "Perfect",
        pgn: "2fs",
    },
    IrregularVerb {
        // נתן Qal perfect (weqatal) 1cp + 2ms suffix "we will give you (into the
        // hand of the Philistines)" (Judg 15:13) — ûnᵉṯannûḵā.
        surface: "וּנְתַנּוּךָ",
        root: "נתן",
        binyan: "Qal",
        form: "Perfect",
        pgn: "1cp",
    },
    IrregularVerb {
        // Reduplicated מהה (Hithpalpel, gold-tagged Hithpael) imperative 2mp
        // "tarry/linger" (Judg 19:8) — wᵉhiṯmahmᵉhû.
        surface: "וְהִתְמַהְמְהוּ",
        root: "מהה",
        binyan: "Hithpael",
        form: "Imperative",
        pgn: "2mp",
    },
    IrregularVerb {
        // Geminate בלל Qal wayyiqtol 3ms "and he gave fodder (to the donkeys)"
        // (Judg 19:21) — wayyāḇol.
        surface: "וַיָּבָול",
        root: "בלל",
        binyan: "Qal",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        // פקד Hithpael wayyiqtol 3mp "and they were mustered" (Judg 20:15) —
        // wayyiṯpāqᵉḏû.
        surface: "וַיִּתְפָּקְדוּ",
        root: "פקד",
        binyan: "Hithpael",
        form: "Wayyiqtol",
        pgn: "3mp",
    },
    // ── Judges/Samuel/Kings tail (gold-precise, OSHB) ──
    IrregularVerb {
        // נתק Qal weqatal 1cp + 3ms suffix "we will draw him away" (Judg 20:32).
        surface: "וּנְתַקְּנֻהוּ",
        root: "נתק",
        binyan: "Qal",
        form: "Perfect",
        pgn: "1cp",
    },
    IrregularVerb {
        // רדף Hiphil perfect 3cp + 3ms suffix "they pursued him" (Judg 20:43).
        surface: "הִרְדִיפֻהוּ",
        root: "רדף",
        binyan: "Hiphil",
        form: "Perfect",
        pgn: "3cp",
    },
    IrregularVerb {
        // פקד Hithpael wayyiqtol 3ms "and it was mustered" (Judg 21:9).
        surface: "וַיִּתְפָּקֵד",
        root: "פקד",
        binyan: "Hithpael",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        // שכר Hithpael imperfect 2fs + paragogic nun "how long will you be drunk"
        // (1Sam 1:14).
        surface: "תִּשְׁתַּכָּרִין",
        root: "שכר",
        binyan: "Hithpael",
        form: "Imperfect",
        pgn: "2fs",
    },
    IrregularVerb {
        // שאל Qal perfect 1cs + 3ms suffix "I asked him of the LORD" (1Sam 1:20).
        surface: "שְׁאִלְתִּיו",
        root: "שאל",
        binyan: "Qal",
        form: "Perfect",
        pgn: "1cs",
    },
    IrregularVerb {
        // שמם Hiphil wayyiqtol 3ms + 3mp suffix "and he devastated them" (1Sam
        // 5:6), with an extra dagesh in bible.db.
        surface: "וַיְשִּׁמֵּם",
        root: "שמם",
        binyan: "Hiphil",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        // ישר Qal wayyiqtol 3fp "and the cows went straight" (1Sam 6:12).
        surface: "וַיִשַּׁרְנָה",
        root: "ישר",
        binyan: "Qal",
        form: "Wayyiqtol",
        pgn: "3fp",
    },
    IrregularVerb {
        // מאן Piel wayyiqtol 3mp "and they refused" (1Sam 8:19), patah variant.
        surface: "וַיְמַאֲנוּ",
        root: "מאן",
        binyan: "Piel",
        form: "Wayyiqtol",
        pgn: "3mp",
    },
    IrregularVerb {
        // רצץ Qal perfect 2ms + 1cp suffix "you have oppressed us" (1Sam 12:4).
        surface: "רַצּוֹתָנוּ",
        root: "רצץ",
        binyan: "Qal",
        form: "Perfect",
        pgn: "2ms",
    },
    IrregularVerb {
        // III-aleph חטא Qal participle mp absolute "sinning" (1Sam 14:33).
        surface: "חֹטִאים",
        root: "חטא",
        binyan: "Qal",
        form: "Participle (act.)",
        pgn: "mp",
    },
    IrregularVerb {
        // שאר Hiphil cohortative 1cp "let us not leave a man" (1Sam 14:36).
        surface: "נַשְׁאֵר",
        root: "שאר",
        binyan: "Hiphil",
        form: "Cohortative",
        pgn: "1cp",
    },
    IrregularVerb {
        // רוח Qal perfect 3ms (weqatal) "and Saul was refreshed" (1Sam 16:23).
        surface: "וְרָוַח",
        root: "רוח",
        binyan: "Qal",
        form: "Perfect",
        pgn: "3ms",
    },
    IrregularVerb {
        // שמע Niphal wayyiqtol 3mp "and the words were heard" (1Sam 17:31).
        surface: "וַיְּשָּׁמְעוּ",
        root: "שמע",
        binyan: "Niphal",
        form: "Wayyiqtol",
        pgn: "3mp",
    },
    IrregularVerb {
        // אהב Qal perfect 3fs + 3ms suffix "Michal loved him" (1Sam 18:28),
        // patah variant.
        surface: "אַהֵבַתְהוּ",
        root: "אהב",
        binyan: "Qal",
        form: "Perfect",
        pgn: "3fs",
    },
    IrregularVerb {
        // יסף Hiphil wayyiqtol 3ms "and Saul continued/did again" (1Sam 18:29).
        surface: "וַיֹּאסֶף",
        root: "יסף",
        binyan: "Hiphil",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        // חבא Niphal perfect (weqatal) 2ms "and you shall hide yourself" (1Sam
        // 19:2); cf. the bare נַחְבֵּאתָ.
        surface: "וְנַחְבֵּאתָ",
        root: "חבא",
        binyan: "Niphal",
        form: "Perfect",
        pgn: "2ms",
    },
    IrregularVerb {
        // רמה Piel perfect 2fs + 1cs suffix "you deceived me" (1Sam 19:17).
        surface: "רִמִּיתִנִי",
        root: "רמה",
        binyan: "Piel",
        form: "Perfect",
        pgn: "2fs",
    },
    IrregularVerb {
        // שלח Qal participle ms construct + 2ms suffix "the one who sends you"
        // (1Sam 21:3).
        surface: "שֹׁלֵחֲךָ",
        root: "שלח",
        binyan: "Qal",
        form: "Participle (act.)",
        pgn: "ms",
    },
    IrregularVerb {
        // נתן Qal imperative 2ms + 3fs suffix "give it (to me)" (1Sam 21:10).
        surface: "תְּנֶנָּה",
        root: "נתן",
        binyan: "Qal",
        form: "Imperative",
        pgn: "2ms",
    },
    IrregularVerb {
        // שנה Piel wayyiqtol 3ms + 3ms suffix "and he changed his behavior" (1Sam
        // 21:14).
        surface: "וַיְשַׁנּוֹ",
        root: "שנה",
        binyan: "Piel",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        // III-aleph קרא Qal wayyiqtol 1cs + paragogic he "and I summoned you"
        // (1Sam 28:15).
        surface: "וָאֶקְרָאֶה",
        root: "קרא",
        binyan: "Qal",
        form: "Wayyiqtol",
        pgn: "1cs",
    },
    IrregularVerb {
        // III-aleph פלא Niphal perfect 3fs "your love was wonderful" (2Sam 1:26).
        surface: "נִפְלְאַתָה",
        root: "פלא",
        binyan: "Niphal",
        form: "Perfect",
        pgn: "3fs",
    },
    IrregularVerb {
        // III-He עשה Qal imperfect 1cs "what shall I do" (variant of אֶעֱשֶׂה).
        surface: "אֶעֶשֶׂה",
        root: "עשה",
        binyan: "Qal",
        form: "Imperfect",
        pgn: "1cs",
    },
    IrregularVerb {
        // נגש Hophal perfect 3cp "your feet were brought into fetters" (2Sam
        // 3:34), patah variant of הֻגָּשׁוּ.
        surface: "הֻגַּשׁוּ",
        root: "נגש",
        binyan: "Hophal",
        form: "Perfect",
        pgn: "3cp",
    },
    IrregularVerb {
        // PeAleph אחז Qal wayyiqtol 3ms holam contraction "and he took hold"
        // (2Sam 6:6).
        surface: "וַיֹּאחֶז",
        root: "אחז",
        binyan: "Qal",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        // Geminate קלל Niphal perfect (weqatal) 1cs "and I will be lightly
        // esteemed" (2Sam 6:22).
        surface: "וּנְקַלֹּתִי",
        root: "קלל",
        binyan: "Niphal",
        form: "Perfect",
        pgn: "1cs",
    },
    IrregularVerb {
        // III-He עוה Hiphil infinitive construct + 3ms suffix "when he commits
        // iniquity" (2Sam 7:14), proclitic בְּ.
        surface: "בְּהַעֲותוֹ",
        root: "עוה",
        binyan: "Hiphil",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        // III-He נכה Hiphil wayyiqtol 3ms + 3ms suffix "and the one struck the
        // other" (2Sam 14:6).
        surface: "וַיַּכּוֹ",
        root: "נכה",
        binyan: "Hiphil",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        // נשק Qal infinitive construct "to kiss" (2Sam 20:9), proclitic לְ.
        surface: "לִנְשָׁק",
        root: "נשק",
        binyan: "Qal",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        // יקע Hiphil perfect (weqatal) 1cp + 3mp suffix "and we will hang them"
        // (2Sam 21:6).
        surface: "וְהוֹקַעֲנוּם",
        root: "יקע",
        binyan: "Hiphil",
        form: "Perfect",
        pgn: "1cp",
    },
    IrregularVerb {
        // Geminate ברר Niphal imperfect 2ms "you show yourself pure" (2Sam
        // 22:27).
        surface: "תִּתָּבָר",
        root: "ברר",
        binyan: "Niphal",
        form: "Imperfect",
        pgn: "2ms",
    },
    IrregularVerb {
        // Geminate דקק Hiphil imperfect 1cs + 3mp suffix "I beat them small"
        // (2Sam 22:43).
        surface: "אֲדִקֵּם",
        root: "דקק",
        binyan: "Hiphil",
        form: "Imperfect",
        pgn: "1cs",
    },
    IrregularVerb {
        // ארך Hiphil perfect (weqatal) 1cs "then I will lengthen your days" (1Kgs
        // 3:14), patah variant.
        surface: "וְהַאַרַכְתִּי",
        root: "ארך",
        binyan: "Hiphil",
        form: "Perfect",
        pgn: "1cs",
    },
    IrregularVerb {
        // III-aleph נשא Qal participle fs construct "carrying (gold)" (1Kgs
        // 10:22).
        surface: "נֹשְׂאֵת",
        root: "נשא",
        binyan: "Qal",
        form: "Participle (act.)",
        pgn: "fs",
    },
    IrregularVerb {
        // III-He צוה Piel imperfect 1cs + 2ms suffix "I will command you" (1Kgs
        // 11:38), patah variant of אֲצַוֶּךָ.
        surface: "אֲצַוַּךָ",
        root: "צוה",
        binyan: "Piel",
        form: "Imperfect",
        pgn: "1cs",
    },
    IrregularVerb {
        // קשר Qal wayyiqtol 3ms "and he conspired" (1Kgs 15:27), silent-sheva
        // variant of וַיִּקְשֹׁר.
        surface: "וַיִּקשֹׁר",
        root: "קשר",
        binyan: "Qal",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        // Reduplicated כול (Pilpel, gold-tagged Piel) perfect 3ms + 3mp suffix
        // "and he sustained them" (1Kgs 18:4).
        surface: "וְכִלְכְּלָם",
        root: "כול",
        binyan: "Piel",
        form: "Perfect",
        pgn: "3ms",
    },
    IrregularVerb {
        // עזב Qal infinitive construct + 2mp suffix "when you forsake" (1Kgs
        // 18:18), proclitic בַּ.
        surface: "בַּעַזָבְכֶם",
        root: "עזב",
        binyan: "Qal",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        // עצר Qal imperfect 3ms + 2ms suffix "lest the rain stop you" (1Kgs
        // 18:44).
        surface: "יַעַצָרְכָה",
        root: "עצר",
        binyan: "Qal",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        // III-aleph רפא Piel perfect 1cs "I have healed these waters" (2Kgs
        // 2:21).
        surface: "רִפִּאתִי",
        root: "רפא",
        binyan: "Piel",
        form: "Perfect",
        pgn: "1cs",
    },
    IrregularVerb {
        // בקע Piel wayyiqtol 3fp "and they (the bears) tore (the children)" (2Kgs
        // 2:24).
        surface: "וַתְּבַקַּעְנָה",
        root: "בקע",
        binyan: "Piel",
        form: "Wayyiqtol",
        pgn: "3fp",
    },
    IrregularVerb {
        // III-He ראה Qal wayyiqtol 3ms "and he saw" (patah variant of וַיַּרְא).
        surface: "וַיַּרַא",
        root: "ראה",
        binyan: "Qal",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        // יטב Hiphil wayyiqtol 3fs "and she beautified (her head)" (2Kgs 9:30,
        // Jezebel).
        surface: "וַתֵּיטֶב",
        root: "יטב",
        binyan: "Hiphil",
        form: "Wayyiqtol",
        pgn: "3fs",
    },
    IrregularVerb {
        // רוץ Qal participle mp absolute + article, archaic -în "the runners /
        // royal guard" (2Kgs 11:13).
        surface: "הָרָצִין",
        root: "רוץ",
        binyan: "Qal",
        form: "Participle (act.)",
        pgn: "mp",
    },
    IrregularVerb {
        // III-He עשה Qal wayyiqtol 3ms "and he did (evil)" (patah variant of
        // וַיַּעֲשֶׂה).
        surface: "וַיַּעַשֶׂה",
        root: "עשה",
        binyan: "Qal",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    // ── Isaiah tail (gold-precise, OSHB) ──
    IrregularVerb {
        // עכס Piel imperfect 3fp "making a tinkling with their feet" (Isa 3:16).
        surface: "תְּעַכַּסְנָה",
        root: "עכס",
        binyan: "Piel",
        form: "Imperfect",
        pgn: "3fp",
    },
    IrregularVerb {
        // Hollow חוש Hiphil imperfect 3ms + paragogic he "let him hasten his work"
        // (Isa 5:19).
        surface: "יָחִישָׁה",
        root: "חוש",
        binyan: "Hiphil",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        // III-He גבה Hiphil infinitive absolute "make it deep" (Isa 7:11).
        surface: "הַגְבֵּהַּ",
        root: "גבה",
        binyan: "Hiphil",
        form: "Inf. Absolute",
        pgn: "",
    },
    IrregularVerb {
        // PeAleph אחז Qal imperfect 3mp + paragogic nun "they will be seized"
        // (Isa 13:8), holam contraction.
        surface: "יֹאחֵזוּן",
        root: "אחז",
        binyan: "Qal",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        // III-He נגה Hiphil imperfect 3ms "(the moon) will not shed its light"
        // (Isa 13:10).
        surface: "יַגִּיהַ",
        root: "נגה",
        binyan: "Hiphil",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        // רטש Piel imperfect 3fp "will dash the young to pieces" (Isa 13:18).
        surface: "תְּרַטַּשְׁנָה",
        root: "רטש",
        binyan: "Piel",
        form: "Imperfect",
        pgn: "3fp",
    },
    IrregularVerb {
        // זנח Hiphil perfect (weqatal) 3cp "and the rivers will become foul" (Isa
        // 19:6).
        surface: "וְהֶאֶזְנִיחוּ",
        root: "זנח",
        binyan: "Hiphil",
        form: "Perfect",
        pgn: "3cp",
    },
    IrregularVerb {
        // סחר Qal participle mp construct + 3fs suffix "her merchants" (Isa 23:8).
        surface: "סֹחֲרֶיה",
        root: "סחר",
        binyan: "Qal",
        form: "Participle (act.)",
        pgn: "mp",
    },
    IrregularVerb {
        // כבד Niphal participle mp construct "the honoured of the earth" (Isa
        // 23:8), dagesh-less variant of נִכְבַּדֵּי.
        surface: "נִכְבַדֵּי",
        root: "כבד",
        binyan: "Niphal",
        form: "Participle (act.)",
        pgn: "mp",
    },
    IrregularVerb {
        // עבר Qal imperative 2fs "pass over to Tarshish" (Isa 23:12).
        surface: "עֲבֹרִי",
        root: "עבר",
        binyan: "Qal",
        form: "Imperative",
        pgn: "2fs",
    },
    IrregularVerb {
        // III-He קוה Piel perfect 1cp + 2ms suffix "we have waited for you" (Isa
        // 26:8).
        surface: "קִוִּינוּךָ",
        root: "קוה",
        binyan: "Piel",
        form: "Perfect",
        pgn: "1cp",
    },
    IrregularVerb {
        // שׂדד Piel imperfect 3ms "(does he) harrow his ground" (Isa 28:24).
        surface: "וִיַשַׂדֵּד",
        root: "שדד",
        binyan: "Piel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        // III-He תמה Qal imperative 2mp "be amazed and wonder" (Isa 29:9).
        surface: "וּתְמָהוּ",
        root: "תמה",
        binyan: "Qal",
        form: "Imperative",
        pgn: "2mp",
    },
    IrregularVerb {
        // יקש Qal imperfect 3mp + paragogic nun "they lay a snare" (Isa 29:21).
        surface: "יְקֹשׁוּן",
        root: "יקש",
        binyan: "Qal",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        // בטח Qal participle fp absolute "you complacent women" (Isa 32:9),
        // sheva-less variant.
        surface: "בֹּטחוֹת",
        root: "בטח",
        binyan: "Qal",
        form: "Participle (act.)",
        pgn: "fp",
    },
    IrregularVerb {
        // אכל Qal imperfect 3fs + 2mp suffix "your breath shall devour you" (Isa
        // 33:11).
        surface: "תֹּאכַלְכֶם",
        root: "אכל",
        binyan: "Qal",
        form: "Imperfect",
        pgn: "3fs",
    },
    IrregularVerb {
        // שלך Hophal imperfect 3mp "their slain shall be cast out" (Isa 34:3).
        surface: "יֻשְׁלָכוּ",
        root: "שלך",
        binyan: "Hophal",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        // III-aleph קרא Qal imperative 2mp "seek and read" (Isa 34:16).
        surface: "וּקְרָאוּ",
        root: "קרא",
        binyan: "Qal",
        form: "Imperative",
        pgn: "2mp",
    },
    IrregularVerb {
        // Hollow שׂושׂ Qal imperfect 3mp + 3mp suffix "the desert shall be glad
        // for them" (Isa 35:1).
        surface: "יְשֻׂשׂוּם",
        root: "שוש",
        binyan: "Qal",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        // מהר Niphal participle mp construct "to the anxious of heart" (Isa
        // 35:4), proclitic לְ.
        surface: "לְנִמְהֲרֵי",
        root: "מהר",
        binyan: "Niphal",
        form: "Participle (act.)",
        pgn: "mp",
    },
    IrregularVerb {
        // ישע Hiphil imperfect 3ms + 2mp suffix "he will save you" (Isa 35:4).
        surface: "וְיֹשַׁעֲכֶם",
        root: "ישע",
        binyan: "Hiphil",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        // נשת Qal perfect 3fs "their tongue is parched with thirst" (Isa 41:17).
        surface: "נָשָׁתָּה",
        root: "נשת",
        binyan: "Qal",
        form: "Perfect",
        pgn: "3fs",
    },
    IrregularVerb {
        // III-He אתה Qal wayyiqtol 3ms "and he comes / tramples rulers" (Isa
        // 41:25).
        surface: "וַיַּאת",
        root: "אתה",
        binyan: "Qal",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        // III-He ראה Qal imperfect 1cs "and I looked, and there was no one" (Isa
        // 41:28).
        surface: "וְאֵרֶא",
        root: "ראה",
        binyan: "Qal",
        form: "Imperfect",
        pgn: "1cs",
    },
    IrregularVerb {
        // חזק Hiphil imperfect 1cs "and I will hold your hand" (Isa 42:6).
        surface: "וְאַחְזֵק",
        root: "חזק",
        binyan: "Hiphil",
        form: "Imperfect",
        pgn: "1cs",
    },
    IrregularVerb {
        // צוח Qal imperfect 3mp "let them shout from the mountaintops" (Isa
        // 42:11).
        surface: "יִצְוָחוּ",
        root: "צוח",
        binyan: "Qal",
        form: "Imperfect",
        pgn: "3mp",
    },
    // ── Isaiah 47 → Jeremiah 30 tail (gold-precise, OSHB) ──
    IrregularVerb { surface: "חֶשְׂפִּי", root: "חשף", binyan: "Qal", form: "Imperative", pgn: "2fs" },
    IrregularVerb { surface: "תֵדָעִי", root: "ידע", binyan: "Qal", form: "Imperfect", pgn: "2fs" },
    IrregularVerb { surface: "תַּעֲרוֹצִי", root: "ערץ", binyan: "Qal", form: "Imperfect", pgn: "2fs" },
    IrregularVerb { surface: "וְיוֹשִׁיעֻךְ", root: "ישע", binyan: "Hiphil", form: "Imperfect", pgn: "3ms" },
    IrregularVerb { surface: "יָגָעַתְּ", root: "יגע", binyan: "Qal", form: "Perfect", pgn: "2fs" },
    IrregularVerb { surface: "מְקֹרָאִי", root: "קרא", binyan: "Pual", form: "Participle (act.)", pgn: "ms" },
    IrregularVerb { surface: "קְרָאָנִי", root: "קרא", binyan: "Qal", form: "Perfect", pgn: "3ms" },
    IrregularVerb { surface: "חַקֹּתִיךְ", root: "חקק", binyan: "Qal", form: "Perfect", pgn: "1cs" },
    IrregularVerb { surface: "תִלְבָּשִׁי", root: "לבש", binyan: "Qal", form: "Imperfect", pgn: "2fs" },
    IrregularVerb { surface: "וְשֹׁמְמֹתַיִךְ", root: "שמם", binyan: "Qal", form: "Participle (act.)", pgn: "fp" },
    IrregularVerb { surface: "תִּנָּשֶׂאנָה", root: "נשא", binyan: "Niphal", form: "Imperfect", pgn: "3fp" },
    IrregularVerb { surface: "מֵינִיקֹתַיִךְ", root: "ינק", binyan: "Hiphil", form: "Participle (act.)", pgn: "fp" },
    IrregularVerb { surface: "קֹרְאֹתַיִךְ", root: "קרא", binyan: "Qal", form: "Participle (act.)", pgn: "fp" },
    IrregularVerb { surface: "נָּאווּ", root: "נאה", binyan: "Qal", form: "Perfect", pgn: "3cp" },
    IrregularVerb { surface: "יְשׂוֹחֵחַ", root: "שיח", binyan: "Piel", form: "Imperfect", pgn: "3ms" },
    IrregularVerb { surface: "תַּחְשֹׂכִי", root: "חשך", binyan: "Qal", form: "Jussive", pgn: "2fs" },
    IrregularVerb { surface: "תִּשְׁכָּחִי", root: "שכח", binyan: "Qal", form: "Imperfect", pgn: "2fs" },
    IrregularVerb { surface: "יַבְדִּילַנִי", root: "בדל", binyan: "Hiphil", form: "Imperfect", pgn: "3ms" },
    IrregularVerb { surface: "הַנֵּחָמִים", root: "חמם", binyan: "Niphal", form: "Participle (act.)", pgn: "mp" },
    IrregularVerb { surface: "יוֹעִילוּךְ", root: "יעל", binyan: "Hiphil", form: "Imperfect", pgn: "3mp" },
    IrregularVerb { surface: "יַצִּילֻךְ", root: "נצל", binyan: "Hiphil", form: "Imperfect", pgn: "3mp" },
    IrregularVerb { surface: "אֶקְּצוֹף", root: "קצף", binyan: "Qal", form: "Imperfect", pgn: "1cs" },
    IrregularVerb { surface: "תִּנְגֹּשׂוּ", root: "נגש", binyan: "Qal", form: "Imperfect", pgn: "2mp" },
    IrregularVerb { surface: "תֵּאָמַנָה", root: "אמן", binyan: "Niphal", form: "Imperfect", pgn: "3fp" },
    IrregularVerb { surface: "יַעַבְדוּךְ", root: "עבד", binyan: "Qal", form: "Imperfect", pgn: "3mp" },
    IrregularVerb { surface: "יְעָטָנִי", root: "עטה", binyan: "Qal", form: "Imperfect", pgn: "3ms" },
    IrregularVerb { surface: "יִבְעָלוּךְ", root: "בעל", binyan: "Qal", form: "Imperfect", pgn: "3mp" },
    IrregularVerb { surface: "אֶגְאָלְתִּי", root: "גאל", binyan: "Hiphil", form: "Perfect", pgn: "1cs" },
    IrregularVerb { surface: "יַכִּירָנוּ", root: "נכר", binyan: "Hiphil", form: "Imperfect", pgn: "3ms" },
    IrregularVerb { surface: "אֲחִתְּךָ", root: "חתת", binyan: "Hiphil", form: "Imperfect", pgn: "1cs" },
    IrregularVerb { surface: "יִרְעוּךְ", root: "רעה", binyan: "Qal", form: "Imperfect", pgn: "3mp" },
    IrregularVerb { surface: "תּוֹכִחֻךְ", root: "יכח", binyan: "Hiphil", form: "Imperfect", pgn: "3fp" },
    IrregularVerb { surface: "יִמְצָאוּנְהָ", root: "מצא", binyan: "Qal", form: "Imperfect", pgn: "3mp" },
    IrregularVerb { surface: "פָּשָׁעַתְּ", root: "פשע", binyan: "Qal", form: "Perfect", pgn: "2fs" },
    IrregularVerb { surface: "וְנִקְוּוּ", root: "קוה", binyan: "Niphal", form: "Perfect", pgn: "3cp" },
    IrregularVerb { surface: "אֶרְפָּה", root: "רפא", binyan: "Qal", form: "Imperfect", pgn: "1cs" },
    IrregularVerb { surface: "יְשָׁדְדֵם", root: "שדד", binyan: "Qal", form: "Imperfect", pgn: "3ms" },
    IrregularVerb { surface: "וְשַׁחֵתוּ", root: "שחת", binyan: "Piel", form: "Perfect", pgn: "3cp" },
    IrregularVerb { surface: "יַעַבְרֶנְהוּ", root: "עבר", binyan: "Qal", form: "Imperfect", pgn: "3ms" },
    IrregularVerb { surface: "יַעַבְרֻנְהוּ", root: "עבר", binyan: "Qal", form: "Imperfect", pgn: "3mp" },
    IrregularVerb { surface: "שָׁמַענוּ", root: "שמע", binyan: "Qal", form: "Perfect", pgn: "1cp" },
    IrregularVerb { surface: "יְהָתֵלּוּ", root: "התל", binyan: "Hiphil", form: "Imperfect", pgn: "3mp" },
    IrregularVerb { surface: "יִנָּשׂוּא", root: "נשא", binyan: "Niphal", form: "Imperfect", pgn: "3mp" },
    IrregularVerb { surface: "תַּעֲלֹזִי", root: "עלז", binyan: "Qal", form: "Imperfect", pgn: "2fs" },
    IrregularVerb { surface: "צָלֵחָה", root: "צלח", binyan: "Qal", form: "Perfect", pgn: "3fs" },
    IrregularVerb { surface: "יֹאחֱזוּךְ", root: "אחז", binyan: "Qal", form: "Imperfect", pgn: "3mp" },
    IrregularVerb { surface: "יְלִדְתִּנִי", root: "ילד", binyan: "Qal", form: "Perfect", pgn: "2fs" },
    IrregularVerb { surface: "מְקַלְלַונִי", root: "קלל", binyan: "Piel", form: "Participle (act.)", pgn: "mp" },
    IrregularVerb { surface: "אֵחַתָּה", root: "חתת", binyan: "Niphal", form: "Cohortative", pgn: "1cs" },
    IrregularVerb { surface: "תֶּמְחִי", root: "מחה", binyan: "Hiphil", form: "Jussive", pgn: "2ms" },
    IrregularVerb { surface: "וּצְעָקִי", root: "צעק", binyan: "Qal", form: "Imperative", pgn: "2fs" },
    IrregularVerb { surface: "נֵּחַנְתְּ", root: "חנן", binyan: "Niphal", form: "Perfect", pgn: "2fs" },
    IrregularVerb { surface: "אֶתְּקֶנְךָּ", root: "נתק", binyan: "Qal", form: "Imperfect", pgn: "1cs" },
    IrregularVerb { surface: "יִקְרְאוֹ", root: "קרא", binyan: "Qal", form: "Imperfect", pgn: "3ms" },
    IrregularVerb { surface: "נִבֵיתָ", root: "נבא", binyan: "Niphal", form: "Perfect", pgn: "2ms" },
    IrregularVerb { surface: "וַיְנַאֲפוּ", root: "נאף", binyan: "Piel", form: "Wayyiqtol", pgn: "3mp" },
    IrregularVerb { surface: "שֹׁאסַיִךְ", root: "שסה", binyan: "Qal", form: "Participle (act.)", pgn: "mp" },
    IrregularVerb { surface: "אֶרְפָּאֵךְ", root: "רפא", binyan: "Qal", form: "Imperfect", pgn: "1cs" },
    // ── Jeremiah 31 → Amos tail (gold-precise, OSHB) ──
    IrregularVerb { surface: "תִּתְחַמָּקִין", root: "חמק", binyan: "Hithpael", form: "Imperfect", pgn: "2fs" },
    IrregularVerb { surface: "וַנַּעַשׂ", root: "עשה", binyan: "Qal", form: "Wayyiqtol", pgn: "1cp" },
    IrregularVerb { surface: "וּקְרָאֶנָּה", root: "קרא", binyan: "Qal", form: "Imperative", pgn: "2ms" },
    IrregularVerb { surface: "לְהוֹצִאֵהוּ", root: "יצא", binyan: "Hiphil", form: "Inf. Construct", pgn: "" },
    IrregularVerb { surface: "תַגְבִּיהַ", root: "גבה", binyan: "Hiphil", form: "Imperfect", pgn: "2ms" },
    IrregularVerb { surface: "תִמָּצֶאינָה", root: "מצא", binyan: "Niphal", form: "Imperfect", pgn: "3fp" },
    IrregularVerb { surface: "יָדָעַתְּ", root: "ידע", binyan: "Qal", form: "Perfect", pgn: "2fs" },
    IrregularVerb { surface: "תִזָכַרְןָ", root: "זכר", binyan: "Niphal", form: "Imperfect", pgn: "3fp" },
    IrregularVerb { surface: "וְהִנֶּחָמְתִּי", root: "נחם", binyan: "Hithpael", form: "Perfect", pgn: "1cs" },
    IrregularVerb { surface: "תִּישָׁמְנָה", root: "ישם", binyan: "Qal", form: "Imperfect", pgn: "3fp" },
    IrregularVerb { surface: "וְנֵאשֲׁאַר", root: "שאר", binyan: "Niphal", form: "Participle (act.)", pgn: "ms" },
    IrregularVerb { surface: "וַתְּחַלֶּלְנָה", root: "חלל", binyan: "Piel", form: "Wayyiqtol", pgn: "2fp" },
    IrregularVerb { surface: "הַאִדָּרֹשׁ", root: "דרש", binyan: "Niphal", form: "Inf. Absolute", pgn: "" },
    IrregularVerb { surface: "וַתִּזְבָּחִים", root: "זבח", binyan: "Qal", form: "Wayyiqtol", pgn: "2fs" },
    IrregularVerb { surface: "וְהִנִּיחוּךְ", root: "נוח", binyan: "Hiphil", form: "Perfect", pgn: "3cp" },
    IrregularVerb { surface: "וַתִּגְבְּהֶינָה", root: "גבה", binyan: "Qal", form: "Wayyiqtol", pgn: "3fp" },
    IrregularVerb { surface: "בְּצַדֶּקְתֵּךְ", root: "צדק", binyan: "Piel", form: "Inf. Construct", pgn: "" },
    IrregularVerb { surface: "וָאִוָּדַע", root: "ידע", binyan: "Niphal", form: "Wayyiqtol", pgn: "1cs" },
    IrregularVerb { surface: "הַגְבֵּהַ", root: "גבה", binyan: "Hiphil", form: "Inf. Absolute", pgn: "" },
    IrregularVerb { surface: "יִתְקַּלְּסוּ", root: "קלס", binyan: "Hithpael", form: "Imperfect", pgn: "3mp" },
    IrregularVerb { surface: "וְהִפְשִׁיטוּךְ", root: "פשט", binyan: "Hiphil", form: "Perfect", pgn: "3cp" },
    IrregularVerb { surface: "וַעֲזָבוּךְ", root: "עזב", binyan: "Qal", form: "Perfect", pgn: "3cp" },
    IrregularVerb { surface: "תִּשֶּׂאינָה", root: "נשא", binyan: "Qal", form: "Imperfect", pgn: "2fp" },
    IrregularVerb { surface: "וּנְמַקֹּתֶם", root: "מקק", binyan: "Niphal", form: "Perfect", pgn: "2mp" },
    IrregularVerb { surface: "וָאֶחַלֶּלְךָ", root: "חלל", binyan: "Piel", form: "Wayyiqtol", pgn: "1cs" },
    IrregularVerb { surface: "עֲשִׂיתִנִי", root: "עשה", binyan: "Qal", form: "Perfect", pgn: "1cs" },
    IrregularVerb { surface: "גָּבְהָא", root: "גבה", binyan: "Qal", form: "Perfect", pgn: "3fs" },
    IrregularVerb { surface: "וַיְּיִף", root: "יפה", binyan: "Qal", form: "Wayyiqtol", pgn: "3ms" },
    IrregularVerb { surface: "חֲטֹאתוֹ", root: "חטא", binyan: "Qal", form: "Inf. Construct", pgn: "" },
    IrregularVerb { surface: "תֶּהְדֹּפוּ", root: "הדף", binyan: "Qal", form: "Imperfect", pgn: "2mp" },
    IrregularVerb { surface: "וְהֵטִבֹתִי", root: "יטב", binyan: "Hiphil", form: "Perfect", pgn: "1cs" },
    IrregularVerb { surface: "וְהִנַּבֵּאתִי", root: "נבא", binyan: "Hithpael", form: "Perfect", pgn: "1cs" },
    IrregularVerb { surface: "יִטַמְּאוּ", root: "טמא", binyan: "Hithpael", form: "Imperfect", pgn: "3mp" },
    IrregularVerb { surface: "וְהִתְגַּדִּלְתִּי", root: "גדל", binyan: "Hithpael", form: "Perfect", pgn: "1cs" },
    IrregularVerb { surface: "וְרָצִאתִי", root: "רצה", binyan: "Qal", form: "Perfect", pgn: "1cs" },
    IrregularVerb { surface: "לֶאֱכָול", root: "אכל", binyan: "Qal", form: "Inf. Construct", pgn: "" },
    IrregularVerb { surface: "מְהֻקְצָעוֹת", root: "קצע", binyan: "Hophal", form: "Participle (act.)", pgn: "fp" },
    IrregularVerb { surface: "וְאֶמְאָסְאךָ", root: "מאס", binyan: "Qal", form: "Imperfect", pgn: "1cs" },
    IrregularVerb { surface: "תְּנָאַפְנָה", root: "נאף", binyan: "Piel", form: "Imperfect", pgn: "3fp" },
    IrregularVerb { surface: "תְנָאַפְנָה", root: "נאף", binyan: "Piel", form: "Imperfect", pgn: "3fp" },
    IrregularVerb { surface: "חֻמְצָתוֹ", root: "חמץ", binyan: "Qal", form: "Inf. Construct", pgn: "" },
    IrregularVerb { surface: "אַיְסִרֵם", root: "יסר", binyan: "Hiphil", form: "Imperfect", pgn: "1cs" },
    IrregularVerb { surface: "יִרְדְּפוֹ", root: "רדף", binyan: "Qal", form: "Imperfect", pgn: "3mp" },
    IrregularVerb { surface: "יִטַמָּאוּ", root: "טמא", binyan: "Hithpael", form: "Imperfect", pgn: "3mp" },
    IrregularVerb { surface: "וְקָאם", root: "קום", binyan: "Qal", form: "Perfect", pgn: "3ms" },
    IrregularVerb { surface: "שִׁחֶתְךָ", root: "שחת", binyan: "Piel", form: "Perfect", pgn: "3ms" },
    IrregularVerb { surface: "וְהִשְׁלַכְתֶּנָה", root: "שלך", binyan: "Hiphil", form: "Perfect", pgn: "2fp" },
    IrregularVerb { surface: "יְתָעֵבוּ", root: "תעב", binyan: "Piel", form: "Imperfect", pgn: "3mp" },
    IrregularVerb { surface: "יֶחֱנַן", root: "חנן", binyan: "Qal", form: "Imperfect", pgn: "3ms" },
    IrregularVerb { surface: "תִּתְעַלַּפְנָה", root: "עלף", binyan: "Hithpael", form: "Imperfect", pgn: "3fp" },
    IrregularVerb { surface: "וּבְצַעַם", root: "בצע", binyan: "Qal", form: "Imperative", pgn: "2ms" },
    IrregularVerb { surface: "אֹיבֵיהֶם", root: "איב", binyan: "Qal", form: "Participle (act.)", pgn: "mp" },
    IrregularVerb { surface: "תַּגְבִּיהַּ", root: "גבה", binyan: "Hiphil", form: "Imperfect", pgn: "2ms" },
    // ── Micah → Psalms 103 tail (gold-precise, OSHB) ──
    IrregularVerb { surface: "נְשַׁדֻּנוּ", root: "שדד", binyan: "Niphal", form: "Perfect", pgn: "1cp" },
    IrregularVerb { surface: "תֵּאתֶה", root: "אתה", binyan: "Qal", form: "Imperfect", pgn: "3fs" },
    IrregularVerb { surface: "הֶחֱזִיקֵךְ", root: "חזק", binyan: "Hiphil", form: "Perfect", pgn: "3ms" },
    IrregularVerb { surface: "הֲתֵיטְבִי", root: "יטב", binyan: "Hiphil", form: "Imperfect", pgn: "2fs" },
    IrregularVerb { surface: "נִכְבַּדֶּיהָ", root: "כבד", binyan: "Niphal", form: "Participle (act.)", pgn: "mp" },
    IrregularVerb { surface: "תְּמָהוּ", root: "תמה", binyan: "Qal", form: "Imperative", pgn: "2mp" },
    IrregularVerb { surface: "שַׂמְתּוֹ", root: "שים", binyan: "Qal", form: "Perfect", pgn: "2ms" },
    IrregularVerb { surface: "יְחִיתַן", root: "חתת", binyan: "Hiphil", form: "Imperfect", pgn: "3ms" },
    IrregularVerb { surface: "יְבָזּוּם", root: "בזז", binyan: "Qal", form: "Imperfect", pgn: "3mp" },
    IrregularVerb { surface: "פוּצַי", root: "פוץ", binyan: "Qal", form: "Participle (act.)", pgn: "ms" },
    IrregularVerb { surface: "נוּגֵי", root: "יגה", binyan: "Niphal", form: "Participle (act.)", pgn: "mp" },
    IrregularVerb { surface: "תְּבַצַּעְנָה", root: "בצע", binyan: "Piel", form: "Imperfect", pgn: "3fp" },
    IrregularVerb { surface: "וְלָנֶה", root: "לון", binyan: "Qal", form: "Perfect", pgn: "3fs" },
    IrregularVerb { surface: "וַתִּתְהַלַּכְנָה", root: "הלך", binyan: "Hithpael", form: "Wayyiqtol", pgn: "3fp" },
    IrregularVerb { surface: "צַמְתֻּנִי", root: "צום", binyan: "Qal", form: "Perfect", pgn: "2mp" },
    IrregularVerb { surface: "וְאֵסָעֲרֵם", root: "סער", binyan: "Piel", form: "Imperfect", pgn: "1cs" },
    IrregularVerb { surface: "וְהוֹשְׁבוֹתִים", root: "שוב", binyan: "Hiphil", form: "Perfect", pgn: "1cs" },
    IrregularVerb { surface: "וַאעְשִׁר", root: "עשר", binyan: "Hiphil", form: "Imperfect", pgn: "1cs" },
    IrregularVerb { surface: "חֲדָלוּ", root: "חדל", binyan: "Qal", form: "Imperative", pgn: "2mp" },
    IrregularVerb { surface: "בְּהִנָּבְאֹתוֹ", root: "נבא", binyan: "Niphal", form: "Inf. Construct", pgn: "" },
    IrregularVerb { surface: "הִלָּחֲמו", root: "לחם", binyan: "Niphal", form: "Inf. Construct", pgn: "" },
    IrregularVerb { surface: "גֵאַלְנוּךָ", root: "גאל", binyan: "Piel", form: "Perfect", pgn: "1cp" },
    IrregularVerb { surface: "אָרוֹתִיהָ", root: "ארר", binyan: "Qal", form: "Perfect", pgn: "1cs" },
    IrregularVerb { surface: "נֵאָרִים", root: "ארר", binyan: "Niphal", form: "Participle (act.)", pgn: "mp" },
    IrregularVerb { surface: "וּפִשְׁתֶּם", root: "פוש", binyan: "Qal", form: "Perfect", pgn: "2mp" },
    IrregularVerb { surface: "יִרַדֹּף", root: "רדף", binyan: "Qal", form: "Imperfect", pgn: "3ms" },
    IrregularVerb { surface: "יְעָצָנִי", root: "יעץ", binyan: "Qal", form: "Perfect", pgn: "3ms" },
    IrregularVerb { surface: "יִשְׂבּעוּ", root: "שבע", binyan: "Qal", form: "Imperfect", pgn: "3mp" },
    IrregularVerb { surface: "וַיֵּדֶא", root: "נדא", binyan: "Hiphil", form: "Wayyiqtol", pgn: "3ms" },
    IrregularVerb { surface: "יְדַשְּׁנֶה", root: "דשן", binyan: "Piel", form: "Imperfect", pgn: "3ms" },
    IrregularVerb { surface: "וְנַשְּׂאֵם", root: "נשא", binyan: "Piel", form: "Imperative", pgn: "2ms" },
    IrregularVerb { surface: "תִּלְכְּדוֹ", root: "לכד", binyan: "Qal", form: "Imperfect", pgn: "3fs" },
    IrregularVerb { surface: "יִרְוִיֻן", root: "רוה", binyan: "Qal", form: "Imperfect", pgn: "3mp" },
    IrregularVerb { surface: "חֲמוּדוֹ", root: "חמד", binyan: "Qal", form: "Participle (act.)", pgn: "ms" },
    IrregularVerb { surface: "קַוּה", root: "קוה", binyan: "Piel", form: "Imperative", pgn: "2ms" },
    IrregularVerb { surface: "עֲזָבָנִי", root: "עזב", binyan: "Qal", form: "Perfect", pgn: "3ms" },
    IrregularVerb { surface: "וְיִתְאָו", root: "אוה", binyan: "Hithpael", form: "Imperfect", pgn: "3ms" },
    IrregularVerb { surface: "יְכַבְּדָנְנִי", root: "כבד", binyan: "Piel", form: "Imperfect", pgn: "3ms" },
    IrregularVerb { surface: "יֶחֱמַתְנִי", root: "יחם", binyan: "Piel", form: "Perfect", pgn: "3fs" },
    IrregularVerb { surface: "פַּלַּג", root: "פלג", binyan: "Piel", form: "Imperative", pgn: "2ms" },
    IrregularVerb { surface: "וְהוֹרִידֵמוֹ", root: "ירד", binyan: "Hiphil", form: "Imperative", pgn: "2ms" },
    IrregularVerb { surface: "תְּרָצְּחוּ", root: "רצח", binyan: "Piel", form: "Imperfect", pgn: "2mp" },
    IrregularVerb { surface: "יְשַׁבְּחוּנְךָ", root: "שבח", binyan: "Piel", form: "Imperfect", pgn: "3mp" },
    IrregularVerb { surface: "לַמְנַצֵּח", root: "נצח", binyan: "Piel", form: "Participle (act.)", pgn: "ms" },
    IrregularVerb { surface: "יְחָנֵּנוּ", root: "חנן", binyan: "Qal", form: "Imperfect", pgn: "3ms" },
    IrregularVerb { surface: "תִּנְדֹּף", root: "נדף", binyan: "Qal", form: "Imperfect", pgn: "2ms" },
    IrregularVerb { surface: "תּוֹפֵפוֹת", root: "תפף", binyan: "Qal", form: "Participle (act.)", pgn: "fp" },
    IrregularVerb { surface: "וְתִפְשֻׂוּהוּ", root: "תפש", binyan: "Qal", form: "Imperative", pgn: "2mp" },
    IrregularVerb { surface: "יְבָרֲכֶנְהוּ", root: "ברך", binyan: "Piel", form: "Jussive", pgn: "3ms" },
    IrregularVerb { surface: "עֲנָקַתְמוֹ", root: "ענק", binyan: "Qal", form: "Perfect", pgn: "3fs" },
    IrregularVerb { surface: "תִּהֲלַךְ", root: "הלך", binyan: "Qal", form: "Imperfect", pgn: "3fs" },
    IrregularVerb { surface: "דִכִּאתָ", root: "דכא", binyan: "Piel", form: "Perfect", pgn: "2ms" },
    IrregularVerb { surface: "יִשָּׂאוּנְךָ", root: "נשא", binyan: "Qal", form: "Imperfect", pgn: "3mp" },
    IrregularVerb { surface: "הַיְחָבְרְךָ", root: "חבר", binyan: "Qal", form: "Imperfect", pgn: "3ms" },
    IrregularVerb { surface: "קֹרִאים", root: "קרא", binyan: "Qal", form: "Participle (act.)", pgn: "mp" },
    IrregularVerb { surface: "בְּנֶאֶמְנֵי", root: "אמן", binyan: "Niphal", form: "Participle (act.)", pgn: "mp" },
    IrregularVerb { surface: "לְחֶנְנָהּ", root: "חנן", binyan: "Qal", form: "Inf. Construct", pgn: "" },
    IrregularVerb { surface: "הַמְעַטְּרֵכִי", root: "עטר", binyan: "Piel", form: "Participle (act.)", pgn: "ms" },
    // ── Psalms 113 → Job 39 tail (gold-precise, OSHB) ──
    IrregularVerb { surface: "הַמַּגְבִּיהִי", root: "גבה", binyan: "Hiphil", form: "Participle (act.)", pgn: "ms" },
    IrregularVerb { surface: "עֲזָרָנִי", root: "עזר", binyan: "Qal", form: "Perfect", pgn: "3ms" },
    IrregularVerb { surface: "נְתָנָנִי", root: "נתן", binyan: "Qal", form: "Perfect", pgn: "3ms" },
    IrregularVerb { surface: "בֵּרַכְנוּכֶם", root: "ברך", binyan: "Piel", form: "Perfect", pgn: "1cp" },
    IrregularVerb { surface: "תַּבַּעְנָה", root: "נבע", binyan: "Hiphil", form: "Imperfect", pgn: "3fs" },
    IrregularVerb { surface: "שֶׁיְּחָנֵּנוּ", root: "חנן", binyan: "Qal", form: "Imperfect", pgn: "3ms" },
    IrregularVerb { surface: "מְצָאנוּהָ", root: "מצא", binyan: "Qal", form: "Perfect", pgn: "1cp" },
    IrregularVerb { surface: "אֶזְכְּרֵכִי", root: "זכר", binyan: "Qal", form: "Imperfect", pgn: "1cs" },
    IrregularVerb { surface: "יְיֵדָע", root: "ידע", binyan: "Qal", form: "Imperfect", pgn: "3ms" },
    IrregularVerb { surface: "צַרְתָּנִי", root: "צור", binyan: "Qal", form: "Perfect", pgn: "2ms" },
    IrregularVerb { surface: "מְסִבָּי", root: "סבב", binyan: "Hiphil", form: "Participle (act.)", pgn: "mp" },
    IrregularVerb { surface: "וְנִכְבְּדֵיהֶם", root: "כבד", binyan: "Niphal", form: "Participle (act.)", pgn: "mp" },
    IrregularVerb { surface: "תְּאֵהֲבוּ", root: "אהב", binyan: "Qal", form: "Imperfect", pgn: "2mp" },
    IrregularVerb { surface: "תִנְצְרֶכָּה", root: "נצר", binyan: "Qal", form: "Imperfect", pgn: "3fs" },
    IrregularVerb { surface: "יַיְשִׁרוּ", root: "ישר", binyan: "Hiphil", form: "Jussive", pgn: "3mp" },
    IrregularVerb { surface: "בַּעֲזוֹז", root: "עזז", binyan: "Qal", form: "Inf. Construct", pgn: "" },
    IrregularVerb { surface: "וַחֲכָמוּ", root: "חכם", binyan: "Qal", form: "Imperative", pgn: "2mp" },
    IrregularVerb { surface: "מִמְּתַכַּבֵּד", root: "כבד", binyan: "Hithpael", form: "Participle (act.)", pgn: "ms" },
    IrregularVerb { surface: "תִּשְׁמוּרֵם", root: "שמר", binyan: "Qal", form: "Imperfect", pgn: "3fs" },
    IrregularVerb { surface: "מַגְבִּיהַּ", root: "גבה", binyan: "Hiphil", form: "Participle (act.)", pgn: "ms" },
    IrregularVerb { surface: "חַדַל", root: "חדל", binyan: "Qal", form: "Imperative", pgn: "2ms" },
    IrregularVerb { surface: "יְלָדֶךָ", root: "ילד", binyan: "Qal", form: "Perfect", pgn: "3ms" },
    IrregularVerb { surface: "וַיֶּאֱתָיֵנִי", root: "אתה", binyan: "Qal", form: "Wayyiqtol", pgn: "3ms" },
    IrregularVerb { surface: "וַתֵּלֶא", root: "לאה", binyan: "Qal", form: "Wayyiqtol", pgn: "2ms" },
    IrregularVerb { surface: "יִתְפַלָּצוּן", root: "פלץ", binyan: "Hithpael", form: "Imperfect", pgn: "3mp" },
    IrregularVerb { surface: "יַשְׂבִּעַנִי", root: "שבע", binyan: "Hiphil", form: "Imperfect", pgn: "3ms" },
    IrregularVerb { surface: "הִתְרָחַצְתִּי", root: "רחץ", binyan: "Hithpael", form: "Perfect", pgn: "1cs" },
    IrregularVerb { surface: "אַדַבְּרָה", root: "דבר", binyan: "Piel", form: "Cohortative", pgn: "1cs" },
    IrregularVerb { surface: "יַחֲרִישׁו", root: "חרש", binyan: "Hiphil", form: "Imperfect", pgn: "3mp" },
    IrregularVerb { surface: "תָּעֻפָה", root: "עוף", binyan: "Qal", form: "Imperfect", pgn: "3fs" },
    IrregularVerb { surface: "תְּהָתֵלּוּ", root: "התל", binyan: "Hiphil", form: "Imperfect", pgn: "2mp" },
    IrregularVerb { surface: "אֲאַמִּצְכֶם", root: "אמץ", binyan: "Piel", form: "Imperfect", pgn: "1cs" },
    IrregularVerb { surface: "וַתֵּכַהּ", root: "כהה", binyan: "Qal", form: "Wayyiqtol", pgn: "3fs" },
    IrregularVerb { surface: "יֶהְדְּפֻהוּ", root: "הדף", binyan: "Qal", form: "Imperfect", pgn: "3mp" },
    IrregularVerb { surface: "יְנִדֻּהוּ", root: "נדד", binyan: "Hiphil", form: "Imperfect", pgn: "3mp" },
    IrregularVerb { surface: "וּתְדַכּאוּנַנִי", root: "דכא", binyan: "Piel", form: "Imperfect", pgn: "2mp" },
    IrregularVerb { surface: "עִוְּתָנִי", root: "עות", binyan: "Piel", form: "Perfect", pgn: "3ms" },
    IrregularVerb { surface: "בַּחֲמוּדוֹ", root: "חמד", binyan: "Qal", form: "Participle (act.)", pgn: "ms" },
    IrregularVerb { surface: "תְּאָכְלֵהוּ", root: "אכל", binyan: "Qal", form: "Imperfect", pgn: "3fs" },
    IrregularVerb { surface: "וּשְׁלם", root: "שלם", binyan: "Piel", form: "Perfect", pgn: "3ms" },
    IrregularVerb { surface: "בְּרִבָם", root: "ריב", binyan: "Niphal", form: "Inf. Construct", pgn: "" },
    IrregularVerb { surface: "יְחִתֵּנִי", root: "חתת", binyan: "Hiphil", form: "Imperfect", pgn: "3ms" },
    IrregularVerb { surface: "וְיִרְוַח", root: "רוח", binyan: "Qal", form: "Imperfect", pgn: "3ms" },
    IrregularVerb { surface: "עָשָׂתְנִי", root: "עשה", binyan: "Qal", form: "Perfect", pgn: "3fs" },
    IrregularVerb { surface: "עֶרְכָה", root: "ערך", binyan: "Qal", form: "Imperative", pgn: "2ms" },
    IrregularVerb { surface: "רִיבוֹתָ", root: "ריב", binyan: "Qal", form: "Perfect", pgn: "2ms" },
    IrregularVerb { surface: "מְצָאנֻהוּ", root: "מצא", binyan: "Qal", form: "Perfect", pgn: "1cp" },
    IrregularVerb { surface: "תְּמַלֶּאנָה", root: "מלא", binyan: "Piel", form: "Imperfect", pgn: "3fp" },
    IrregularVerb { surface: "תְּפַלַּחְנָה", root: "פלח", binyan: "Piel", form: "Imperfect", pgn: "3fp" },
    IrregularVerb { surface: "יְגַמֶּא", root: "גמא", binyan: "Piel", form: "Imperfect", pgn: "3ms" },
    IrregularVerb { surface: "אֱצֹּר", root: "נצר", binyan: "Qal", form: "Imperfect", pgn: "1cs" },
    IrregularVerb { surface: "יִלְכְּדֻנוֹ", root: "לכד", binyan: "Qal", form: "Imperfect", pgn: "3mp" },
    IrregularVerb { surface: "יְיַשֶׁר", root: "ישר", binyan: "Piel", form: "Imperfect", pgn: "3ms" },
    IrregularVerb { surface: "תִּתְאָו", root: "תאה", binyan: "Piel", form: "Imperfect", pgn: "2mp" },
    IrregularVerb { surface: "תּוֹסְףְּ", root: "יסף", binyan: "Hiphil", form: "Jussive", pgn: "2ms" },
    IrregularVerb { surface: "יִחַדְּ", root: "יחד", binyan: "Qal", form: "Jussive", pgn: "3ms" },
    IrregularVerb { surface: "תְּשַׁלַּחְנָה", root: "שלח", binyan: "Piel", form: "Imperfect", pgn: "3fp" },
    IrregularVerb { surface: "גּבֶר", root: "גבר", binyan: "Qal", form: "Perfect", pgn: "3ms" },
    IrregularVerb { surface: "דִּכָּא", root: "דכא", binyan: "Piel", form: "Perfect", pgn: "3ms" },
    // ── Job 39 → 2Chr 36 tail (gold-precise, OSHB) ──
    IrregularVerb { surface: "אִוָּלֶד", root: "ילד", binyan: "Niphal", form: "Imperfect", pgn: "1cs" },
    IrregularVerb { surface: "יַגְבִּיהַּ", root: "גבה", binyan: "Hiphil", form: "Imperfect", pgn: "3ms" },
    IrregularVerb { surface: "יִנְקָב", root: "נקב", binyan: "Qal", form: "Imperfect", pgn: "3ms" },
    IrregularVerb { surface: "נָאווּ", root: "נאה", binyan: "Qal", form: "Perfect", pgn: "3cp" },
    IrregularVerb { surface: "צְאֶינָה", root: "יצא", binyan: "Qal", form: "Imperative", pgn: "2fp" },
    IrregularVerb { surface: "וּרְאֶינָה", root: "ראה", binyan: "Qal", form: "Imperative", pgn: "2fp" },
    IrregularVerb { surface: "אֶשָׁקְךָ", root: "נשק", binyan: "Qal", form: "Imperfect", pgn: "1cs" },
    IrregularVerb { surface: "כְּמוֹצְאֵת", root: "מצא", binyan: "Hiphil", form: "Participle (act.)", pgn: "fs" },
    IrregularVerb { surface: "וּמְצֶאןָ", root: "מצא", binyan: "Qal", form: "Imperative", pgn: "2fp" },
    IrregularVerb { surface: "לֵכְןָ", root: "הלך", binyan: "Qal", form: "Imperative", pgn: "2fp" },
    IrregularVerb { surface: "קְרֶאןָ", root: "קרא", binyan: "Qal", form: "Imperative", pgn: "2fp" },
    IrregularVerb { surface: "וַאֲלַקֳטָּה", root: "לקט", binyan: "Piel", form: "Cohortative", pgn: "1cs" },
    IrregularVerb { surface: "תַעֲבוּרִי", root: "עבר", binyan: "Qal", form: "Imperfect", pgn: "2fs" },
    IrregularVerb { surface: "תִדְבָּקִין", root: "דבק", binyan: "Qal", form: "Imperfect", pgn: "2fs" },
    IrregularVerb { surface: "תִּדְבָּקִין", root: "דבק", binyan: "Qal", form: "Imperfect", pgn: "2fs" },
    IrregularVerb { surface: "תַּעַשִׂין", root: "עשה", binyan: "Qal", form: "Imperfect", pgn: "2fs" },
    IrregularVerb { surface: "וְאֶחֳזִי", root: "אחז", binyan: "Qal", form: "Imperative", pgn: "2fs" },
    IrregularVerb { surface: "וַתֹּאחֶז", root: "אחז", binyan: "Qal", form: "Wayyiqtol", pgn: "3fs" },
    IrregularVerb { surface: "אֲהֵבַתֶךְ", root: "אהב", binyan: "Qal", form: "Perfect", pgn: "3fs" },
    IrregularVerb { surface: "נּוּגוֹת", root: "יגה", binyan: "Niphal", form: "Participle (act.)", pgn: "fp" },
    IrregularVerb { surface: "נִאֵר", root: "נאר", binyan: "Piel", form: "Perfect", pgn: "3ms" },
    IrregularVerb { surface: "שֶׁקִּוִּינֻהוּ", root: "קוה", binyan: "Piel", form: "Perfect", pgn: "1cp" },
    IrregularVerb { surface: "וַיַּקַּף", root: "נקף", binyan: "Hiphil", form: "Wayyiqtol", pgn: "3ms" },
    IrregularVerb { surface: "לְקֹוָו", root: "קוה", binyan: "Qal", form: "Participle (act.)", pgn: "mp" },
    IrregularVerb { surface: "לְיַאֵשׁ", root: "יאש", binyan: "Piel", form: "Inf. Construct", pgn: "" },
    IrregularVerb { surface: "יִתְקְפוֹ", root: "תקף", binyan: "Qal", form: "Imperfect", pgn: "3ms" },
    IrregularVerb { surface: "לֶאֶכוֹל", root: "אכל", binyan: "Qal", form: "Inf. Construct", pgn: "" },
    IrregularVerb { surface: "יוּקָשִׁים", root: "יקש", binyan: "Pual", form: "Participle (act.)", pgn: "mp" },
    IrregularVerb { surface: "קֵהָה", root: "קהה", binyan: "Piel", form: "Perfect", pgn: "3ms" },
    IrregularVerb { surface: "יוֹדֵע", root: "ידע", binyan: "Qal", form: "Participle (act.)", pgn: "ms" },
    IrregularVerb { surface: "וִיטִּיבְךָ", root: "יטב", binyan: "Hiphil", form: "Jussive", pgn: "3ms" },
    IrregularVerb { surface: "וְיָנֵאץ", root: "נאץ", binyan: "Hiphil", form: "Imperfect", pgn: "3ms" },
    IrregularVerb { surface: "וַתָּבוֹאינָה", root: "בוא", binyan: "Qal", form: "Wayyiqtol", pgn: "3fp" },
    IrregularVerb { surface: "נִקְרֵאתי", root: "קרא", binyan: "Niphal", form: "Perfect", pgn: "1cs" },
    IrregularVerb { surface: "וַתִּתְפָּעֶם", root: "פעם", binyan: "Hithpael", form: "Wayyiqtol", pgn: "3fs" },
    IrregularVerb { surface: "יַעֲמֹדְנָה", root: "עמד", binyan: "Qal", form: "Imperfect", pgn: "3fp" },
    IrregularVerb { surface: "יִנַּשְּׂאוּ", root: "נשא", binyan: "Hithpael", form: "Imperfect", pgn: "3mp" },
    IrregularVerb { surface: "הִתְחַבְּרוּת", root: "חבר", binyan: "Hithpael", form: "Inf. Construct", pgn: "" },
    IrregularVerb { surface: "וּמְתֻרְגָּם", root: "תרגם", binyan: "Pual", form: "Participle (act.)", pgn: "ms" },
    IrregularVerb { surface: "פְתֻוּחוֹת", root: "פתח", binyan: "Qal", form: "Participle (act.)", pgn: "fp" },
    IrregularVerb { surface: "בוֹדָאם", root: "בדא", binyan: "Qal", form: "Participle (act.)", pgn: "ms" },
    IrregularVerb { surface: "לְרַמּוֹתַנִי", root: "רמה", binyan: "Piel", form: "Inf. Construct", pgn: "" },
    IrregularVerb { surface: "וַיִּקָהֲלוּ", root: "קהל", binyan: "Niphal", form: "Wayyiqtol", pgn: "3mp" },
    IrregularVerb { surface: "וַיֶּחֱלֶא", root: "חלא", binyan: "Qal", form: "Wayyiqtol", pgn: "3ms" },
    IrregularVerb { surface: "נְתַנּוּךָ", root: "נתן", binyan: "Qal", form: "Perfect", pgn: "1cp" },
    IrregularVerb { surface: "לִירוֹא", root: "ירה", binyan: "Qal", form: "Inf. Construct", pgn: "" },
    IrregularVerb { surface: "וַיְסֻכוּם", root: "סוך", binyan: "Qal", form: "Wayyiqtol", pgn: "3mp" },
    IrregularVerb { surface: "הִטֶּהָרוּ", root: "טהר", binyan: "Hithpael", form: "Perfect", pgn: "3cp" },
    IrregularVerb { surface: "תִּשְׁתַּחֲוּוּ", root: "שחה", binyan: "Hishtaphel", form: "Imperfect", pgn: "2mp" },
    IrregularVerb { surface: "וַיִּנַּשֵּׂא", root: "נשא", binyan: "Hithpael", form: "Wayyiqtol", pgn: "3ms" },
    IrregularVerb { surface: "וַיַּישְּׁרֵם", root: "ישר", binyan: "Piel", form: "Wayyiqtol", pgn: "3ms" },
    IrregularVerb { surface: "וַיֶּתַע", root: "תעה", binyan: "Hiphil", form: "Wayyiqtol", pgn: "3ms" },
    IrregularVerb { surface: "וַיַּגְבִּיהֶהָ", root: "גבה", binyan: "Hiphil", form: "Wayyiqtol", pgn: "3ms" },
    IrregularVerb { surface: "וְהֵעָתֶר", root: "עתר", binyan: "Niphal", form: "Inf. Construct", pgn: "" },
    IrregularVerb { surface: "וְהָכִונוּ", root: "כון", binyan: "Hiphil", form: "Imperative", pgn: "2mp" },
    IrregularVerb { surface: "לִמְעָול", root: "מעל", binyan: "Qal", form: "Inf. Construct", pgn: "" },
    IrregularVerb { surface: "דְּעֶה", root: "ידע", binyan: "Qal", form: "Imperative", pgn: "2ms" },
    IrregularVerb { surface: "שְׁתוֹ", root: "שתה", binyan: "Qal", form: "Inf. Absolute", pgn: "" },
    IrregularVerb { surface: "צַדְּקֶךָּ", root: "צדק", binyan: "Piel", form: "Inf. Construct", pgn: "" },
    // Hithpael with the prefix-ת assimilated into the dageshed C1 (תִּתְכַּסֶּה →
    // תִּכַּסֶּה), Prov 26:26 "hatred covers itself".
    IrregularVerb { surface: "תִּכַּסֶּה", root: "כסה", binyan: "Hithpael", form: "Imperfect", pgn: "3fs" },
    // III-He רבה Piel imperative 2ms "increase your army" (Judg 9:29).
    IrregularVerb { surface: "רַבֶּה", root: "רבה", binyan: "Piel", form: "Imperative", pgn: "2ms" },
    // III-He רדה Hiphil imperfect 3ms, apocopated "he subdues nations" (Isa 41:2).
    IrregularVerb { surface: "יַרְדְּ", root: "רדה", binyan: "Hiphil", form: "Imperfect", pgn: "3ms" },
    // III-aleph שׁנא "change" Qal imperfect 3ms "the fine gold is changed" (Lam 4:1).
    IrregularVerb { surface: "יִשְׁנֶא", root: "שנא", binyan: "Qal", form: "Imperfect", pgn: "3ms" },
    // שׁנא "change" Pual imperfect 3ms "the boldness of his face is changed" (Eccl 8:1).
    IrregularVerb { surface: "יְשֻׁנֶּא", root: "שנא", binyan: "Pual", form: "Imperfect", pgn: "3ms" },
];

/// Attested unmodeled-stem verb surfaces, harvested from gold (see module docs).
pub const IRREGULAR_VERBS: &[IrregularVerb] = &[
    IrregularVerb {
        // Denominal שׂמאל "go to the left" (from שְׂמֹאל) — a quadriliteral the
        // triliteral generator can't represent, so list its attested Hiphil
        // forms. Cohortative 1cs, Gen 13:9 (וְאַשְׂמְאִילָה, "I will go left");
        // the counterpart ימן "go right" is already curated above.
        surface: "וְאַשְׂמְאִילָה",
        root: "שמאל",
        binyan: "Hiphil",
        form: "Cohortative",
        pgn: "1cs",
    },
    IrregularVerb {
        // שׂמאל Hiphil imperfect 2mp, Isa 30:21 (תַשְׂמְאִילוּ).
        surface: "תַשְׂמְאִילוּ",
        root: "שמאל",
        binyan: "Hiphil",
        form: "Imperfect",
        pgn: "2mp",
    },
    IrregularVerb {
        // שׂמאל Hiphil participle mp, 1 Chr 12:2 (וּמַשְׂמִאלִים), paired with
        // the curated ימן participle מַיְמִינִים from the same verse.
        surface: "וּמַשְׂמִאלִים",
        root: "שמאל",
        binyan: "Hiphil",
        form: "Participle (act.)",
        pgn: "mp",
    },
    IrregularVerb {
        surface: "אֲכַלְכֵּל",
        root: "כול",
        binyan: "Pilpel",
        form: "Imperfect",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "אֲצַפְצֵף",
        root: "צפפ",
        binyan: "Pilpel",
        form: "Imperfect",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "אֲקוֹמֵם",
        root: "לבב",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "אֲרוֹמְמֶךָּ",
        root: "רומ",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "אֲרוֹמִמְךָ",
        root: "רומ",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "אֲרוֹמִמְךָ",
        root: "רומ",
        binyan: "Polel",
        form: "Cohortative",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "אֲשׂוֹחֵחַ",
        root: "שיח",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "אֵרוֹמָם",
        root: "רומ",
        binyan: "Hithpolel",
        form: "Imperfect",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "אֶשְׁתַּחֲוֶה",
        root: "שחה",
        binyan: "Hishtaphel",
        form: "Imperfect",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "אֶשְׁתַּעֲשָׁע",
        root: "שעע",
        binyan: "Hithpalpel",
        form: "Imperfect",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "אֶשְׁתּוֹלְלוּ",
        root: "שלל",
        binyan: "Hithpolel",
        form: "Perfect",
        pgn: "3cp",
    },
    IrregularVerb {
        surface: "אֶתְבּוֹנֵן",
        root: "בינ",
        binyan: "Hithpolel",
        form: "Imperfect",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "אֶתְבּוֹנָן",
        root: "בינ",
        binyan: "Hithpolel",
        form: "Imperfect",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "אֶתְקוֹטָט",
        root: "קוט",
        binyan: "Hithpolel",
        form: "Imperfect",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "אֶתְרוֹעָע",
        root: "רוע",
        binyan: "Hithpolel",
        form: "Imperfect",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "אֻמְלְלָה",
        root: "אמל",
        binyan: "Pulal",
        form: "Perfect",
        pgn: "3fs",
    },
    IrregularVerb {
        surface: "אֻמְלְלוּ",
        root: "אמל",
        binyan: "Pulal",
        form: "Perfect",
        pgn: "3cp",
    },
    IrregularVerb {
        surface: "אֻמְלָל",
        root: "אמל",
        binyan: "Pulal",
        form: "Perfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "אֻמְלָלָה",
        root: "אמל",
        binyan: "Pulal",
        form: "Perfect",
        pgn: "3fs",
    },
    IrregularVerb {
        surface: "אֻמְלָלוּ",
        root: "אמל",
        binyan: "Pulal",
        form: "Perfect",
        pgn: "3cp",
    },
    IrregularVerb {
        surface: "בְּהִשְׁתַּחֲוָיָתִי",
        root: "שחה",
        binyan: "Hishtaphel",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        surface: "בְּסַּאסְּאָה",
        root: "",
        binyan: "Pilpel",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        surface: "בּוֹשַׁסְכֶם",
        root: "בשס",
        binyan: "Poel",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        surface: "הִסְתּוֹפֵף",
        root: "ספפ",
        binyan: "Hithpoel",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        surface: "הִשְׁתַּחֲוֵיתִי",
        root: "שחה",
        binyan: "Hishtaphel",
        form: "Perfect",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "הִתְבֹּנַנְתָּ",
        root: "בינ",
        binyan: "Hithpolel",
        form: "Perfect",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "הִתְבּוֹנְנוּ",
        root: "בינ",
        binyan: "Hithpolel",
        form: "Imperative",
        pgn: "2mp",
    },
    IrregularVerb {
        surface: "הִתְבּוֹנָן",
        root: "בינ",
        binyan: "Hithpolel",
        form: "Perfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "הִתְבּוֹנָנוּ",
        root: "בינ",
        binyan: "Hithpolel",
        form: "Perfect",
        pgn: "3cp",
    },
    IrregularVerb {
        surface: "הִתְגַּלְגָּלוּ",
        root: "גלל",
        binyan: "Hithpalpel",
        form: "Perfect",
        pgn: "3cp",
    },
    IrregularVerb {
        surface: "הִתְמַהְמְהָם",
        root: "מהה",
        binyan: "Hithpalpel",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        surface: "הִתְמַהְמְהוּ",
        root: "מהה",
        binyan: "Hithpalpel",
        form: "Imperative",
        pgn: "2mp",
    },
    IrregularVerb {
        surface: "הִתְמַהְמָהְנוּ",
        root: "מהה",
        binyan: "Hithpalpel",
        form: "Perfect",
        pgn: "1cp",
    },
    IrregularVerb {
        surface: "הִתְמַהְמָהְתִּי",
        root: "מהה",
        binyan: "Hithpalpel",
        form: "Perfect",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "הִתְמֹגָגוּ",
        root: "מוג",
        binyan: "Hithpolel",
        form: "Perfect",
        pgn: "3cp",
    },
    IrregularVerb {
        surface: "הִתְמוֹטְטָה",
        root: "מוט",
        binyan: "Hithpolel",
        form: "Perfect",
        pgn: "3fs",
    },
    IrregularVerb {
        surface: "הִתְפּוֹרְרָה",
        root: "פרר",
        binyan: "Hithpolel",
        form: "Perfect",
        pgn: "3fs",
    },
    IrregularVerb {
        surface: "הִתְקַלְקָלוּ",
        root: "קלל",
        binyan: "Hithpalpel",
        form: "Perfect",
        pgn: "3cp",
    },
    IrregularVerb {
        surface: "הִתְקוֹשְׁשׁוּ",
        root: "קשש",
        binyan: "Hithpolel",
        form: "Imperative",
        pgn: "2mp",
    },
    IrregularVerb {
        surface: "הִתְרֹעֲעָה",
        root: "רעע",
        binyan: "Hithpolel",
        form: "Perfect",
        pgn: "3fs",
    },
    IrregularVerb {
        surface: "הִתְרֹעָעִי",
        root: "רוע",
        binyan: "Hithpolel",
        form: "Imperative",
        pgn: "2fs",
    },
    IrregularVerb {
        surface: "הַמְצַפְצְפִים",
        root: "צפפ",
        binyan: "Pilpel",
        form: "Participle (act.)",
        pgn: "mp",
    },
    IrregularVerb {
        surface: "הַמְשֹׁרֲרִים",
        root: "שיר",
        binyan: "Polel",
        form: "Participle (act.)",
        pgn: "mp",
    },
    IrregularVerb {
        surface: "הָתְפָּקְדוּ",
        root: "פקד",
        binyan: "Hothpaal",
        form: "Perfect",
        pgn: "3cp",
    },
    IrregularVerb {
        surface: "הֻטַּמָּאָה",
        root: "טמא",
        binyan: "Hothpaal",
        form: "Perfect",
        pgn: "3fs",
    },
    IrregularVerb {
        surface: "וְאֶשְׁתַּחֲוֶה",
        root: "שחה",
        binyan: "Hishtaphel",
        form: "Imperfect",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "וְאֶשְׁתַּעֲשַׁע",
        root: "שעע",
        binyan: "Hithpalpel",
        form: "Imperfect",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "וְאֶשְׁתּוֹמֵם",
        root: "שממ",
        binyan: "Hithpolel",
        form: "Imperfect",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "וְגִלְגַּלְתִּיךָ",
        root: "גלל",
        binyan: "Pilpel",
        form: "Perfect",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "וְדוֹמַמְתִּי",
        root: "דממ",
        binyan: "Polel",
        form: "Perfect",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "וְהִשְׁתַּחֲוִיתֶם",
        root: "שחה",
        binyan: "Hishtaphel",
        form: "Perfect",
        pgn: "2mp",
    },
    IrregularVerb {
        surface: "וְהִשְׁתַּחֲוִיתָ",
        root: "שחה",
        binyan: "Hishtaphel",
        form: "Perfect",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "וְהִשְׁתַּחֲוֵיתִי",
        root: "שחה",
        binyan: "Hishtaphel",
        form: "Perfect",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "וְהִשְׁתַּחֲוּוּ",
        root: "שחה",
        binyan: "Hishtaphel",
        form: "Perfect",
        pgn: "3cp",
    },
    IrregularVerb {
        surface: "וְהִתְאֹשָׁשׁוּ",
        root: "אשש",
        binyan: "Hithpolel",
        form: "Imperative",
        pgn: "2mp",
    },
    IrregularVerb {
        surface: "וְהִתְבּוֹנְנוּ",
        root: "בינ",
        binyan: "Hithpolel",
        form: "Imperative",
        pgn: "2mp",
    },
    IrregularVerb {
        surface: "וְהִתְבּוֹנֵן",
        root: "בינ",
        binyan: "Hithpolel",
        form: "Imperative",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "וְהִתְבּוֹנַנְתָּ",
        root: "בינ",
        binyan: "Hithpolel",
        form: "Perfect",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "וְהִתְגֹּעֲשׁוּ",
        root: "געש",
        binyan: "Hithpolel",
        form: "Perfect",
        pgn: "3cp",
    },
    IrregularVerb {
        surface: "וְהִתְהֹלְלוּ",
        root: "הלל",
        binyan: "Hithpolel",
        form: "Imperative",
        pgn: "2mp",
    },
    IrregularVerb {
        surface: "וְהִתְהֹלָלוּ",
        root: "הלל",
        binyan: "Hithpolel",
        form: "Perfect",
        pgn: "3cp",
    },
    IrregularVerb {
        surface: "וְהִתְחוֹלֵל",
        root: "חול",
        binyan: "Hithpolel",
        form: "Imperative",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "וְהִתְנוֹדְדָה",
        root: "נוד",
        binyan: "Hithpolel",
        form: "Perfect",
        pgn: "3fs",
    },
    IrregularVerb {
        surface: "וְהִתְעֹרַרְתִּי",
        root: "עור",
        binyan: "Hithpolel",
        form: "Perfect",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "וְהִתְשׁוֹטַטְנָה",
        root: "שוט",
        binyan: "Hithpolel",
        form: "Imperative",
        pgn: "2fp",
    },
    IrregularVerb {
        surface: "וְהַמְשֹׁרְרִים",
        root: "שיר",
        binyan: "Polel",
        form: "Participle (act.)",
        pgn: "mp",
    },
    IrregularVerb {
        surface: "וְהַמְשֹׁרֲרִים",
        root: "שיר",
        binyan: "Polel",
        form: "Participle (act.)",
        pgn: "mp",
    },
    IrregularVerb {
        surface: "וְהַמְשׁוֹרֲרִים",
        root: "שיר",
        binyan: "Polel",
        form: "Participle (act.)",
        pgn: "mp",
    },
    IrregularVerb {
        surface: "וְטֵאטֵאתִיהָ",
        root: "",
        binyan: "Pilpel",
        form: "Perfect",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "וְיִכּוֹנָנוּ",
        root: "כונ",
        binyan: "Hithpolel",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "וְיִשְׁתַּחֲוּוּ",
        root: "שחה",
        binyan: "Hishtaphel",
        form: "Jussive",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "וְיִשְׁתַּחֲוּוּ",
        root: "שחה",
        binyan: "Hishtaphel",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "וְיִתְבּוֹנְנוּ",
        root: "בינ",
        binyan: "Hithpolel",
        form: "Jussive",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "וְיִתְלֹנָן",
        root: "לונ",
        binyan: "Hithpolel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "וְיִתְמַרְמַר",
        root: "מרר",
        binyan: "Hithpalpel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "וְיִתְרוֹמֵם",
        root: "רומ",
        binyan: "Hithpolel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "וְכִלְכַּלְתִּי",
        root: "כול",
        binyan: "Pilpel",
        form: "Perfect",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "וְלַמְשֹׁרֲרִים",
        root: "שיר",
        binyan: "Polel",
        form: "Participle (act.)",
        pgn: "mp",
    },
    IrregularVerb {
        surface: "וְנִוַּסְּרוּ",
        root: "יסר",
        binyan: "Nithpael",
        form: "Perfect",
        pgn: "3cp",
    },
    IrregularVerb {
        surface: "וְנִכַּפֵּר",
        root: "כפר",
        binyan: "Nithpael",
        form: "Perfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "וְנִשְׁתַּחֲוֶה",
        root: "שחה",
        binyan: "Hishtaphel",
        form: "Imperfect",
        pgn: "1cp",
    },
    IrregularVerb {
        surface: "וְנוֹדַד",
        root: "נדד",
        binyan: "Poal",
        form: "Perfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "וְסִכְסַכְתִּי",
        root: "סוכ",
        binyan: "Pilpel",
        form: "Perfect",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "וְעֹלַלְתִּי",
        root: "עלל",
        binyan: "Poel",
        form: "Perfect",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "וְקַרְקַר",
        root: "קור",
        binyan: "Pilpel",
        form: "Perfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "וְקֹשְׁשׁוּ",
        root: "קשש",
        binyan: "Poel",
        form: "Perfect",
        pgn: "3cp",
    },
    IrregularVerb {
        surface: "וְקוֹנְנוּ",
        root: "קינ",
        binyan: "Polel",
        form: "Perfect",
        pgn: "3cp",
    },
    IrregularVerb {
        surface: "וְקוֹנְנוּהָ",
        root: "קינ",
        binyan: "Polel",
        form: "Perfect",
        pgn: "3cp",
    },
    IrregularVerb {
        surface: "וְשֹׁבַבְתִּיךָ",
        root: "שוב",
        binyan: "Polel",
        form: "Perfect",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "וְשׁוֹבַבְתִּיךָ",
        root: "שוב",
        binyan: "Polel",
        form: "Perfect",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "וְתִכּוֹנֵן",
        root: "כונ",
        binyan: "Hithpolel",
        form: "Jussive",
        pgn: "3fs",
    },
    IrregularVerb {
        surface: "וִיבֹקְקוּ",
        root: "בקק",
        binyan: "Poel",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "וִיסוֹבְבוּ",
        root: "סבב",
        binyan: "Poel",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "וִירֹמְמוּהוּ",
        root: "רומ",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "וִירוֹמִמְךָ",
        root: "רומ",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "וַאֲמֹתְתֵהוּ",
        root: "מות",
        binyan: "Polel",
        form: "Wayyiqtol",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "וַאֲסֹבְבָה",
        root: "סבב",
        binyan: "Poel",
        form: "Cohortative",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "וַאֲסוֹבְבָה",
        root: "סבב",
        binyan: "Poel",
        form: "Cohortative",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "וַאֲרֹמְמֶנְהוּ",
        root: "רומ",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "וַיְזוֹרֵר",
        root: "זרר",
        binyan: "Poel",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "וַיְכַלְכְּלֵם",
        root: "כול",
        binyan: "Pilpel",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "וַיְכַלְכֵּל",
        root: "כול",
        binyan: "Pilpel",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "וַיְכֹנְנֶךָ",
        root: "כונ",
        binyan: "Polel",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "וַיְכֻנֶנּוּ",
        root: "כונ",
        binyan: "Polel",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "וַיְכוֹנְנֶהָ",
        root: "כונ",
        binyan: "Polel",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "וַיְכוֹנְנוּ",
        root: "כונ",
        binyan: "Polel",
        form: "Wayyiqtol",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "וַיְכוֹנְנוּנִי",
        root: "כונ",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "וַיְמֹדֶד",
        root: "מוד",
        binyan: "Polel",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "וַיְמֹתְתֵהוּ",
        root: "מות",
        binyan: "Polel",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "וַיְעֹלְלֻהוּ",
        root: "עלל",
        binyan: "Poel",
        form: "Wayyiqtol",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "וַיְפַצְפְּצֵנִי",
        root: "פוצ",
        binyan: "Pilpel",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "וַיְפַרְפְּרֵנִי",
        root: "פרר",
        binyan: "Pilpel",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "וַיְקֹנֵן",
        root: "קינ",
        binyan: "Polel",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "וַיְקוֹנֵן",
        root: "קינ",
        binyan: "Polel",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "וַיְרֹצְצוּ",
        root: "רצצ",
        binyan: "Poel",
        form: "Wayyiqtol",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "וַיִּשְׁתַּחֲוֶה",
        root: "שחה",
        binyan: "Hishtaphel",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "וַיִּשְׁתַּחֲוֻּ",
        root: "שחה",
        binyan: "Hithpael",
        form: "Wayyiqtol",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "וַיִּשְׁתַּחֲוּוּ",
        root: "שחה",
        binyan: "Hishtaphel",
        form: "Wayyiqtol",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "וַיִּשְׁתַּחֲוּוּ",
        root: "שחה",
        binyan: "Hithpael",
        form: "Wayyiqtol",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "וַיִּשְׁתָּחוּ",
        root: "שחה",
        binyan: "Hithpael",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "וַיִּשְׁתּוֹמֵם",
        root: "שממ",
        binyan: "Hithpolel",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "וַיִּתְגֹּדְדוּ",
        root: "גדד",
        binyan: "Hithpolel",
        form: "Wayyiqtol",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "וַיִּתְהֹלֵל",
        root: "הלל",
        binyan: "Hithpolel",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "וַיִּתְמַהְמָהּ",
        root: "מהה",
        binyan: "Hithpalpel",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "וַיִּתְמַרְמַר",
        root: "מרר",
        binyan: "Hithpalpel",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "וַיִּתְמֹדֵד",
        root: "מדד",
        binyan: "Hithpolel",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "וַיִּתְפֹּצְצוּ",
        root: "פוצ",
        binyan: "Hithpolel",
        form: "Wayyiqtol",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "וַיִּתְרֹצֲצוּ",
        root: "רצצ",
        binyan: "Hithpolel",
        form: "Wayyiqtol",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "וַנִּתְעוֹדָד",
        root: "עוד",
        binyan: "Hithpolel",
        form: "Wayyiqtol",
        pgn: "1cp",
    },
    IrregularVerb {
        surface: "וַתְּחוֹלֵל",
        root: "חול",
        binyan: "Polel",
        form: "Wayyiqtol",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "וַתְּכוֹנֵן",
        root: "כונ",
        binyan: "Polel",
        form: "Wayyiqtol",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "וַתְּרוֹמֵם",
        root: "רומ",
        binyan: "Polel",
        form: "Wayyiqtol",
        pgn: "3fs",
    },
    IrregularVerb {
        surface: "וַתְּשֹׁקְקֶהָ",
        root: "שוק",
        binyan: "Polel",
        form: "Wayyiqtol",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "וַתִּשְׁתַּחֲוֶיןָ",
        root: "שחה",
        binyan: "Hishtaphel",
        form: "Wayyiqtol",
        pgn: "3fp",
    },
    IrregularVerb {
        surface: "וַתִּשְׁתָּחוּ",
        root: "שחה",
        binyan: "Hishtaphel",
        form: "Wayyiqtol",
        pgn: "3fs",
    },
    IrregularVerb {
        surface: "וַתִּתְבֹּנֶן",
        root: "בינ",
        binyan: "Hithpolel",
        form: "Wayyiqtol",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "וַתִּתְחַלְחַל",
        root: "חול",
        binyan: "Hithpalpel",
        form: "Wayyiqtol",
        pgn: "3fs",
    },
    IrregularVerb {
        surface: "וָאֲכַלְכְּלֵם",
        root: "כול",
        binyan: "Pilpel",
        form: "Wayyiqtol",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "וָאֶשְׁתַּחֲוֶה",
        root: "שחה",
        binyan: "Hishtaphel",
        form: "Wayyiqtol",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "וָאֶשְׁתּוֹמֵם",
        root: "שממ",
        binyan: "Hithpolel",
        form: "Wayyiqtol",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "וָאֶתְבּוֹנֵן",
        root: "בינ",
        binyan: "Hithpolel",
        form: "Wayyiqtol",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "וָאֶתְקוֹטָטָה",
        root: "קוט",
        binyan: "Hithpolel",
        form: "Wayyiqtol",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "וּלְהִשְׁתַּחֲות",
        root: "שחה",
        binyan: "Hishtaphel",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        surface: "וּלְכַלְכֵּל",
        root: "כול",
        binyan: "Pilpel",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        surface: "וּמְכַרְכֵּר",
        root: "כרר",
        binyan: "Pilpel",
        form: "Participle (act.)",
        pgn: "ms",
    },
    IrregularVerb {
        surface: "וּמְצַפְצֵף",
        root: "צפפ",
        binyan: "Pilpel",
        form: "Participle (act.)",
        pgn: "ms",
    },
    IrregularVerb {
        surface: "וּמְרוֹמַם",
        root: "רומ",
        binyan: "Polal",
        form: "Participle (pas.)",
        pgn: "ms",
    },
    IrregularVerb {
        surface: "וּמְשֹׁרֲרוֹת",
        root: "שיר",
        binyan: "Polel",
        form: "Participle (act.)",
        pgn: "fp",
    },
    IrregularVerb {
        surface: "וּמִשְׁתַּחֲוֶה",
        root: "שחה",
        binyan: "Hishtaphel",
        form: "Participle (act.)",
        pgn: "ms",
    },
    IrregularVerb {
        surface: "וּמִתְגֹּדְדִים",
        root: "גדד",
        binyan: "Hithpoel",
        form: "Participle (act.)",
        pgn: "mp",
    },
    IrregularVerb {
        surface: "וּמִתְקוֹמְמִי",
        root: "לבב",
        binyan: "Hithpolel",
        form: "Participle (act.)",
        pgn: "ms",
    },
    IrregularVerb {
        surface: "וּמִתַּעְתְּעִים",
        root: "תעע",
        binyan: "Hithpalpel",
        form: "Participle (act.)",
        pgn: "mp",
    },
    IrregularVerb {
        surface: "וּמוֹתְתֵנִי",
        root: "מות",
        binyan: "Polel",
        form: "Imperative",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "וּנְרוֹמְמָה",
        root: "רומ",
        binyan: "Polel",
        form: "Cohortative",
        pgn: "1cp",
    },
    IrregularVerb {
        surface: "וּתְחוֹלֵל",
        root: "חול",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "וּתְכוֹנֵן",
        root: "כונ",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "וּתְמֹגְגֵנִי",
        root: "מוג",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "וּתְרוֹמְמֶךָּ",
        root: "רומ",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "3fs",
    },
    IrregularVerb {
        surface: "חֳמַרְמְרוּ",
        root: "חמר",
        binyan: "Pealal",
        form: "Perfect",
        pgn: "3cp",
    },
    IrregularVerb {
        surface: "חֳמַרְמָרוּ",
        root: "חמר",
        binyan: "Pealal",
        form: "Perfect",
        pgn: "3cp",
    },
    IrregularVerb {
        surface: "חֹלֲלָה",
        root: "חלל",
        binyan: "Poel",
        form: "Perfect",
        pgn: "3fs",
    },
    IrregularVerb {
        surface: "חוֹלָלְתִּי",
        root: "חול",
        binyan: "Polal",
        form: "Perfect",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "חוֹלָלְתָּ",
        root: "חול",
        binyan: "Polal",
        form: "Perfect",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "יְבוֹנְנֵהוּ",
        root: "בינ",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יְהוֹלֵל",
        root: "הלל",
        binyan: "Poel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יְחֹנֵנוּ",
        root: "חננ",
        binyan: "Poel",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "יְחֹקְקוּ",
        root: "חקק",
        binyan: "Poel",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "יְחוֹלֵל",
        root: "חול",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יְחוֹלָלוּ",
        root: "חול",
        binyan: "Polal",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "יְכַלְכְּלֶךָ",
        root: "כול",
        binyan: "Pilpel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יְכַלְכְּלֻהוּ",
        root: "כול",
        binyan: "Pilpel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יְכַלְכְּלוּךָ",
        root: "כול",
        binyan: "Pilpel",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "יְכַלְכֵּל",
        root: "כול",
        binyan: "Pilpel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        // Quadriliteral root כרסם "devour" (Ps 80:14); the triliteral generator
        // cannot model a four-radical root. + 3fs object suffix in the surface.
        surface: "יְכַרְסְמֶנָּה",
        root: "כרסם",
        binyan: "Piel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יְכוֹנְנֶהָ",
        root: "כונ",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יְכוֹנֵן",
        root: "כונ",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יְמוֹלֵל",
        root: "מול",
        binyan: "Poel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יְנֹפֵף",
        root: "נופ",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יְנוֹבֵב",
        root: "נוב",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יְסַכְסֵךְ",
        root: "סוכ",
        binyan: "Pilpel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יְסֹבְבֵנִי",
        root: "סבב",
        binyan: "Poel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יְסֹבְבֶנְהוּ",
        root: "סבב",
        binyan: "Poel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יְסֹעֵר",
        root: "סער",
        binyan: "Poel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יְסוֹבְבֶנּוּ",
        root: "סבב",
        binyan: "Poel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יְסוֹבְבֻהָ",
        root: "סבב",
        binyan: "Poel",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "יְעֹעֵרוּ",
        root: "עור",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "יְעוֹדֵד",
        root: "עוד",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יְעוֹלְלוּ",
        root: "עלל",
        binyan: "Poel",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "יְעוֹפֵף",
        root: "עופ",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יְפֹצֵץ",
        root: "פוצ",
        binyan: "Poel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יְקוֹמֵם",
        root: "לבב",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יְקוֹמֵמוּ",
        root: "לבב",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "יְקוֹסֵס",
        root: "קסס",
        binyan: "Poel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יְרֹעָע",
        root: "רוע",
        binyan: "Polal",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יְרֹשֵׁשׁ",
        root: "רשש",
        binyan: "Poel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יְרוֹמְמֵנִי",
        root: "רומ",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יְרוֹמֵם",
        root: "רומ",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יְרוֹפָפוּ",
        root: "רפפ",
        binyan: "Poal",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "יְרוֹצֵצוּ",
        root: "רוצ",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "יְשַׁעַשְׁעוּ",
        root: "שעע",
        binyan: "Pilpel",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "יְשֹׁדֵד",
        root: "שדד",
        binyan: "Poel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יְשֹׁטְטוּ",
        root: "שוט",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "יְשׁוֹבֵב",
        root: "שוב",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יְשׁוֹטְטוּ",
        root: "שוט",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "יְשׁוֹרֵר",
        root: "שיר",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יִּתְאוֹנֵן",
        root: "אננ",
        binyan: "Hithpolel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יִשְׁתַּחֲוֶה",
        root: "שחה",
        binyan: "Hishtaphel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יִשְׁתַּחֲוּוּ",
        root: "שחה",
        binyan: "Hishtaphel",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "יִשְׁתַּקְשְׁקוּן",
        root: "שקק",
        binyan: "Hithpalpel",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "יִשְׁתּוֹמֵם",
        root: "שממ",
        binyan: "Hithpolel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יִתְבֹּשָׁשׁוּ",
        root: "בוש",
        binyan: "Hithpolel",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "יִתְבּוֹלָל",
        root: "בלל",
        binyan: "Hithpolel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יִתְבּוֹנָן",
        root: "בינ",
        binyan: "Hithpolel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יִתְבּוֹנָנוּ",
        root: "בינ",
        binyan: "Hithpolel",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "יִתְגֹּדַד",
        root: "גדד",
        binyan: "Hithpolel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יִתְגֹּדָדוּ",
        root: "גדד",
        binyan: "Hithpolel",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "יִתְגֹּעֲשׁוּ",
        root: "געש",
        binyan: "Hithpolel",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "יִתְגּוֹרָרוּ",
        root: "גור",
        binyan: "Hithpolel",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "יִתְהוֹלְלוּ",
        root: "הלל",
        binyan: "Hithpolel",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "יִתְכּוֹנָן",
        root: "כונ",
        binyan: "Hithpolel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יִתְלוֹנָן",
        root: "לונ",
        binyan: "Hithpolel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יִתְמַהְמָהּ",
        root: "מהה",
        binyan: "Hithpalpel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יִתְנֹדֲדוּ",
        root: "נדד",
        binyan: "Hithpolel",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "יִתְעֹרָר",
        root: "עור",
        binyan: "Hithpolel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יִתְעוֹפֵף",
        root: "עופ",
        binyan: "Hithpolel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יִתְרוֹעֲעוּ",
        root: "רוע",
        binyan: "Hithpolel",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "יַשְׁחֶנָּה",
        root: "שחה",
        binyan: "Hiphil",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יָפְיָפִיתָ",
        root: "יפה",
        binyan: "Pealal",
        form: "Perfect",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "יוֹדַעְתִּי",
        root: "ידע",
        binyan: "Poel",
        form: "Perfect",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "כְּמִתְאֹנְנִים",
        root: "אננ",
        binyan: "Hithpolel",
        form: "Participle (act.)",
        pgn: "mp",
    },
    IrregularVerb {
        surface: "כְּמִתְלַהְלֵהַּ",
        root: "להה",
        binyan: "Hithpalpel",
        form: "Participle (act.)",
        pgn: "ms",
    },
    IrregularVerb {
        surface: "כִּלְכַּלְתָּם",
        root: "כול",
        binyan: "Pilpel",
        form: "Perfect",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "כִּמְתַעְתֵּעַ",
        root: "תעע",
        binyan: "Pilpel",
        form: "Participle (act.)",
        pgn: "ms",
    },
    IrregularVerb {
        surface: "כוֹנַנְתָּהּ",
        root: "כונ",
        binyan: "Polel",
        form: "Perfect",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "לְהִשְׁתַּחֲות",
        root: "שחה",
        binyan: "Hithpael",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        surface: "לְהִתְגֹּלֵל",
        root: "גלל",
        binyan: "Hithpolel",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        surface: "לְהִתְמַהְמֵהַּ",
        root: "מהה",
        binyan: "Hithpalpel",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        surface: "לְהִתְנוֹסֵס",
        root: "נוס",
        binyan: "Hithpolel",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        surface: "לְהִתְעוֹלֵל",
        root: "עלל",
        binyan: "Hithpolel",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        surface: "לְהִתְרֹעֵעַ",
        root: "רעע",
        binyan: "Hithpolel",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        surface: "לְחַרְחַר",
        root: "חרר",
        binyan: "Pilpel",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        surface: "לְכַלְכְּלֶךָ",
        root: "כול",
        binyan: "Pilpel",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        surface: "לְכַלְכֵּל",
        root: "כול",
        binyan: "Pilpel",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        surface: "לְכַלְכֶּלְךָ",
        root: "כול",
        binyan: "Pilpel",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        surface: "מְגוֹלָלָה",
        root: "גלל",
        binyan: "Poal",
        form: "Participle (pas.)",
        pgn: "fs",
    },
    IrregularVerb {
        surface: "מְהוֹלָל",
        root: "הלל",
        binyan: "Poal",
        form: "Participle (pas.)",
        pgn: "ms",
    },
    IrregularVerb {
        surface: "מְהוֹלָלַי",
        root: "הלל",
        binyan: "Poal",
        form: "Participle (pas.)",
        pgn: "mp",
    },
    IrregularVerb {
        surface: "מְזַעְזְעֶיךָ",
        root: "זוע",
        binyan: "Pilpel",
        form: "Participle (act.)",
        pgn: "mp",
    },
    IrregularVerb {
        surface: "מְחֹלְלֶךָ",
        root: "חול",
        binyan: "Polel",
        form: "Participle (act.)",
        pgn: "ms",
    },
    IrregularVerb {
        surface: "מְחֹלָל",
        root: "חלל",
        binyan: "Polal",
        form: "Participle (pas.)",
        pgn: "ms",
    },
    IrregularVerb {
        surface: "מְטַלְטֶלְךָ",
        root: "טול",
        binyan: "Pilpel",
        form: "Participle (act.)",
        pgn: "ms",
    },
    IrregularVerb {
        surface: "מְכַלְכֵּל",
        root: "כול",
        binyan: "Pilpel",
        form: "Participle (act.)",
        pgn: "ms",
    },
    IrregularVerb {
        surface: "מְכַרְכֵּר",
        root: "כרר",
        binyan: "Pilpel",
        form: "Participle (act.)",
        pgn: "ms",
    },
    IrregularVerb {
        // Quadriliteral root כרבל "wrap/robe" (1 Chr 15:27), Pual passive
        // participle; a four-radical root the triliteral generator cannot model.
        surface: "מְכֻרְבָּל",
        root: "כרבל",
        binyan: "Pual",
        form: "Participle (pas.)",
        pgn: "ms",
    },
    IrregularVerb {
        surface: "מְקַרְקַר",
        root: "קור",
        binyan: "Pilpel",
        form: "Participle (act.)",
        pgn: "ms",
    },
    IrregularVerb {
        surface: "מְשֹׁרֲרִים",
        root: "שיר",
        binyan: "Polel",
        form: "Participle (act.)",
        pgn: "mp",
    },
    IrregularVerb {
        surface: "מִּמִתְקוֹמְמַי",
        root: "לבב",
        binyan: "Hithpolel",
        form: "Participle (act.)",
        pgn: "mp",
    },
    IrregularVerb {
        surface: "מִמִּתְקוֹמְמִים",
        root: "לבב",
        binyan: "Hithpolel",
        form: "Participle (act.)",
        pgn: "mp",
    },
    IrregularVerb {
        surface: "מִסְתּוֹלֵל",
        root: "סלל",
        binyan: "Hithpolel",
        form: "Participle (act.)",
        pgn: "ms",
    },
    IrregularVerb {
        surface: "מִשְׁתַּחֲוִיתֶם",
        root: "שחה",
        binyan: "Hishtaphel",
        form: "Participle (act.)",
        pgn: "mp",
    },
    IrregularVerb {
        surface: "מִתְבּוֹסֶסֶת",
        root: "בוס",
        binyan: "Hithpolel",
        form: "Participle (act.)",
        pgn: "fs",
    },
    IrregularVerb {
        surface: "מִתְגֹּלֵל",
        root: "גלל",
        binyan: "Hithpolel",
        form: "Participle (act.)",
        pgn: "ms",
    },
    IrregularVerb {
        surface: "מִתְגּוֹרֵר",
        root: "גור",
        binyan: "Hithpolel",
        form: "Participle (act.)",
        pgn: "ms",
    },
    IrregularVerb {
        surface: "מִתְגּוֹרֵר",
        root: "גרר",
        binyan: "Hithpolel",
        form: "Participle (act.)",
        pgn: "ms",
    },
    IrregularVerb {
        surface: "מִתְחוֹלֵל",
        root: "חול",
        binyan: "Hithpolel",
        form: "Participle (act.)",
        pgn: "ms",
    },
    IrregularVerb {
        surface: "מִתְמַהְמֵהַּ",
        root: "מהה",
        binyan: "Hithpalpel",
        form: "Participle (act.)",
        pgn: "ms",
    },
    IrregularVerb {
        surface: "מִתְנוֹדֵד",
        root: "נוד",
        binyan: "Hithpolel",
        form: "Participle (act.)",
        pgn: "ms",
    },
    IrregularVerb {
        surface: "מִתְנוֹסְסוֹת",
        root: "נסס",
        binyan: "Hithpolel",
        form: "Participle (act.)",
        pgn: "fp",
    },
    IrregularVerb {
        surface: "מִתְעוֹרֵר",
        root: "עור",
        binyan: "Hithpolel",
        form: "Participle (act.)",
        pgn: "ms",
    },
    IrregularVerb {
        surface: "מִתְקוֹמָמָה",
        root: "לבב",
        binyan: "Hithpolel",
        form: "Participle (act.)",
        pgn: "fs",
    },
    IrregularVerb {
        surface: "מִתְרוֹנֵן",
        root: "רונ",
        binyan: "Hithpolel",
        form: "Participle (act.)",
        pgn: "ms",
    },
    IrregularVerb {
        surface: "מִתְרוֹשֵׁשׁ",
        root: "רוש",
        binyan: "Hithpolel",
        form: "Participle (act.)",
        pgn: "ms",
    },
    IrregularVerb {
        surface: "מוֹתְתַנִי",
        root: "מות",
        binyan: "Polel",
        form: "Perfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "נִשְׁתַּחֲוֶה",
        root: "שחה",
        binyan: "Hishtaphel",
        form: "Imperfect",
        pgn: "1cp",
    },
    IrregularVerb {
        surface: "נִשְׁתַּחֲוֶה",
        root: "שחה",
        binyan: "Hishtaphel",
        form: "Cohortative",
        pgn: "1cp",
    },
    IrregularVerb {
        surface: "נִשְׁתָּוָה",
        root: "שוה",
        binyan: "Nithpael",
        form: "Perfect",
        pgn: "3fs",
    },
    IrregularVerb {
        surface: "סְחַרְחַר",
        root: "סחר",
        binyan: "Pealal",
        form: "Perfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "סַלְסְלֶהָ",
        root: "סלל",
        binyan: "Pilpel",
        form: "Imperative",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "עַרְעֵר",
        root: "ערר",
        binyan: "Pilpel",
        form: "Inf. Absolute",
        pgn: "",
    },
    IrregularVerb {
        surface: "עֹנְנֵיכֶם",
        root: "עננ",
        binyan: "Poel",
        form: "Participle (act.)",
        pgn: "mp",
    },
    IrregularVerb {
        surface: "עוֹלְלָה",
        root: "עלל",
        binyan: "Poel",
        form: "Perfect",
        pgn: "3fs",
    },
    IrregularVerb {
        surface: "עוֹלַל",
        root: "עלל",
        binyan: "Poal",
        form: "Perfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "עוֹלַלְתָּ",
        root: "עלל",
        binyan: "Poel",
        form: "Perfect",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "פַּרְשֵׁז",
        root: "פרש",
        binyan: "Pilel",
        form: "Inf. Absolute",
        pgn: "",
    },
    IrregularVerb {
        surface: "צִמְּתוּתֻנִי",
        root: "צמת",
        binyan: "Pilpel",
        form: "Perfect",
        pgn: "3cp",
    },
    IrregularVerb {
        surface: "קִלְקַל",
        root: "קלל",
        binyan: "Pilpel",
        form: "Perfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "רַעֲנָנָה",
        root: "רענ",
        binyan: "Palel",
        form: "Perfect",
        pgn: "3fs",
    },
    IrregularVerb {
        surface: "רֹמְמָתְהוּ",
        root: "רומ",
        binyan: "Polel",
        form: "Perfect",
        pgn: "3fs",
    },
    IrregularVerb {
        surface: "רֻאּוּ",
        root: "ראה",
        binyan: "Qal passive",
        form: "Perfect",
        pgn: "3cp",
    },
    IrregularVerb {
        surface: "רֻטֲפַשׁ",
        root: "",
        binyan: "Qal passive",
        form: "Perfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "רוֹמְמוּ",
        root: "רומ",
        binyan: "Polel",
        form: "Imperative",
        pgn: "2mp",
    },
    IrregularVerb {
        surface: "רוֹמֵמָה",
        root: "רממ",
        binyan: "Polel",
        form: "Participle (act.)",
        pgn: "fs",
    },
    IrregularVerb {
        surface: "שִׁעֲשָׁעְתִּי",
        root: "שעע",
        binyan: "Pilpel",
        form: "Perfect",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "שַׁאֲנָנוּ",
        root: "שאנ",
        binyan: "Palel",
        form: "Perfect",
        pgn: "3cp",
    },
    IrregularVerb {
        surface: "שׁוֹבְבָתֶךְ",
        root: "שוב",
        binyan: "Polel",
        form: "Perfect",
        pgn: "3fs",
    },
    IrregularVerb {
        surface: "תְּהוֹתְתוּ",
        root: "הות",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "2mp",
    },
    IrregularVerb {
        surface: "תְּחוֹלֵל",
        root: "חול",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "3fs",
    },
    IrregularVerb {
        surface: "תְּחוֹלֶלְכֶם",
        root: "חול",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "3fs",
    },
    IrregularVerb {
        surface: "תְּכוֹנֵן",
        root: "כונ",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "תְּמֹגְגֶנָּה",
        root: "מוג",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "תְּמוֹתֵת",
        root: "מות",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "3fs",
    },
    IrregularVerb {
        surface: "תְּסֹכְכֵנִי",
        root: "סוכ",
        binyan: "Poel",
        form: "Imperfect",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "תְּסוֹבְבֵנִי",
        root: "סבב",
        binyan: "Poel",
        form: "Imperfect",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "תְּסוֹבְבֶךָּ",
        root: "סבב",
        binyan: "Poel",
        form: "Jussive",
        pgn: "3fs",
    },
    IrregularVerb {
        surface: "תְּסוֹבֵב",
        root: "סבב",
        binyan: "Poel",
        form: "Imperfect",
        pgn: "3fs",
    },
    IrregularVerb {
        surface: "תְּעוֹרֵר",
        root: "עור",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "3fs",
    },
    IrregularVerb {
        surface: "תְּצַפְצֵף",
        root: "צפפ",
        binyan: "Pilpel",
        form: "Imperfect",
        pgn: "3fs",
    },
    IrregularVerb {
        surface: "תְּצוֹדֵדְנָה",
        root: "צוד",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "2fp",
    },
    IrregularVerb {
        surface: "תְּקוֹמֵם",
        root: "לבב",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "תְּקוֹנֵנָּה",
        root: "קינ",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "3fp",
    },
    IrregularVerb {
        surface: "תְּרוֹמְמֵנִי",
        root: "רומ",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "תְּרוֹמַמְנָה",
        root: "רומ",
        binyan: "Polal",
        form: "Imperfect",
        pgn: "3fp",
    },
    IrregularVerb {
        surface: "תְּשַׂגְשֵׂגִי",
        root: "סוג",
        binyan: "Pilpel",
        form: "Imperfect",
        pgn: "2fs",
    },
    IrregularVerb {
        surface: "תְּשָׁעֳשָׁעוּ",
        root: "שעע",
        binyan: "Polpal",
        form: "Imperfect",
        pgn: "2mp",
    },
    IrregularVerb {
        surface: "תְּשׁוֹבֵב",
        root: "שוב",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "תְּתַחֲרֶה",
        root: "חרה",
        binyan: "Tiphil",
        form: "Imperfect",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "תְעוֹלֵל",
        root: "עלל",
        binyan: "Poel",
        form: "Imperfect",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "תְעוֹנֵנוּ",
        root: "עננ",
        binyan: "Poel",
        form: "Imperfect",
        pgn: "2mp",
    },
    IrregularVerb {
        surface: "תְרֹמֵם",
        root: "רומ",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "תְרוֹמֵם",
        root: "רומ",
        binyan: "Polel",
        form: "Imperfect",
        pgn: "3fs",
    },
    IrregularVerb {
        surface: "תִּכּוֹנָנִי",
        root: "כונ",
        binyan: "Hithpolel",
        form: "Imperfect",
        pgn: "2fs",
    },
    IrregularVerb {
        surface: "תִּשְׁתּוֹחֲחִי",
        root: "שחח",
        binyan: "Hithpolel",
        form: "Imperfect",
        pgn: "2fs",
    },
    IrregularVerb {
        surface: "תִּשּׁוֹמֵם",
        root: "שממ",
        binyan: "Hithpolel",
        form: "Imperfect",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "תִּתְבֹּנָנוּ",
        root: "בינ",
        binyan: "Hithpolel",
        form: "Jussive",
        pgn: "2mp",
    },
    IrregularVerb {
        surface: "תִּתְבּוֹנְנוּ",
        root: "בינ",
        binyan: "Hithpolel",
        form: "Imperfect",
        pgn: "2mp",
    },
    IrregularVerb {
        surface: "תִּתְגֹּדְדִי",
        root: "גדד",
        binyan: "Hithpolel",
        form: "Imperfect",
        pgn: "2fs",
    },
    IrregularVerb {
        surface: "תִּתְגּוֹדָדִי",
        root: "גדד",
        binyan: "Hithpolel",
        form: "Imperfect",
        pgn: "2fs",
    },
    IrregularVerb {
        surface: "תִּתְלוֹצָצוּ",
        root: "ליצ",
        binyan: "Hithpolel",
        form: "Jussive",
        pgn: "2mp",
    },
    IrregularVerb {
        surface: "תִּתְמוֹגַגְנָה",
        root: "מוג",
        binyan: "Hithpolel",
        form: "Imperfect",
        pgn: "3fp",
    },
    IrregularVerb {
        surface: "תִּתְנוֹדָד",
        root: "נוד",
        binyan: "Hithpolel",
        form: "Imperfect",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "תִּתְעַרְעָר",
        root: "ערר",
        binyan: "Hithpalpel",
        form: "Imperfect",
        pgn: "3fs",
    },
    IrregularVerb {
        surface: "תִרְגַּלְתִּי",
        root: "רגל",
        binyan: "Tiphil",
        form: "Perfect",
        pgn: "1cs",
    },
    IrregularVerb {
        // Silent-sheva spelling variant of the Hishtaphel 2ms imperfect found in
        // bible.db (תִשְׁתַּחְוֶה for תִשְׁתַּחֲוֶה).
        surface: "תִשְׁתַּחְוֶה",
        root: "שחה",
        binyan: "Hishtaphel",
        form: "Imperfect",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "תִשְׁתַּחֲוֶה",
        root: "שחה",
        binyan: "Hishtaphel",
        form: "Imperfect",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "תִשְׁתּוֹחָח",
        root: "שחח",
        binyan: "Hithpolel",
        form: "Imperfect",
        pgn: "3fs",
    },
    IrregularVerb {
        surface: "תִתְגֹּדְדוּ",
        root: "גדד",
        binyan: "Hithpolel",
        form: "Imperfect",
        pgn: "2mp",
    },
    IrregularVerb {
        surface: "תִתְמוֹגָג",
        root: "מוג",
        binyan: "Hithpolel",
        form: "Imperfect",
        pgn: "3fs",
    },
    // Suppletive היה / חיה (Strong 1961/2421): aleph-preformative & apocopated
    // wayyiqtol (וָאֱהִי, וַיֶּהִי), doubly-weak Niphal (נִהְיְתָה), paragogic-nun
    // imperfects (תִּהְיֶיןָ) — forms the triliteral generator cannot produce.
    IrregularVerb {
        surface: "הֱיוּ",
        root: "היה",
        binyan: "Qal",
        form: "Imperative",
        pgn: "2mp",
    },
    IrregularVerb {
        surface: "הֶחֱיִתָנוּ",
        root: "חיה",
        binyan: "Hiphil",
        form: "Perfect",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "הַחֲיִתֶם",
        root: "חיה",
        binyan: "Hiphil",
        form: "Perfect",
        pgn: "2mp",
    },
    IrregularVerb {
        surface: "הָיִיתְ",
        root: "היה",
        binyan: "Qal",
        form: "Perfect",
        pgn: "2fs",
    },
    IrregularVerb {
        surface: "הָיוֹ",
        root: "היה",
        binyan: "Qal",
        form: "Inf. Absolute",
        pgn: "",
    },
    IrregularVerb {
        surface: "וְהַחֲיִתֶם",
        root: "חיה",
        binyan: "Hiphil",
        form: "Perfect",
        pgn: "2mp",
    },
    IrregularVerb {
        surface: "וְהָיִיתְ",
        root: "היה",
        binyan: "Qal",
        form: "Perfect",
        pgn: "2fs",
    },
    IrregularVerb {
        surface: "וְהָיִתָ",
        root: "היה",
        binyan: "Qal",
        form: "Perfect",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "וְהָיִתָה",
        root: "היה",
        binyan: "Qal",
        form: "Perfect",
        pgn: "3fs",
    },
    IrregularVerb {
        surface: "וְחָיִתָה",
        root: "חיה",
        binyan: "Qal",
        form: "Perfect",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "וְיֶחִי",
        root: "חיה",
        binyan: "Qal",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "וְלִהְיֹתְךָ",
        root: "היה",
        binyan: "Qal",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        surface: "וְלִהְיוֹת",
        root: "היה",
        binyan: "Qal",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        surface: "וְנִהְיָתָה",
        root: "היה",
        binyan: "Niphal",
        form: "Perfect",
        pgn: "3fs",
    },
    IrregularVerb {
        surface: "וְתִהְיֶנָה",
        root: "היה",
        binyan: "Qal",
        form: "Jussive",
        pgn: "3fp",
    },
    IrregularVerb {
        surface: "וִיחַיֵּהוּ",
        root: "חיה",
        binyan: "Piel",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "וֶחְיֵה",
        root: "חיה",
        binyan: "Qal",
        form: "Imperative",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "וַיְחַיֶּהָ",
        root: "חיה",
        binyan: "Piel",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "וַיֶּהִי",
        root: "היה",
        binyan: "Qal",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "וַיֶּחִי",
        root: "חיה",
        binyan: "Qal",
        form: "Wayyiqtol",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "וַתְּחַיֶּיןָ",
        root: "חיה",
        binyan: "Piel",
        form: "Wayyiqtol",
        pgn: "3fp",
    },
    IrregularVerb {
        surface: "וַתְּחַיֶּיןָ",
        root: "חיה",
        binyan: "Piel",
        form: "Wayyiqtol",
        pgn: "2fp",
    },
    IrregularVerb {
        surface: "וַתִּהְיֶיןָ",
        root: "היה",
        binyan: "Qal",
        form: "Wayyiqtol",
        pgn: "3fp",
    },
    IrregularVerb {
        surface: "וַתִּהְיֶנָה",
        root: "היה",
        binyan: "Qal",
        form: "Wayyiqtol",
        pgn: "3fp",
    },
    IrregularVerb {
        surface: "וַתֶּהִי",
        root: "היה",
        binyan: "Qal",
        form: "Wayyiqtol",
        pgn: "3fs",
    },
    IrregularVerb {
        surface: "וָאֱהִי",
        root: "היה",
        binyan: "Qal",
        form: "Wayyiqtol",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "וּלְהַחֲיוֹת",
        root: "חיה",
        binyan: "Hiphil",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        surface: "וּלְחַיּוֹתָם",
        root: "חיה",
        binyan: "Piel",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        surface: "חִיָּתְנִי",
        root: "חיה",
        binyan: "Piel",
        form: "Perfect",
        pgn: "3fs",
    },
    IrregularVerb {
        surface: "חַיֵּיהוּ",
        root: "חיה",
        binyan: "Piel",
        form: "Imperative",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "חָיוֹ",
        root: "חיה",
        binyan: "Qal",
        form: "Inf. Absolute",
        pgn: "",
    },
    IrregularVerb {
        surface: "יִּהְיֶה",
        root: "היה",
        binyan: "Qal",
        form: "Imperfect",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "יִּהְיוּ",
        root: "היה",
        binyan: "Qal",
        form: "Imperfect",
        pgn: "3mp",
    },
    IrregularVerb {
        surface: "יֶהִי",
        root: "היה",
        binyan: "Qal",
        form: "Jussive",
        pgn: "3ms",
    },
    IrregularVerb {
        surface: "לְחַיּוֹתָם",
        root: "חיה",
        binyan: "Piel",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        surface: "לִחְיוֹת",
        root: "חיה",
        binyan: "Qal",
        form: "Inf. Construct",
        pgn: "",
    },
    IrregularVerb {
        surface: "נִּהְיָתָה",
        root: "היה",
        binyan: "Niphal",
        form: "Perfect",
        pgn: "3fs",
    },
    IrregularVerb {
        surface: "נִהְיְתָה",
        root: "היה",
        binyan: "Niphal",
        form: "Perfect",
        pgn: "3fs",
    },
    IrregularVerb {
        surface: "נִהְיֵיתִי",
        root: "היה",
        binyan: "Niphal",
        form: "Perfect",
        pgn: "1cs",
    },
    IrregularVerb {
        surface: "נִהְיֵיתָ",
        root: "היה",
        binyan: "Niphal",
        form: "Perfect",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "נִהְיָתָה",
        root: "היה",
        binyan: "Niphal",
        form: "Perfect",
        pgn: "3fs",
    },
    IrregularVerb {
        surface: "תִּהְיֵה",
        root: "היה",
        binyan: "Qal",
        form: "Jussive",
        pgn: "2ms",
    },
    IrregularVerb {
        surface: "תִּהְיֶין",
        root: "היה",
        binyan: "Qal",
        form: "Imperfect",
        pgn: "3fp",
    },
    IrregularVerb {
        surface: "תִּהְיֶיןָ",
        root: "היה",
        binyan: "Qal",
        form: "Imperfect",
        pgn: "3fp",
    },
    IrregularVerb {
        surface: "תִהְיֶיןָ",
        root: "היה",
        binyan: "Qal",
        form: "Imperfect",
        pgn: "3fp",
    },
];
