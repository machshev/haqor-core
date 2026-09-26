//! A quick-xml reader that keeps each text run whole.
//!
//! Since quick-xml 0.38 an entity or character reference (`&amp;`, `&#x221A;`)
//! is reported as its own [`Event::GeneralRef`] rather than folded into the
//! surrounding [`Event::Text`], so "A &amp; B" arrives as three events. The
//! parsers here act on each text event as a unit — trimming it, filtering
//! Hebrew fragments, emitting one styled BDB span per run — so a split run would
//! change what they store, and a reference they did not match would vanish.
//! [`Reader`] merges every run back into a single [`Event::Text`] with its
//! references resolved, which is what quick-xml delivered before.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use anyhow::{Context, Result, bail};
use quick_xml::XmlVersion;
use quick_xml::escape::resolve_predefined_entity;
use quick_xml::events::{BytesText, Event};

/// The version every source declares, or implies by declaring none: attribute
/// values are normalised by its rules.
pub(crate) const VERSION: XmlVersion = XmlVersion::Implicit1_0;

/// Reads owned events from an XML source, one [`Event::Text`] per text run.
pub(crate) struct Reader<R> {
    inner: quick_xml::Reader<R>,
    buf: Vec<u8>,
    /// The event that ended the last text run, handed out next.
    pending: Option<Event<'static>>,
}

impl Reader<BufReader<File>> {
    pub(crate) fn open(path: &Path) -> Result<Self> {
        let inner = quick_xml::Reader::from_file(path)
            .with_context(|| format!("opening {}", path.display()))?;
        Ok(Self::new(inner))
    }
}

impl<R: BufRead> Reader<R> {
    fn new(inner: quick_xml::Reader<R>) -> Self {
        Self {
            inner,
            buf: Vec::new(),
            pending: None,
        }
    }

    /// The next event. Consecutive text and references come back as one
    /// [`Event::Text`], whose content is the literal characters.
    pub(crate) fn next(&mut self) -> Result<Event<'static>> {
        let first = match self.pending.take() {
            Some(event) => event,
            None => self.read()?,
        };
        let Some(mut run) = text(&first)? else {
            return Ok(first);
        };
        loop {
            let event = self.read()?;
            match text(&event)? {
                Some(more) => run.push_str(&more),
                None => {
                    self.pending = Some(event);
                    break;
                }
            }
        }
        // `from_escaped` stores the content as given: the references are
        // already resolved, and there is nothing left to unescape.
        Ok(Event::Text(BytesText::from_escaped(run)))
    }

    fn read(&mut self) -> Result<Event<'static>> {
        self.buf.clear();
        Ok(self.inner.read_event_into(&mut self.buf)?.into_owned())
    }
}

/// The characters a text or reference event stands for; None for any other.
///
/// Only XML's predefined entities and numeric character references resolve;
/// the sources declare no others, so an unknown name is an error rather than a
/// gap in the output.
fn text(event: &Event<'_>) -> Result<Option<String>> {
    match event {
        Event::Text(t) => Ok(Some(t.xml10_content().into_owned())),
        Event::GeneralRef(r) => {
            let char_ref = r
                .resolve_char_ref()
                .context("reading an XML character reference")?;
            if let Some(c) = char_ref {
                return Ok(Some(c.to_string()));
            }
            let name = r.xml10_content();
            match resolve_predefined_entity(&name) {
                Some(s) => Ok(Some(s.to_owned())),
                None => bail!("undeclared XML entity &{name};"),
            }
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every event of `xml`, text runs as their content and the rest by kind.
    fn events(xml: &str) -> Vec<String> {
        let mut reader = Reader::new(quick_xml::Reader::from_str(xml));
        let mut out = Vec::new();
        loop {
            match reader.next().unwrap() {
                Event::Eof => break,
                Event::Text(t) => out.push(format!("text {:?}", t.xml10_content())),
                Event::Start(e) => out.push(format!("start {}", e.name().as_ref())),
                Event::End(e) => out.push(format!("end {}", e.name().as_ref())),
                Event::Empty(e) => out.push(format!("empty {}", e.name().as_ref())),
                other => out.push(format!("{other:?}")),
            }
        }
        out
    }

    #[test]
    fn a_run_with_references_is_one_text_event() {
        assert_eq!(
            events("<a>A &amp; B &lt;c&gt;<b/>&#x221A;&#8730;</a>"),
            [
                "start a",
                "text \"A & B <c>\"",
                "empty b",
                "text \"√√\"",
                "end a",
            ]
        );
    }

    #[test]
    fn an_undeclared_entity_is_an_error() {
        let mut reader = Reader::new(quick_xml::Reader::from_str("<a>&nbsp;</a>"));
        reader.next().unwrap();
        assert!(reader.next().is_err());
    }
}
