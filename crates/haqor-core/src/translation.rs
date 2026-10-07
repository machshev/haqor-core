//! The English translation of the Old Testament, as `haqor.db`'s
//! `translation_verse` table stores it.
//!
//! The translation is adapted from the unfoldingWord® Literal Text (ULT,
//! CC BY-SA 4.0), whose every English word is aligned to the Hebrew word or
//! words it renders. `haqor-db-gen`'s `translation` module imports it, places those
//! Hebrew words on the corpus, and files each English verse under the Hebrew
//! verse its words render, so a verse's English sits beside its Hebrew
//! whatever the two numberings say (Malachi 4:1 in English is 3:19 here, and
//! a psalm's title is its first verse).
//!
//! In the stored form a verse is running text in which `[text|links]` marks
//! English that renders particular Hebrew words. `links` is a comma-separated
//! list, the main word first, each a word `position` in the same verse,
//! `verse:position` in another verse of the chapter, or
//! `chapter:verse:position`. Text between `{` and `}` is supplied: English
//! the sense needs that no Hebrew word says. A backslash escapes the next
//! character.

/// A run of a verse's English: one or more words rendering the same Hebrew,
/// or text (punctuation, spaces, a word aligned to nothing) rendering none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranslationSpan {
    pub text: String,
    /// Whether the English is supplied for sense rather than rendering any
    /// word, which printed literal translations set in italics.
    pub supplied: bool,
    /// The Hebrew words the text renders, the main one first; empty for text
    /// rendering none.
    pub words: Vec<TranslationWord>,
}

/// A Hebrew word an English span renders. Usually in the span's own verse,
/// but where the English and Hebrew divide verses differently it may be in
/// the next or previous one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TranslationWord {
    pub chapter: u8,
    pub verse: u8,
    /// The word's position in its verse, as the reader numbers them.
    pub position: u16,
}

/// Read one verse's stored English, `chapter:verse` being where it is filed.
/// `None` when it does not parse, which text the generator wrote always does.
pub fn parse(text: &str, chapter: u8, verse: u8) -> Option<Vec<TranslationSpan>> {
    let mut spans: Vec<TranslationSpan> = Vec::new();
    let mut supplied = false;
    let mut plain = String::new();
    let mut chars = text.chars();
    // Plain text so far ends here: as a span of its own, unless it is empty.
    let flush = |plain: &mut String, spans: &mut Vec<TranslationSpan>, supplied: bool| {
        if !plain.is_empty() {
            spans.push(TranslationSpan {
                text: std::mem::take(plain),
                supplied,
                words: Vec::new(),
            });
        }
    };
    while let Some(c) = chars.next() {
        match c {
            '\\' => plain.push(chars.next()?),
            '{' | '}' => {
                flush(&mut plain, &mut spans, supplied);
                supplied = c == '{';
            }
            '[' => {
                flush(&mut plain, &mut spans, supplied);
                let mut words_text = String::new();
                loop {
                    match chars.next()? {
                        '\\' => words_text.push(chars.next()?),
                        '|' => break,
                        c => words_text.push(c),
                    }
                }
                let mut links = String::new();
                loop {
                    match chars.next()? {
                        ']' => break,
                        c => links.push(c),
                    }
                }
                let words = links
                    .split(',')
                    .map(|link| word(link, chapter, verse))
                    .collect::<Option<Vec<_>>>()?;
                spans.push(TranslationSpan {
                    text: words_text,
                    supplied,
                    words,
                });
            }
            c => plain.push(c),
        }
    }
    flush(&mut plain, &mut spans, supplied);
    Some(spans)
}

/// One link of a `[text|links]`, relative to the verse it is filed under.
fn word(link: &str, chapter: u8, verse: u8) -> Option<TranslationWord> {
    let parts: Vec<&str> = link.split(':').collect();
    let (chapter, verse, position) = match parts[..] {
        [p] => (chapter, verse, p),
        [v, p] => (chapter, v.parse().ok()?, p),
        [c, v, p] => (c.parse().ok()?, v.parse().ok()?, p),
        _ => return None,
    };
    Some(TranslationWord {
        chapter,
        verse,
        position: position.parse().ok()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(verse: u8, position: u16) -> TranslationWord {
        TranslationWord {
            chapter: 1,
            verse,
            position,
        }
    }

    #[test]
    fn reads_linked_and_plain_text() {
        let spans = parse("[In the beginning|0] [God|2] [created|1].", 1, 1).unwrap();
        let shape: Vec<(&str, Vec<TranslationWord>)> = spans
            .iter()
            .map(|s| (s.text.as_str(), s.words.clone()))
            .collect();
        assert_eq!(
            shape,
            [
                ("In the beginning", vec![at(1, 0)]),
                (" ", vec![]),
                ("God", vec![at(1, 2)]),
                (" ", vec![]),
                ("created", vec![at(1, 1)]),
                (".", vec![]),
            ]
        );
        assert!(spans.iter().all(|s| !s.supplied));
    }

    /// A span may render several words, and words in other verses and
    /// chapters.
    #[test]
    fn reads_links_beyond_the_verse() {
        let spans = parse("[the heavens|4,3,2:0,3:19:5]", 1, 1).unwrap();
        assert_eq!(
            spans[0].words,
            [
                at(1, 4),
                at(1, 3),
                at(2, 0),
                TranslationWord {
                    chapter: 3,
                    verse: 19,
                    position: 5
                }
            ]
        );
    }

    #[test]
    fn marks_supplied_text() {
        let spans = parse("[darkness|7] {[was|8]} [over|8]", 1, 2).unwrap();
        let shape: Vec<(&str, bool)> = spans
            .iter()
            .map(|s| (s.text.as_str(), s.supplied))
            .collect();
        assert_eq!(
            shape,
            [
                ("darkness", false),
                (" ", false),
                ("was", true),
                (" ", false),
                ("over", false)
            ]
        );
    }

    #[test]
    fn unescapes() {
        let spans = parse(r"\[sic\] [a \| b \] c|0]", 1, 1).unwrap();
        assert_eq!(spans[0].text, "[sic] ");
        assert_eq!(spans[1].text, "a | b ] c");
    }

    #[test]
    fn rejects_broken_text() {
        assert!(parse("[unclosed|0", 1, 1).is_none());
        assert!(parse("[bad link|x]", 1, 1).is_none());
        assert!(parse("[too many|1:2:3:4]", 1, 1).is_none());
    }
}
