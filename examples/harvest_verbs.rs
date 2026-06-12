//! Throwaway: harvest the curated irregular-verb table (unmodeled stems) from
//! the OSHB gold, for the surfaces still in review_missing. Emits Rust tuples
//! ready to paste into src/morphology/irregular_verb.rs.
use haqor_core::generate::normalize_surface;
use quick_xml::Reader;
use quick_xml::events::Event;
use rusqlite::Connection;
use std::collections::{BTreeMap, HashMap, HashSet};

// OSHM Hebrew verb stem letter -> binyan label (unmodeled stems only here).
fn stem_name(c: char) -> Option<&'static str> {
    Some(match c {
        'o' => "Polel",
        'O' => "Polal",
        'r' => "Hithpolel",
        'm' => "Poel",
        'M' => "Poal",
        'k' => "Palel",
        'K' => "Pulal",
        'Q' => "Qal passive",
        'l' => "Pilpel",
        'L' => "Polpal",
        'f' => "Hithpalpel",
        'D' => "Nithpael",
        'j' => "Pealal",
        'i' => "Pilel",
        'u' => "Hothpaal",
        'c' => "Tiphil",
        'v' => "Hishtaphel",
        'w' => "Nithpalel",
        'y' => "Nithpoel",
        'z' => "Hithpoel",
        _ => return None, // q N p P h H t are modeled -> excluded
    })
}

// Full stem map (incl. modeled stems) — used for allow-listed irregular lemmas
// whose surface the generator structurally cannot produce (sibilant metathesis,
// suppletion) even though gold labels them with a modeled binyan.
fn stem_name_full(c: char) -> Option<&'static str> {
    Some(match c {
        'q' => "Qal",
        'N' => "Niphal",
        'p' => "Piel",
        'P' => "Pual",
        'h' => "Hiphil",
        'H' => "Hophal",
        't' => "Hithpael",
        _ => return stem_name(c),
    })
}

// Strong's numbers of genuinely-irregular verbs to harvest wholesale regardless
// of (modeled) stem — forms the triliteral generator structurally cannot make:
//   7812 שׁחה הִשְׁתַּחֲוָה  (Hithpael w/ sibilant metathesis + hollow + III-he)
//   1961 היה,  2421 חיה     (suppletive: aleph-preformative אֶהְיֶה/וָאֱהִי, apocope)
const IRREGULAR_STRONGS: &[i64] = &[7812, 1961, 2421];

fn conj_name(c: char) -> Option<&'static str> {
    Some(match c {
        'p' | 'q' => "Perfect",
        'i' => "Imperfect",
        'w' => "Wayyiqtol",
        'h' => "Cohortative",
        'j' => "Jussive",
        'v' => "Imperative",
        'r' => "Participle (act.)",
        's' => "Participle (pas.)",
        'a' => "Inf. Absolute",
        'c' => "Inf. Construct",
        _ => return None,
    })
}

fn main() -> anyhow::Result<()> {
    // Strong's -> (lemma word, root) from the lexicon.
    let lex = Connection::open("data/lexicon.db")?;
    let mut lemma: HashMap<i64, String> = HashMap::new();
    {
        let mut s = lex.prepare(
            "SELECT strong, word FROM english WHERE word IS NOT NULL AND strong IS NOT NULL",
        )?;
        for r in s.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))? {
            let (k, v) = r?;
            lemma.insert(k, v);
        }
    }
    let mut root_of: HashMap<i64, String> = HashMap::new();
    {
        let mut s = lex.prepare(
            "SELECT li.strong, b.root FROM lexical_index li JOIN bdb b ON b.bdb_id = li.bdb_id \
             WHERE b.root IS NOT NULL AND li.strong IS NOT NULL",
        )?;
        for r in s.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))? {
            let (k, v) = r?;
            root_of.entry(k).or_insert(v);
        }
    }

    // gold: norm surface -> set of (morph, lemma-strong)
    let mut gold: HashMap<String, HashSet<(String, i64)>> = HashMap::new();
    let mut paths: Vec<_> = std::fs::read_dir("src_texts/morphhb/wlc")?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "xml"))
        .collect();
    paths.sort();
    for path in paths {
        let mut reader = Reader::from_file(&path)?;
        let mut buf = Vec::new();
        let (mut in_word, mut morph, mut lem, mut text) =
            (false, String::new(), String::new(), String::new());
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(e) if e.name().as_ref() == b"w" => {
                    in_word = true;
                    text.clear();
                    morph.clear();
                    lem.clear();
                    if let Some(a) = e.try_get_attribute("morph")? {
                        morph = a.decode_and_unescape_value(reader.decoder())?.into_owned();
                    }
                    if let Some(a) = e.try_get_attribute("lemma")? {
                        lem = a.decode_and_unescape_value(reader.decoder())?.into_owned();
                    }
                }
                Event::Text(t) if in_word => {
                    let f = t.unescape()?;
                    if f.as_ref() > "z" {
                        text.push_str(f.as_ref());
                    }
                }
                Event::End(e) if e.name().as_ref() == b"w" => {
                    in_word = false;
                    let ns = normalize_surface(&text);
                    // strong = leading integer of the last '/'-segment of the lemma attr
                    let strong = lem
                        .rsplit('/')
                        .next()
                        .unwrap_or("")
                        .chars()
                        .take_while(|c| c.is_ascii_digit())
                        .collect::<String>()
                        .parse::<i64>()
                        .unwrap_or(0);
                    if !ns.is_empty() && !morph.is_empty() {
                        gold.entry(ns).or_default().insert((morph.clone(), strong));
                    }
                }
                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }
    }

    let con = Connection::open("data/hebrew.db")?;
    let miss: Vec<String> = con
        .prepare("SELECT text FROM review_missing")?
        .query_map([], |r| r.get(0))?
        .collect::<Result<_, _>>()?;

    // Emit one tuple per (surface, root, binyan, form, pgn), de-duplicated.
    // BTreeMap keyed by surface for stable, readable output.
    let mut out: BTreeMap<String, Vec<(String, String, String, String)>> = BTreeMap::new();
    let mut n_surf = 0;
    for m in &miss {
        let Some(readings) = gold.get(m) else {
            continue;
        };
        let mut rows: Vec<(String, String, String, String)> = Vec::new();
        for (morph, strong) in readings {
            let Some(body) = morph.strip_prefix('H') else {
                continue;
            };
            for seg in body.split('/') {
                let mut ch = seg.chars();
                if ch.next() != Some('V') {
                    continue;
                }
                let Some(stem) = ch.next() else { continue };
                let Some(conj) = ch.next() else { continue };
                let allow = IRREGULAR_STRONGS.contains(strong);
                // Unmodeled stems always; modeled stems only for allow-listed lemmas.
                let Some(binyan) = (if allow {
                    stem_name_full(stem)
                } else {
                    stem_name(stem)
                }) else {
                    continue;
                };
                let Some(form) = conj_name(conj) else {
                    continue;
                };
                let rest: Vec<char> = ch.collect();
                let pgn: String = match conj {
                    'r' | 's' => rest.iter().take(2).collect(),
                    'a' | 'c' => String::new(),
                    _ => rest.iter().collect(),
                };
                let root = root_of
                    .get(strong)
                    .cloned()
                    .or_else(|| lemma.get(strong).cloned())
                    .unwrap_or_else(|| "?".to_string());
                let row = (root, binyan.to_string(), form.to_string(), pgn);
                if !rows.contains(&row) {
                    rows.push(row);
                }
            }
        }
        if !rows.is_empty() {
            n_surf += 1;
            out.insert(m.clone(), rows);
        }
    }

    let total_rows: usize = out.values().map(|v| v.len()).sum();
    eprintln!("// {n_surf} surfaces, {total_rows} rows");
    for (surf, rows) in &out {
        for (root, binyan, form, pgn) in rows {
            println!("    (\"{surf}\", \"{root}\", \"{binyan}\", \"{form}\", \"{pgn}\"),");
        }
    }
    Ok(())
}
