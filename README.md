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
datasets (`STEPBible-Data`, `MACULA-Hebrew`, `unfoldingWord`,
`OpenBible-Geocoding`) are committed in a prepared form: `haqor db prepare`
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
converted in the same way as Klein.

**Interlinear translations** — STEP Bible's
[TAHOT dataset](https://github.com/STEPBible/STEPBible-Data), licensed CC BY
4.0. Haqor vendors it in `src_texts/STEPBible-Data/` in a prepared form
(`scripts/fetch-stepbible-data.sh`): each word's reference, Hebrew and
translation as published, with its Strong's numbers replaced by ids of
Haqor's own.

**Thematic cross references** — *The Treasury of Scripture Knowledge*
(Samuel Bagster & Sons, 1830s; commonly credited to R. A. Torrey), from the
data file published by [Just Verses](http://www.justverses.com/jv/app/downloadTSK.vm),
which describes its downloads as public domain biblical information. The file
is kept unchanged in `src_texts/TSK/`, with its source and checksums; Haqor
re-numbers its KJV verse references onto the Hebrew text and writes "Yahweh"
where its KJV phrases read "the LORD".

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
unfoldingWord Hebrew Bible (UHB, CC BY-SA 4.0). Haqor files each English
verse under the Hebrew verse it renders, places the aligned words on its own
Hebrew text, and leaves out the ULT's footnotes and paragraphing. The original
work by unfoldingWord is available from
[unfoldingword.org/ult](https://www.unfoldingword.org/ult). The adapted text,
the `translation_verse` table of `haqor.db`, is shared under the same licence,
CC BY-SA 4.0. Haqor vendors the Old Testament's words and alignments in
`src_texts/unfoldingWord/` (`scripts/fetch-unfoldingword.sh`), with each
text's licence.

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

## Licence

Haqor's core is free software, licensed under the GNU Affero General Public
License, version 3 or (at your option) any later version; see
[LICENSE](LICENSE). Anyone may use, share and change it, and anyone who
distributes it, or runs a modified version (the sync server included) for
others to use over a network, must offer them its source under the same terms.
The source texts and the databases built from them keep their own licences,
listed under [Attribution](#attribution).
