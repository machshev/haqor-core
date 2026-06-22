//! Triliteral root parsing and gizra (irregular-class) detection.

use super::hebrew::letter;

/// Irregular root classes. A root may sit in several at once (doubly weak).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Gizra {
    /// Strong (regular) — no guttural and no weak letters.
    Strong,
    /// I-Guttural: first radical is א/ה/ח/ע (also ר for non-doubling).
    PeGuttural,
    /// II-Guttural: middle radical is א/ה/ח/ע/ר.
    AyinGuttural,
    /// III-Guttural: third radical is ח/ע (א and ה have their own classes).
    LamedGuttural,
    /// I-Aleph: first radical is א (e.g., אכל, אמר). Subset of PeGuttural with
    /// its own Qal Imperfect pattern (yōʔkal, yōʔmar).
    PeAleph,
    /// III-Aleph: third radical is א (e.g., מצא, ברא). The א quiesces, lengthening
    /// the preceding vowel.
    LamedAleph,
    /// III-He: third radical written ה but historically III-Yod
    /// (e.g., בנה, עשה). Whole sub-paradigm with apocopated jussive/wayyiqtol.
    LamedHe,
    /// I-Nun: first radical is נ (e.g., נפל, נתן). The nun assimilates as
    /// a dagesh in the following radical when no vowel intervenes.
    PeNun,
    /// I-Yod: first radical is י, historically either original I-Vav (most
    /// verbs — ישב, ילד, ידע) or true I-Yod (יטב, יבש).
    PeYod,
    /// Hollow (II-Vav/Yod): middle radical is ו or י (e.g., קום, שים).
    Hollow,
    /// Geminate: 2nd and 3rd radicals are identical (e.g., סבב, חנן).
    Geminate,
}

/// A 3-radical Hebrew verb/noun root with its detected irregular classes.
#[derive(Debug, Clone)]
pub struct Root {
    pub letters: [char; 3],
    pub classes: Vec<Gizra>,
}

impl Root {
    /// Parse a root from a Hebrew string. Strips niqqud and whitespace,
    /// expects exactly three consonants. Final-form letters (ך/ם/ן/ף/ץ) are
    /// normalised back to their base forms.
    pub fn parse(s: &str) -> Result<Self, RootError> {
        let letters: Vec<char> = s
            .chars()
            .filter_map(|c| {
                let n = c as u32;
                if !(0x05D0..=0x05EA).contains(&n) {
                    return None;
                }
                Some(match c {
                    '\u{05DA}' => letter::KAF,
                    '\u{05DD}' => letter::MEM,
                    '\u{05DF}' => letter::NUN,
                    '\u{05E3}' => letter::PE,
                    '\u{05E5}' => letter::TSADE,
                    other => other,
                })
            })
            .collect();
        if letters.len() != 3 {
            return Err(RootError::WrongLength(letters.len()));
        }
        let letters = [letters[0], letters[1], letters[2]];
        Ok(Root::from_letters(letters))
    }

    pub fn from_letters(letters: [char; 3]) -> Self {
        let classes = detect_gizra(letters);
        Root { letters, classes }
    }

    pub fn pe(&self) -> char {
        self.letters[0]
    }
    pub fn ayin(&self) -> char {
        self.letters[1]
    }
    pub fn lamed(&self) -> char {
        self.letters[2]
    }

    pub fn has(&self, g: Gizra) -> bool {
        self.classes.contains(&g)
    }

    /// Returns true if this root has no irregular features at all.
    pub fn is_strong(&self) -> bool {
        self.classes == [Gizra::Strong]
    }
}

#[derive(Debug)]
pub enum RootError {
    WrongLength(usize),
}

impl std::fmt::Display for RootError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RootError::WrongLength(n) => {
                write!(f, "root must have exactly 3 consonants, got {n}")
            }
        }
    }
}

impl std::error::Error for RootError {}

/// Stative lexemes whose third radical he is consonantal (written with mappiq),
/// not a weak III-He mater. They inflect as strong triliterals. See the use site
/// in [`detect_gizra`].
pub(crate) fn is_consonantal_he_root(letters: [char; 3]) -> bool {
    use letter::*;
    matches!(
        letters,
        [GIMEL, BET, HE]   // גבה  be high
        | [NUN, GIMEL, HE] // נגה  shine
        | [TAV, MEM, HE]   // תמה  be astonished
        | [KAF, MEM, HE] // כמה  long for
    )
}

fn detect_gizra(letters: [char; 3]) -> Vec<Gizra> {
    use letter::*;
    let mut out = Vec::new();
    let [p, a, l] = letters;

    let is_gut = |c: char| matches!(c, ALEF | HE | HET | AYIN);

    // PeAleph is a more specific subset of PeGuttural — emit both so callers
    // can choose the most specific rule.
    if p == ALEF {
        out.push(Gizra::PeAleph);
    }
    if is_gut(p) {
        out.push(Gizra::PeGuttural);
    } else if p == NUN {
        out.push(Gizra::PeNun);
    } else if p == YOD {
        out.push(Gizra::PeYod);
    }

    // Middle radical. A vav/yod middle is hollow only in a genuinely
    // biconsonantal root (קום, שׂים, בוא). When the third radical is he the
    // root is III-He (היה, חיה, צוה, קוה): the middle yod/vav is a true
    // consonant and the verb inflects as lamed-he, not hollow. A few roots have
    // a genuine medial-vav/yod radical and inflect as strong triliterals rather
    // than hollow: איב (ʾōyēḇ "enemy", qōṭēl participle) and גוע (yiḡwaʕ "expire").
    let true_triliteral_c2 = matches!(letters, [ALEF, YOD, BET] | [GIMEL, VAV, AYIN]);
    if (a == VAV || a == YOD) && l != HE && !true_triliteral_c2 {
        out.push(Gizra::Hollow);
    } else if is_gut(a) || a == RESH {
        out.push(Gizra::AyinGuttural);
    }

    // Third radical
    if l == ALEF {
        out.push(Gizra::LamedAleph);
    } else if l == HE {
        // A handful of stative lexemes carry a *consonantal* final he (written
        // with mappiq: gāḇah גָּבַהּ, nāḡah נָגַהּ, tāmah תָּמַהּ, kāmah כָּמַהּ).
        // These do not inflect as weak III-He — the he is a true third radical
        // and the verb runs as a strong triliteral — so they must be kept out of
        // the LamedHe class. Root letters alone can't tell גָּבַהּ from בָּנָה, so
        // this is a lexical exception list.
        if !is_consonantal_he_root(letters) {
            out.push(Gizra::LamedHe);
        }
    } else if matches!(l, HET | AYIN) {
        out.push(Gizra::LamedGuttural);
    }

    // Geminate: 2nd == 3rd. Only flag when middle isn't already classed
    // as hollow (a ≠ vav/yod); a geminate hollow is vanishingly rare. A
    // final he is the weak III-He marker, not a true repeated radical, so
    // C2==C3==he (קהה, כהה, דהה) is III-He, not geminate.
    if a == l && a != VAV && a != YOD && l != HE {
        out.push(Gizra::Geminate);
    }

    if out.is_empty() {
        out.push(Gizra::Strong);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strong_root() {
        let r = Root::parse("קטל").unwrap();
        assert!(r.is_strong());
    }

    #[test]
    fn pe_nun() {
        let r = Root::parse("נפל").unwrap();
        assert!(r.has(Gizra::PeNun));
    }

    #[test]
    fn lamed_he() {
        let r = Root::parse("בנה").unwrap();
        assert!(r.has(Gizra::LamedHe));
    }

    #[test]
    fn hollow() {
        let r = Root::parse("קום").unwrap();
        assert!(r.has(Gizra::Hollow));
    }

    #[test]
    fn geminate() {
        let r = Root::parse("סבב").unwrap();
        assert!(r.has(Gizra::Geminate));
    }

    #[test]
    fn lamed_he_with_weak_middle_is_not_hollow() {
        // היה, חיה, צוה, קוה: a vav/yod middle radical is consonantal when the
        // root is III-He, so these inflect as lamed-he, never hollow.
        for r in ["היה", "צוה", "קוה"] {
            let root = Root::parse(r).unwrap();
            assert!(root.has(Gizra::LamedHe), "{r} should be III-He");
            assert!(!root.has(Gizra::Hollow), "{r} should not be hollow");
        }
    }

    #[test]
    fn doubly_weak_pe_yod_lamed_he() {
        // ירה (to teach, throw)
        let r = Root::parse("ירה").unwrap();
        assert!(r.has(Gizra::PeYod));
        assert!(r.has(Gizra::LamedHe));
    }

    #[test]
    fn pe_aleph_implies_pe_guttural() {
        let r = Root::parse("אכל").unwrap();
        assert!(r.has(Gizra::PeAleph));
        assert!(r.has(Gizra::PeGuttural));
    }

    #[test]
    fn niqqud_stripped() {
        let r = Root::parse("קָטַל").unwrap();
        assert_eq!(r.letters[0], letter::QOF);
    }
}
