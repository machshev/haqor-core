//! Import MACULA Hebrew's syntax trees.
//!
//! MACULA Hebrew (Clear Bible / Biblica, CC BY 4.0) parses every verse of the
//! Hebrew Bible into a tree: clauses, the phrases within them (nominal,
//! prepositional, …) and the function each plays in its clause (subject,
//! verb, object, …). `src_texts/MACULA-Hebrew` holds its "lowfat" XML, one
//! file per chapter, fetched by `scripts/fetch-macula-hebrew.sh`.
//!
//! The trees' leaves are morphemes: a prefixed conjunction, preposition or
//! article, or a pronominal suffix, is a leaf of its own and may sit in a
//! different phrase from the word it is written on. Each leaf names its word
//! by the Westminster Leningrad Codex's numbering, which counts the written
//! form of a ketiv/qere pair (MACULA leaves the ketiv out of the tree) and
//! now and then divides a word differently from Haqor's text (הַ֥לְלוּ יָ֨הּ
//! for הַלְלוּיָהּ). So leaves are placed on Haqor's words by consonants:
//! both texts spell the verse's qere with the same letters, and a word's
//! first consonant decides which of Haqor's words it falls in.
//!
//! The result is the `syntax_tree` table, one row per verse, the tree written
//! in the compact form [`haqor_core::syntax`] reads.

use std::collections::HashMap;
use std::io::BufRead;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use haqor_core::data_support::fold_consonants;
use log::{debug, info};
use quick_xml::events::{BytesStart, Event};
use rusqlite::{Connection, params};

use crate::runtime_db::pack_ref;

pub const SCHEMA: &str = "
DROP TABLE IF EXISTS syntax_tree;
CREATE TABLE syntax_tree(ref INTEGER PRIMARY KEY, tree TEXT NOT NULL);
";

/// MACULA's (USFM) book codes, in its own English book order.
const BOOKS: [&str; 39] = [
    "GEN", "EXO", "LEV", "NUM", "DEU", "JOS", "JDG", "RUT", "1SA", "2SA", "1KI", "2KI", "1CH",
    "2CH", "EZR", "NEH", "EST", "JOB", "PSA", "PRO", "ECC", "SNG", "ISA", "JER", "LAM", "EZK",
    "DAN", "HOS", "JOL", "AMO", "OBA", "JON", "MIC", "NAM", "HAB", "ZEP", "HAG", "ZEC", "MAL",
];

/// `src_texts/MACULA-Hebrew`.
pub fn source_dir(src_texts: &Path) -> PathBuf {
    src_texts.join("MACULA-Hebrew")
}

/// A (book, chapter, verse) in Haqor's book numbering.
type Verse = (u8, u8, u8);

/// A verse as MACULA names it (`GEN 1:1`), in Haqor's book numbering.
fn parse_verse(id: &str) -> Option<Verse> {
    let (book, rest) = id.split_once(' ')?;
    let (chapter, verse) = rest.split_once(':')?;
    let index = BOOKS.iter().position(|&b| b == book)?;
    Some((
        crate::tsk::book_of_key(index + 1)?,
        chapter.parse().ok()?,
        verse.parse().ok()?,
    ))
}

/// A node of a tree as read, before its leaves are placed on Haqor's words.
#[derive(Debug, Clone, PartialEq)]
enum Node {
    /// A clause or phrase. `class` is empty for MACULA's unlabelled groups,
    /// which gather a conjunction with what it joins.
    Group {
        class: String,
        role: String,
        children: Vec<Node>,
    },
    /// A morpheme of the word MACULA numbers `word` in the verse, `part`
    /// its place within the word (the last digit of its `xml:id`; 0 when it
    /// has none). The tree keeps constituents together rather than in
    /// reading order, so it is these two that give the order.
    Morpheme {
        word: u16,
        part: u8,
        role: String,
        text: String,
        gloss: String,
    },
}

fn attribute(e: &BytesStart<'_>, key: &str) -> Result<String> {
    Ok(e.try_get_attribute(key)?
        .map(|a| a.normalized_value(crate::xml::VERSION))
        .transpose()?
        .map(|s| s.into_owned())
        .unwrap_or_default())
}

/// The morpheme a `<w>` element stands for. Its word number is the `!n` of
/// its `ref` (`GEN 1:1!3`). Its text is the element's content, which
/// [`read_trees`] fills in; `unicode` stands in for an empty element, but is
/// not preferred: it gives each part of a two-word name the whole name
/// (`תּ֣וּבַל קַ֔יִן` on both תּ֣וּבַל and קַ֔יִן).
fn morpheme(e: &BytesStart<'_>) -> Result<Node> {
    let reference = attribute(e, "ref")?;
    let word = reference
        .rsplit_once('!')
        .and_then(|(_, n)| n.parse().ok())
        .with_context(|| format!("unreadable word reference {reference:?}"))?;
    let mut gloss = attribute(e, "english")?;
    if gloss.is_empty() {
        // `gloss` writes multi-word glosses with dots: `he.created`.
        gloss = attribute(e, "gloss")?.replace('.', " ");
    }
    let part = attribute(e, "xml:id")?
        .chars()
        .last()
        .and_then(|c| c.to_digit(10))
        .unwrap_or_default() as u8;
    Ok(Node::Morpheme {
        word,
        part,
        role: attribute(e, "role")?,
        text: attribute(e, "unicode")?,
        gloss,
    })
}

/// Every verse's tree in one chapter file, in order. A verse's tree is its
/// one top-level group, or an unlabelled group gathering several.
fn read_chapter(path: &Path) -> Result<Vec<(Verse, Node)>> {
    let mut reader = crate::xml::Reader::open(path)?;
    read_trees(&mut reader).with_context(|| format!("reading {}", path.display()))
}

fn read_trees<R: BufRead>(reader: &mut crate::xml::Reader<R>) -> Result<Vec<(Verse, Node)>> {
    let mut trees = Vec::new();
    let mut verse = None;
    // The open groups, innermost last; the bottom one gathers the verse.
    let mut stack: Vec<Node> = Vec::new();
    // Inside a `<w>`, whose text content is not needed: `unicode` has it.
    let mut in_word: Option<Node> = None;
    loop {
        match reader.next()? {
            Event::Eof => break,
            Event::Start(e) | Event::Empty(e) if e.name().as_ref() == "sentence" => {
                let id = attribute(&e, "id")?;
                verse = Some(parse_verse(&id).with_context(|| format!("unknown verse {id:?}"))?);
                stack = vec![Node::Group {
                    class: String::new(),
                    role: String::new(),
                    children: Vec::new(),
                }];
            }
            Event::End(e) if e.name().as_ref() == "sentence" => {
                let Some(Node::Group { mut children, .. }) = stack.pop() else {
                    bail!("unbalanced groups before </sentence>");
                };
                if !stack.is_empty() {
                    bail!("unclosed groups at </sentence>");
                }
                let tree = match children.len() {
                    1 => children.pop().unwrap(),
                    _ => Node::Group {
                        class: String::new(),
                        role: String::new(),
                        children,
                    },
                };
                trees.push((verse.take().context("</sentence> without a verse")?, tree));
            }
            Event::Start(e) if e.name().as_ref() == "wg" => stack.push(Node::Group {
                class: attribute(&e, "class")?,
                role: attribute(&e, "role")?,
                children: Vec::new(),
            }),
            Event::End(e) if e.name().as_ref() == "wg" => {
                let group = stack.pop().context("</wg> without <wg>")?;
                push_child(&mut stack, group)?;
            }
            Event::Start(e) if e.name().as_ref() == "w" => in_word = Some(morpheme(&e)?),
            Event::Text(t) => {
                if let Some(Node::Morpheme { text, .. }) = in_word.as_mut() {
                    let content = t.xml10_content();
                    if !content.trim().is_empty() {
                        *text = content.trim().to_string();
                    }
                }
            }
            Event::Empty(e) if e.name().as_ref() == "w" => push_child(&mut stack, morpheme(&e)?)?,
            Event::End(e) if e.name().as_ref() == "w" => {
                let word = in_word.take().context("</w> without <w>")?;
                push_child(&mut stack, word)?;
            }
            _ => {}
        }
    }
    Ok(trees)
}

fn push_child(stack: &mut [Node], child: Node) -> Result<()> {
    match stack.last_mut() {
        Some(Node::Group { children, .. }) => {
            children.push(child);
            Ok(())
        }
        _ => bail!("a tree node outside any <sentence>"),
    }
}

/// A word's letters, the key words are placed by: its consonants, finals
/// folded, without the dot that tells sin from shin, which the two texts mark
/// differently in a few words and which never moves a word boundary.
fn letters(word: &str) -> String {
    fold_consonants(word).replace('\u{5c2}', "")
}

/// MACULA's word numbers in reading order, each with its consonants.
fn macula_words(tree: &Node) -> Vec<(u16, String)> {
    fn collect<'a>(tree: &'a Node, out: &mut Vec<(u16, u8, &'a str)>) {
        match tree {
            Node::Group { children, .. } => children.iter().for_each(|c| collect(c, out)),
            Node::Morpheme {
                word, part, text, ..
            } => out.push((*word, *part, text)),
        }
    }
    let mut morphemes = Vec::new();
    collect(tree, &mut morphemes);
    // Stable, so morphemes without a part number keep the tree's order.
    morphemes.sort_by_key(|&(word, part, _)| (word, part));
    let mut words: Vec<(u16, String)> = Vec::new();
    for (word, _, text) in morphemes {
        match words.last_mut() {
            Some((last, consonants)) if *last == word => consonants.push_str(&letters(text)),
            _ => words.push((word, letters(text))),
        }
    }
    words
}

/// How a verse's MACULA words fell on Haqor's.
#[derive(Debug, Default, PartialEq, Eq)]
struct Placement {
    /// MACULA's word number → Haqor's word position.
    positions: HashMap<u16, u16>,
    /// Whether the two texts spell the verse with the same consonants. When
    /// they do not, the words are still placed, by consonant count, but may
    /// drift.
    aligned: bool,
}

/// Place MACULA's words on Haqor's (`haqor`: each position's consonants, in
/// order): each goes to the word its first consonant falls in.
fn place(macula: &[(u16, String)], haqor: &[String]) -> Placement {
    let mut owner = Vec::new();
    for (position, consonants) in haqor.iter().enumerate() {
        owner.extend(std::iter::repeat_n(
            position as u16,
            consonants.chars().count(),
        ));
    }
    let macula_text: String = macula.iter().map(|(_, c)| c.as_str()).collect();
    let haqor_text: String = haqor.concat();
    let mut positions = HashMap::new();
    let mut offset = 0;
    for (word, consonants) in macula {
        let position = owner
            .get(offset)
            .or(owner.last())
            .copied()
            .unwrap_or_default();
        positions.insert(*word, position);
        offset += consonants.chars().count();
    }
    Placement {
        positions,
        aligned: macula_text == haqor_text,
    }
}

/// Write a leaf's text or gloss into its `{text|gloss}`, with a backslash
/// before each character that would end it early (`|`, `}`) and before a
/// backslash. MACULA's glosses bracket the words English supplies ("[be]
/// everyone"), so brackets do occur.
fn push_escaped(out: &mut String, value: &str) {
    for c in value.chars() {
        if matches!(c, '|' | '}' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
}

/// Write `tree` in the compact form, its leaves placed by `positions`.
///
/// A group is `[class:role children…]` (`:role` only when it has one, and the
/// class empty for an unlabelled group); a leaf is its word position,
/// `position:role` when it has a role. Neighbouring morphemes of one word with
/// one role are one leaf; a leaf that is not the whole of its word also gives
/// its own text and gloss, `position{text|gloss}`, since the reader cannot
/// tell from the word which part of it the leaf is ([`push_escaped`] says how
/// the two are escaped). Children are separated by spaces.
fn write_tree(
    tree: &Node,
    positions: &HashMap<u16, u16>,
    morphemes: &HashMap<u16, usize>,
    out: &mut String,
) -> Result<()> {
    let Node::Group {
        class,
        role,
        children,
    } = tree
    else {
        // A bare morpheme as a whole verse's tree does not occur; wrap it.
        let group = Node::Group {
            class: String::new(),
            role: String::new(),
            children: vec![tree.clone()],
        };
        return write_tree(&group, positions, morphemes, out);
    };
    out.push('[');
    out.push_str(class);
    if !role.is_empty() {
        out.push(':');
        out.push_str(role);
    }
    let mut index = 0;
    while index < children.len() {
        out.push(' ');
        match &children[index] {
            group @ Node::Group { .. } => {
                write_tree(group, positions, morphemes, out)?;
                index += 1;
            }
            Node::Morpheme { word, role, .. } => {
                let position = positions[word];
                // The run of morphemes on the same word with the same role.
                let run: Vec<&Node> = children[index..]
                    .iter()
                    .take_while(|c| {
                        matches!(c, Node::Morpheme { word: w, role: r, .. }
                        if positions[w] == position && r == role)
                    })
                    .collect();
                index += run.len();
                out.push_str(&position.to_string());
                if !role.is_empty() {
                    out.push(':');
                    out.push_str(role);
                }
                if run.len() < morphemes[&position] {
                    let mut text = String::new();
                    let mut glosses = Vec::new();
                    for node in &run {
                        if let Node::Morpheme {
                            text: t, gloss: g, ..
                        } = node
                        {
                            text.push_str(t);
                            if !g.is_empty() {
                                glosses.push(g.as_str());
                            }
                        }
                    }
                    out.push('{');
                    push_escaped(out, text.trim());
                    out.push('|');
                    push_escaped(out, &glosses.join(" "));
                    out.push('}');
                }
            }
        }
    }
    out.push(']');
    Ok(())
}

/// How many morphemes fall on each of Haqor's word positions.
fn count_morphemes(tree: &Node, positions: &HashMap<u16, u16>, out: &mut HashMap<u16, usize>) {
    match tree {
        Node::Group { children, .. } => children
            .iter()
            .for_each(|c| count_morphemes(c, positions, out)),
        Node::Morpheme { word, .. } => *out.entry(positions[word]).or_default() += 1,
    }
}

/// What an import found, for the log and the tests.
#[derive(Debug, Default)]
pub struct SyntaxSummary {
    pub verses: usize,
    /// Verses whose consonants differ between MACULA and Haqor's text, whose
    /// leaves may sit on a neighbouring word.
    pub misaligned: usize,
    /// Verses of MACULA's that Haqor's text lacks.
    pub orphaned: usize,
    /// Bytes of tree written, all verses together.
    pub bytes: usize,
}

/// Rebuild the `syntax_tree` table of a runtime database in place from
/// `src_texts/MACULA-Hebrew`.
pub fn build_syntax_trees(db: &Connection, src_texts: &Path) -> Result<SyntaxSummary> {
    let dir = source_dir(src_texts).join("lowfat");
    let mut files = std::fs::read_dir(&dir)
        .with_context(|| {
            format!(
                "reading {} (run scripts/fetch-macula-hebrew.sh)",
                dir.display()
            )
        })?
        .map(|entry| Ok(entry?.path()))
        .collect::<Result<Vec<_>>>()?;
    files.retain(|p| p.extension().is_some_and(|e| e == "xml"));
    files.sort();
    if files.is_empty() {
        bail!(
            "no MACULA chapter files in {} (run scripts/fetch-macula-hebrew.sh)",
            dir.display()
        );
    }

    let tx = db.unchecked_transaction()?;
    tx.execute_batch(SCHEMA)?;
    let mut summary = SyntaxSummary::default();
    {
        let mut words = tx.prepare(
            "SELECT s.text FROM word w JOIN surface s USING(surface_id) \
             WHERE w.ref = ?1 ORDER BY w.position",
        )?;
        let mut insert = tx.prepare("INSERT INTO syntax_tree(ref, tree) VALUES (?1, ?2)")?;
        for file in &files {
            for ((book, chapter, verse), tree) in read_chapter(file)? {
                let reference = pack_ref(book.into(), chapter.into(), verse.into());
                let haqor = words
                    .query_map([reference], |row| row.get::<_, String>(0))?
                    .map(|text| text.map(|t| letters(&t)))
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                if haqor.is_empty() {
                    debug!("MACULA {book} {chapter}:{verse}: no such verse in the corpus");
                    summary.orphaned += 1;
                    continue;
                }
                let macula = macula_words(&tree);
                let placement = place(&macula, &haqor);
                if !placement.aligned {
                    debug!(
                        "MACULA {book} {chapter}:{verse}: letters differ from the corpus: \
                         {macula:?} / {haqor:?}"
                    );
                    summary.misaligned += 1;
                }
                let mut morphemes = HashMap::new();
                count_morphemes(&tree, &placement.positions, &mut morphemes);
                let mut out = String::new();
                write_tree(&tree, &placement.positions, &morphemes, &mut out)?;
                summary.bytes += out.len();
                summary.verses += 1;
                insert.execute(params![reference, out])?;
            }
        }
    }
    tx.commit()?;
    info!(
        "Syntax trees: {} verses ({} KiB; {} with consonants differing from the corpus, \
         {} outside it)",
        summary.verses,
        summary.bytes / 1024,
        summary.misaligned,
        summary.orphaned
    );
    Ok(summary)
}

/// `db gen-syntax`: rebuild the `syntax_tree` table of the runtime database at
/// `path` in place and re-stamp it, so syncing it to the app reinstalls it.
pub fn gen_syntax(path: &Path, src_texts: &Path) -> Result<SyntaxSummary> {
    let db = Connection::open(path).with_context(|| format!("opening {}", path.display()))?;
    let summary = build_syntax_trees(&db, src_texts)?;
    crate::runtime_db::restamp_built(&db)?;
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trees(xml: &str) -> Vec<(Verse, Node)> {
        let mut reader = crate::xml::Reader::from_str(xml);
        read_trees(&mut reader).unwrap()
    }

    /// Genesis 1:1, abridged from MACULA: a prepositional phrase whose
    /// preposition is a prefix of its noun, the verb and subject as bare
    /// words, and an object phrase.
    const GENESIS_1_1: &str = r#"<chapter><sentence id="GEN 1:1">
        <p><milestone unit="verse" id="GEN 1:1">GEN 1:1</milestone> text</p>
        <wg class="cl" rule="PP-V-S-O">
          <wg role="pp" class="pp" rule="PrepNp">
            <w ref="GEN 1:1!1" english="in" unicode="בְּ">בְּ</w>
            <w ref="GEN 1:1!1" english="beginning" unicode="רֵאשִׁ֖ית">רֵאשִׁ֖ית</w>
          </wg>
          <w role="v" ref="GEN 1:1!2" gloss="he.created" unicode="בָּרָ֣א">בָּרָ֣א</w>
          <w role="s" ref="GEN 1:1!3" english="God" unicode="אֱלֹהִ֑ים">אֱלֹהִ֑ים</w>
          <wg role="o" class="np">
            <w ref="GEN 1:1!4" unicode="אֵ֥ת">אֵ֥ת</w>
            <wg class="np">
              <w ref="GEN 1:1!5" english="the" unicode="הַ">הַ</w>
              <w ref="GEN 1:1!5" english="heavens" unicode="שָּׁמַ֖יִם">שָּׁמַ֖יִם</w>
            </wg>
          </wg>
        </wg></sentence></chapter>"#;

    #[test]
    fn reads_a_tree() {
        let trees = trees(GENESIS_1_1);
        assert_eq!(trees.len(), 1);
        let (verse, tree) = &trees[0];
        assert_eq!(*verse, (1, 1, 1));
        let Node::Group {
            class, children, ..
        } = tree
        else {
            panic!("{tree:?}");
        };
        assert_eq!(class, "cl");
        assert_eq!(children.len(), 4);
        assert_eq!(
            children[1],
            Node::Morpheme {
                word: 2,
                part: 0,
                role: "v".into(),
                text: "בָּרָ֣א".into(),
                gloss: "he created".into(),
            }
        );
    }

    #[test]
    fn maps_books_to_tanakh_order() {
        assert_eq!(parse_verse("GEN 1:1"), Some((1, 1, 1)));
        assert_eq!(parse_verse("RUT 2:3"), Some((31, 2, 3)));
        assert_eq!(parse_verse("PSA 151:1"), Some((27, 151, 1)));
        assert_eq!(parse_verse("MAL 3:23"), Some((26, 3, 23)));
        assert_eq!(parse_verse("MAT 1:1"), None);
    }

    fn compact(tree: &Node, haqor: &[&str]) -> String {
        let macula = macula_words(tree);
        let haqor: Vec<String> = haqor.iter().map(|w| letters(w)).collect();
        let placement = place(&macula, &haqor);
        assert!(placement.aligned);
        let mut morphemes = HashMap::new();
        count_morphemes(tree, &placement.positions, &mut morphemes);
        let mut out = String::new();
        write_tree(tree, &placement.positions, &morphemes, &mut out).unwrap();
        out
    }

    #[test]
    fn writes_whole_words_as_positions() {
        let (_, tree) = &trees(GENESIS_1_1)[0];
        assert_eq!(
            compact(tree, &["בְּרֵאשִׁית", "בָּרָא", "אֱלֹהִים", "אֵת", "הַשָּׁמַיִם"]),
            "[cl [pp:pp 0] 1:v 2:s [np:o 3 [np 4]]]"
        );
    }

    /// A conjunction written on the verb it joins to the clause before sits
    /// apart from it in the tree: each part keeps its own text and gloss.
    #[test]
    fn writes_split_words_with_their_parts() {
        let xml = r#"<chapter><sentence id="GEN 1:3">
            <wg>
              <w ref="GEN 1:3!1" english="and" unicode="וַ">וַ</w>
              <wg class="cl">
                <w role="v" ref="GEN 1:3!1" english="said" unicode="יֹּ֥אמֶר">יֹּ֥אמֶר</w>
                <w role="s" ref="GEN 1:3!2" english="God" unicode="אֱלֹהִ֖ים">אֱלֹהִ֖ים</w>
              </wg>
            </wg></sentence></chapter>"#;
        let (_, tree) = &trees(xml)[0];
        assert_eq!(
            compact(tree, &["וַיֹּאמֶר", "אֱלֹהִים"]),
            "[ 0{וַ|and} [cl 0:v{יֹּ֥אמֶר|said} 1:s]]"
        );
    }

    /// Words divided differently by the two texts still land: הַ֥לְלוּ יָ֨הּ
    /// is one word, הַלְלוּיָהּ, in Haqor's, and a ketiv MACULA leaves out
    /// leaves a gap in its numbering that consonants step over.
    #[test]
    fn places_words_by_consonants() {
        let consonants = |words: &[(u16, &str)]| -> Vec<(u16, String)> {
            words.iter().map(|(n, w)| (*n, letters(w))).collect()
        };
        let haqor = |words: &[&str]| -> Vec<String> { words.iter().map(|w| letters(w)).collect() };
        let joined = place(
            &consonants(&[(1, "הַ֥לְלוּ"), (2, "יָ֨הּ"), (3, "שִׁ֣ירוּ")]),
            &haqor(&["הַלְלוּיָהּ", "שִׁירוּ"]),
        );
        assert!(joined.aligned);
        assert_eq!(joined.positions[&1], 0);
        assert_eq!(joined.positions[&2], 0);
        assert_eq!(joined.positions[&3], 1);

        let ketiv = place(
            &consonants(&[(10, "הוֹדַ֙עְתָּ֙"), (12, "אֶת"), (13, "עַבְדְּךָ")]),
            &haqor(&["הוֹדַעְתָּ", "אֶת", "עַבְדְּךָ"]),
        );
        assert!(ketiv.aligned);
        assert_eq!(ketiv.positions[&12], 1);
        assert_eq!(ketiv.positions[&13], 2);
    }

    /// The whole source against the built corpus: every verse has a tree,
    /// nearly all spelled exactly as the corpus spells them.
    #[test]
    fn complete_source_imports() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let src_texts = root.join("src_texts");
        let database = root.join("data/haqor.db");
        if !source_dir(&src_texts).exists() || !database.exists() {
            eprintln!("skipping: fetched MACULA source or data/haqor.db unavailable");
            return;
        }
        // Into a copy: the test must not rewrite the shared database.
        let db = Connection::open_in_memory().unwrap();
        db.execute("ATTACH DATABASE ?1 AS src", [database.to_string_lossy()])
            .unwrap();
        db.execute_batch(
            "CREATE TABLE word(ref INTEGER, position INTEGER, surface_id INTEGER,
                               PRIMARY KEY(ref, position)) WITHOUT ROWID;
             INSERT INTO word SELECT ref, position, surface_id FROM src.word
               WHERE ref < (40 << 16);
             CREATE TABLE surface(surface_id INTEGER PRIMARY KEY, text TEXT);
             INSERT INTO surface SELECT surface_id, text FROM src.surface;
             DETACH DATABASE src;",
        )
        .unwrap();
        let summary = build_syntax_trees(&db, &src_texts).unwrap();
        eprintln!("{summary:?}");
        assert_eq!(summary.verses, 23_213);
        assert_eq!(summary.orphaned, 0);
        assert!(summary.misaligned < 10, "{}", summary.misaligned);

        let tree: String = db
            .query_row(
                "SELECT tree FROM syntax_tree WHERE ref = ?1",
                [pack_ref(1, 1, 1)],
                |row| row.get(0),
            )
            .unwrap();
        assert!(tree.starts_with("[cl [pp:pp 0] 1:v 2:s [np:o"), "{tree}");
    }
}
