# 4. Object-suffix handling: generation vs. peeling

Date: 2026-06-18

## Status

Proposed

## Context

Verb forms carry pronominal **object suffixes** (`יִשְׁמְרֵהוּ` "he keeps him",
`קָחֶנּוּ` "take it", `תְּבָרֲכַנִּי` "she blesses me"). These dominate the
remaining unparsed tail: of ~918 in-memory-unparsed gold verb tokens, the large
majority are object-suffixed forms on weak/theme hosts the generator does not
currently produce.

### Current architecture: generate-and-index

`generate_paradigm` *expands* every host into its full set of suffixed surfaces
(`object_suffixed` in `verb.rs`, built by `imperfect_object_suffixes`,
`qal_perfect_object_suffixes`, `derived_perfect_object_suffixes`,
`hiphil_perfect_object_suffixes`, `lamed_he_imperfect_object_suffixes`,
`imperfect_vocalic_object_suffixes`). Each suffixed surface is indexed in the
`ReverseIndex` with its `object_suffix: Option<Pgn>`. Parsing is then a hash
lookup — no peeling.

This is correct but **multiplies the index**: every host grade × ~15 suffix
endings × 22³ roots. The index is already ~54M entries; broadening obj-suffix
coverage to the missing host grades has repeatedly pushed the in-memory eval to
~8 min / ~23 GB and been reverted (see grind notes). Both the eval **and**
`gen-hebrew` build this index, so the cost lands everywhere.

The missing tokens are host-grade gaps: the suffix machinery composes with the
*primary* host but not with the theme-restored / guttural / hollow / weak
twins (e.g. the I-guttural Hophal host `תָעָבְדֵם` had to be added as a narrow
post-pass). There is no single missing rule — it is a long tail of host grades.

## Decision (options under consideration — not yet chosen)

### Option A — keep generating, add host grades incrementally
Add narrow, gated obj-suffix host compositions one grade at a time (as done for
I-guttural Hophal: +4 tokens, no blowup). Each is a post-pass over finished
hosts feeding the existing suffix builders.
- **Pro:** safe, proven, no architecture change, monotonic recall.
- **Con:** slow (~2–6 tokens each, hundreds to go); each needs a ~10-min
  in-memory eval to confirm it didn't inflate the index; never reaches 0.

### Option B — suffix-peeling at parse time
Mirror the existing **proclitic peeling** (`peeling_targets`): strip a
recognised pronominal suffix, then match the host.
- **Blocker:** unlike a proclitic, the suffix *reshapes the stem* — `יִשְׁמֹר`
  → `יִשְׁמְרֵהוּ` reduces the theme to sheva and adds a linking vowel. Peeling
  `-ēhû` yields `יִשְׁמְר-`, which is not a standalone form; reconstructing the
  host grade (`יִשְׁמֹר`) is a **many-to-one de-reduction** (a sheva can come
  from holam/patah/tsere themes), so naive peeling is ambiguous.
- **Pro:** removes the host×suffix cross-product from the index entirely.
- **Con:** large rewrite; the ~6 host builders encode shape-specific reductions
  that would all have to be inverted; energic (`-ennû`/`-annî`), `-mô`, and
  paragogic interactions add cases.

### Option C — host sub-index + peel (recommended to prototype)
Hybrid: index the **host link-stems once** (not host × suffix). Store, per host
grade, the suffix-linking stem (`יִשְׁמְר-`, `יִשְׁמָע-`, `יְבָרֲכ-`) keyed by
its consonant+vowel skeleton. At parse time, peel one of the ~15 known suffix
endings and look the remaining stem up in this sub-index; the matched stem
carries the host's `(binyan, form, pgn)` and the peeled ending gives the
`object_suffix`.
- **Pro:** index holds *hosts*, not host×suffix → ~10–15× fewer obj-suffix
  entries; de-reduction ambiguity is avoided (we match the *already-reduced*
  link-stem the builders produce, we don't invert it); reuses the existing
  builders to enumerate link-stems.
- **Con:** a second index structure + parse path; must compose with proclitic
  peeling (a form can have both, `וַיְבָרֲכֵהוּ`); careful dedup vs. the
  primary index.

## Consequences

- **Recommendation:** pursue **Option A now** (safe incremental tokens) while
  **prototyping Option C** (the real fix) behind measurement — build the host
  link-stem sub-index for one binyan/form, confirm it matches the generated
  suffixed surfaces 1:1 and shrinks the index, then widen.
- "0% missing" on this bucket is reachable only via B/C; Option A alone
  asymptotes short of it.
- Whichever path: the accuracy harness measures the *generator*, so verify
  obj-suffix coverage against the **in-memory** eval (and product coverage via
  `gen-hebrew` db queries), not the from-db proxy.
