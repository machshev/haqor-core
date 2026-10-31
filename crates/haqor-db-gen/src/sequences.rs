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

use crate::runtime_db::pack_ref;

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

DROP TABLE IF EXISTS event_sequence_grouping;
DROP TABLE IF EXISTS event_sequence_group;
DROP TABLE IF EXISTS event_sequence_group_member;
DROP TABLE IF EXISTS event_sequence_milestone;
DROP TABLE IF EXISTS event_sequence_event;
DROP TABLE IF EXISTS event_sequence_place;

-- A way of dividing a sequence's events into groups, in the file's order.
CREATE TABLE event_sequence_grouping(
    seq_id INTEGER NOT NULL,
    ord    INTEGER NOT NULL,
    key    TEXT    NOT NULL,
    title  TEXT    NOT NULL,
    note   TEXT    NOT NULL,
    PRIMARY KEY(seq_id, ord)
) WITHOUT ROWID;

-- Its groups, in the file's order with each before those inside it:
-- `parent` is the group (`ord`) one is inside, or NULL for one at the top.
CREATE TABLE event_sequence_group(
    seq_id   INTEGER NOT NULL,
    grouping INTEGER NOT NULL,
    ord      INTEGER NOT NULL,
    parent   INTEGER,
    title    TEXT    NOT NULL,
    note     TEXT    NOT NULL,
    PRIMARY KEY(seq_id, grouping, ord)
) WITHOUT ROWID;

-- The events of each group that holds events rather than groups.
CREATE TABLE event_sequence_group_member(
    seq_id   INTEGER NOT NULL,
    grouping INTEGER NOT NULL,
    ord      INTEGER NOT NULL,
    event_id INTEGER NOT NULL,
    PRIMARY KEY(seq_id, grouping, event_id)
) WITHOUT ROWID;

-- Feasts and other marks of time an event of a sequence falls at, with the
-- verse (packed) saying so.
CREATE TABLE event_sequence_milestone(
    seq_id   INTEGER NOT NULL,
    ord      INTEGER NOT NULL,
    event_id INTEGER NOT NULL,
    feast    TEXT    NOT NULL,
    label    TEXT    NOT NULL,
    ref      INTEGER NOT NULL,
    PRIMARY KEY(seq_id, ord)
) WITHOUT ROWID;

-- What kind of event a sequence's event is (its kinds split by `,`, the
-- first the chief), and the region it happens in; empty where not given.
CREATE TABLE event_sequence_event(
    event_id INTEGER PRIMARY KEY,
    kinds    TEXT    NOT NULL,
    region   TEXT    NOT NULL
);

-- The places it happens at, for the map, the first the chief; `label` is
-- what the place is called there where not its record's name, else empty.
CREATE TABLE event_sequence_place(
    event_id  INTEGER NOT NULL,
    ord       INTEGER NOT NULL,
    entity_id INTEGER NOT NULL,
    label     TEXT    NOT NULL,
    PRIMARY KEY(event_id, ord)
) WITHOUT ROWID;
";

/// The kinds an event of a sequence may be, as the file writes them.
pub const KINDS: &[&str] = &[
    "healing",
    "exorcism",
    "raising",
    "sign",
    "teaching",
    "parable",
    "journey",
    "calling",
    "encounter",
    "conflict",
    "revelation",
    "feast",
    "john-baptist",
    "preaching",
    "testimony",
    "baptism",
    "ecclesia",
    "birth",
    "childhood",
    "covenant",
    "letter",
];

/// The regions an event of a sequence may happen in.
pub const REGIONS: &[&str] = &[
    "north",
    "galilee",
    "east",
    "samaria",
    "perea",
    "judea",
    "egypt",
    "syria",
    "cyprus",
    "asia-minor",
    "macedonia",
    "achaia",
    "malta",
    "italy",
];

/// A sequence as the file gives it.
struct Sequence {
    id: String,
    title: String,
    note: String,
    /// The title of the event it expands.
    expands: Option<String>,
    chains: Vec<Chain>,
    groupings: Vec<Grouping>,
    milestones: Vec<Milestone>,
    /// What each event is and where, by title.
    details: Vec<(String, Details)>,
}

/// A division of a sequence's events into groups.
struct Grouping {
    id: String,
    title: String,
    note: String,
    /// In order, each before those inside it.
    groups: Vec<Group>,
}

struct Group {
    title: String,
    note: String,
    /// The index of the group it is inside.
    parent: Option<usize>,
    /// Its events' titles; empty for a group of groups.
    events: Vec<String>,
    /// Whether it holds groups rather than events.
    has_groups: bool,
}

/// A feast or other mark of time an event falls at.
struct Milestone {
    event: String,
    feast: String,
    label: String,
    reference: String,
}

#[derive(Default)]
struct Details {
    kinds: Vec<String>,
    region: String,
    /// TIPNR place keys, each with what it is called here (or empty).
    places: Vec<(String, String)>,
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
        known(
            sequence,
            &[
                "id",
                "title",
                "note",
                "expands",
                "chains",
                "groupings",
                "milestones",
                "events",
            ],
        )?;
        let titles = |value: &Value| -> Result<Vec<String>> {
            value
                .as_array()
                .into_iter()
                .flatten()
                .map(|e| e.as_str().map(str::to_string))
                .collect::<Option<_>>()
                .with_context(|| format!("an event that is not a title in {value}"))
        };
        let mut groupings = Vec::new();
        for grouping in sequence["groupings"].as_array().into_iter().flatten() {
            known(grouping, &["id", "title", "note", "groups"])?;
            fn walk(
                list: &Value,
                parent: Option<usize>,
                out: &mut Vec<Group>,
                known: &dyn Fn(&Value, &[&str]) -> Result<()>,
                titles: &dyn Fn(&Value) -> Result<Vec<String>>,
            ) -> Result<()> {
                for group in list.as_array().into_iter().flatten() {
                    known(group, &["title", "note", "events", "groups"])?;
                    let has_groups = group.get("groups").is_some();
                    out.push(Group {
                        title: group["title"].as_str().unwrap_or_default().to_string(),
                        note: group["note"].as_str().unwrap_or_default().to_string(),
                        parent,
                        events: titles(&group["events"])?,
                        has_groups,
                    });
                    if has_groups {
                        let at = out.len() - 1;
                        walk(&group["groups"], Some(at), out, known, titles)?;
                    }
                }
                Ok(())
            }
            let mut groups = Vec::new();
            walk(&grouping["groups"], None, &mut groups, &known, &titles)?;
            groupings.push(Grouping {
                id: text(grouping, "id"),
                title: text(grouping, "title"),
                note: text(grouping, "note"),
                groups,
            });
        }
        let mut milestones = Vec::new();
        for milestone in sequence["milestones"].as_array().into_iter().flatten() {
            known(milestone, &["event", "feast", "label", "ref"])?;
            milestones.push(Milestone {
                event: text(milestone, "event"),
                feast: text(milestone, "feast"),
                label: text(milestone, "label"),
                reference: text(milestone, "ref"),
            });
        }
        let mut details = Vec::new();
        for (title, about) in sequence["events"].as_object().into_iter().flatten() {
            known(about, &["kinds", "region", "places"])?;
            let mut places = Vec::new();
            for place in about["places"].as_array().into_iter().flatten() {
                match place {
                    Value::String(key) => places.push((key.clone(), String::new())),
                    Value::Object(_) => {
                        known(place, &["key", "label"])?;
                        places.push((text(place, "key"), text(place, "label")));
                    }
                    _ => bail!("an unreadable place {place} of {title:?}"),
                }
            }
            details.push((
                title.clone(),
                Details {
                    kinds: about["kinds"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .map(|k| k.as_str().map(str::to_string))
                        .collect::<Option<_>>()
                        .with_context(|| format!("an unreadable kind of {title:?}"))?,
                    region: text(about, "region"),
                    places,
                },
            ));
        }
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
            groupings,
            milestones,
            details,
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
    pub groupings: usize,
    pub groups: usize,
    pub milestones: usize,
    /// Events given kinds, a region or places.
    pub details: usize,
}

/// Write the sequences of `path` into the tables, their events found by
/// title among `bible_event`'s, which must be built, and their places in
/// `places` (TIPNR place keys to their ids and names), whose locations
/// `name_location` must hold. Fails, naming every fault, on a title naming
/// no event or several, an event in two sequences or expanded by one it is
/// in, an empty chain or basis, a sequence nested in itself, an order that
/// goes round in a circle, a grouping that leaves out an event, puts one in
/// two groups or puts a group before one that comes before it, a milestone
/// or detail of an event not in the sequence, an unknown kind or region, an
/// unreadable reference, or a place with no record or position.
pub(crate) fn build(
    tx: &Connection,
    path: &Path,
    places: &HashMap<String, (u32, String)>,
) -> Result<SequencesSummary> {
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
        /// Key, title, note, and groups: parent, title, note, events.
        groupings: Vec<(String, String, String, Vec<BuiltGroup>)>,
        /// Event, feast, label, packed reference.
        milestones: Vec<(u32, String, String, i64)>,
        details: Vec<BuiltDetails>,
    }
    /// Event, kinds, region, places (entity and label).
    type BuiltDetails = (u32, Vec<String>, String, Vec<(u32, String)>);
    struct BuiltGroup {
        parent: Option<usize>,
        title: String,
        note: String,
        events: Vec<u32>,
    }
    let mut located =
        tx.prepare("SELECT EXISTS(SELECT 1 FROM name_location WHERE entity_id = ?1)")?;
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
        // A member of the sequence, by title.
        let member = |title: &str, at: &str, faults: &mut Vec<String>| {
            let id = find(title, at, faults)?;
            if !seen.contains(&id) {
                faults.push(format!("{at}: {title:?} is not in the sequence"));
                return None;
            }
            Some(id)
        };

        let mut groupings = Vec::new();
        let mut grouping_keys = HashSet::new();
        for grouping in &sequence.groupings {
            let at = format!("{at}, grouping {:?}", grouping.id);
            if grouping.id.is_empty() || !grouping_keys.insert(grouping.id.as_str()) {
                faults.push(format!("{at}: an empty or repeated id"));
            }
            if grouping.title.trim().is_empty() || grouping.groups.is_empty() {
                faults.push(format!("{at}: no title or no groups"));
            }
            // Each event in one group, and the groups in an order no edge
            // goes back against: the leaves, in the file's order, are the
            // order of the whole.
            let mut leaf_of: HashMap<u32, usize> = HashMap::new();
            let mut groups = Vec::new();
            for (i, group) in grouping.groups.iter().enumerate() {
                let at = format!("{at}, group {:?}", group.title);
                if group.title.trim().is_empty() {
                    faults.push(format!("{at}: a group without a title"));
                }
                let empty = if group.has_groups {
                    !grouping.groups.iter().any(|g| g.parent == Some(i))
                } else {
                    group.events.is_empty()
                };
                if empty || (group.has_groups && !group.events.is_empty()) {
                    faults.push(format!("{at}: neither events nor groups alone"));
                }
                let mut events = Vec::new();
                for title in &group.events {
                    if let Some(id) = member(title, &at, &mut faults) {
                        if leaf_of.insert(id, i).is_some() {
                            faults.push(format!("{at}: {title:?} is in two groups"));
                        }
                        events.push(id);
                    }
                }
                groups.push(BuiltGroup {
                    parent: group.parent,
                    title: group.title.trim().to_string(),
                    note: group.note.trim().to_string(),
                    events,
                });
            }
            for id in &members {
                if !leaf_of.contains_key(id) {
                    faults.push(format!("{at}: event {id} is in no group"));
                }
            }
            for (before, after, basis) in &edges {
                if let (Some(b), Some(a)) = (leaf_of.get(before), leaf_of.get(after))
                    && b > a
                {
                    faults.push(format!(
                        "{at}: {basis} puts event {before} of group {:?} before event {after} \
                         of the earlier group {:?}",
                        grouping.groups[*b].title, grouping.groups[*a].title
                    ));
                }
            }
            groupings.push((
                grouping.id.clone(),
                grouping.title.trim().to_string(),
                grouping.note.trim().to_string(),
                groups,
            ));
        }

        let mut milestones = Vec::new();
        for milestone in &sequence.milestones {
            let at = format!("{at}, milestone {:?}", milestone.label);
            if milestone.label.trim().is_empty() {
                faults.push(format!("{at}: no label"));
            }
            let event = member(&milestone.event, &at, &mut faults);
            let reference = crate::journeys::parse_ref(&milestone.reference);
            if reference.is_none() {
                faults.push(format!(
                    "{at}: an unreadable reference {:?}",
                    milestone.reference
                ));
            }
            if let (Some(event), Some((book, chapter, verse))) = (event, reference) {
                milestones.push((
                    event,
                    milestone.feast.clone(),
                    milestone.label.trim().to_string(),
                    pack_ref(book.into(), chapter.into(), verse.into()),
                ));
            }
        }

        let mut details = Vec::new();
        for (title, about) in &sequence.details {
            let at = format!("{at}, event {title:?}");
            let Some(event) = member(title, &at, &mut faults) else {
                continue;
            };
            for kind in &about.kinds {
                if !KINDS.contains(&kind.as_str()) {
                    faults.push(format!("{at}: an unknown kind {kind:?}"));
                }
            }
            if !about.region.is_empty() && !REGIONS.contains(&about.region.as_str()) {
                faults.push(format!("{at}: an unknown region {:?}", about.region));
            }
            let mut at_places = Vec::new();
            for (key, label) in &about.places {
                let Some((entity, _)) = places.get(key) else {
                    faults.push(format!("{at}: {key} is no TIPNR place record"));
                    continue;
                };
                if !located.query_row([entity], |row| row.get::<_, bool>(0))? {
                    faults.push(format!("{at}: {key} has no position"));
                }
                at_places.push((*entity, label.trim().to_string()));
            }
            details.push((event, about.kinds.clone(), about.region.clone(), at_places));
        }

        built.push(Built {
            key: sequence.id.clone(),
            title: sequence.title.clone(),
            note: sequence.note.trim().to_string(),
            expands,
            members,
            edges,
            groupings,
            milestones,
            details,
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
        for (g, (key, title, note, groups)) in sequence.groupings.iter().enumerate() {
            tx.execute(
                "INSERT INTO event_sequence_grouping(seq_id, ord, key, title, note) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![i as i64, g as i64, key, title, note],
            )?;
            summary.groupings += 1;
            for (ord, group) in groups.iter().enumerate() {
                tx.execute(
                    "INSERT INTO event_sequence_group(seq_id, grouping, ord, parent, title, note) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        i as i64,
                        g as i64,
                        ord as i64,
                        group.parent.map(|p| p as i64),
                        group.title,
                        group.note
                    ],
                )?;
                summary.groups += 1;
                for event in &group.events {
                    tx.execute(
                        "INSERT INTO event_sequence_group_member(seq_id, grouping, ord, event_id) \
                         VALUES (?1, ?2, ?3, ?4)",
                        params![i as i64, g as i64, ord as i64, event],
                    )?;
                }
            }
        }
        for (ord, (event, feast, label, reference)) in sequence.milestones.iter().enumerate() {
            tx.execute(
                "INSERT INTO event_sequence_milestone(seq_id, ord, event_id, feast, label, ref) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![i as i64, ord as i64, event, feast, label, reference],
            )?;
            summary.milestones += 1;
        }
        for (event, kinds, region, at) in &sequence.details {
            tx.execute(
                "INSERT INTO event_sequence_event(event_id, kinds, region) VALUES (?1, ?2, ?3)",
                params![event, kinds.join(","), region],
            )?;
            summary.details += 1;
            for (ord, (entity, label)) in at.iter().enumerate() {
                tx.execute(
                    "INSERT INTO event_sequence_place(event_id, ord, entity_id, label) \
                     VALUES (?1, ?2, ?3, ?4)",
                    params![event, ord as i64, entity, label],
                )?;
            }
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
        // Place 100 has a position; 101 has none.
        db.execute_batch(
            "CREATE TABLE name_location(entity_id INTEGER NOT NULL);
             INSERT INTO name_location(entity_id) VALUES (100);",
        )
        .unwrap();
    }

    fn places() -> HashMap<String, (u32, String)> {
        HashMap::from([
            ("Kanah@Jos.19.28".into(), (100, "Kanah".into())),
            ("Ephraim@2Sa.13.23".into(), (101, "Ephraim".into())),
        ])
    }

    #[test]
    fn groupings_milestones_and_details_are_kept() {
        let db = Connection::open_in_memory().unwrap();
        events(&db, &["A", "B", "C", "D"]);
        let path = write(
            "groupings",
            r#"{"sequences": [
                {"id": "s", "title": "S", "chains": [
                    {"basis": "Mark", "events": ["A", "B", "C", "D"]}
                ],
                "groupings": [
                    {"id": "parts", "title": "Parts", "groups": [
                        {"title": "One", "events": ["A"]},
                        {"title": "Two", "groups": [
                            {"title": "Two a", "events": ["B", "C"]},
                            {"title": "Two b", "events": ["D"]}
                        ]}
                    ]}
                ],
                "milestones": [
                    {"event": "B", "feast": "pesach", "label": "Passover", "ref": "Jhn.2.13"}
                ],
                "events": {
                    "A": {"kinds": ["sign", "feast"], "region": "galilee",
                          "places": [{"key": "Kanah@Jos.19.28", "label": "Cana"}]},
                    "C": {"kinds": ["teaching"]}
                }}
            ]}"#,
        );
        let summary = build(&db, &path, &places()).unwrap();
        assert_eq!(
            (
                summary.groupings,
                summary.groups,
                summary.milestones,
                summary.details
            ),
            (1, 4, 1, 2)
        );
        let groups: Vec<(i64, Option<i64>, String)> = db
            .prepare("SELECT ord, parent, title FROM event_sequence_group ORDER BY ord")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(
            groups,
            [
                (0, None, "One".into()),
                (1, None, "Two".into()),
                (2, Some(1), "Two a".into()),
                (3, Some(1), "Two b".into()),
            ]
        );
        let in_two_a: i64 = db
            .query_row(
                "SELECT count(*) FROM event_sequence_group_member WHERE ord = 2",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(in_two_a, 2);
        let (kinds, region, label): (String, String, String) = db
            .query_row(
                "SELECT kinds, region, label FROM event_sequence_event JOIN \
                 event_sequence_place USING(event_id) WHERE event_id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(
            (kinds.as_str(), region.as_str(), label.as_str()),
            ("sign,feast", "galilee", "Cana")
        );
        let reference: i64 = db
            .query_row("SELECT ref FROM event_sequence_milestone", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(reference, pack_ref(43, 2, 13));
    }

    #[test]
    fn grouping_faults_are_named() {
        let db = Connection::open_in_memory().unwrap();
        events(&db, &["A", "B", "C", "Outside"]);
        let path = write(
            "grouping-faults",
            r#"{"sequences": [
                {"id": "s", "title": "S", "chains": [
                    {"basis": "Mark", "events": ["A", "B", "C"]}
                ],
                "groupings": [
                    {"id": "g", "title": "G", "groups": [
                        {"title": "Late", "events": ["C", "B"]},
                        {"title": "Early", "events": ["A", "B"]}
                    ]}
                ],
                "milestones": [
                    {"event": "Outside", "label": "Passover", "ref": "John.2.13"}
                ],
                "events": {
                    "A": {"kinds": ["wonder"], "region": "rome",
                          "places": ["Ephraim@2Sa.13.23", "Atlantis@Gen.1.1"]}
                }}
            ]}"#,
        );
        let fault = build(&db, &path, &places()).unwrap_err().to_string();
        for expected in [
            "\"B\" is in two groups",
            "Mark puts event 2 of group \"Early\" before event 3",
            "\"Outside\" is not in the sequence",
            "an unreadable reference \"John.2.13\"",
            "an unknown kind \"wonder\"",
            "an unknown region \"rome\"",
            "Ephraim@2Sa.13.23 has no position",
            "Atlantis@Gen.1.1 is no TIPNR place record",
        ] {
            assert!(fault.contains(expected), "{expected:?} not in {fault}");
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
        let summary = build(&db, &path, &HashMap::new()).unwrap();
        assert_eq!(
            summary,
            SequencesSummary {
                sequences: 2,
                members: 6,
                edges: 5,
                ..Default::default()
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
        let fault = build(&db, &path, &HashMap::new()).unwrap_err().to_string();
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
