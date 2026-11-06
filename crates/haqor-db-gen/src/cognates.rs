//! The regular sound correspondences between Hebrew and the other Semitic
//! languages, for finding a Hebrew root's candidate cognates in their
//! dictionaries (ADR 7).
//!
//! Hebrew merged several Proto-Semitic consonants that other languages keep
//! apart: ח is *ḥ or *ḫ, ע is *ʿ or *ġ, ז is *z or *ḏ, צ is *ṣ, *ḍ or *ẓ, and
//! a shin is *š or *ṯ. A Hebrew root therefore stands for a few Proto-Semitic
//! roots, and each of those is spelled one way in each language. This module
//! expands a root key into those spellings. They are candidates, to be looked
//! up and judged from the dictionaries' glosses; a correspondence never proves
//! a cognate.
//!
//! The weak letters are expanded too: a Hebrew initial י is often an older *w
//! (ילד, Arabic ولد), a final ה stands for a *w or *y, and a middle ו or י
//! for either.

/// A Proto-Semitic consonant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Proto {
    Glottal, // *ʔ
    B,       // *b
    G,       // *g
    D,       // *d
    Dh,      // *ḏ
    H,       // *h
    W,       // *w
    Z,       // *z
    HDot,    // *ḥ
    Kh,      // *ḫ
    TDot,    // *ṭ
    ZDot,    // *ẓ (*θ̣)
    Y,       // *y
    K,       // *k
    L,       // *l
    M,       // *m
    N,       // *n
    S,       // *s (s₃)
    Ayin,    // *ʕ
    Ghain,   // *ġ
    P,       // *p
    SDot,    // *ṣ
    DDot,    // *ḍ (*ṣ́)
    Q,       // *q
    R,       // *r
    Shin,    // *š (s₁)
    SinLat,  // *ś (s₂)
    Th,      // *ṯ
    T,       // *t
}

use Proto::*;

/// A language whose dictionaries a root is looked up in, each spelled as its
/// dictionaries file roots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lang {
    /// Arabic script, as Lane and the Arabic dictionaries head their roots.
    Arabic,
    /// Hebrew letters, as SEDRA and Jastrow spell Aramaic and Syriac.
    Aramaic,
    /// Latin transliteration of Ge'ez consonants (`qtl`, `ḥ`, `ś`, `ḍ`).
    Geez,
    /// Latin transliteration of Akkadian consonants; the gutturals it lost
    /// spell nothing.
    Akkadian,
    /// Latin transliteration of Ugaritic consonants.
    Ugaritic,
    /// Latin transliteration of Old South Arabian (`s¹`, `s²`, `s³`).
    SouthArabian,
}

/// The Proto-Semitic consonants a Hebrew consonant may continue, the commonest
/// first; `initial`/`last` mark the root position, where the weak letters
/// stand for older ones.
fn proto_of(letter: &str, initial: bool, last: bool) -> &'static [Proto] {
    match letter {
        "א" => &[Glottal],
        "ב" => &[B],
        "ג" => &[G],
        "ד" => &[D],
        "ה" if last => &[Y, W, H],
        "ה" => &[H],
        "ו" => &[W, Y],
        "ז" => &[Z, Dh],
        "ח" => &[HDot, Kh],
        "ט" => &[TDot],
        "י" if initial => &[W, Y],
        "י" => &[Y, W],
        "כ" => &[K],
        "ל" => &[L],
        "מ" => &[M],
        "נ" => &[N],
        "ס" => &[S],
        "ע" => &[Ayin, Ghain],
        "פ" => &[P],
        "צ" => &[SDot, DDot, ZDot],
        "ק" => &[Q],
        "ר" => &[R],
        "ש" => &[Shin, Th],
        "שׂ" => &[SinLat],
        "ת" => &[T],
        _ => &[],
    }
}

/// How `lang` spells a Proto-Semitic consonant; empty where it lost it.
pub fn spell(lang: Lang, p: Proto) -> &'static str {
    match lang {
        Lang::Arabic => match p {
            Glottal => "ء",
            B => "ب",
            G => "ج",
            D => "د",
            Dh => "ذ",
            H => "ه",
            W => "و",
            Z => "ز",
            HDot => "ح",
            Kh => "خ",
            TDot => "ط",
            ZDot => "ظ",
            Y => "ي",
            K => "ك",
            L => "ل",
            M => "م",
            N => "ن",
            S => "س",
            Ayin => "ع",
            Ghain => "غ",
            P => "ف",
            SDot => "ص",
            DDot => "ض",
            Q => "ق",
            R => "ر",
            Shin => "س",
            SinLat => "ش",
            Th => "ث",
            T => "ت",
        },
        Lang::Aramaic => match p {
            Glottal => "א",
            B => "ב",
            G => "ג",
            D | Dh => "ד",
            H => "ה",
            W => "ו",
            Z => "ז",
            HDot | Kh => "ח",
            TDot | ZDot => "ט",
            Y => "י",
            K => "כ",
            L => "ל",
            M => "מ",
            N => "נ",
            S | SinLat => "ס",
            Ayin | Ghain | DDot => "ע",
            P => "פ",
            SDot => "צ",
            Q => "ק",
            R => "ר",
            Shin => "ש",
            Th | T => "ת",
        },
        Lang::Geez => match p {
            Glottal => "ʾ",
            B => "b",
            G => "g",
            D => "d",
            Dh | Z => "z",
            H => "h",
            W => "w",
            HDot => "ḥ",
            Kh => "ḫ",
            TDot => "ṭ",
            ZDot | SDot => "ṣ",
            Y => "y",
            K => "k",
            L => "l",
            M => "m",
            N => "n",
            S | Shin | Th => "s",
            Ayin | Ghain => "ʿ",
            P => "f",
            DDot => "ḍ",
            Q => "q",
            R => "r",
            SinLat => "ś",
            T => "t",
        },
        Lang::Akkadian => match p {
            Glottal | H | HDot | Ayin | Ghain => "",
            B => "b",
            G => "g",
            D => "d",
            Dh | Z => "z",
            W => "w",
            Kh => "ḫ",
            TDot => "ṭ",
            ZDot | SDot | DDot => "ṣ",
            Y => "y",
            K => "k",
            L => "l",
            M => "m",
            N => "n",
            S => "s",
            P => "p",
            Q => "q",
            R => "r",
            Shin | SinLat | Th => "š",
            T => "t",
        },
        Lang::Ugaritic => match p {
            Glottal => "ʾ",
            B => "b",
            G => "g",
            D => "d",
            Dh => "ḏ",
            H => "h",
            W => "w",
            Z => "z",
            HDot => "ḥ",
            Kh => "ḫ",
            TDot => "ṭ",
            ZDot => "ẓ",
            Y => "y",
            K => "k",
            L => "l",
            M => "m",
            N => "n",
            S => "s",
            Ayin => "ʿ",
            Ghain => "ġ",
            P => "p",
            SDot | DDot => "ṣ",
            Q => "q",
            R => "r",
            Shin | SinLat => "š",
            Th => "ṯ",
            T => "t",
        },
        Lang::SouthArabian => match p {
            Glottal => "ʾ",
            B => "b",
            G => "g",
            D => "d",
            Dh => "ḏ",
            H => "h",
            W => "w",
            Z => "z",
            HDot => "ḥ",
            Kh => "ḫ",
            TDot => "ṭ",
            ZDot => "ẓ",
            Y => "y",
            K => "k",
            L => "l",
            M => "m",
            N => "n",
            S => "s³",
            Ayin => "ʿ",
            Ghain => "ġ",
            P => "f",
            SDot => "ṣ",
            DDot => "ḍ",
            Q => "q",
            R => "r",
            Shin => "s¹",
            SinLat => "s²",
            Th => "ṯ",
            T => "t",
        },
    }
}

/// The letters of a root key, a sin (ש with its dot) counting as one.
fn letters(root: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for c in root.chars() {
        if c == '\u{05C2}' {
            if let Some(last) = out.last_mut() {
                last.push(c);
            }
        } else if ('\u{05D0}'..='\u{05EA}').contains(&c) {
            out.push(c.to_string());
        }
    }
    out
}

/// The Proto-Semitic roots a Hebrew root key may continue, the likeliest
/// first.
pub fn proto_roots(root: &str) -> Vec<Vec<Proto>> {
    let letters = letters(root);
    let n = letters.len();
    let mut roots: Vec<Vec<Proto>> = vec![Vec::new()];
    for (i, letter) in letters.iter().enumerate() {
        let options = proto_of(letter, i == 0, i + 1 == n);
        if options.is_empty() {
            return Vec::new();
        }
        roots = roots
            .into_iter()
            .flat_map(|r| {
                options.iter().map(move |p| {
                    let mut r = r.clone();
                    r.push(*p);
                    r
                })
            })
            .collect();
    }
    roots
}

/// A Hebrew root key's candidate spellings in `lang`, the likeliest first,
/// without repeats.
pub fn cognate_keys(root: &str, lang: Lang) -> Vec<String> {
    let mut keys: Vec<String> = Vec::new();
    for proto in proto_roots(root) {
        let key: String = proto.iter().map(|p| spell(lang, *p)).collect();
        if !key.is_empty() && !keys.contains(&key) {
            keys.push(key);
        }
    }
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    fn has(root: &str, lang: Lang, key: &str) -> bool {
        cognate_keys(root, lang).iter().any(|k| k == key)
    }

    #[test]
    fn arabic_keeps_the_consonants_hebrew_merged() {
        assert!(has("שמע", Lang::Arabic, "سمع"));
        assert!(has("ארצ", Lang::Arabic, "ءرض"));
        assert!(has("זכר", Lang::Arabic, "ذكر"));
        assert!(has("שלש", Lang::Arabic, "ثلث"));
        assert!(has("ילד", Lang::Arabic, "ولد"));
        assert!(has("עשׂה", Lang::Arabic, "عشي"));
        // The commonest reading leads.
        assert_eq!(cognate_keys("שמע", Lang::Arabic)[0], "سمع");
    }

    #[test]
    fn aramaic_spells_them_its_own_way() {
        assert!(has("ארצ", Lang::Aramaic, "ארע"));
        assert!(has("שלש", Lang::Aramaic, "תלת"));
        assert!(has("זהב", Lang::Aramaic, "דהב"));
    }

    #[test]
    fn akkadian_loses_its_gutturals() {
        assert!(has("שמע", Lang::Akkadian, "šm"));
        assert!(has("ארצ", Lang::Akkadian, "rṣ"));
    }

    #[test]
    fn south_arabian_keeps_three_sibilants() {
        assert!(has("שמע", Lang::SouthArabian, "s¹mʿ"));
        assert!(has("עשׂה", Lang::SouthArabian, "ʿs²y"));
    }

    #[test]
    fn an_unknown_letter_gives_nothing() {
        assert!(cognate_keys("abc", Lang::Arabic).is_empty());
    }
}
