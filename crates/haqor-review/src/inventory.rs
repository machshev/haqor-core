//! Track inventory fingerprints and load current reader inputs by chapter.
use anyhow::{Context, Result, ensure};
use haqor_core::bible::Bible;
use rusqlite::{Connection, OpenFlags, params};
use serde_json::{Value, json};
use std::{collections::HashMap, path::Path};

use crate::Source;

const OT: &str = "SELECT w.ref,w.position,s.text,json_object(
 'root',i.root,'gloss',g.text,'pos',m.part_of_speech,'form',m.form,'tense',m.tense,
 'person',m.person,'gender',m.gender,'number',m.number,'state',m.state,
 'prefix',m.prefix,'suffix',m.obj_suffix,'oshb_lemma',o.lemma,'oshb_morph',o.morph)
 FROM word w JOIN surface s USING(surface_id)
 LEFT JOIN word_info i ON i.info_id=coalesce(w.info_id,s.info_id)
 LEFT JOIN morph_cell m USING(cell_id) LEFT JOIN gloss g ON g.gloss_id=coalesce(w.gloss_id,i.gloss_id)
 LEFT JOIN heb.oshb_primary o ON o.book=w.ref>>16 AND o.chapter=((w.ref>>8)&255)
 AND o.verse=(w.ref&255) AND o.position=w.position
 WHERE w.ref BETWEEN ?1 AND ?2 ORDER BY w.ref,w.position";
const PES: &str = "SELECT n.ref,n.ord,coalesce(nullif(w.vocalised,''),w.word),json_object(
 'lemma',l.lexeme,'root',r.root,'category',l.category,'gender',w.gender,'person',w.person,
 'number',w.number,'state',w.state,'tense',w.tense,'form',w.form,'suffix_person',w.suffix_person,
 'suffix_gender',w.suffix_gender,'suffix_number',w.suffix_number,
 'glosses',(SELECT json_group_array(meaning) FROM syriac_gloss WHERE lexeme_id=l.lexeme_id))
 FROM nt_word n LEFT JOIN syriac_word w USING(word_id) LEFT JOIN syriac_lexeme l USING(lexeme_id)
 LEFT JOIN syriac_root r USING(root_id) WHERE n.ref BETWEEN ?1 AND ?2 ORDER BY n.ref,n.ord";
const GRK: &str = "SELECT w.ref,w.position,w.word,json_object('lemma',l.lemma,'gloss',l.gloss,
 'grammar',g.code,'english',e.text) FROM greek_word w LEFT JOIN greek_lemma l USING(lemma_id)
 LEFT JOIN greek_grammar g USING(grammar_id) LEFT JOIN greek_english e USING(english_id)
 WHERE w.ref BETWEEN ?1 AND ?2 ORDER BY w.ref,w.position";

pub fn seed(
    state: &mut Connection,
    dir: &Path,
    sources: &[Source],
    low: i64,
    high: i64,
) -> Result<()> {
    let tx = state.transaction()?;
    let total = visit(dir, sources, low, high, |source, r, words, english| {
        save(&tx, source, r, words, english)
    })?;
    ensure!(total > 0, "no source verses matched this selection");
    tx.commit()?;
    println!("Inventoried {total} source verses.");
    Ok(())
}

fn visit(
    dir: &Path,
    sources: &[Source],
    low: i64,
    high: i64,
    mut emit: impl FnMut(Source, i64, &[Value], Option<&Value>) -> Result<()>,
) -> Result<usize> {
    let corpus =
        Connection::open_with_flags(dir.join("haqor.db"), OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let bible = Bible::open(dir)?;

    let mut total = 0;
    for &source in sources {
        if (matches!(source, Source::Ot) && low >> 16 >= 40)
            || (!matches!(source, Source::Ot) && high >> 16 < 40)
        {
            continue;
        }

        let sql = match source {
            Source::Ot => {
                let path = dir
                    .join("hebrew.db")
                    .canonicalize()
                    .context("OT inventory needs hebrew.db with OSHB occurrence tags")?;
                corpus.execute("ATTACH ? AS heb", [path.to_str().context("invalid path")?])?;
                OT
            }
            Source::Peshitta => PES,
            Source::Greek => GRK,
        };
        let mut query = corpus.prepare(sql)?;
        let mut rows = query.query(params![low, high])?;
        let mut words = Vec::new();
        let mut previous = None;
        let mut chapter = None;
        let mut english = HashMap::new();
        while let Some(row) = rows.next()? {
            let r: i64 = row.get(0)?;
            if previous != Some(r) {
                if let Some(last) = previous {
                    emit(source, last, &words, english.get(&(last as u8)))?;
                    words.clear();
                }
                if chapter != Some(r >> 8) {
                    let (book, ch) = ((r >> 16) as u8, (r >> 8) as u8);
                    let spans = match source {
                        Source::Greek => bible.greek_chapter_translation(book, ch)?,
                        _ => bible.chapter_translation(book, ch)?,
                    };
                    english = spans.into_iter().map(|(v, spans)| (v, Value::Array(spans.into_iter()
                        .map(|s| json!({"text":s.text,"supplied":s.supplied,"links":s.words.into_iter()
                            .map(|w| [u16::from(w.chapter),u16::from(w.verse),w.position]).collect::<Vec<_>>()})).collect()))).collect();
                    chapter = Some(r >> 8);
                }
                previous = Some(r);
                total += 1;
            }
            words.push(
                json!({"position":row.get::<_,i64>(1)?,"surface":row.get::<_,String>(2)?,
                "source_analysis":serde_json::from_str::<Value>(&row.get::<_,String>(3)?)?}),
            );
        }
        if let Some(last) = previous {
            emit(source, last, &words, english.get(&(last as u8)))?;
        }
    }
    Ok(total)
}

// The full hash detects explicit reseeds with changed context. The surface hash
// protects positional identity when rebuilding the packet for an existing job.
fn fingerprint(packet: &Value) -> Result<String> {
    let words = packet["words"]
        .as_array()
        .context("inventory missing words")?;
    let positions: Vec<_> = words
        .iter()
        .map(|w| (&w["position"], &w["surface"]))
        .collect();
    Ok(format!(
        "@{}:{}",
        crate::articles::revision(&packet.to_string()),
        crate::articles::revision(&serde_json::to_string(&positions)?)
    ))
}

pub fn register(db: &Connection, dir: &Path) -> Result<()> {
    use rusqlite::functions::FunctionFlags;
    let dir = dir.to_owned();
    let cached = std::sync::Mutex::new(std::collections::VecDeque::<(
        String,
        i64,
        HashMap<i64, (String, String)>,
    )>::new());
    let direct = FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DIRECTONLY;
    db.create_scalar_function("inventory_fingerprint", 1, direct, |ctx| {
        let raw: String = ctx.get(0)?;
        let f = || -> Result<String> { fingerprint(&serde_json::from_str(&raw)?) };
        f().map_err(|e| rusqlite::Error::UserFunctionError(e.into()))
    })?;
    db.create_scalar_function("review_input", 3, direct, move |ctx| {
        let raw: String = ctx.get(2)?;
        if !raw.starts_with('@') {
            return Ok(raw);
        }
        let edition: String = ctx.get(0)?;
        let reference: i64 = ctx.get(1)?;
        let read = || -> Result<String> {
            let mut cache = cached
                .lock()
                .map_err(|_| anyhow::anyhow!("inventory cache unavailable"))?;
            let ch = reference >> 8;
            if !cache.iter().any(|(s, c, _)| s == &edition && *c == ch) {
                let source = match edition.as_str() {
                    "ot" => Source::Ot,
                    "peshitta" => Source::Peshitta,
                    "greek" => Source::Greek,
                    _ => anyhow::bail!("unknown edition"),
                };
                let mut rows = HashMap::new();
                visit(
                    &dir,
                    &[source],
                    ch << 8,
                    (ch << 8) + 255,
                    |_, r, words, english| {
                        let packet = json!({"words":words,"english":english});
                        rows.insert(r, (fingerprint(&packet)?, packet.to_string()));
                        Ok(())
                    },
                )?;
                if cache.len() == 4 {
                    cache.pop_front();
                }
                cache.push_back((edition.clone(), ch, rows));
            }
            let (signature, packet) = cache
                .iter()
                .find(|(s, c, _)| s == &edition && *c == ch)
                .and_then(|(_, _, rows)| rows.get(&reference))
                .context("seeded verse missing from current source database")?;
            ensure!(
                signature.split_once(':').map(|s| s.1) == raw.split_once(':').map(|s| s.1),
                "source word positions changed; reseed {edition} {}:{}:{}",
                reference >> 16,
                ch & 255,
                reference & 255
            );
            Ok(packet.clone())
        };
        read().map_err(|e| rusqlite::Error::UserFunctionError(e.into()))
    })?;
    Ok(())
}

pub fn compact(db: &Connection) -> Result<()> {
    db.execute(
        "UPDATE verse SET packet=inventory_fingerprint(packet) WHERE substr(packet,1,1)<>'@'",
        [],
    )?;
    Ok(())
}

fn save(
    db: &Connection,
    source: Source,
    r: i64,
    words: &[Value],
    english: Option<&Value>,
) -> Result<()> {
    let source = source.name();
    let packet = fingerprint(&json!({"words":words,"english":english}))?;
    // Re-seeding an identical packet preserves its completed work. A source
    // edit can change the context even for unchanged positions in this verse.
    let changed = db
        .prepare_cached(
            "INSERT INTO verse VALUES(?,?,?) ON CONFLICT(source,ref)
        DO UPDATE SET packet=excluded.packet WHERE verse.packet<>excluded.packet",
        )?
        .execute(params![source, r, packet])?;
    if changed == 0 {
        return Ok(());
    }
    db.prepare_cached("DELETE FROM job WHERE source=? AND ref=?")?
        .execute(params![source, r])?;
    let mut insert = db.prepare_cached("INSERT INTO job(source,ref,pos,state) VALUES(?,?,?,?)")?;
    for word in words {
        insert.execute(params![
            source,
            r,
            word["position"].as_i64().context("missing position")?,
            "ready"
        ])?;
    }
    insert.execute(params![source, r, -1, "waiting"])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reseeding_preserves_review_until_source_context_changes() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(crate::jobs::SCHEMA).unwrap();
        let words = [json!({"position":0,"surface":"אַף"})];
        save(&db, Source::Ot, 66305, &words, None).unwrap();
        db.execute("UPDATE job SET state='done'", []).unwrap();
        save(&db, Source::Ot, 66305, &words, None).unwrap();
        assert_eq!(
            db.query_row("SELECT count(*) FROM job WHERE state='done'", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            2
        );
        save(&db, Source::Ot, 66305, &words, Some(&json!(["also"]))).unwrap();
        assert_eq!(
            db.query_row("SELECT state FROM job WHERE pos=-1", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "waiting"
        );
        assert_eq!(
            db.query_row("SELECT state FROM job WHERE pos=0", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "ready"
        );
    }
}
