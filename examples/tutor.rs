//! Drive the spaced-repetition tutor headless against the in-repo data DBs,
//! always answering "Good", and print each study item:
//!   cargo run --example tutor -- [steps] [letters_ratio]
//!
//! `letters_ratio` (default 30) is the letters↔words focus slider; 100
//! reproduces a learner who finishes the alphabet with almost no vocabulary.
//! Uses a throwaway in-memory progress.db so runs are reproducible.

use haqor_core::bible::Bible;
use haqor_core::tutor::{Grade, StudyItem, Track, TutorSettings};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let steps: usize = std::env::args().nth(1).map_or(Ok(60), |a| a.parse())?;
    let letters_ratio: u8 = std::env::args()
        .nth(2)
        .map_or(Ok(TutorSettings::default().letters_ratio), |a| a.parse())?;

    let bible = Bible::open("data")?;
    bible.attach_progress(":memory:")?; // throwaway, reproducible
    bible.set_tutor_settings(&TutorSettings {
        letters_ratio,
        ..TutorSettings::default()
    })?;

    // Each card is answered with submit_review, whose return value is the next
    // card (one round-trip). ReadVerse carries no grade, so we advance with
    // next_study_item after showing it.
    let now = 1_700_000_000;
    let mut item = bible.next_study_item(now)?;
    for i in 0..steps {
        item = match item {
            StudyItem::NewGlyph(g) => {
                println!(
                    "{i:>3}  NEW GLYPH   {}  (consonant={}, host={:?}, voiced={:?})",
                    g.glyph, g.is_consonant, g.host, g.voiced
                );
                bible.submit_review(Track::Glyph, &g.glyph, Grade::Good, now)?
            }
            StudyItem::ReviewGlyph(g) => {
                println!(
                    "{i:>3}  rev glyph   {}  host={:?} voiced={:?} d={:?} vd={:?}",
                    g.glyph, g.host, g.voiced, g.distractors, g.voiced_distractors
                );
                bible.submit_review(Track::Glyph, &g.glyph, Grade::Good, now)?
            }
            StudyItem::NewWord(w) => {
                println!(
                    "{i:>3}  NEW WORD  {}  \"{}\"  ({}x)  {} [{}]",
                    w.surface, w.translit, w.occurrences, w.gloss, w.morph
                );
                bible.submit_review(Track::Word, &w.surface, Grade::Good, now)?
            }
            StudyItem::ReviewWord(w) => {
                println!("{i:>3}  rev word  {}  d={:?}", w.surface, w.distractors);
                bible.submit_review(Track::Word, &w.surface, Grade::Good, now)?
            }
            StudyItem::NewFormDrill(w) => {
                println!(
                    "{i:>3}  NEW FORM  {}  → {} [{}]",
                    w.surface, w.gloss, w.morph
                );
                bible.submit_review(Track::Form, &w.surface, Grade::Good, now)?
            }
            StudyItem::ReviewFormDrill(w) => {
                println!("{i:>3}  rev form  {}", w.surface);
                bible.submit_review(Track::Form, &w.surface, Grade::Good, now)?
            }
            StudyItem::NewSuffixDrill(s) => {
                println!(
                    "{i:>3}  NEW SUFFIX  [{}] {}+{} = {} ({})",
                    s.key, s.stem, s.suffix, s.meaning, s.surface
                );
                bible.submit_review(Track::Suffix, &s.key, Grade::Good, now)?
            }
            StudyItem::ReviewSuffixDrill(s) => {
                println!(
                    "{i:>3}  rev suffix  [{}] on {}  d={:?}",
                    s.key, s.surface, s.distractors
                );
                bible.submit_review(Track::Suffix, &s.key, Grade::Good, now)?
            }
            StudyItem::ExplainIntro(key) => {
                println!("{i:>3}  INTRO     [{key}]");
                bible.next_study_item(now)?
            }
            StudyItem::ExplainMark(g) => {
                println!("{i:>3}  MARK      {}", g.glyph);
                bible.next_study_item(now)?
            }
            StudyItem::ExplainGrammar(c) => {
                println!("{i:>3}  GRAMMAR   [{}] {}", c.concept, c.title);
                bible.next_study_item(now)?
            }
            StudyItem::ExplainFinalForms(g) => {
                println!("{i:>3}  FINALS    {}", g.glyph);
                bible.next_study_item(now)?
            }
            StudyItem::ReadVerse(v) => {
                println!(
                    "{i:>3}  ===READ===  {} {}:{}   examples={:?}",
                    v.book, v.chapter, v.verse, v.examples
                );
                bible.next_study_item(now)?
            }
            StudyItem::Done => {
                println!("{i:>3}  DONE — everything readable");
                break;
            }
        };
    }

    let p = bible.tutor_progress()?;
    println!(
        "\nprogress: {} letters, {} vowels, {} words known, {}/{} verses readable",
        p.letters_known, p.vowels_known, p.words_known, p.verses_readable, p.total_verses
    );
    Ok(())
}
