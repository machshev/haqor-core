# haqor-core

Rust libraries and tools providing Bible resources and the learning engine for
the Haqor app.

## Workspace

The repository root is a virtual Cargo workspace. Its packages are split by
responsibility:

- `crates/haqor-core`: app-facing Bible access, tutor, grammar, glosses, and text helpers
- `crates/haqor-morphology`: DB-free Hebrew morphology generation and parsing
- `crates/haqor-db-gen`: source-text parsing and generated-database pipelines
- `crates/haqor-admin`: loopback-only lexical-overlay web server
- `crates/haqor-cli`: the `haqor` command-line tool

Applications depend directly on `haqor-core`. Its public paths include
`haqor_core::bible`, `haqor_core::tutor`, and `haqor_core::morphology`.

## Data sources

The reader's primary Old Testament lemma and morphology data comes from the
[Open Scriptures Hebrew Bible](https://github.com/openscriptures/morphhb),
licensed under CC BY 4.0. Haqor's mechanically generated verb and noun analyses
remain in `hebrew.db` as reviewable alternatives and as a fallback where the
OSHB and UXLC token streams cannot be aligned safely.

The reader's context-sensitive Old Testament interlinear translations come
from [STEP Bible's TAHOT dataset](https://github.com/STEPBible/STEPBible-Data),
also licensed under CC BY 4.0. BDB remains the source for full lexicon entries.
TAHOT is vendored, prepared (see below), in `src_texts/STEPBible-Data/`.
After updating it, regenerate the reader glosses:

```sh
./scripts/fetch-stepbible-data.sh    # only to update the vendored copy
cargo run --release -- db refresh-reader-glosses
```

### Vendored sources

Every dataset the build reads lives in `src_texts/`, so a build needs no
network and does not depend on an upstream copy staying online. The external
datasets (`STEPBible-Data`, `MACULA-Hebrew`, `unfoldingWord`, `Murdock`,
`OpenBible-Geocoding`, `Theographic`) are committed in a prepared form: `haqor db prepare`
reduces each, as published, to what the build reads, in plain text
(TSV/JSON lines) whose diffs can be reviewed. That is about 47 MB in place of
about 600 MB, and the build skips parsing their XML and USFM. Preparing does
not depend on Haqor's own text: aligning to the corpus happens in the build.
STEP Bible's files are joined by Strong's numbers, which are used up while
preparing, so none is vendored or built into a database.

To update a dataset, move the pin in its `scripts/fetch-*.sh`, run it (it
downloads the dataset at that commit, verifies checksums or git's hashes, and
prepares it), and review the diff. Each dataset's `SOURCE` file records where
it came from.

Full `gen-hebrew --force` builds also include TAHOT, but the refresh command is
the intended low-resource path when only the interlinear source has changed.

Klein's etymological dictionary and Jastrow's dictionary of the Targumim,
Talmud and Midrash come from Sefaria's digitisations and sit beside BDB in
`lexicon.db` (`dictionary`) and `haqor.db` (`dictionary_entry`). Sefaria
publishes them only inside its nightly database dump, so the entries Haqor
uses — Klein's biblical layer, Jastrow's biblical and Aramaic words, and any
entry spelled like a BDB headword or a SEDRA lexeme — are checked in under
`src_texts/Sefaria/`, and `gen-lexicon` reads them from there. Refreshing them
is a deliberate step, reviewed by its diff:

```sh
nix develop -c ./scripts/fetch-sefaria-lexicons.sh
cargo run --release -- db gen-lexicon
```

The script streams the ~2.5 GB dump, keeps only its `lexicon_entry`
collection, and records the dump date and checksum in `src_texts/Sefaria/SOURCE`.

## Commands

The CLI is the workspace's default member, so it remains available from the
workspace root:

```sh
cargo run -- db gen-hebrew --force
cargo run -- admin
```

### The runtime database

The four databases in `data/` are the generation pipeline's cache: each is one
stage's output, so the fast iteration loops rebuild only what changed. What the
app ships — and what `Bible::open` reads — is a single curated `haqor.db`,
built from all four:

```sh
cargo run --release -- db gen-runtime                      # data/haqor.db
cargo run --release -- db gen-runtime --blob-codec zstd    # ~8 MiB smaller
```

It resolves every word's analysis once at build time, packs verse references,
interns repeated strings and drops what only the generator needed — 87 MiB of
generation databases become 37 MiB, or 30 MiB compressed. `--blob-codec none`
is the default because it keeps verse text and lexicon entries readable with
`sqlite3`; shipped builds use `zstd`, whose trained dictionary travels inside
the database. See [ADR 6](doc/adr/0006-single-runtime-database.md).

`gen-runtime` is a prerequisite for everything that reads data: the CLI, the
tests that need a corpus, and the app all open `haqor.db`. Regenerate it
whenever an earlier stage changes, or they keep reading the previous build.

### Cross references: quotations and parallels

`gen-runtime` also fills a `quotation` table of linked verse pairs: NT verses
that quote or echo an OT verse, found by aligning the Peshitta's roots against
the Hebrew text's directly (Aramaic and Hebrew share most roots), so no
translation is involved; and pairs within one testament that share wording
(Kings/Chronicles, Psalm 18/2 Samuel 22, repeated oracles, synoptic
parallels), aligned on Hebrew or Syriac roots alone. Same-testament scores are
scaled onto the OT/NT scale, so one strength filter serves both.
`Bible::cross_references` returns a verse's links and `Bible::quotations` lists
them by rank, optionally within one book and chapter range and one
`QuotationScope` (other testament / same testament). To tune the matcher
without a full rebuild:

```sh
cargo run --release -- db gen-quotes                         # rebuild the table in place, report recall
cargo run --release -- db gen-quotes --explain 40:2:15=15:11:1   # why a pair does or doesn't match
cargo run --release -- db gen-quotes --dry-run --sweep within.min_score=6,7,8   # try same-testament floors
```

### Thematic cross references

`gen-runtime` also fills a `thematic_reference` table from the *Treasury of
Scripture Knowledge* in `src_texts/TSK`: the hand-curated references a
wide-margin Bible prints, attached to the key words and phrases of each verse
(in the KJV's wording, with "the LORD" written as "Yahweh")
(63,678 phrases, ~379k targets). They are a separate set from the quotation
table's, which are found by root alignment. The TSK numbers verses as the KJV
does, so each OT reference is re-numbered onto the Hebrew text through the
TAHOT files' paired numbering (Malachi 4:5 becomes 3:23, Psalm verses shift
past their titles), which `gen-runtime` reads from
`src_texts/STEPBible-Data/`. `Bible::thematic_references` returns a verse's phrases and their targets. To rebuild the table alone:

```sh
cargo run --release -- db gen-tsk      # RUST_LOG=haqor_db_gen::tsk=debug lists the references it drops
```

### Syntax trees

`gen-runtime` also fills a `syntax_tree` table from
[MACULA Hebrew](https://github.com/Clear-Bible/macula-hebrew)'s parse of every
Old Testament verse: its clauses, the phrases inside them, and the function
each plays in its clause (subject, verb, object, predicate, adverbial).
`gen-runtime` reads the trees, prepared, from `src_texts/MACULA-Hebrew/`:

```sh
cargo run --release -- db gen-syntax   # rebuild the table alone
```

MACULA numbers words as the Westminster Leningrad Codex does, counting the
written form of a ketiv/qere pair and dividing a few words differently, so
the import places each word on the corpus by its letters (all but a handful of
verses spell their letters identically). `Bible::syntax_tree` and
`Bible::chapter_syntax_trees` return the parsed trees.

### English translation

`gen-runtime` also fills a `translation_verse` table with an English
translation of the Old Testament adapted from the
[unfoldingWord Literal Text](https://www.unfoldingword.org/ult) (ULT), whose
every English word is aligned to the Hebrew word or words it renders. Its
sources are the ULT and the unfoldingWord Hebrew Bible its alignments
name; `gen-runtime` reads their words and alignments, prepared, from
`src_texts/unfoldingWord/`:

```sh
cargo run --release -- db gen-translation   # rebuild the table alone
```

Both unfoldingWord texts follow the English verse numbering, so the import
goes by words rather than references. The UHB's words are lined up against
the corpus by their letters, book by book: the two are the same Leningrad
text, differing where the UHB writes a ketiv, divides a word differently, or
has a verse Leningrad lacks (Nehemiah 7:68). Each alignment then names a word
of the corpus, and each English verse is filed under the Hebrew verse most of
its words render, so Malachi 4:1 sits beside 3:19 and a psalm's title beside
its first verse. 616,411 of 616,562 English words link to a word of the
corpus. `Bible::chapter_translation` returns a chapter's English as spans,
each naming the Hebrew words it renders and whether it is supplied (the
ULT's braced, italic words). With `--blob-codec zstd` the verses are compressed against a
dictionary trained on them, stored beside the build's own in `blob_dict` (the
Hebrew one knows nothing useful about English), taking the table from about
4.4 MiB to 2.0 MiB.

The fetch script pins the ULT at a `master` commit rather than its latest
release (v91), which leaves out the books still being checked (Numbers,
1–2 Chronicles, Ecclesiastes, Isaiah, Jeremiah and Ezekiel); their drafts are
complete and fully aligned.

The New Testament has two English texts. Beside the Greek, in
`greek_translation_verse`, is the ULT's New Testament: its alignments name
words of the unfoldingWord Greek New Testament (UGNT), a critical text, whose
words are lined up against the TR (`greek_word`) by their unaccented
letters, words the two order differently placed afterwards where they are
spelled the same or alike nearby. Where the two divide verses differently,
an English verse is divided by the TR verses its words render (Philippians
1:16–17 come in the other order). Where the TR reads otherwise, its words go
without English, and English rendering a word the TR lacks renders nothing;
the 94 verses where the TR has six or more words without English (the
doxology of Matthew 6:13 and the Comma Johanneum among them) are given Haqor's own English in `data/tr_translation_supplement.tsv`,
written in the ULT's style and linked to the TR's words. 177,406 of
181,182 English words link to a word of the TR.
`Bible::greek_chapter_translation` reads a chapter.

Beside the Peshitta, `translation_verse` holds James Murdock's translation
(1852, public domain), from CrossWire's SWORD module, which
`scripts/fetch-murdock.sh` exports with `mod2imp` (in the dev shell) and
`db prepare murdock` reduces to `src_texts/Murdock/murdock.tsv`. The module
runs a few verses together and misplaces others, and Murdock numbers a few
as the KJV does where the Peshitta does not; the import puts each right
(`murdock::FIXES`), so that every verse of the Peshitta has its English. His
footnotes and the Gospels' colophons are left out, and "lampstand" and
"love" stand for his "candlestick" and "charity". His English renders no
particular words.

### People, places and word senses

`gen-runtime` also links every word of the Hebrew text that names a person,
place or other named thing to its record in STEP Bible's TIPNR, which tells
apart those sharing a name (the many Zechariahs, the two Bethlehems), and
gives each word its sense from STEP Bible's TBESH (שָׁכַב "lie down" read
as "sleep" in one verse and "be dead" in another). Both come through
TAHOT's tags, aligned to the corpus as its glosses are; places take their
positions from OpenBible.info's geocoding, every identification with its
confidence (TIPNR's own position where OpenBible has none), and a region its
bounds and a river its course. The New Testament, the Peshitta, is not
tagged word by word: it names them by verse, from the references TIPNR
lists under their Greek forms. The sources are prepared in
`src_texts/STEPBible-Data/` and `src_texts/OpenBible-Geocoding/`:

```sh
cargo run --release -- db gen-names   # rebuild the tables alone
```

38,720 words name one of 4,247 records, 4,759 New Testament verses name
one, and 1,005 places have a position. 259,392 words have one of 6,714 senses: `surface_sense` gives each surface
the sense most of its occurrences have, and `word_sense` only the
occurrences that differ. TAHOT, TIPNR and TBESH join on Strong's numbers,
which are used up while the sources are prepared: the tables use ids of
their own, and `db prepare` refuses to write a file holding a Strong's
number. `Bible::word_name`, `Bible::name_entity`, `Bible::name_verses`,
`Bible::chapter_places` and `Bible::word_sense` read them.

### Events of the Bible

`gen-names` also builds the events of the Bible for the app's timeline
(`bible_event`, with the verses telling each in `bible_event_passage` and the
people and places taking part in `bible_event_name`), from Theographic Bible
Metadata, prepared in `src_texts/Theographic/`
(`scripts/fetch-theographic.sh`). Its chronology is a young-earth one: the
creation in 4004 BC, the flood in 2348 BC, the exodus in 1491 BC. Its people
and places are found among TIPNR's records by name and the verses naming them,
and its verses re-numbered onto the Hebrew text. `data/event_overrides.json`
corrects it (see `data/EVENTS.md`). `Bible::bible_events` reads them.

### Greek New Testament

`gen-runtime` also builds the Textus Receptus — Scrivener's 1894 edition,
the Greek behind the KJV — beside the Peshitta, a word a row in
`greek_word`, each tagged with its dictionary form and gloss, its grammar
(Robinson's codes as STEP Bible extends them), its English in context and
the person or place it names. Its verses are numbered as the KJV numbers
them.

The words and tags come from STEP Bible's TAGNT, which amalgamates the
major editions word by word: the TR is the words it gives the TR, spelled
as the TR spells them, with the readings TAGNT gives only as variants of
another edition's word looked up in TBESG. TAGNT's word order follows
Nestle-Aland, its displacements for the TR are not always dependable, and
it now and then leaves out a TR word, so each chapter is lined up against
Dr Maurice Robinson's public-domain transcription of Scrivener, which is
untagged and unaccented but in the edition's own words, order and verses.
Of Scrivener's 140,655 words, 140,151 take a TAGNT word spelled the same,
269 one spelled a little otherwise (respelled as Scrivener has it where
the TR spells it so elsewhere or the two differ only in their endings),
168 the tags of the same spelling elsewhere, and 12 stay unaccented and
untagged; 296 words TAGNT gives the TR that Scrivener lacks are left out.
TAGNT's punctuation, which follows Nestle-Aland, stays only where the TR
has the same words either side of it. The sources are prepared with
STEP Bible's, into `src_texts/STEPBible-Data/` (`tr.tsv`,
`greek_lemmas.tsv`):

```sh
cargo run --release -- db gen-greek   # rebuild the tables alone
```

`Bible::greek_chapter` reads a chapter.

Each dictionary form has Abbott-Smith's and Thayer's entries
(`greek_dictionary`, the forms each describes in `greek_dictionary_form`)
and the roots it is built on (`greek_root`). An entry finds its forms by the
Strong's numbers TAGNT and the lexicon share, or failing those its spelling;
like STEP Bible's, the numbers are used up while the sources are prepared.
A form's root is where its derivation ends — Abbott-Smith's `(< λέγω)`,
Thayer's `(fr. ἀγαθοποιός)`, a compound's hyphenated last element
(`ἐκ-βάλλω`) — followed back to a word derived from nothing given; a
compound of two words has both roots, its last element's first, and a
preposition, privative or adverb prefixed to a word is none. Of the TR's
5,419 dictionary forms, 5,409 have an entry, built on 2,698 roots. Both
lexicons write "Yahweh" for the divine name where they had "Jehovah". They
are prepared with STEP Bible's, into `src_texts/STEPBible-Data/`
(`greek_lexicon.jsonl`, `greek_roots.tsv`). `Bible::greek_lexicon_entries`,
`Bible::greek_roots`, `Bible::greek_root_lemmas` and
`Bible::greek_root_occurrences` read them.

`gen-runtime` and `gen-greek` also link the Peshitta's words to the Greek's
(`syriac_greek`): no open edition aligns the two word by word, so they are
aligned here, over the 7,957 verses both have, by IBM Model 1 trained in
each direction. The Syriac proclitics (ܘ, ܕ, ܠ, ܒ) and pronoun suffixes
SEDRA writes within a word stand as words of their own, and the Greek
article is left out, so that καί, the article and the pronouns are not
mistaken for the words they stand beside. A pair is kept where both
directions link it at least twice, and it is either word's commonest
equivalent or a large part of one's renderings; 2,132 pairs give 1,546 of
the Peshitta's 3,301 lexemes, 97% of its words, an equivalent (ܗܝܡܢܘܬܐ
πίστις, ܚܝܐ σῴζω). `Bible::greek_equivalents` and
`Bible::syriac_equivalents` read them.

### LAN progress sync

Run a personal server on the LAN that the app can reach:

```sh
cargo run -p haqor-sync-server --release -- \
  --bind 0.0.0.0:8788 \
  --progress "$HOME/.local/share/haqor/progress.db" \
  --token "choose-a-long-random-secret"
```

Then in the app open **Learn to read → Study pace → Progress sync**, enter the
machine's LAN address (for example `http://192.168.1.10:8788`) and the same
token. The app syncs when it launches and shortly after each answer. The
built-in service uses HTTP with a bearer token, so run it only on a trusted
LAN (or place it behind a VPN or HTTPS reverse proxy).

The admin server can also be run independently:

```sh
cargo run -p haqor-admin -- server --bind 127.0.0.1:8787
```

Build or test every package with:

```sh
cargo check --workspace --all-targets --all-features
cargo test --workspace --all-features
```

## Attribution

Haqor's databases are curated from the sources below. Several carry attribution
requirements; this section, together with the app's **About** view, is where
that credit is given. Source names appear here rather than in table and column
names — the runtime schema is named for what data *is*, not where it came from
(see [ADR 6](doc/adr/0006-single-runtime-database.md)).

**Hebrew Bible text** — the Unicode/XML Leningrad Codex (UXLC) from
[tanach.us](https://tanach.us), transcribed from the *Westminster Leningrad
Codex*, which is in the public domain.

**Hebrew lemmas and morphology** — the
[Open Scriptures Hebrew Bible](https://github.com/openscriptures/morphhb)
(morphhb), licensed CC BY 4.0.

**Hebrew lexicon** — the
[OSHB Hebrew Lexicon](https://github.com/openscriptures/HebrewLexicon):
*Brown-Driver-Briggs*, *Strong's Hebrew Dictionary* and the lexical index
bridging them. The digitised files are released CC BY 4.0 — credit the Open
Scriptures Hebrew Bible Project — while the underlying text of Brown, Driver,
Briggs and of Strong's remains in the public domain. Haqor's own lexicon is an
edited and expanded derivative of these entries.

**Etymological dictionary** — Ernest Klein, *A Comprehensive Etymological
Dictionary of the Hebrew Language for Readers of English* (Carta Jerusalem,
1987), in the [digitisation by Sefaria](https://www.sefaria.org/Klein_Dictionary),
licensed CC BY-NC. Haqor carries the entries a reader of the Hebrew Bible or the
Peshitta can reach, converted from Sefaria's HTML into its own entry format;
the wording is unchanged. The non-commercial terms apply to this data wherever
Haqor is redistributed.

**Rabbinic and Aramaic dictionary** — Marcus Jastrow, *A Dictionary of the
Targumim, the Talmud Babli and Yerushalmi, and the Midrashic Literature*
(Luzac, London, 1903), in the public domain, from the
[digitisation by Sefaria](https://www.sefaria.org/Jastrow). Filtered and
converted in the same way as Klein, with "Yahweh" written for "Jehovah".

**Greek lexicons** — G. Abbott-Smith, *A Manual Greek Lexicon of the New
Testament* (Scribner's, New York, 1922), in the public domain, as the
[Translatable Exegetical Tools project](https://github.com/translatable-exegetical-tools/Abbott-Smith)
marks it up in TEI; and J. H. Thayer, *A Greek-English Lexicon of the New
Testament* (1889), in the public domain, as a
[machine-read dataset](https://github.com/nigelmsipa/thayers-greek-lexicon-dataset)
released under CC0 gives it. The dataset has had a single reading pass and
is not proofread; an entry split over a page is joined. Haqor vendors both
in a prepared form with STEP Bible's (`scripts/fetch-stepbible-data.sh`),
keyed by the TR's dictionary forms, their derivations followed to each
word's root and "Yahweh" written for "Jehovah".

**Interlinear translations** — STEP Bible's
[TAHOT dataset](https://github.com/STEPBible/STEPBible-Data), licensed CC BY
4.0. Haqor vendors it in `src_texts/STEPBible-Data/` in a prepared form
(`scripts/fetch-stepbible-data.sh`): each word's reference, Hebrew and
translation as published, with its Strong's numbers replaced by ids of
Haqor's own.

**People and places** — STEP Bible's
[TIPNR](https://github.com/STEPBible/STEPBible-Data) (Translators
Individualised Proper Names with all References), data created by
www.STEPBible.org based on work at Tyndale House Cambridge, licensed CC BY
4.0. Haqor keeps each record's description, summary, name forms and links,
with the Strong's numbers in its prose removed, and links each to the words
naming it through TAHOT.

**Word senses** — the glosses and sense divisions of STEP Bible's
[TBESH](https://github.com/STEPBible/STEPBible-Data) (Translators Brief
lexicon of Extended Strongs for Hebrew), created by Tyndale House scholars,
licensed CC BY 4.0. Its Meaning column (the Online Bible's abridged BDB) is
not used.

**Places on the map** — OpenBible.info's
[Bible Geocoding Data](https://github.com/openbibleinfo/Bible-Geocoding-Data),
licensed CC BY 4.0. Haqor keeps each identification's position, kind,
modern location and confidence, and the ground a region covers or the
course a river runs: OpenBible.info's own rough bounds, or OpenStreetMap's
geometry as it gathers it (© OpenStreetMap contributors, ODbL 1.0).

**Thematic cross references** — *The Treasury of Scripture Knowledge*
(Samuel Bagster & Sons, 1830s; commonly credited to R. A. Torrey), from the
data file published by [Just Verses](http://www.justverses.com/jv/app/downloadTSK.vm),
which describes its downloads as public domain biblical information. The file
is kept unchanged in `src_texts/TSK/`, with its source and checksums; Haqor
re-numbers its KJV verse references onto the Hebrew text and writes "Yahweh"
where its KJV phrases read "the LORD".

**Bible timeline** — the events of
[Theographic Bible Metadata](https://github.com/robertrouse/theographic-bible-metadata),
licensed CC BY-SA 4.0: each event's year, how long it lasted, the verses
telling it and the people and places taking part. Its Old Testament dates are
Floyd Nolen Jones's (*The Chronology of the Old Testament*), from the creation
in 4004 BC, and those of Acts Todd Atwood's. Haqor corrects a few dates and
titles, re-numbers the verses onto the Hebrew text, finds the people and
places among STEP Bible's TIPNR records, and leaves Jesus out of the events
before his birth; haqor-core's `data/event_overrides.json` lists the changes.
The adapted events in `haqor.db` are shared under the same licence, CC BY-SA
4.0.

**Syntax trees** — MACULA Hebrew Linguistic Datasets, available at
https://github.com/Clear-Bible/macula-hebrew/, (C) 2022-2024 Biblica, Inc,
licensed CC BY 4.0. Haqor carries their clauses, phrases and clause-level
functions (the Westminster Hebrew Syntax of the J. Alan Groves Center, CC BY
4.0), with English glosses for parts of words from Cherith Analytics'
glosses (CC BY 4.0). Haqor vendors the trees alone in `src_texts/MACULA-Hebrew/`
(`scripts/fetch-macula-hebrew.sh`), each verse's clauses, phrases and words
written in a compact text form.

**English translation** — adapted from the unfoldingWord Literal Text (ULT),
(C) unfoldingWord, licensed CC BY-SA 4.0, using its alignment to the
unfoldingWord Hebrew Bible (UHB) and Greek New Testament (UGNT), both CC BY-SA
4.0. Haqor files each English verse under the verse it renders, places the
aligned words on its own Hebrew text and on the Textus Receptus, gives its
own English for verses where the Textus Receptus reads otherwise
(`data/tr_translation_supplement.tsv`), and leaves out the ULT's footnotes
and paragraphing. The original
work by unfoldingWord is available from
[unfoldingword.org/ult](https://www.unfoldingword.org/ult). The adapted text,
the Old Testament's rows of `haqor.db`'s `translation_verse` table and its
`greek_translation_verse` table, is shared under the same licence, CC BY-SA
4.0. Haqor vendors the words and alignments in `src_texts/unfoldingWord/`
(`scripts/fetch-unfoldingword.sh`), with each text's licence.

**English of the Peshitta** — James Murdock, *The New Testament; or, the Book
of the Holy Gospel of our Lord and our God, Jesus the Messiah: a literal
translation from the Syriac Peshito version* (1852), public domain, from
CrossWire's SWORD module (`scripts/fetch-murdock.sh`), in
`src_texts/Murdock/`.

**Syriac New Testament** — the text of the British and Foreign Bible Society's
edition, with lexical and morphological data from SEDRA:

> This work makes use of the Syriac Electronic Data Retrieval Archive (SEDRA)
> by George A. Kiraz, distributed by the Syriac Computing Institute.

SEDRA III's terms also ask that work using it cite:

> G. Kiraz, 'Automatic Concordance Generation of Syriac Texts', in *VI Symposium
> Syriacum 1992*, ed. R. Lavenant, Orientalia Christiana Analecta 247, Rome,
> 1994.

Haqor reads the ASCII SEDRA III files and renders them in Unicode, in Syriac
script and transliterated into Hebrew letters for the reader. That script
conversion is the only change: the entries, morphology and text content are
unmodified.

**Greek New Testament** — the Textus Receptus (Scrivener 1894) from STEP
Bible's [TAGNT](https://github.com/STEPBible/STEPBible-Data) (Translators
Amalgamated Greek NT), data created by www.STEPBible.org based on work at
Tyndale House Cambridge, licensed CC BY 4.0, its English renderings based on
the Berean Study Bible; with dictionary forms and glosses for readings TAGNT
gives as variants from STEP Bible's TBESG (Translators Brief lexicon of
Extended Strongs for Greek), also CC BY 4.0. The TR's words are ordered and
divided into verses as in Dr Maurice A. Robinson's transcription of
Scrivener's 1894 edition
([greektext-scrivener](https://github.com/byztxt/greektext-scrivener)), in
the public domain. Haqor's changes: the TR's words chosen from TAGNT's
editions and put in that order, its punctuation kept only where the TR
shares it, accents written in their canonical Unicode form, and the
Strong's numbers replaced by ids of Haqor's own.

## Licence

Haqor's core is free software, licensed under the GNU Affero General Public
License, version 3 or (at your option) any later version; see
[LICENSE](LICENSE). Anyone may use, share and change it, and anyone who
distributes it, or runs a modified version (the sync server included) for
others to use over a network, must offer them its source under the same terms.
The source texts and the databases built from them keep their own licences,
listed under [Attribution](#attribution).
