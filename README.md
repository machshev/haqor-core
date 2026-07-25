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
Fetch the pinned TAHOT inputs before regenerating `hebrew.db`:

```sh
./scripts/fetch-stepbible-data.sh
cargo run --release -- db refresh-reader-glosses
```

The source files remain in the ignored `src_texts/STEPBible-Data/` directory;
the fetch script verifies their checksums before the generator consumes them.
Full `gen-hebrew --force` builds also include TAHOT, but the refresh command is
the intended low-resource path when only the interlinear source has changed.

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
app ships is a single curated `haqor.db`, built from all four:

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

**Interlinear translations** — STEP Bible's
[TAHOT dataset](https://github.com/STEPBible/STEPBible-Data), licensed CC BY
4.0. The files are fetched from their canonical repository at pinned checksums
by `scripts/fetch-stepbible-data.sh` rather than redistributed here.

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
