//! Hebrew character primitives.
//!
//! Forms are built up as sequences of `Cons` (consonant + attached marks),
//! then rendered to Unicode strings using the project's traditional combining
//! order (letter → dagesh → shin/sin dot → vowel) — the same order used by
//! the existing bible-text data, not Unicode NFC.

/// Hebrew consonants (non-final forms).
pub mod letter {
    pub const ALEF: char = '\u{05D0}';
    pub const BET: char = '\u{05D1}';
    pub const GIMEL: char = '\u{05D2}';
    pub const DALET: char = '\u{05D3}';
    pub const HE: char = '\u{05D4}';
    pub const VAV: char = '\u{05D5}';
    pub const ZAYIN: char = '\u{05D6}';
    pub const HET: char = '\u{05D7}';
    pub const TET: char = '\u{05D8}';
    pub const YOD: char = '\u{05D9}';
    pub const KAF: char = '\u{05DB}';
    pub const LAMED: char = '\u{05DC}';
    pub const MEM: char = '\u{05DE}';
    pub const NUN: char = '\u{05E0}';
    pub const SAMEKH: char = '\u{05E1}';
    pub const AYIN: char = '\u{05E2}';
    pub const PE: char = '\u{05E4}';
    pub const TSADE: char = '\u{05E6}';
    pub const QOF: char = '\u{05E7}';
    pub const RESH: char = '\u{05E8}';
    pub const SHIN: char = '\u{05E9}';
    pub const TAV: char = '\u{05EA}';
}

/// Hebrew niqqud (vowel points + dagesh + sin/shin dots).
pub mod niqqud {
    pub const SHEVA: char = '\u{05B0}';
    pub const HATAF_SEGOL: char = '\u{05B1}';
    pub const HATAF_PATAH: char = '\u{05B2}';
    pub const HATAF_QAMATS: char = '\u{05B3}';
    pub const HIRIQ: char = '\u{05B4}';
    pub const TSERE: char = '\u{05B5}';
    pub const SEGOL: char = '\u{05B6}';
    pub const PATAH: char = '\u{05B7}';
    pub const QAMATS: char = '\u{05B8}';
    pub const HOLAM: char = '\u{05B9}';
    pub const QUBUTS: char = '\u{05BB}';
    pub const DAGESH: char = '\u{05BC}';
    pub const SHIN_DOT: char = '\u{05C1}';
    pub const SIN_DOT: char = '\u{05C2}';
    pub const QAMATS_QATAN: char = '\u{05C7}';
}

/// The five Hebrew letters that take final forms at word-end.
fn final_form(c: char) -> Option<char> {
    match c {
        letter::KAF => Some('\u{05DA}'),
        letter::MEM => Some('\u{05DD}'),
        letter::NUN => Some('\u{05DF}'),
        letter::PE => Some('\u{05E3}'),
        letter::TSADE => Some('\u{05E5}'),
        _ => None,
    }
}

/// Hebrew gutturals: behave specially in nearly every verb class
/// (no dagesh, prefer composite/`hataf` shewa, "compensatory lengthening",
/// patah-furtive at word end).
pub fn is_guttural(c: char) -> bool {
    matches!(c, letter::ALEF | letter::HE | letter::HET | letter::AYIN)
}

pub fn is_sibilant(c: char) -> bool {
    matches!(
        c,
        letter::ZAYIN | letter::SAMEKH | letter::TSADE | letter::SHIN
    )
}

/// Resh behaves like a guttural for non-doubling, but takes vowels normally.
pub fn rejects_dagesh(c: char) -> bool {
    is_guttural(c) || c == letter::RESH
}

/// A Hebrew vowel point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vowel {
    Sheva,
    HatafSegol,
    HatafPatah,
    HatafQamats,
    Hiriq,
    Tsere,
    Segol,
    Patah,
    Qamats,
    Holam,
    Qubuts,
    QamatsQatan,
}

impl Vowel {
    fn niqqud(self) -> char {
        match self {
            Vowel::Sheva => niqqud::SHEVA,
            Vowel::HatafSegol => niqqud::HATAF_SEGOL,
            Vowel::HatafPatah => niqqud::HATAF_PATAH,
            Vowel::HatafQamats => niqqud::HATAF_QAMATS,
            Vowel::Hiriq => niqqud::HIRIQ,
            Vowel::Tsere => niqqud::TSERE,
            Vowel::Segol => niqqud::SEGOL,
            Vowel::Patah => niqqud::PATAH,
            Vowel::Qamats => niqqud::QAMATS,
            Vowel::Holam => niqqud::HOLAM,
            Vowel::Qubuts => niqqud::QUBUTS,
            Vowel::QamatsQatan => niqqud::QAMATS_QATAN,
        }
    }

    /// When a guttural is forced to take a vocal sheva, it instead takes the
    /// matching composite vowel (hataf). For other vowels under a guttural,
    /// no transformation is needed.
    pub fn guttural_sheva(self) -> Vowel {
        match self {
            Vowel::Sheva => Vowel::HatafPatah,
            other => other,
        }
    }
}

/// Sin/shin dot selector (only meaningful when `letter == SHIN`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SinShin {
    Shin,
    Sin,
}

/// What role a `Cons` plays in the verb/noun stem. Affixes are `Affix`;
/// radicals 1/2/3 are the three root consonants. The `Radical` tag lets
/// weak-verb rewrites find the correct slot even when an affix happens to
/// use the same letter as a radical (e.g. 1cp imperfect of a I-Nun root
/// has *two* nuns, and we need to operate on the radical, not the prefix).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Affix,
    Radical(u8),
    /// Matres lectionis or other inserted letters that aren't radicals.
    Mater,
}

/// A single consonant slot with its attached marks.
///
/// One `Cons` is the basic unit a verb/noun form is built from. Matres-
/// lectionis (consonants that mark a vowel, e.g. vav in קוּם, yod in הִקְטִיל,
/// he in קָנֶה) are represented as their own `Cons` with `vowel = None`.
#[derive(Debug, Clone, Copy)]
pub struct Cons {
    pub letter: char,
    pub vowel: Option<Vowel>,
    pub dagesh: bool,
    pub sin_shin: Option<SinShin>,
    pub role: Role,
}

impl Cons {
    pub fn new(letter: char) -> Self {
        Cons {
            letter,
            vowel: None,
            dagesh: false,
            sin_shin: if letter == letter::SHIN {
                Some(SinShin::Shin)
            } else {
                None
            },
            role: Role::Affix,
        }
    }

    pub fn radical(letter: char, n: u8) -> Self {
        let mut c = Cons::new(letter);
        c.role = Role::Radical(n);
        c
    }

    pub fn mater(letter: char) -> Self {
        let mut c = Cons::new(letter);
        c.role = Role::Mater;
        c
    }

    pub fn with_vowel(mut self, v: Vowel) -> Self {
        self.vowel = Some(v);
        self
    }

    pub fn with_dagesh(mut self) -> Self {
        self.dagesh = true;
        self
    }

    pub fn as_sin(mut self) -> Self {
        self.sin_shin = Some(SinShin::Sin);
        self
    }
}

/// Parse a fully-pointed Hebrew word into `Cons` slots. Each consonant is one
/// slot; the marks that follow it (dagesh, shin/sin dot, vowel) attach to it.
/// Final-form letters are normalised back to their base forms so `render`
/// re-applies them. Characters outside the niqqud set we model (cantillation,
/// meteg, maqaf, …) are ignored, so cantillated text parses to its bare
/// consonant+vowel form.
pub fn parse_pointed(text: &str) -> Vec<Cons> {
    let mut out: Vec<Cons> = Vec::new();
    for ch in text.chars() {
        let n = ch as u32;
        if (0x05D0..=0x05EA).contains(&n) {
            let base = match ch {
                '\u{05DA}' => letter::KAF,
                '\u{05DD}' => letter::MEM,
                '\u{05DF}' => letter::NUN,
                '\u{05E3}' => letter::PE,
                '\u{05E5}' => letter::TSADE,
                c => c,
            };
            out.push(Cons::new(base));
        } else if let Some(last) = out.last_mut() {
            match n {
                0x05B0 => last.vowel = Some(Vowel::Sheva),
                0x05B1 => last.vowel = Some(Vowel::HatafSegol),
                0x05B2 => last.vowel = Some(Vowel::HatafPatah),
                0x05B3 => last.vowel = Some(Vowel::HatafQamats),
                0x05B4 => last.vowel = Some(Vowel::Hiriq),
                0x05B5 => last.vowel = Some(Vowel::Tsere),
                0x05B6 => last.vowel = Some(Vowel::Segol),
                0x05B7 => last.vowel = Some(Vowel::Patah),
                0x05B8 => last.vowel = Some(Vowel::Qamats),
                0x05B9 => last.vowel = Some(Vowel::Holam),
                0x05BB => last.vowel = Some(Vowel::Qubuts),
                0x05BC => last.dagesh = true,
                0x05C1 => last.sin_shin = Some(SinShin::Shin),
                0x05C2 => last.sin_shin = Some(SinShin::Sin),
                0x05C7 => last.vowel = Some(Vowel::QamatsQatan),
                _ => {}
            }
        }
    }
    out
}

/// One of the בגדכפת (begedkefet) letters that take a dagesh lene when
/// they begin a syllable (word start or after a silent sheva).
pub(crate) fn is_begedkefet(c: char) -> bool {
    matches!(
        c,
        letter::BET | letter::GIMEL | letter::DALET | letter::KAF | letter::PE | letter::TAV
    )
}

/// Render a slice of `Cons` to a Hebrew Unicode string.
///
/// Applies final-letter forms to the last consonant where applicable, and
/// applies a dagesh lene to a word-initial begedkefet letter that wasn't
/// already dageshed. Combining order for each slot:
/// letter → dagesh → shin/sin dot → vowel.
pub fn render(seq: &[Cons]) -> String {
    let mut out = String::new();
    let last_idx = seq.len().saturating_sub(1);
    for (i, c) in seq.iter().enumerate() {
        let letter = if i == last_idx {
            final_form(c.letter).unwrap_or(c.letter)
        } else {
            c.letter
        };
        out.push(letter);
        // Dagesh: forte (already marked) or lene at word start on begedkefet.
        let needs_dagesh = c.dagesh || (i == 0 && is_begedkefet(c.letter));
        if needs_dagesh {
            out.push(niqqud::DAGESH);
        }
        match c.sin_shin {
            Some(SinShin::Shin) => out.push(niqqud::SHIN_DOT),
            Some(SinShin::Sin) => out.push(niqqud::SIN_DOT),
            None => {}
        }
        if let Some(v) = c.vowel {
            out.push(v.niqqud());
        } else if i == last_idx && c.letter == letter::KAF {
            // Masoretic convention writes a silent sheva under a vowel-less
            // word-final kaf (מֶלֶךְ, מָלַךְ, וַיֵּלֶךְ). It's the one final letter
            // that reliably carries it, so the generator's bare output would
            // otherwise never match the pointed surface.
            out.push(Vowel::Sheva.niqqud());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_strong_qal_perfect_3ms() {
        // קָטַל — qof+qamats, tet+patah, lamed
        let seq = [
            Cons::new(letter::QOF).with_vowel(Vowel::Qamats),
            Cons::new(letter::TET).with_vowel(Vowel::Patah),
            Cons::new(letter::LAMED),
        ];
        assert_eq!(render(&seq), "\u{05E7}\u{05B8}\u{05D8}\u{05B7}\u{05DC}",);
    }

    #[test]
    fn render_applies_final_form() {
        // מֶלֶךְ → final kaf, which takes a Masoretic silent sheva when vowel-less
        let seq = [
            Cons::new(letter::MEM).with_vowel(Vowel::Segol),
            Cons::new(letter::LAMED).with_vowel(Vowel::Segol),
            Cons::new(letter::KAF),
        ];
        assert_eq!(render(&seq), "\u{05DE}\u{05B6}\u{05DC}\u{05B6}\u{05DA}\u{05B0}",);
    }

    #[test]
    fn render_shin_with_dot() {
        // שָׁמַר — traditional order: letter, shin-dot, then vowel.
        let seq = [
            Cons::new(letter::SHIN).with_vowel(Vowel::Qamats),
            Cons::new(letter::MEM).with_vowel(Vowel::Patah),
            Cons::new(letter::RESH),
        ];
        assert_eq!(
            render(&seq),
            "\u{05E9}\u{05C1}\u{05B8}\u{05DE}\u{05B7}\u{05E8}",
        );
    }
}
