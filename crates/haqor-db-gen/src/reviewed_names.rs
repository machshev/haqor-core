use anyhow::{Context, Result, ensure};
use haqor_core::reviewed_names::ReviewedNames;
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

pub fn extend(records: &mut Vec<Value>, review: &ReviewedNames) -> Result<()> {
    let mut ids: HashSet<u64> = records.iter().filter_map(|r| r["id"].as_u64()).collect();
    let keys: HashSet<String> = records
        .iter()
        .filter_map(|r| r["key"].as_str().map(String::from))
        .collect();
    for (key, record) in &review.records {
        ensure!(
            !keys.contains(key) && ids.insert(record.id.into()),
            "reviewed identity {key} collides with an existing record"
        );
        let b = &record.bearer;
        records.push(json!({"id":record.id,"key":key,"name":b.name,"kind":b.kind,"category":b.kind,"summary":b.description,"description":b.description,
            "forms":[{"hebrew":if haqor_core::data_support::fold_consonants(&record.form).is_empty(){""}else{&record.form},"english":[b.name],"significance":"Named"}]}));
    }
    Ok(())
}

/// A reviewed verse replaces the source edition's assignments, including
/// corrections to non-names. Verify its whole token sequence before applying.
pub fn apply(
    db: &Connection,
    review: &ReviewedNames,
    source: &str,
    ids: &HashMap<String, u32>,
    scoped: &HashSet<u32>,
) -> Result<()> {
    if source == "greek" && crate::translation::has_table(db, "reviewed_greek_baseline")? {
        // gen-names also works in place, without rebuilding greek_word. Undo
        // the prior overlay so a removed identification returns to TAGNT.
        db.execute_batch(
            "UPDATE greek_word SET entity_id=b.entity_id FROM reviewed_greek_baseline b
            WHERE greek_word.ref=b.ref AND greek_word.position=b.position;
            DROP TABLE reviewed_greek_baseline;",
        )?;
    }
    let Some(verses) = review.verses.get(source) else {
        return Ok(());
    };
    let sql = match source {
        "ot" => {
            "SELECT position,s.text FROM word JOIN surface s USING(surface_id) WHERE ref=? ORDER BY position"
        }
        "greek" => "SELECT position,word FROM greek_word WHERE ref=? ORDER BY position",
        "peshitta" => {
            "SELECT ord,coalesce(nullif(vocalised,''),word) FROM nt_word JOIN syriac_word USING(word_id) WHERE ref=? ORDER BY ord"
        }
        _ => anyhow::bail!("unknown edition {source}"),
    };
    db.execute_batch("CREATE TEMP TABLE IF NOT EXISTS reviewed_name_scope(id INTEGER PRIMARY KEY); DELETE FROM reviewed_name_scope;")?;
    {
        let mut add = db.prepare("INSERT INTO reviewed_name_scope VALUES(?)")?;
        for id in scoped {
            add.execute([id])?;
        }
    }
    if source == "greek" {
        db.execute_batch("CREATE TABLE reviewed_greek_baseline(ref INTEGER,position INTEGER,entity_id INTEGER,PRIMARY KEY(ref,position)) WITHOUT ROWID;")?;
    }
    let mut tokens = db.prepare(sql)?;
    for (&reference, words) in verses {
        let actual = tokens
            .query_map([reference], |r| {
                Ok((r.get::<_, u32>(0)?, r.get::<_, String>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        ensure!(
            !words.is_empty()
                && actual
                    .iter()
                    .map(|(p, s)| (*p, s))
                    .eq(words.iter().map(|w| (w.position, &w.surface))),
            "stale {source} name positions at {reference}"
        );
        if source == "greek" {
            db.execute("INSERT INTO reviewed_greek_baseline SELECT ref,position,entity_id FROM greek_word WHERE ref=?",[reference])?;
        } else {
            db.execute(
                "DELETE FROM word_name WHERE ref=? AND entity_id IN reviewed_name_scope",
                [reference],
            )?;
        }
        if source == "peshitta" {
            db.execute(
                "DELETE FROM name_verse WHERE ref=? AND entity_id IN reviewed_name_scope",
                [reference],
            )?;
        }
        for word in words {
            let id = word
                .key
                .as_ref()
                .map(|k| ids.get(k).with_context(|| format!("unknown bearer {k}")))
                .transpose()?;
            if source == "greek" {
                db.execute(
                    "UPDATE greek_word SET entity_id=?1 WHERE ref=?2 AND position=?3 AND (?1 IS NOT NULL OR entity_id IN reviewed_name_scope)",
                    params![id, reference, word.position],
                )?;
            } else if let Some(id) = id {
                db.execute(
                    "INSERT INTO word_name VALUES(?,?,?) ON CONFLICT(ref,position) DO UPDATE SET entity_id=excluded.entity_id",
                    params![reference, word.position, id],
                )?;
                if source == "peshitta" {
                    db.execute("INSERT INTO name_verse VALUES(?,?,1) ON CONFLICT(entity_id,ref) DO UPDATE SET words=words+1",params![id,reference])?;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use haqor_core::reviewed_names::{Bearer, Mention};
    #[test]
    fn namesakes_and_places_keep_distinct_occurrences_in_each_edition() -> Result<()> {
        let db = Connection::open_in_memory()?;
        db.execute_batch("CREATE TABLE greek_word(ref,position,word,entity_id);
            CREATE TABLE nt_word(ref,ord,word_id);
            CREATE TABLE syriac_word(word_id,vocalised,word);
            CREATE TABLE word_name(ref,position,entity_id,PRIMARY KEY(ref,position));
            CREATE TABLE name_verse(entity_id,ref,words,PRIMARY KEY(entity_id,ref));
            INSERT INTO greek_word VALUES (1,0,'A',1),(1,1,'A',1),(1,2,'feast',9);
            INSERT INTO nt_word VALUES(1,0,1),(1,1,2); INSERT INTO syriac_word VALUES(1,'B','B'),(2,'C','C');
            INSERT INTO name_verse VALUES(1,1,2),(9,1,1);")?;
        let mut review = ReviewedNames::default();
        let mut next = 9;
        let key = "A@Act.1.1";
        let person = Bearer {
            name: "A".into(),
            kind: "person".into(),
            description: "Second bearer".into(),
        };
        review.add_record(key, &person, "A", &mut next)?;
        review.add_record(key, &person, "A", &mut next)?;
        assert_eq!(next, 10);
        let mut records = vec![
            json!({"id":1,"key":"A@Gen.1.1"}),
            json!({"id":2,"key":"B@Gen.1.1"}),
        ];
        extend(&mut records, &review)?;
        let ids = records
            .iter()
            .map(|r| {
                (
                    r["key"].as_str().unwrap().into(),
                    r["id"].as_u64().unwrap() as u32,
                )
            })
            .collect();
        let mention = |position, surface: &str, key: Option<&str>| Mention {
            position,
            surface: surface.into(),
            key: key.map(String::from),
            evidence: None,
        };
        review.verses.entry("greek".into()).or_default().insert(
            1,
            vec![
                mention(0, "A", Some("A@Gen.1.1")),
                mention(1, "A", Some(key)),
                mention(2, "feast", None),
            ],
        );
        review.verses.entry("peshitta".into()).or_default().insert(
            1,
            vec![mention(0, "B", Some("B@Gen.1.1")), mention(1, "C", None)],
        );
        for source in ["greek", "peshitta"] {
            apply(&db, &review, source, &ids, &HashSet::from([1, 2, 10]))?;
        }
        assert_eq!(
            db.query_row(
                "SELECT group_concat(entity_id,',') FROM greek_word",
                [],
                |r| r.get::<_, String>(0)
            )?,
            "1,10,9"
        );
        assert_eq!(
            db.query_row(
                "SELECT entity_id FROM word_name WHERE ref=1 AND position=0",
                [],
                |r| r.get::<_, u32>(0)
            )?,
            2
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM name_verse", [], |r| r
                .get::<_, u32>(0))?,
            2
        );
        review.verses.remove("greek");
        apply(&db, &review, "greek", &ids, &HashSet::new())?;
        assert_eq!(
            db.query_row(
                "SELECT entity_id FROM greek_word WHERE position=1",
                [],
                |r| r.get::<_, u32>(0)
            )?,
            1
        );
        review
            .verses
            .get_mut("peshitta")
            .unwrap()
            .get_mut(&1)
            .unwrap()[0]
            .surface = "changed".into();
        assert!(apply(&db, &review, "peshitta", &ids, &HashSet::new()).is_err());
        Ok(())
    }
}
