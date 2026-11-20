//! Human-readable progress for the review coordinator.
use crate::jobs::Job;
use std::{collections::BTreeMap, time::Instant};

const BOOKS: [&str; 66] = [
    "Genesis",
    "Exodus",
    "Leviticus",
    "Numbers",
    "Deuteronomy",
    "Joshua",
    "Judges",
    "1 Samuel",
    "2 Samuel",
    "1 Kings",
    "2 Kings",
    "Isaiah",
    "Jeremiah",
    "Ezekiel",
    "Hosea",
    "Joel",
    "Amos",
    "Obadiah",
    "Jonah",
    "Micah",
    "Nahum",
    "Habakkuk",
    "Zephaniah",
    "Haggai",
    "Zechariah",
    "Malachi",
    "Psalms",
    "Proverbs",
    "Job",
    "Song of Songs",
    "Ruth",
    "Lamentations",
    "Ecclesiastes",
    "Esther",
    "Daniel",
    "Ezra",
    "Nehemiah",
    "1 Chronicles",
    "2 Chronicles",
    "Matthew",
    "Mark",
    "Luke",
    "John",
    "Acts",
    "Romans",
    "1 Corinthians",
    "2 Corinthians",
    "Galatians",
    "Ephesians",
    "Philippians",
    "Colossians",
    "1 Thessalonians",
    "2 Thessalonians",
    "1 Timothy",
    "2 Timothy",
    "Titus",
    "Philemon",
    "Hebrews",
    "James",
    "1 Peter",
    "2 Peter",
    "1 John",
    "2 John",
    "3 John",
    "Jude",
    "Revelation",
];
fn passage(job: &Job) -> String {
    let book = BOOKS
        .get(((job.reference >> 16) as usize).wrapping_sub(1))
        .unwrap_or(&"?");
    format!(
        "{} {book} {}:{}",
        job.source,
        (job.reference >> 8) & 255,
        job.reference & 255
    )
}
pub fn label(job: &Job) -> String {
    let at = passage(job);
    if job.pos < 0 {
        format!("{at} verse review")
    } else {
        format!("{at} word[{}]", job.pos)
    }
}

pub struct Progress {
    pub active: BTreeMap<String, Instant>,
    pub done: i64,
    pub total: i64,
    pub limit: usize,
}
impl Progress {
    pub fn log(&self, event: &str) {
        eprintln!(
            "[{}/{} active | {}/{} verses reviewed] {event}",
            self.active.len(),
            self.limit,
            self.done,
            self.total
        );
    }
    pub fn heartbeat(&self) {
        let jobs = self
            .active
            .iter()
            .map(|(name, since)| format!("{name} ({}s)", since.elapsed().as_secs()))
            .collect::<Vec<_>>();
        self.log(&jobs.join(", "));
    }
}
