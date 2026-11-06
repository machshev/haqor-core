# 7. Haqor's own Hebrew lexicon

Date: 2026-11-06

## Status

Proposed. The schema and its validator come first, then a tool that gathers
the evidence for a root, then entries, then the app.

## Context

The word sheet shows four lexicons one after another: BDB, Klein, Jastrow and
SEDRA. They often disagree, and nothing in the app reconciles them, so the
reader is left to weigh four dense entries written in four styles for four
purposes.

None of them gives what the project wants:

- **Comparative evidence.** Gesenius and BDB compare each root with its cognates
  in the other Semitic languages. The OSHB digitisation of BDB that Haqor uses
  kept the markup for those cognates but lost their script: of 449 Arabic
  cognates 447 read just "Arabic", and all 123 Syriac and 55 Ethiopic ones are
  empty. Only the Akkadian (104) and Aramaic (14) survive.
- **Gesenius itself.** The public-domain Gesenius is being OCR'd in
  `../gesenius-lexicon`, but the scans are poor and the entries mix Hebrew,
  Arabic, Syriac, Greek and Latin, which OCR handles badly. It will not yield a
  clean lexicon soon.
- **Meaning drawn from scripture.** A lexicon's definitions are its authors'
  conclusions, which they reached from evidence they cite only in part. A
  meaning should be shown from that evidence: the verses where the word's use
  fixes its sense, its parallels and opposites, its cognates, and the Greek
  the Septuagint and the New Testament use for it.
- **Doctrine from scripture alone.** Where a word bears on doctrine (נֶפֶשׁ,
  רוּחַ, שְׁאוֹל, עוֹלָם), the entry should say so, resting on scripture only,
  never on tradition or commentators.
- **Structure.** Entries should share sections, tags and metadata, so they can
  be filtered and searched, not only read.

## Decision

Haqor writes its own lexicon, one root at a time, starting with the roots that
occur most often. It is a lexicon in its own right, not a survey of others.

### Evidence, not authority

Other lexicons are where to look for evidence, never what an entry cites. When
BDB or Gesenius reaches a conclusion, the entry goes to the evidence behind it
(the verse, the cognate in its own language's dictionary, the inscription, the
Septuagint's rendering) and cites that. An entry never says "BDB gives…",
and it does not list where lexicons agree or differ.

Every claim cites its evidence, and the validator enforces it:

- a sense cites the verses that show it;
- a cognate cites the dictionary entry or text of its own language;
- a doctrinal statement cites scripture and nothing else.

### Distilled first

An entry opens with what a reader needs at a glance, then gives the detail
behind it:

1. `gloss`: a few words, for the word sheet and the tutor.
2. `summary`: two to four sentences: the root's core meaning and how it
   is used.
3. The words the root forms, each with its own gloss and senses, each sense
   with its evidence.
4. Cognates, Greek equivalents, usage (parallels, opposites, phrases), and
   doctrine.

### Shape and storage

- One JSON file per root in `data/lexicon/`, named by the entry's `id`, with its
  schema described in `data/lexicon/README.md`. JSON follows the project's other
  curated data (`artifacts.json`, `apologetics.json`) and needs no new
  dependency.
- `data/lexicon/sources.json` registers every external work an entry cites:
  citation, licence, URL, and where the copy in `src_texts` is. A cited work is
  kept in `src_texts` wherever its licence allows, so evidence can be checked
  without leaving the repository.
- `haqor-db-gen` reads the files into typed structures that reject unknown
  fields, checks every verse reference against the text and every source
  against the registry, and builds them into the runtime database with tables
  indexed by root, word, tag and reference for searching.

### Built together

Entries are written with the user, starting from a brief of the evidence: every
occurrence of the root grouped by word and stem with its English rendering, the
Greek it is given in the New Testament's quotations, its parallels, and the
other lexicons' text, only so their evidence can be followed up. An entry starts
as `draft` and becomes `reviewed` once the user has checked it.

### In the app

Where a root has an entry, the word sheet shows it first and puts the other
lexicons under a collapsed heading. Elsewhere the sheet stays as it is.

## Consequences

- The lexicon grows slowly, and for a long time most words have no entry. The
  app has to handle both.
- Requiring evidence for every claim makes entries slower to write, but every
  one can be checked, and a wrong claim can be found and fixed from the
  evidence.
- Cognate languages without an openly licensed dictionary (Ugaritic,
  Phoenician) can be cited only from their texts, or not at all.
- Each source added to `sources.json` needs its licence checked and an entry in
  the app's `dataSourceCredits` and both READMEs' Attribution sections.
