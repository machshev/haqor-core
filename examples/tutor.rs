//! Drive the spaced-repetition tutor headless against the in-repo data DBs,
//! always answering "Good", and print each study item:
//!   cargo run --example tutor -- [steps]
//!
//! Uses a throwaway in-memory progress.db so runs are reproducible.

use haqor_core::bible::Bible;
use haqor_core::tutor::{Grade, StudyItem, Track};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let steps: usize = std::env::args().nth(1).map_or(Ok(60), |a| a.parse())?;

    let bible = Bible::open("data")?;
    bible.attach_progress(":memory:")?; // throwaway, reproducible

    // Each card is answered with submit_review, whose return value is the next
    // card (one round-trip). ReadVerse carries no grade, so we advance with
    // next_study_item after showing it.
    let now = 1_700_000_000;
    let mut item = bible.next_study_item(now)?;
    for i in 0..steps {
        item = match item {
            StudyItem::NewGlyph(g) => {
                println!("{i:>3}  NEW GLYPH   {}  (consonant={})", g.glyph, g.is_consonant);
                bible.submit_review(Track::Glyph, &g.glyph, Grade::Good, now)?
            }
            StudyItem::ReviewGlyph(g) => {
                println!("{i:>3}  rev glyph   {}", g.glyph);
                bible.submit_review(Track::Glyph, &g.glyph, Grade::Good, now)?
            }
            StudyItem::NewWord(w) => {
                println!(
                    "{i:>3}  NEW WORD    {}  ({}x)  {} [{}] new_glyphs={}",
                    w.surface, w.occurrences, w.gloss, w.morph, w.new_glyphs.len()
                );
                bible.submit_review(Track::Word, &w.surface, Grade::Good, now)?
            }
            StudyItem::ReviewWord(w) => {
                println!("{i:>3}  rev word    {}", w.surface);
                bible.submit_review(Track::Word, &w.surface, Grade::Good, now)?
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
        "\nprogress: {} glyphs, {} words known, {}/{} verses readable",
        p.glyphs_known, p.words_known, p.verses_readable, p.total_verses
    );
    Ok(())
}
