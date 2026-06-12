use haqor_core::generate::normalize_surface;
use quick_xml::Reader;
use quick_xml::events::Event;
use rusqlite::Connection;
use std::collections::HashMap;
fn main() -> anyhow::Result<()> {
    let mut gold: HashMap<String, Vec<String>> = HashMap::new();
    let mut paths: Vec<_> = std::fs::read_dir("src_texts/morphhb/wlc")?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "xml"))
        .collect();
    paths.sort();
    for path in paths {
        let mut r = Reader::from_file(&path)?;
        let mut buf = Vec::new();
        let (mut iw, mut mo, mut tx) = (false, String::new(), String::new());
        loop {
            match r.read_event_into(&mut buf)? {
                Event::Start(e) if e.name().as_ref() == b"w" => {
                    iw = true;
                    tx.clear();
                    mo.clear();
                    if let Some(a) = e.try_get_attribute("morph")? {
                        mo = a.decode_and_unescape_value(r.decoder())?.into_owned();
                    }
                }
                Event::Text(t) if iw => {
                    let f = t.unescape()?;
                    if f.as_ref() > "z" {
                        tx.push_str(f.as_ref());
                    }
                }
                Event::End(e) if e.name().as_ref() == b"w" => {
                    iw = false;
                    let ns = normalize_surface(&tx);
                    if !ns.is_empty() && !mo.is_empty() {
                        gold.entry(ns).or_default().push(mo.clone());
                    }
                }
                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }
    }
    let con = Connection::open("data/hebrew.db")?;
    let miss: Vec<(String, i64)> = con
        .prepare("SELECT text,occurrences FROM review_missing")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let mut cat: HashMap<&str, (i64, i64)> = HashMap::new();
    for (m, occ) in &miss {
        let key = match gold.get(m) {
            None => "not-in-gold",
            Some(rs) => {
                let mut v = false;
                let mut n = false;
                let mut o = false;
                for mo in rs {
                    let body = mo
                        .strip_prefix('H')
                        .or_else(|| mo.strip_prefix('A'))
                        .unwrap_or(mo);
                    for seg in body.split('/') {
                        let c: Vec<char> = seg.chars().collect();
                        let b: &[char] = if matches!(c.first(), Some('H' | 'A')) {
                            &c[1..]
                        } else {
                            &c
                        };
                        match b.first() {
                            Some('V') => v = true,
                            Some('N' | 'A') => n = true,
                            Some('R' | 'C' | 'T' | 'D' | 'S' | 'P') => {}
                            Some(_) => o = true,
                            None => {}
                        }
                    }
                }
                if mo_is_aramaic(rs) {
                    "aramaic"
                } else if v {
                    "verb"
                } else if n {
                    "noun/adj"
                } else if o {
                    "particle"
                } else {
                    "?"
                }
            }
        };
        let e = cat.entry(key).or_default();
        e.0 += 1;
        e.1 += occ;
    }
    let mut v: Vec<_> = cat.into_iter().collect();
    v.sort_by_key(|(_, (s, _))| -s);
    let tot: i64 = v.iter().map(|(_, (s, _))| s).sum();
    println!("remaining {tot} surfaces by category (surfaces, tokens):");
    for (k, (s, t)) in v {
        println!("  {k:<14} {s:>5} surf  {t:>5} tok");
    }
    Ok(())
}
fn mo_is_aramaic(rs: &[String]) -> bool {
    rs.iter().all(|m| m.starts_with('A'))
}
