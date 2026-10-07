//! Verse syntax trees, as `haqor.db`'s `syntax_tree` table stores them.
//!
//! The trees are MACULA Hebrew's (Clear Bible / Biblica, CC BY 4.0): each
//! verse's clauses, the phrases inside them, and the function each plays in
//! its clause. `haqor-db-gen`'s `syntax` module imports them and places their
//! leaves on the verse's words; this module reads the compact form it writes.
//!
//! In that form a group is `[class:role children…]`, `:role` only when it has
//! one and the class empty for an unlabelled group; a leaf is its word
//! position, `position:role` with a role, and `position{text|gloss}` (role
//! before the brace) when it is only part of its word. Children are separated
//! by spaces; inside the braces a backslash escapes the next character.

/// One node of a verse's syntax tree: a clause or phrase with its children, or
/// a leaf standing for a word or part of one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxNode {
    /// What kind of group this is: `cl` (clause), `np` (nominal phrase),
    /// `pp` (prepositional), `vp` (verbal), `adjp`, `advp`, `nump`
    /// (numeral), `relp` (relative), `cjp` (conjoined), `ijp`
    /// (interjection). Empty for a leaf, and for MACULA's unlabelled groups,
    /// which gather a conjunction with what it joins.
    pub class: String,
    /// The function the node plays in its clause: `s` (subject), `v` (verb),
    /// `o` (object), `o2` (second object), `p` (predicate), `adv`
    /// (adverbial), `pp` (prepositional phrase as modifier). Empty when the
    /// node has none of its own, as most phrases inside phrases do not.
    pub role: String,
    /// The word a leaf stands for; `None` for a group.
    pub word: Option<SyntaxWord>,
    pub children: Vec<SyntaxNode>,
}

/// The word a leaf of a [`SyntaxNode`] stands for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxWord {
    /// The word's position in the verse, as the reader numbers them.
    pub position: u16,
    /// When the leaf is only part of its word (a prefixed conjunction,
    /// preposition or article, or a suffix, parsed apart from the word it is
    /// written on), that part's text and English gloss.
    pub part: Option<WordPart>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordPart {
    pub text: String,
    pub gloss: String,
}

impl SyntaxNode {
    /// The leaves under this node, in tree order.
    pub fn leaves(&self) -> Vec<&SyntaxWord> {
        fn collect<'a>(node: &'a SyntaxNode, out: &mut Vec<&'a SyntaxWord>) {
            if let Some(word) = &node.word {
                out.push(word);
            }
            node.children.iter().for_each(|c| collect(c, out));
        }
        let mut out = Vec::new();
        collect(self, &mut out);
        out
    }
}

/// Read a tree in the compact form. `None` when it does not parse, which a
/// tree the generator wrote always does.
pub fn parse(tree: &str) -> Option<SyntaxNode> {
    let mut chars = tree.chars().peekable();
    let node = parse_node(&mut chars)?;
    chars.next().is_none().then_some(node)
}

type Chars<'a> = std::iter::Peekable<std::str::Chars<'a>>;

fn parse_node(chars: &mut Chars<'_>) -> Option<SyntaxNode> {
    if chars.peek() == Some(&'[') {
        chars.next();
        let (class, role) = label(chars);
        let mut children = Vec::new();
        loop {
            match chars.next()? {
                ' ' => children.push(parse_node(chars)?),
                ']' => break,
                _ => return None,
            }
        }
        return Some(SyntaxNode {
            class,
            role,
            word: None,
            children,
        });
    }
    let (position, role) = label(chars);
    let part = if chars.peek() == Some(&'{') {
        chars.next();
        let text = escaped(chars, '|')?;
        let gloss = escaped(chars, '}')?;
        Some(WordPart { text, gloss })
    } else {
        None
    };
    Some(SyntaxNode {
        class: String::new(),
        role,
        word: Some(SyntaxWord {
            position: position.parse().ok()?,
            part,
        }),
        children: Vec::new(),
    })
}

/// `name` or `name:role`, up to the next space, bracket or brace.
fn label(chars: &mut Chars<'_>) -> (String, String) {
    let mut name = String::new();
    let mut role = None::<String>;
    while let Some(&c) = chars.peek() {
        if matches!(c, ' ' | '[' | ']' | '{') {
            break;
        }
        chars.next();
        match (&mut role, c) {
            (None, ':') => role = Some(String::new()),
            (None, c) => name.push(c),
            (Some(role), c) => role.push(c),
        }
    }
    (name, role.unwrap_or_default())
}

/// Text up to an unescaped `end`, which is consumed.
fn escaped(chars: &mut Chars<'_>, end: char) -> Option<String> {
    let mut out = String::new();
    loop {
        match chars.next()? {
            '\\' => out.push(chars.next()?),
            c if c == end => return Some(out),
            c => out.push(c),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(position: u16, role: &str) -> SyntaxNode {
        SyntaxNode {
            class: String::new(),
            role: role.into(),
            word: Some(SyntaxWord {
                position,
                part: None,
            }),
            children: Vec::new(),
        }
    }

    fn group(class: &str, role: &str, children: Vec<SyntaxNode>) -> SyntaxNode {
        SyntaxNode {
            class: class.into(),
            role: role.into(),
            word: None,
            children,
        }
    }

    #[test]
    fn parses_genesis_1_1() {
        assert_eq!(
            parse("[cl [pp:pp 0] 1:v 2:s [np:o 3 [np 4]]]").unwrap(),
            group(
                "cl",
                "",
                vec![
                    group("pp", "pp", vec![leaf(0, "")]),
                    leaf(1, "v"),
                    leaf(2, "s"),
                    group(
                        "np",
                        "o",
                        vec![leaf(3, ""), group("np", "", vec![leaf(4, "")])]
                    ),
                ]
            )
        );
    }

    #[test]
    fn parses_word_parts_and_escapes() {
        let tree = parse(r"[ 0{וַ|and} [cl 0:v{יֹּ֥אמֶר|said\|spoke} 1:s{כֹּ֔ל|[be\] everyone}]]").unwrap();
        assert_eq!(tree.class, "");
        let leaves = tree.leaves();
        assert_eq!(leaves.len(), 3);
        assert_eq!(
            leaves[0].part,
            Some(WordPart {
                text: "וַ".into(),
                gloss: "and".into()
            })
        );
        assert_eq!(leaves[1].part.as_ref().unwrap().gloss, "said|spoke");
        assert_eq!(leaves[2].part.as_ref().unwrap().gloss, "[be] everyone");
        assert_eq!(tree.children[1].children[0].role, "v");
    }

    #[test]
    fn rejects_malformed_trees() {
        assert_eq!(parse(""), None);
        assert_eq!(parse("[cl 0"), None);
        assert_eq!(parse("[cl 0] 1"), None);
        assert_eq!(parse("[cl x]"), None);
    }
}
