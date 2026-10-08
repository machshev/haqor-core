# Candidate datasets

A handoff note: datasets Haqor might add, what each would give the reader, and
what is known about its licence and how it would join to what Haqor already
has, and the status of those imported since. Compiled 2026-10-07, updated
2026-10-08; licences marked *checked* were read from the source on one of
those days, the rest still need checking.

## What Haqor already has

The Leningrad Codex (UXLC), the OSHB morphology and lexicon (BDB, Strong's),
Klein and Jastrow (Sefaria), STEP Bible's TAHOT, TIPNR and TBESH, SEDRA's
Peshitta NT, the Treasury of Scripture Knowledge, MACULA Hebrew's syntax
trees, an English translation adapted from the unfoldingWord Literal Text
(CC BY-SA 4.0, aligned word by word to the Hebrew), OpenBible.info's
geocoding, and the quotations and parallels the build finds by aligning roots.
The external datasets are vendored, prepared, in `src_texts`; Haqor keeps no
Strong's numbers, so a dataset keyed by them joins through TAHOT while it is
prepared. See [Data sources](../README.md#data-sources) and
[Attribution](../README.md#attribution).

## Licensing stance

Haqor is AGPL-3.0-or-later so that Bible study tools stay freely available.
Data keeps its own licence, so for a dataset the questions are whether Haqor
may redistribute it freely and what credit it requires:

- **CC BY and public domain** fit without conditions beyond credit.
- **CC BY-SA** fits the copyleft intent; what Haqor derives from it stays
  CC BY-SA.
- **CC BY-NC** is acceptable: Haqor already ships Klein under it, and the
  non-commercial term only forbids what Haqor does not want anyway. Data
  under it cannot be relicensed more freely, so keep it in its own tables.

Every addition needs an entry in `dataSourceCredits`
(`haqor/lib/src/app_info.dart`) and in both READMEs' Attribution sections.

## Strong candidates

### MACULA Hebrew (Clear Bible / Biblica)

- **Status:** syntax trees imported (October 2026: `syntax_tree`, see the
  README's "Syntax trees"). Its participant referents, semantic roles and
  SDBH word senses are not yet used.
- **Source:** <https://github.com/Clear-Bible/macula-hebrew>
- **Licence:** CC BY 4.0 for the dataset as a whole (*checked*, `LICENSE.md`),
  with the required credit "MACULA Hebrew Linguistic Datasets, available at
  https://github.com/Clear-Bible/macula-hebrew/". Its SDBH-derived fields are
  marked "©2000-2021 United Bible Societies. Used with permission"; SDBH
  itself is now CC BY-SA (below), so check how those fields should be
  credited.
- **What it gives:** syntax trees for the whole Hebrew Bible (clauses,
  phrases), semantic roles (who does what to whom), participant referents
  (who "he" or "it" is: `@subjref`, `@participantref`), SDBH semantic domains
  and word senses, Septuagint Greek equivalents, and English glosses (Cherith,
  CC BY 4.0).
- **Fit:** the largest new capability on the list: clause structure and
  pronoun referents in the reader.
- **Joining:** built on the Westminster Leningrad Codex and OSHB morphology,
  which Haqor also uses, with its own word and morph identifiers. Verify that
  its words map one-to-one onto Haqor's (UXLC) word positions, and list where
  they don't.
- **Effort:** large: a new import, schema, and reader UI.

### STEP Bible TIPNR and TBESH

- **Status:** imported (October 2026: `name_entity`, `word_name` and
  `sense` with their companions; see the README's "People, places and word
  senses"). Each word of the Hebrew text is linked to the person or place
  it names and given its sense, through TAHOT's tags. TIPNR's ,
   and  descriptions, which it says are adapted from Claude 3
  Opus's output, are not used: Haqor shows TIPNR's own description and
  summary.
- **Source:** <https://github.com/STEPBible/STEPBible-Data>: `Proper
  Nouns/TIPNR …`, `Lexicons/TBESH …`.
- **Licence:** CC BY 4.0 (*checked*, repository description and file
  headers). TBESH's Meaning column is the exception: it is the Online
  Bible's abridged BDB, and the file asks that Online Bible's permission be
  gained before using it, so Haqor takes only TBESH's glosses and sense
  divisions (Tyndale House's own).
- **What it gives:**
  - **TIPNR** tells apart the people and places that share a name (the many
    Zechariahs, two Bethlehems), each with a short description, family
    links and every reference.
  - **TBESH** is STEP's brief lexicon of Extended Strong's for Hebrew. It
    splits Strong's numbers that cover several words or senses.
- **Fit:** names in the reader could say *which* Zechariah; glosses and
  occurrence searches could follow the finer Strong's numbers.
- **Joining:** both key on Extended Strong's numbers, which TAHOT gives per
  word, so TAHOT is the bridge to Haqor's words.
- **Effort:** small to medium; the cheapest start on the list. Pin the commit
  and checksums in the fetch script, as for TAHOT.

### OpenBible.info cross references

- **Source:** <https://www.openbible.info/labs/cross-references/> (2 MB zip).
- **Licence:** CC BY (*checked*: the site's footer licenses all content
  "unless otherwise indicated" under a Creative Commons Attribution License;
  confirm the version and that the download isn't marked otherwise).
- **What it gives:** mostly the TSK, keyed verse to verse, with a community
  vote count on every link.
- **Fit:** the votes are the strength ranking the TSK lacks: they could
  order a verse's thematic references strongest first, or drive a strength
  filter like the quotations'.
- **Joining:** English (KJV-style) verse numbering. Re-number OT references
  with the TSK import's `Versification` (`crates/haqor-db-gen/src/tsk.rs`).
  Match each link to the TSK's targets by verse; the TSK keys its references
  by key phrase, so a vote attaches to a target, not a phrase.
- **Effort:** small to medium.

### Semantic Dictionary of Biblical Hebrew (UBS)

- **Source:** <https://github.com/ubsicap/ubs-open-license>, under
  `dictionaries/hebrew` (XML and JSON).
- **Licence:** CC BY-SA 4.0 (*checked* via translation.bible's open-access
  pages; read the repository's `LICENSE.md` before importing).
- **What it gives:** a modern lexicon that organises each word by its senses
  and semantic domains, with definitions, glosses and every reference. It
  covers about 90% of Old Testament words so far.
- **Fit:** adds to BDB, Klein and Jastrow, which are arranged by root or
  headword, rather than repeating them.
- **Joining:** SDBH entries carry lemmas and references. MACULA already maps
  SDBH senses onto words, so importing MACULA first may do most of the join.
- **Effort:** medium.

## Worth a look

### STEP Bible TVTMS

- **Source:** STEPBible-Data, `Versification/TVTMS …`. Licence CC BY 4.0
  (*checked*).
- **What it gives:** STEP's versification tables between the English, Hebrew,
  Latin, Greek and other traditions.
- **Fit:** the TSK import infers the KJV-to-Hebrew numbering from TAHOT's
  per-word data. TVTMS would make that mapping explicit, and reusable for
  every English-numbered dataset (OpenBible's cross references above, for
  one).

### OpenBible.info geocoding

- **Status:** imported (October 2026: `name_location`). Each TIPNR place
  finds its OpenBible place by the OpenBible name TIPNR gives it, and keeps
  every identification with its confidence.
- **Source:** <https://github.com/openbibleinfo/Bible-Geocoding-Data>.
  Licence CC BY 4.0 (*checked*).
- **What it gives:** coordinates for every place named in the Bible.
- **Fit:** with TIPNR to say which place a name is, a place name could open
  a map.

### Tyndale Open Study Notes and Bible Dictionary

- **Source:** Tyndale House Publishers' open resources, distributed by
  unfoldingWord and others: the *Tyndale Open Study Notes*, theme notes,
  book introductions and profiles of people, and the *Tyndale Bible
  Dictionary*. Licence believed to be CC BY-SA 4.0 (*not checked*; find the
  canonical download and read its licence).
- **What it gives:** verse notes and dictionary articles in plain English,
  and profiles of the main people.
- **Fit:** the profiles would sit beside TIPNR's records of the same people,
  and the dictionary beside its places; the notes would give a verse menu
  something to say. Like the ULT, anything derived stays CC BY-SA.
- **Joining:** notes are keyed by English verse references (re-number as
  the TSK import does); profiles and articles by English name, which TIPNR's
  English forms can match.

### Gesenius' Hebrew Grammar

- **Source:** *Gesenius' Hebrew Grammar*, edited by E. Kautzsch, translated
  by A. E. Cowley (2nd English edition, Oxford, 1910): in the public domain.
  Wikisource has a transcription (its own text is CC BY-SA; *not checked*),
  and there are others to compare.
- **What it gives:** the standard reference grammar, numbered by section
  (§ 22 *Peculiarities of the Gutturals*).
- **Fit:** the tutor's grammar points could link to the section that treats
  each, for a learner who wants the full account; sections are cited by
  number everywhere, so a link is stable.
- **Joining:** by hand: a table from each grammar concept to its sections.

### Targums Onkelos and Jonathan

- **Source:** Sefaria. Licence *not checked*; Sefaria licenses each text
  separately.
- **What it gives:** verse-aligned Aramaic renderings of the Torah and the
  Prophets. They pair with Jastrow, which exists largely to read them.

### ETCBC BHSA and Dead Sea Scrolls

- **Sources:** <https://github.com/ETCBC/bhsa>, <https://github.com/ETCBC/dss>.
- **Licence:** CC BY-NC 4.0 for both (*checked*, READMEs). The BHSA
  repository's GitHub metadata says MIT, but its README licenses the data
  CC BY-NC 4.0; go by the README.
- **What they give:** BHSA is the richest Hebrew linguistic database (clause
  and phrase structure, valence, trees). The DSS dataset is the scrolls' text,
  from Abegg's data.
- **Fit:** acceptable under the licensing stance above. For syntax, MACULA
  covers much of the same ground under CC BY and is aligned to the
  Westminster Leningrad Codex; BHSA follows the BHS text, which differs
  slightly from UXLC. The DSS data has no CC BY alternative; it would allow
  comparing the scrolls with the Masoretic text.

## Looked at and set aside

- **KJV translators' marginal notes.** eBible.org's KJV USFM (from
  CrossWire's KJV module) has 6,959 of them, OT only ("Heb. …", "or, …",
  "that is, …"). Set aside: they give an English reader a glimpse of the
  Hebrew, which Haqor shows directly, with its lexicons.
- **KJV marginal cross references.** No digitised copy of the KJV's own margin
  references (1611 or Blayney's 1769) was found. The TSK was compiled on top
  of that tradition (CrossWire credits "Canne, Browne, Blayney, Scott, and
  others"), so Haqor already shows most of them, unmarked. Revisit if a
  digitised 1769 margin turns up: it could label the TSK references that come
  from the KJV margin.
- **sync.bible.** Its `crossReferences.json` is the TSK keyed by verse,
  without phrases; its KJV texts carry no notes.
- **The Septuagint.** Not wanted: out of scope for Haqor.
