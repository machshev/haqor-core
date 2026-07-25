# 6. A single curated runtime database

Date: 2026-07-25

## Status

Proposed.

## Context

The app ships four SQLite files (`bible.db`, `sedra.db`, `hebrew.db`,
`lexicon.db`, 87 MiB total) and `haqor_core::bible::Bible::open` attaches all
four to an empty in-memory main schema. That split is right for *generation*:
each file is the cached output of one pipeline stage, so a change to the
morphology generator rebuilds only `hebrew.db` and the fast iteration loops
(`gen-hebrew -n 300`, `review-missing`, `parse-eval --from-db`) stay cheap.

It is wrong for *production*. The shipped files carry the generator's working
state, not the reader's requirements:

- **Redundancy.** `hebrew.db.occurrences` (304,229 rows, 7.8 MiB with its index)
  is exactly `verse_word` projected to `(surface_id, book, chapter, verse)` —
  verified as an empty symmetric difference in both directions.
- **Unused payload.** `oshb_primary.source_word` costs 16 MiB (110k distinct
  pointed forms) and is read at runtime only to derive a word's proclitic
  prefix, of which the whole corpus has **83 distinct shapes**. `surface` ships
  three unused columns, `lexicon.english` (Strong's, 1.5 MiB) and
  `lexicon.roots` are never queried by the runtime, and `sedra.db` ships ~40
  dead columns.
- **Resolution deferred to the reader.** Every word the reader draws re-runs
  `hebrew_word_info`: pick the top verb analysis by `analysis_id`, walk noun
  candidates until a stem folds to a lexicon entry, decide verb-vs-noun, apply
  the token's OSHB analysis over the top, bridge Strong's to a gloss. The
  inputs are all shipped data, so the answer is fixed at build time — but the
  candidate space it searches (`analyses`, 85k rows; `noun_analyses`, 44k rows)
  has to ship so the search can happen.
- **Missing indexes where it counts.** The Syriac NT reader filters
  `sedra.occurrences` by `(book, chapter)` with no index on those columns, so
  each NT chapter render scans 109,654 rows.
- **Update cost.** Every data change means a new release: the installer rewrites
  all four files out of the asset bundle whenever the bundled version differs
  from the installed one, and there is no channel to ship data on its own.

Two things are *not* redundant and must stay. The accented verse text in
`bible.db` cannot be reconstructed from `verse_word` + `surface` (0 of 23,213 OT
verses match: `surface.text` is normalised, with cantillation stripped, maqqef
split and no sof pasuq). The generation DBs themselves stay untouched — they are
the iteration loop.

## Decision

Keep the four generation databases as the pipeline's cache. Add a curation
stage, `haqor db gen-runtime`, that reads all four and emits a single
`haqor.db` — the only file the app ships and the only one `Bible::open` opens.

### The runtime resolves nothing

Word info is not searched for at runtime; it is looked up. `gen-runtime`
enumerates every distinct (surface, token-analysis) pair that actually occurs in
the corpus and stores the finished answer:

- 52,575 surfaces, but only **65,014** distinct (surface, lemma, morph, prefix)
  pairs across all 304,229 tokens, and only **18** tokens corpus-wide have no
  aligned OSHB row.
- So `word_info` is ~65k rows — one per distinct rendering, not one per token —
  and `word` points straight at it. Reading a chapter becomes a covered index
  scan plus a row fetch per word.

This means the resolution logic **moves** out of `haqor-core` rather than being
called from the generator. `hebrew_word_by_surface_id`, `apply_oshb_analysis`,
`oshb_lexeme` and the verb-vs-noun/article-shadow rules become `haqor-db-gen`
code; `haqor-core` keeps only the lookup and the one thing that genuinely cannot
be precomputed — the device-local `lexicon_entries` correction applied on top,
which lives in the writable `progress` attachment. `analyses` and
`noun_analyses` stop shipping altogether; `root_surface` preserves the
root-concordance breadth that legitimately spans all candidates.

Correctness is pinned by a differential test: for every (surface,
token-analysis) pair, the precomputed row must equal what the current live
resolution returns. That test is also the safe deletion order — it runs against
both implementations before the runtime copy is removed.

### Function-based naming

The runtime schema names things by what they *are*, not by where they came
from. `bdb` becomes `lexicon_entry`, `oshb_primary` becomes `token_analysis`,
`sedra.words` becomes `syriac_word` (see the mapping table below). This also
states the intent for the lexicon itself: BDB is an *import source* for the
Haqor lexicon, edited and expanded at build time from study, exactly as BDB was
itself an edited Gesenius. The curated overlay tables are not annotations on a
BDB corpus — they are the lexicon, and BDB is its starting point.

Attribution is unaffected by the renaming — it changes where credit lives, not
whether it is given. OSHB and STEP Bible TAHOT are CC BY 4.0 and SEDRA III asks
for a specific acknowledgement, so credit leaves the `morphology_sources` /
`reader_gloss_sources` tables and lands in a dedicated section at the bottom of
both projects' `README.md` and in a new **About** item in the app menu, which
also shows the app, core and data build versions.

### Blob compression as a build switch

Verse text and lexicon entry bodies are only ever fetched whole, never queried,
so they can ship as compressed per-chapter / per-entry blobs. That trades
`sqlite3`-greppable data for roughly 7 MiB, so it is a `gen-runtime` flag, and
the choice is recorded in `meta` (`blob_codec` = `none` | `zstd`) so the runtime
reads whichever form it is handed. Debug builds default to `none`, production to
`zstd`.

### Two-tier updates: a replaced base and an additive overlay

Data churn is not uniform. Curated lexicon edits, glosses and overrides change
constantly and are kilobytes; the corpus and generated morphology change rarely
and are tens of megabytes. The two tiers have different lifetimes, different
sync rules and different directions of travel.

**`haqor.db` — the base.** Built by `gen-runtime`, versioned by its own build
timestamp in `meta.built` (UTC ISO-8601). Nothing is maintained by hand: the
version is a property of the database, so there is no bump step to forget and no
number that can drift from the data it names. Equality decides whether a client
reinstalls; the format also orders lexicographically, which is what lets the
sync server decide it holds a newer build. When it does, the client downloads
the base whole and swaps the file atomically. There is no row-level base delta;
the base is immutable between builds.

`haqor-core` reports that stamp through `Bible::data_version`, and its own crate
version through `haqor_core::VERSION`, so the app's About view shows what is
actually running rather than what was bundled.

**`overlay.db` — temporary corrections.** A small attached database holding only
rows that shadow the base at query time. The runtime already has this shape —
device-local `lexicon_entries` corrections resolve ahead of the base tables — so
this extends an existing seam with tests already around it. Overlays are
explicitly *temporary*: `haqor-admin` pulls them from the sync server, they are
reviewed, and the next `gen-runtime` folds the accepted ones into the base.

The sync cycle, in order:

1. Client syncs `haqor.db` first, so steps 2 and 3 see the newest base.
2. Client **optimises its local `overlay.db`**, dropping no-op overlays — rows
   whose value the freshly-installed base now already states. This is what
   retires an overlay once it has been folded into a build.
3. Client syncs `overlay.db` contents to the server **additively** — a union,
   not a replace, so two devices' corrections merge rather than clobber.
4. Server drops overlay rows already present in *its* `haqor.db`, cleaning up
   the same absorbed corrections centrally.

Two gaps this leaves, which the implementation has to close:

- **Absorption is the only retirement path.** An overlay that admin reviews and
  *rejects* is never in the base, so no client ever sees it as a no-op and it
  syncs forever. Overlays need a tombstone the server can publish for a
  rejected key, expiring after the next base build.
- **Additive union needs a tiebreak.** Two devices correcting the same surface
  differently both survive step 3. Each overlay row carries `updated_at` and a
  device id, and last-write-wins resolves reads and the merge.

`haqor-sync-server` is the channel for both tiers; it already owns the app's
data connection and holds a canonical copy per user. Its current
bearer-token-over-HTTP model is scoped to a trusted LAN and is **not**
sufficient for distributing base files more widely: a downloaded SQLite file is
an attack surface even when no user text reaches a query, so base payloads need
signature verification before they are opened. That is acceptable for now
because the server runs on the user's own LAN, and it must be revisited before
any hosted deployment.

## Expected outcome

Measured on prototypes built from the current data:

| Stage | Size |
| --- | --- |
| Today, four files | 87.1 MiB |
| Consolidated, pruned, interned | 43.4 MiB |
| + interned morphology cells | 39.9 MiB |
| + precomputed word info, candidates dropped | ~35 MiB |
| + blob compression (`blob_codec=zstd`) | ~28 MiB |

The reader also loses a four-table join per chapter, all per-word analysis
resolution, and the unindexed NT occurrence scan.

## Runtime schema

`ref` is a packed reference, `book << 16 | chapter << 8 | verse`.

```
meta(key, value)                     -- schema_version, built, blob_codec, source hashes
verse(ref PK, words)
word(ref, position, surface_id, info_id, gloss_id)  PK(ref, position) WITHOUT ROWID
word_info(info_id PK, word, root, entry_id, gloss_id, cell_id, flags)
morph_cell(cell_id PK, pos, form, tense, person, gender, number, state, prefix, obj_suffix)
gloss(gloss_id PK, text)             -- interned reader glosses and entry glosses
surface(surface_id PK, text, occurrences, n_candidates, lexical_class, language, info_id)
root(root_id PK, root, gizra, n_forms, n_occurrences)
root_surface(root_id, surface_id)
verse_stat(ref PK, word_count, distinct_count, min_occ, sum_occ, mask)  + verse_stats view
lexicon_entry(entry_id PK, root, word, cons, pos, gloss, body, kind)
entry_index(token_key, strong, entry_id)
word_gloss(surface PK, gloss, note, is_name, reader_override)
surface_override(surface PK, root, gloss)
syriac_root(root_id PK, root)
syriac_lexeme(lexeme_id PK, root_id, lexeme)
syriac_word(word_id PK, lexeme_id, word, vocalised)
syriac_gloss(gloss_id PK, lexeme_id, before, meaning, after)
nt_word(ref, ord, word_id)  PK(ref, ord) WITHOUT ROWID
```

`word_info.flags` packs `vav_con` and `is_name`. `morph_cell` interns the closed
enumerations that `HebrewWord` exposes, so the per-rendering row is a handful of
integers plus a root. `analysis_override` does not appear: primary-analysis
overrides are consumed by `gen-runtime` when it resolves `word_info`, and have
no runtime reader left.

### Name mapping

| Generation DB | Runtime |
| --- | --- |
| `bible.bible` | `verse` |
| `hebrew.verse_word` + `hebrew.oshb_primary` | `word` + `word_info` |
| `hebrew.surface` | `surface` |
| `hebrew.analyses`, `hebrew.noun_analyses`, `hebrew.lexical_analyses` | resolved into `word_info` (+ `root_surface`) |
| `hebrew.occurrences` | dropped (redundant with `word`) |
| `hebrew.roots` | `root` |
| `hebrew.verse_stats` | `verse_stat` (bitmask) + view |
| `hebrew.morphology_sources`, `hebrew.reader_gloss_sources` | dropped (READMEs + About view) |
| `lexicon.bdb` | `lexicon_entry` (`bdb_id` → `entry_id`, `content_json` → `body`, `type` → `kind`) |
| `lexicon.lexical_index` | `entry_index` (`oshb_id` → `token_key`) |
| `lexicon.word_glosses` | `word_gloss` |
| `lexicon.lexicon_overrides` | `surface_override` |
| `lexicon.primary_analysis_overrides` | consumed at build time |
| `lexicon.english`, `lexicon.roots` | dropped (generation only) |
| `sedra.roots` / `lexemes` / `words` / `english` | `syriac_root` / `syriac_lexeme` / `syriac_word` / `syriac_gloss` |
| `sedra.occurrences` | `nt_word` (ref-packed, ordered, no separate index) |

## Consequences

- ~160 schema-qualified references (`hebrewdb.`, `lexdb.`, `sedradb.`,
  `bibledb.`) across `bible.rs` and `tutor.rs` lose their prefixes and pick up
  the new names. `ATTACHED_DBS` collapses to one file; the `progress`
  attachment is unaffected and `overlay` joins it.
- Roughly a thousand lines of resolution logic move from `haqor-core` to
  `haqor-db-gen`. `haqor-core` gets smaller and loses its per-word branching;
  the cost is that a wrong gloss is now diagnosed by rebuilding, not by reading
  the query.
- The in-memory test harness that attaches four `:memory:` schemas, plus
  `tool/sync-dbs.sh`, the installer's `_dbFiles`, and `_dbVersion` all change in
  the app repo.
- `gen-runtime` becomes a mandatory step before `sync-dbs.sh`; a stale
  `haqor.db` is a new failure mode, so `meta` records the source DB hashes and
  the generator refuses to emit from inputs it has already superseded.
- `haqor.db` was the name of the legacy database removed earlier in the
  project's history. Reusing it is deliberate, but git archaeology on that
  filename will surface an unrelated schema.
