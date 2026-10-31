//! Event sequences: runs of the Bible's events whose order is known better
//! than their dates, from Haqor's `data/event_sequences.json` (see
//! `data/EVENT_SEQUENCES.md`).
//!
//! A sequence is not a timeline. Its events are ordered by what the text
//! says comes before what — each such claim an edge with the basis it rests
//! on, "Mark's order" or "John 2:13" — and two events no chain of edges
//! joins are of unknown order. A sequence may expand one event, "Holy
//! Week", which then stands for it wherever that event is drawn; one that
//! expands none stands for itself on the Bible's timeline.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::{Context, Result, bail};
use log::info;
use rusqlite::{Connection, params};
use serde_json::Value;

/// The file of Haqor's sequences, in the data directory.
pub const EVENT_SEQUENCES: &str = "event_sequences.json";

pub const SCHEMA: &str = "
DROP TABLE IF EXISTS event_sequence;
DROP TABLE IF EXISTS event_sequence_member;
DROP TABLE IF EXISTS event_sequence_edge;

-- A sequence of events (bible_event), in the file's order. `expands` is the
-- event it tells the inside of, or NULL for one standing for itself.
CREATE TABLE event_sequence(
    seq_id  INTEGER PRIMARY KEY,
    key     TEXT    NOT NULL UNIQUE,
    title   TEXT    NOT NULL,
    note    TEXT    NOT NULL,
    expands INTEGER
);

-- Its events; an event is in one sequence at most.
CREATE TABLE event_sequence_member(
    seq_id   INTEGER NOT NULL,
    event_id INTEGER NOT NULL UNIQUE,
    PRIMARY KEY(seq_id, event_id)
) WITHOUT ROWID;

-- That one of its events came before another, and on what basis: a row a
-- basis, so an order several witnesses give has several.
CREATE TABLE event_sequence_edge(
    seq_id    INTEGER NOT NULL,
    before_id INTEGER NOT NULL,
    after_id  INTEGER NOT NULL,
    basis     TEXT    NOT NULL,
    PRIMARY KEY(seq_id, before_id, after_id, basis)
) WITHOUT ROWID;
";

/// A sequence as the file gives it.
struct Sequence {
    id: String,
    title: String,
    note: String,
    /// The title of the event it expands.
    expands: Option<String>,
    chains: Vec<Chain>,
}

/// Events in the order `basis` puts them, each before the next; a chain of
/// one makes its event a member without ordering it.
struct Chain {
    basis: String,
    events: Vec<String>,
}

/// The sequences of the file's JSON, failing on a field it does not know.
fn read(json: &Value) -> Result<Vec<Sequence>> {
    let text = |value: &Value, key: &str| value[key].as_str().unwrap_or_default().to_string();
    let known = |value: &Value, keys: &[&str]| -> Result<()> {
        for key in value.as_object().into_iter().flat_map(|o| o.keys()) {
            if !keys.contains(&key.as_str()) {
                bail!("an unknown field {key:?} in {value}");
            }
        }
        Ok(())
    };
    known(json, &["sequences"])?;
    let mut sequences = Vec::new();
    for sequence in json["sequences"].as_array().into_iter().flatten() {
        known(sequence, &["id", "title", "note", "expands", "chains"])?;
        let mut chains = Vec::new();
        for chain in sequence["chains"].as_array().into_iter().flatten() {
            known(chain, &["basis", "events"])?;
            chains.push(Chain {
                basis: text(chain, "basis"),
                events: chain["events"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|e| e.as_str().map(str::to_string))
                    .collect::<Option<_>>()
                    .with_context(|| format!("an event that is not a title in {chain}"))?,
            });
        }
        sequences.push(Sequence {
            id: text(sequence, "id"),
            title: text(sequence, "title"),
            note: text(sequence, "note"),
            expands: sequence["expands"].as_str().map(str::to_string),
            chains,
        });
    }
    Ok(sequences)
}

/// What a build wrote, for the log and the tests.
#[derive(Debug, Default, PartialEq)]
pub struct SequencesSummary {
    pub sequences: usize,
    pub members: usize,
    pub edges: usize,
}

/// Write the sequences of `path` into the tables, their events found by
/// title among `bible_event`'s, which must be built. Fails, naming every
/// fault, on a title naming no event or several, an event in two sequences
/// or expanded by one it is in, an empty chain or basis, a sequence nested
/// in itself, or an order that goes round in a circle.
pub(crate) fn build(tx: &Connection, path: &Path) -> Result<SequencesSummary> {
    let json: Value = serde_json::from_str(
        &std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?,
    )
    .with_context(|| format!("parsing {}", path.display()))?;
    let sequences = read(&json).with_context(|| format!("reading {}", path.display()))?;

    let mut by_title: HashMap<String, Vec<u32>> = HashMap::new();
    for row in tx
        .prepare("SELECT event_id, title FROM bible_event")?
        .query_map([], |row| {
            Ok((row.get::<_, u32>(0)?, row.get::<_, String>(1)?))
        })?
    {
        let (id, title) = row?;
        by_title.entry(title).or_default().push(id);
    }
    let mut faults = Vec::new();
    let find = |title: &str, at: &str, faults: &mut Vec<String>| -> Option<u32> {
        match by_title.get(title).map(Vec::as_slice) {
            Some([id]) => Some(*id),
            Some(_) => {
                faults.push(format!("{at}: several events are titled {title:?}"));
                None
            }
            None => {
                faults.push(format!("{at}: no event is titled {title:?}"));
                None
            }
        }
    };

    struct Built {
        key: String,
        title: String,
        note: String,
        expands: Option<u32>,
        members: Vec<u32>,
        edges: Vec<(u32, u32, String)>,
    }
    let mut built = Vec::new();
    let mut keys = HashSet::new();
    for sequence in &sequences {
        let at = format!("sequence {:?}", sequence.id);
        if sequence.id.is_empty() || !keys.insert(sequence.id.as_str()) {
            faults.push(format!("{at}: an empty or repeated id"));
        }
        let expands = sequence
            .expands
            .as_deref()
            .and_then(|t| find(t, &at, &mut faults));
        let mut members = Vec::new();
        let mut seen = HashSet::new();
        let mut edges = Vec::new();
        for chain in &sequence.chains {
            if chain.basis.trim().is_empty() || chain.events.is_empty() {
                faults.push(format!("{at}: a chain without a basis or events"));
            }
            let ids: Vec<Option<u32>> = chain
                .events
                .iter()
                .map(|t| find(t, &at, &mut faults))
                .collect();
            for id in ids.iter().flatten() {
                if seen.insert(*id) {
                    members.push(*id);
                }
            }
            for pair in ids.windows(2) {
                if let [Some(before), Some(after)] = *pair {
                    if before == after {
                        faults.push(format!("{at}: event {before} follows itself"));
                    }
                    edges.push((before, after, chain.basis.trim().to_string()));
                }
            }
        }
        if let Some(expands) = expands
            && seen.contains(&expands)
        {
            faults.push(format!("{at}: expands {expands}, one of its own events"));
        }
        if let Some(cycle) = find_cycle(&members, &edges) {
            faults.push(format!(
                "{at}: an order going round in a circle through events {cycle:?}"
            ));
        }
        built.push(Built {
            key: sequence.id.clone(),
            title: sequence.title.clone(),
            note: sequence.note.trim().to_string(),
            expands,
            members,
            edges,
        });
    }

    // An event is in one sequence at most, and a sequence is not, through
    // the events expanded, inside itself.
    let mut home: HashMap<u32, usize> = HashMap::new();
    for (i, sequence) in built.iter().enumerate() {
        for &id in &sequence.members {
            if let Some(other) = home.insert(id, i) {
                faults.push(format!(
                    "event {id} is in sequences {:?} and {:?}",
                    built[other].key, sequence.key
                ));
            }
        }
    }
    let mut expanded = HashSet::new();
    for (i, sequence) in built.iter().enumerate() {
        if let Some(event) = sequence.expands
            && !expanded.insert(event)
        {
            faults.push(format!("event {event} is expanded twice"));
        }
        let mut at = i;
        for _ in 0..=built.len() {
            match built[at].expands.and_then(|e| home.get(&e)) {
                Some(&parent) if parent == i => {
                    faults.push(format!("sequence {:?} is inside itself", sequence.key));
                    break;
                }
                Some(&parent) => at = parent,
                None => break,
            }
        }
    }
    if !faults.is_empty() {
        faults.sort();
        faults.dedup();
        bail!(
            "the event sequences have faults:\n  {}",
            faults.join("\n  ")
        );
    }

    tx.execute_batch(SCHEMA)?;
    let mut summary = SequencesSummary::default();
    let mut insert_sequence = tx.prepare(
        "INSERT INTO event_sequence(seq_id, key, title, note, expands) VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;
    let mut insert_member =
        tx.prepare("INSERT INTO event_sequence_member(seq_id, event_id) VALUES (?1, ?2)")?;
    let mut insert_edge = tx.prepare(
        "INSERT OR IGNORE INTO event_sequence_edge(seq_id, before_id, after_id, basis) \
         VALUES (?1, ?2, ?3, ?4)",
    )?;
    for (i, sequence) in built.iter().enumerate() {
        insert_sequence.execute(params![
            i as i64,
            sequence.key,
            crate::tsk::name_the_lord(&sequence.title),
            sequence.note,
            sequence.expands
        ])?;
        summary.sequences += 1;
        for id in &sequence.members {
            insert_member.execute(params![i as i64, id])?;
            summary.members += 1;
        }
        for (before, after, basis) in &sequence.edges {
            summary.edges += insert_edge.execute(params![i as i64, before, after, basis])?;
        }
    }
    info!(
        "Event sequences: {} with {} events and {} orderings",
        summary.sequences, summary.members, summary.edges
    );
    Ok(summary)
}

/// Events of `members` that `edges` order round in a circle, if any do.
fn find_cycle(members: &[u32], edges: &[(u32, u32, String)]) -> Option<Vec<u32>> {
    let mut next: HashMap<u32, Vec<u32>> = HashMap::new();
    for (before, after, _) in edges {
        next.entry(*before).or_default().push(*after);
    }
    // 0 unvisited, 1 on the path, 2 done.
    let mut state: HashMap<u32, u8> = HashMap::new();
    for &start in members {
        if state.get(&start).copied().unwrap_or(0) != 0 {
            continue;
        }
        let mut path = vec![start];
        let mut stack = vec![(start, 0usize)];
        state.insert(start, 1);
        while let Some((node, i)) = stack.pop() {
            match next.get(&node).and_then(|n| n.get(i)) {
                Some(&child) => {
                    stack.push((node, i + 1));
                    match state.get(&child).copied().unwrap_or(0) {
                        1 => {
                            let from = path.iter().position(|&n| n == child).unwrap_or(0);
                            let mut cycle = path[from..].to_vec();
                            cycle.push(child);
                            return Some(cycle);
                        }
                        0 => {
                            state.insert(child, 1);
                            path.push(child);
                            stack.push((child, 0));
                        }
                        _ => {}
                    }
                }
                None => {
                    state.insert(node, 2);
                    path.pop();
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn events(db: &Connection, titles: &[&str]) {
        db.execute_batch(
            "CREATE TABLE bible_event(event_id INTEGER PRIMARY KEY, title TEXT NOT NULL)",
        )
        .unwrap();
        for (i, title) in titles.iter().enumerate() {
            db.execute(
                "INSERT INTO bible_event(event_id, title) VALUES (?1, ?2)",
                params![i as i64 + 1, title],
            )
            .unwrap();
        }
    }

    /// `json` written to a file of its own, named for `test`.
    fn write(test: &str, json: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "haqor-sequences-{}-{test}.json",
            std::process::id()
        ));
        std::fs::write(&path, json).unwrap();
        path
    }

    #[test]
    fn chains_join_into_one_order() {
        let db = Connection::open_in_memory().unwrap();
        events(&db, &["A", "B", "C", "D", "Week", "E"]);
        let path = write(
            "chains",
            r#"{"sequences": [
                {"id": "outer", "title": "Outer", "chains": [
                    {"basis": "Mark", "events": ["A", "B", "Week"]},
                    {"basis": "John", "events": ["A", "C", "Week"]},
                    {"basis": "Luke", "events": ["A", "B"]},
                    {"basis": "Unordered", "events": ["D"]}
                ]},
                {"id": "inner", "title": "Inner", "expands": "Week", "chains": [
                    {"basis": "Mark", "events": ["E"]}
                ]}
            ]}"#,
        );
        let summary = build(&db, &path).unwrap();
        assert_eq!(
            summary,
            SequencesSummary {
                sequences: 2,
                members: 6,
                edges: 5
            }
        );
        let bases: Vec<String> = db
            .prepare(
                "SELECT basis FROM event_sequence_edge WHERE before_id = 1 AND after_id = 2 \
                 ORDER BY basis",
            )
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(bases, ["Luke", "Mark"]);
        let expands: Option<u32> = db
            .query_row(
                "SELECT expands FROM event_sequence WHERE key = 'inner'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(expands, Some(5));
    }

    #[test]
    fn faults_are_named() {
        let db = Connection::open_in_memory().unwrap();
        events(&db, &["A", "B", "C", "Twin", "Twin"]);
        let path = write(
            "faults",
            r#"{"sequences": [
                {"id": "s", "title": "S", "chains": [
                    {"basis": "Mark", "events": ["A", "B", "C"]},
                    {"basis": "Matthew", "events": ["C", "A"]},
                    {"basis": "Luke", "events": ["Twin", "Nothing"]}
                ]},
                {"id": "t", "title": "T", "expands": "B", "chains": [
                    {"basis": "", "events": ["B"]}
                ]}
            ]}"#,
        );
        let fault = build(&db, &path).unwrap_err().to_string();
        for expected in [
            "going round in a circle",
            "several events are titled \"Twin\"",
            "no event is titled \"Nothing\"",
            "a chain without a basis",
            "event 2 is in sequences \"s\" and \"t\"",
            "expands 2, one of its own events",
        ] {
            assert!(fault.contains(expected), "{expected:?} not in {fault}");
        }
    }

    #[test]
    fn a_circle_is_found() {
        let edge = |a, b| (a, b, String::new());
        assert_eq!(find_cycle(&[1, 2, 3], &[edge(1, 2), edge(2, 3)]), None);
        let cycle = find_cycle(
            &[1, 2, 3, 4],
            &[edge(1, 2), edge(2, 3), edge(3, 4), edge(4, 2)],
        );
        assert_eq!(cycle, Some(vec![2, 3, 4, 2]));
    }
}
