//! Throwaway: harvest noun-headed review_missing surfaces into GOLD_NOUNS rows.
//! A noun's mishqal can't be derived from a root, so the noun side is harvested
//! by design (gold_noun.rs); this completes that harvest for the current
//! backlog. Emits (surface, lemma, gloss) tuples — full normalized surface, so
//! the noun parser's strip-0 pass matches exactly.
use std::collections::{BTreeMap, HashMap};
use haqor_core::generate::normalize_surface;
use quick_xml::Reader; use quick_xml::events::Event;
use rusqlite::Connection;

fn main() -> anyhow::Result<()> {
    let lex = Connection::open("data/lexicon.db")?;
    let mut lemma: HashMap<i64, (String, String)> = HashMap::new();
    {
        let mut s = lex.prepare("SELECT strong, word, COALESCE(gloss,'') FROM english WHERE word IS NOT NULL AND strong IS NOT NULL")?;
        for r in s.query_map([], |r| Ok((r.get::<_,i64>(0)?, r.get::<_,String>(1)?, r.get::<_,String>(2)?)))? {
            let (k,w,g)=r?; lemma.insert(k,(w,g));
        }
    }
    // gold: norm full surface -> Vec of (morph, lemma-attr)
    let mut gold: HashMap<String, Vec<(String,String)>> = HashMap::new();
    let mut paths: Vec<_>=std::fs::read_dir("src_texts/morphhb/wlc")?.filter_map(|e|e.ok().map(|e|e.path())).filter(|p|p.extension().is_some_and(|x|x=="xml")).collect();
    paths.sort();
    for path in paths { let mut r=Reader::from_file(&path)?; let mut buf=Vec::new();
        let (mut iw,mut mo,mut lem,mut tx)=(false,String::new(),String::new(),String::new());
        loop { match r.read_event_into(&mut buf)? {
            Event::Start(e) if e.name().as_ref()==b"w" => { iw=true; tx.clear(); mo.clear(); lem.clear();
                if let Some(a)=e.try_get_attribute("morph")? { mo=a.decode_and_unescape_value(r.decoder())?.into_owned(); }
                if let Some(a)=e.try_get_attribute("lemma")? { lem=a.decode_and_unescape_value(r.decoder())?.into_owned(); } }
            Event::Text(t) if iw => { let f=t.unescape()?; if f.as_ref()>"z" { tx.push_str(f.as_ref()); } }
            Event::End(e) if e.name().as_ref()==b"w" => { iw=false; let ns=normalize_surface(&tx);
                if !ns.is_empty()&&!mo.is_empty() { gold.entry(ns).or_default().push((mo.clone(),lem.clone())); } }
            Event::Eof=>break,_=>{} } buf.clear(); } }

    let con=Connection::open("data/hebrew.db")?;
    let miss: Vec<String>=con.prepare("SELECT text FROM review_missing")?.query_map([], |r| r.get(0))?.collect::<Result<_,_>>()?;

    // head Strong's of the noun/adjective morpheme of a reading, if it's noun-headed
    // (and not verb-headed). Returns Some(strong) for the noun head.
    // Returns (strong, is_proper) for the noun/adjective head, or None.
    fn noun_head(morph: &str, lem: &str) -> Option<(i64, bool)> {
        let Some(body) = morph.strip_prefix('H') else { return None }; // Hebrew only
        let msegs: Vec<&str> = body.split('/').collect();
        let lsegs: Vec<&str> = lem.split('/').collect();
        if msegs.iter().any(|s| { let c:Vec<char>=s.chars().collect(); c.first()==Some(&'V') }) { return None; }
        for (i, s) in msegs.iter().enumerate() {
            let c: Vec<char> = s.chars().collect();
            if matches!(c.first(), Some('N'|'A')) {
                let proper = c.first() == Some(&'N') && c.get(1) == Some(&'p'); // Np = proper noun
                let lseg = lsegs.get(i).or_else(|| lsegs.last())?;
                let strong = lseg.chars().skip_while(|c| !c.is_ascii_digit()).take_while(|c| c.is_ascii_digit()).collect::<String>().parse::<i64>().ok()?;
                return Some((strong, proper));
            }
        }
        None
    }

    let mut common: BTreeMap<String,(String,String)> = BTreeMap::new();
    let mut proper: std::collections::BTreeSet<String> = Default::default();
    for m in &miss {
        let Some(readings)=gold.get(m) else { continue };
        for (mo,lem) in readings {
            if let Some((strong, is_proper))=noun_head(mo,lem) {
                if is_proper { proper.insert(m.clone()); break; }
                if let Some((w,g))=lemma.get(&strong) {
                    let gloss: String = g.split(';').next().unwrap_or("").trim().chars().take(40).collect();
                    common.insert(m.clone(), (w.clone(), gloss)); break;
                }
            }
        }
    }
    eprintln!("// {} common-noun/adj surfaces, {} proper-noun surfaces", common.len(), proper.len());
    let mut cf = std::fs::File::create("/tmp/gold_nouns_new.txt")?;
    use std::io::Write;
    for (surf,(lem,gloss)) in &common {
        writeln!(cf, "    (\"{surf}\", \"{lem}\", \"{}\"),", gloss.replace('"', "'"))?;
    }
    let mut pf = std::fs::File::create("/tmp/proper_new.txt")?;
    for surf in &proper { writeln!(pf, "    \"{surf}\",")?; }
    Ok(())
}
