# The Haqor lexicon

Haqor's own Hebrew lexicon: one JSON file per root, written a root at a time,
the most frequent first. Why it is built this way is in
[ADR 7](../../doc/adr/0007-haqor-lexicon.md); this file is the schema and the
rules for writing an entry.

## Principles

- **The entry stands on its own.** Other lexicons are where to look for
  evidence, never what an entry cites. Follow a lexicon's conclusion back to
  its evidence (a verse, a cognate in its own language's dictionary, an
  inscription) and cite that. Never write "BDB
  gives…", and never compare lexicons.
- **Every claim cites its evidence.** A sense cites the verses that show it, a
  cognate cites its own language's dictionary or text, and a doctrinal
  statement cites scripture only.
- **Weighing evidence.** Scripture's own use of the Hebrew decides its
  meaning, including the New Testament where it quotes or expounds the
  passage. Cognates in the other Semitic languages come next. The Septuagint
  is a translation and no evidence of the Hebrew's meaning; it is kept only
  as the bridge from the New Testament's Greek back to the Hebrew word.
- **Distilled first.** `gloss`, then `summary`, then the detail that backs them
  up. Someone who reads only the first two should come away with the right
  idea.
- **Scripture interprets scripture.** Meaning comes from how the text uses the
  word: where its use fixes the sense, and its parallels and opposites. Doctrine rests on scripture
  alone, never on tradition or commentators, and is written evidence-first:
  what the verses show, then what follows from them.
- **Hebrew names.** Use the text's Hebrew (יַהְוֶה, and its own names for months
  and feasts), with any translation second.

## Files

- `<id>.json`: one root, named by its `id`.
- `sources.json`: every external work an entry cites (below).

## An entry

| Field | |
|---|---|
| `id` | Kebab case ASCII, Haqor's own, never reused (`nephesh`, `amar`). The file is `<id>.json`. |
| `root` | The root as the database keys roots, so entries join to the words: bare consonants with no final forms, sin written `שׂ` and shin a bare `ש` (`נפש`, `מלכ`, `עשׂה`). A noun with no verbal root gives its own consonants. |
| `homograph` | 1, 2… where two unrelated roots share consonants; left out otherwise. |
| `status` | `draft` until the user has reviewed it, then `reviewed`. |
| `updated` | The date of the last substantive change, `YYYY-MM-DD`. |
| `tags` | Kebab case topics to filter by (`life`, `death`, `body`, `speech`). |
| `gloss` | A few words: the root's core meaning, as the word sheet shows it. |
| `summary` | Two to four sentences: the core meaning and how scripture uses it. |
| `words` | The words formed from the root (below), commonest first. |
| `cognates` | The root in the other Semitic languages (below). |
| `doctrine` | What the word's use teaches (below). May be empty. |
| `note` | For whoever edits the file: open questions, things to check. Not shown in the app. |

### A word

| Field | |
|---|---|
| `lemma` | The pointed dictionary form (`נֶפֶשׁ`, `אָמַר`). |
| `pos` | `verb`, `noun`, `adjective`, `adverb`, `preposition`, `conjunction`, `particle`, `pronoun`, `numeral`, `interjection` or `name`. |
| `gender` | `m`, `f` or `mf`, for nouns where it applies. |
| `gloss` | A few words. |
| `summary` | Optional, a sentence or two where the word differs from the root's summary. |
| `stems` | For a verb: each stem it occurs in (`qal`, `niphal`, `piel`, `pual`, `hiphil`, `hophal`, `hithpael`, or a rarer one such as `polel`), each with its own `gloss` and `senses`. |
| `senses` | For anything else: its senses (below). |
| `greek` | Greek words that render it (below). |
| `usage` | `parallels`, `opposites` and `phrases` (below). |

A verb has `stems` and no `senses`; any other word has `senses` and no `stems`.

### A sense

| Field | |
|---|---|
| `gloss` | A few words. |
| `definition` | A sentence or two, drawn from the evidence. |
| `evidence` | At least one scripture citation. Mark the verses whose use fixes the sense `"role": "defining"`. |
| `tags` | Optional. |
| `senses` | Optional narrower senses, in the same shape. |

### A cognate

| Field | |
|---|---|
| `lang` | `akk` Akkadian, `uga` Ugaritic, `phn` Phoenician, `arc` Aramaic, `syc` Syriac, `ar` Arabic, `gez` Ge'ez, `sem` other Semitic. |
| `form` | The word in its own script. |
| `translit` | A transliteration. |
| `gloss` | Its meaning in its own language. |
| `relation` | `cognate`, `possible` (argued but uncertain), or `loan` (borrowed one way or the other). |
| `note` | Optional: how its meaning bears on the Hebrew. |
| `evidence` | At least one source citation: its dictionary entry, or the text it occurs in. |

### Greek

The Greek words that connect the New Testament to this Hebrew word: `lemma`
(`ψυχή`), `gloss`, and `evidence`: New Testament verses that quote or expound
a passage using the Hebrew word (scripture citations), and the Septuagint's
rendering as a source citation. The Septuagint here is the bridge to the New
Testament's vocabulary, not evidence of the Hebrew's meaning; that rests on the
senses' verses and the cognates.

### Usage

- `parallels`: words set beside it in parallelism, each `lemma`, `note`,
  `evidence`.
- `opposites`: words set against it, the same shape.
- `phrases`: fixed expressions it forms, each `phrase` (pointed Hebrew),
  `gloss`, `evidence`.

### Doctrine

Each item has a kebab case `topic` (`mortality`, `resurrection`,
`kingdom-of-god`), a `statement`, and `evidence`, which must be scripture
citations only.

The statement is written evidence-first: what the verses show, in their own
terms, and then what follows from them. It is not a creed with proof texts
attached; a reader should be able to check each step against the verses cited.

## Citations

An `evidence` list holds citations of two kinds.

Scripture:

```json
{ "ref": "Gen.2.7", "role": "defining", "note": "Man becomes a living נֶפֶשׁ when God breathes into him." }
```

- `ref` is the book abbreviation, chapter and verse as elsewhere in `data`
  (`Gen`, `1Sa`, `Mat`). Add `to` for a range (`"to": "Gen.2.9"`).
- `role` is `defining` or `example` (the default).
- `note`: what the verse shows. Optional, but say why when it is not obvious.

A source in `sources.json`:

```json
{ "source": "lane", "locator": "Book I, p. 2826, art. نفس", "note": "breath; soul; self" }
```

- `locator`: where in the work: entry, page, column, line or text number.
- `url`: optional, a direct link where the work has one per entry.

## Sources

`sources.json` lists the works citations name, each with:

- `id`: kebab case, used by citations.
- `citation`: the full bibliographic citation.
- `lang`: the language it is a dictionary or text of.
- `licence`: as the source states it.
- `url`: where it is published.
- `local`: its path under `src_texts`, where a copy is kept; empty while not
  yet vendored.

## Writing an entry

1. `cargo run -- db lexicon-rank -n 50` lists the roots by how often they
   occur in the Old Testament, with the entry each already has. Take the next
   one without an entry.
2. `cargo run -- db lexicon-brief <root> > brief.md` gathers the evidence: the
   words BDB files under the root, their tokens by stem with how the English
   renders each and examples spread across the canon, every reference, the New
   Testament's quotations of verses using the root (`--min-score` to widen),
   the Peshitta's cognate root, and the other lexicons' text, as leads only.
3. Write `<id>.json` from the brief, following the evidence back to the verses
   and to the cognate languages' own dictionaries.
4. `cargo run -- db check-lexicon` checks it, listing every fault.
5. Leave it `draft` until the user has reviewed it.
