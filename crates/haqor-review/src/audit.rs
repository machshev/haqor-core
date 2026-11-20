//! Consistency audit of finished word reviews.
//!
//! - Repeats that disagree: occurrences of one surface with the same source
//!   tagging whose reviews give different readings. With `reopen`, the
//!   minority readings go back to their workers with the majority's reading.
//! - Departures from the OSHB: OT words whose reviewed part of speech, stem or
//!   verb form differs from the OSHB's reading at that position. Listed only:
//!   a reviewer may depart from the OSHB on the evidence of the text.
use anyhow::Result;
use haqor_core::bible::decode_oshb_morph;
use rusqlite::{Connection, params};
use serde_json::Value;
use std::collections::BTreeMap;

/// A review's lexical reading: dictionary form, root, morphology, article and
/// lexeme. The sense is left out: one word legitimately bears different senses
/// in different verses (אֶל to, into, against).
fn reading(a: &Value) -> String {
    let e = &a["entry"];
    format!(
        "{} {} root {} {} → {}#{}",
        a["language"].as_str().unwrap_or(""),
        a["lemma"].as_str().unwrap_or(""),
        a["root"].as_str().unwrap_or(""),
        a["morph"].as_str().unwrap_or("?"),
        e["existing"]
            .as_str()
            .or(e["draft"]["article_id"].as_str())
            .unwrap_or("?"),
        e["word"],
    )
}

fn place(reference: i64, pos: i64) -> String {
    format!(
        "{}:{}:{} [{pos}]",
        reference >> 16,
        (reference >> 8) & 255,
        reference & 255
    )
}

pub fn run(db: &mut Connection, reopen: bool) -> Result<()> {
    crate::jobs::backfill_carry_keys(db)?;
    let rows = db
        .prepare(
            "SELECT k.source,k.surface,k.tag,j.ref,j.pos,review_result(j.result) FROM carry_key k
             JOIN job j USING(source,ref,pos) WHERE j.state='done' AND j.result IS NOT NULL
             ORDER BY k.source,k.surface,k.tag,j.ref,j.pos",
        )?
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, String>(5)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    type Group = BTreeMap<String, Vec<(i64, i64)>>;
    let mut groups: BTreeMap<(String, String, String), Group> = BTreeMap::new();
    let mut departures = Vec::new();
    for (source, surface, tag, reference, pos, raw) in rows {
        let a: Value = serde_json::from_str(&raw)?;
        groups
            .entry((source.clone(), surface.clone(), tag.clone()))
            .or_default()
            .entry(reading(&a))
            .or_default()
            .push((reference, pos));
        // The OT tag is [OSHB lemma, OSHB morphology].
        let oshb: Vec<Value> = serde_json::from_str(&tag).unwrap_or_default();
        if let (true, Some(code), Some(parts), Some(theirs)) = (
            source == "ot",
            a["morph"].as_str(),
            a["segments"].as_str(),
            oshb.get(1).and_then(Value::as_str),
        ) {
            let ours = decode_oshb_morph(code, parts);
            let source_reading = decode_oshb_morph(theirs, parts);
            let fields = |w: &haqor_core::bible::HebrewWord| {
                (w.part_of_speech.clone(), w.form.clone(), w.tense.clone())
            };
            if code.split('/').count() == theirs.split('/').count()
                && fields(&ours) != fields(&source_reading)
            {
                departures.push(format!(
                    "{} {surface}: reviewed {code}, OSHB {theirs}",
                    place(reference, pos)
                ));
            }
        }
    }
    let senses: usize = groups.values().filter(|g| g.len() == 1).count();
    let mut conflicts = 0;
    let mut reopened = 0;
    let tx = db.transaction()?;
    for ((source, surface, _), readings) in &groups {
        if readings.len() < 2 {
            continue;
        }
        conflicts += 1;
        println!("{source} {surface}: reviews of one source tagging disagree lexically");
        let mut by_size: Vec<_> = readings.iter().collect();
        by_size.sort_by_key(|(_, at)| std::cmp::Reverse(at.len()));
        let (majority, majority_at) = by_size[0];
        for (reading, at) in &by_size {
            let places: Vec<String> = at.iter().map(|(r, p)| place(*r, *p)).collect();
            println!("  {} × {reading}: {}", at.len(), places.join(", "));
        }
        if !reopen || majority_at.len() == by_size[1].1.len() {
            continue;
        }
        let majority_places: Vec<String> = majority_at
            .iter()
            .take(5)
            .map(|(r, p)| place(*r, *p))
            .collect();
        for (_, at) in &by_size[1..] {
            for (reference, pos) in at.iter() {
                let reason = format!(
                    "Other reviews of this word with the same source tagging read it as {majority} ({}). Check which reading this verse requires, and give the same analysis unless the verse differs.",
                    majority_places.join(", ")
                );
                let n = tx.execute(
                    "UPDATE job SET state='ready',attempts=0,result=NULL,feedback=? WHERE source=? AND ref=? AND pos=? AND state='done'",
                    params![reason, source, reference, pos],
                )?;
                if n > 0 {
                    tx.execute(
                        "UPDATE job SET state='waiting',attempts=0,result=NULL WHERE source=? AND ref=? AND pos=-1",
                        params![source, reference],
                    )?;
                    tx.execute(
                        "DELETE FROM dependency WHERE source=? AND ref=? AND pos=?",
                        params![source, reference, pos],
                    )?;
                    reopened += 1;
                }
            }
        }
    }
    tx.commit()?;
    if !departures.is_empty() {
        println!("Reviewed readings that depart from the OSHB:");
        for line in &departures {
            println!("  {line}");
        }
    }
    println!(
        "{conflicts} words whose reviews disagree lexically{}; {} departures from the OSHB. ({senses} words read alike, whatever sense each verse gives them.)",
        if reopen {
            format!(", {reopened} minority readings reopened")
        } else {
            String::new()
        },
        departures.len()
    );
    Ok(())
}
