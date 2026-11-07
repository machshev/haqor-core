# The Haqor lexicon

Haqor's own lexicon of scripture's Hebrew and Aramaic: one JSON file per root
spelling, written a root at a time, the most frequent first. The Aramaic of
scripture (Daniel, Ezra, the Peshitta) sits in the same entries as words of the
Hebrew root it answers to. Why it is built this way is in
[ADR 7](../../doc/adr/0007-haqor-lexicon.md); this file is the schema and the
rules for writing an entry.

Further reading:

- [Building an entry, step by step](../../doc/lexicon/building-an-entry.md):
  the process from choosing a root to review, and how evidence is weighed.
- [The origin of the Hebrew alphabet](../../doc/lexicon/alphabet-origin.md):
  how the letters were made (including Douglas Petrovich's proposal), why
  they carry sounds and not meanings, and the older sounds behind merged
  letters.
- [Jeff Benner's letter-meaning approach](../../doc/lexicon/benner-evaluation.md):
  an evaluation: what the lexicon takes from it and what it does not.

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
  passage, and the Peshitta, whose New Testament may be the original of some
  books and is at least a translation from the apostles' own time. Cognates
  in the other Semitic languages come next. The Septuagint
  is a translation and no evidence of the Hebrew's meaning; it is kept only
  as the bridge from the New Testament's Greek back to the Hebrew word.
- **One root, all its words.** Every word is taken to go back to a root,
  even where no one has yet reconstructed how its meaning does. Keep all the
  root's words in its entry, even where a dictionary files them apart, and
  never leave a word out because its link to the root's sense is not yet
  understood: keep it and say in its `summary` that the link is not clear.
  Leave a word out only when the evidence puts it under another root (a loan,
  or cognates pointing elsewhere); proper names have their own entries.
- **One spelling, one entry.** Where cognates show that one Hebrew spelling
  covers two older roots (ערב "evening" behind ġ, "pledge" behind ʿ), the
  entry stays one, and each sense names the cognates that go with it
  (`cognate_ids`), whose `sounds` record the older sound behind the merged
  letter. `db lexicon-splits` lists these across the lexicon.
- **Hebrew and Aramaic together.** Scripture's Aramaic words are words of the
  entry, marked by `lang`, with their own `root` where the spelling differs
  (Aramaic דְּהַב under Hebrew זָהָב). Their meaning is shown from their own
  verses, as the Hebrew's is. Other Aramaic (Targums, inscriptions) is a
  cognate.
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
| `lemma` | The pointed dictionary form (`נֶפֶשׁ`, `אָמַר`), in Hebrew script for the Aramaic too (`אֲמַר`). |
| `lang` | `arc` for Biblical Aramaic, `syc` for the Peshitta's Syriac; left out for Hebrew. |
| `root` | The word's own root key, where its spelling differs from the entry's (`דהב`, `אתר`); left out otherwise. |
| `cognate_ids` | Optional: the `id`s of the entry's cognates that go with this word. Its stems and senses inherit them. |
| `pos` | `verb`, `noun`, `adjective`, `adverb`, `preposition`, `conjunction`, `particle`, `pronoun`, `numeral`, `interjection` or `name`. |
| `gender` | `m`, `f` or `mf`, for nouns where it applies. |
| `gloss` | A few words. |
| `summary` | Optional, a sentence or two where the word differs from the root's summary. |
| `stems` | For a verb: each stem it occurs in (`qal`, `niphal`, `piel`, `pual`, `hiphil`, `hophal`, `hithpael`, or a rarer one such as `polel`), or for an Aramaic verb `peal`, `pael`, `aphel`, `haphel`, `hithpeel`, `hithpaal`, `ethpeel`, `ethpaal` and the rest), each with its own `gloss`, `senses` and optional `cognate_ids`. |
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
| `cognate_ids` | Optional: the entry's cognates that go with this sense, as on a word. |
| `senses` | Optional narrower senses, in the same shape. |

### A cognate

| Field | |
|---|---|
| `id` | Optional, kebab case, unique in the entry (`ar-gharb`), for `cognate_ids` to name. |
| `lang` | `akk` Akkadian, `uga` Ugaritic, `phn` Phoenician, `arc` Aramaic, `syc` Syriac, `ar` Arabic, `gez` Ge'ez, `sem` other Semitic, and `grc` Greek, which is no Semitic language: a Greek word can only be a `loan`. |
| `form` | The word in its own script. |
| `translit` | A transliteration. |
| `gloss` | Its meaning in its own language. |
| `relation` | `cognate`, `possible` (argued but uncertain), or `loan` (borrowed one way or the other). |
| `note` | Optional: how its meaning bears on the Hebrew. |
| `sounds` | Optional: for each letter of the root that stands for more than one older sound, the sound this cognate shows (`{"ע": "ġ"}`; see Sounds). |
| `evidence` | At least one citation of its dictionary entry or the text it occurs in. Scripture's own Aramaic is not a cognate but a word of the entry. |

### Sounds

The 22 letters cover about 29 older consonants, so five letters each stand for
more than one. A cognate in a language that keeps them apart (Arabic,
Ugaritic, South Arabian, Ge'ez, and Aramaic for some) shows which one stands
behind the letter in a given word, and `sounds` records it:

| Letter | Sounds |
|---|---|
| ח | `ḥ`, `ḫ` |
| ע | `ʿ`, `ġ` |
| ש | `š`, `ṯ` |
| ז | `z`, `ḏ` |
| צ | `ṣ`, `ṣ́`, `ṯ̣` |

(Sin, `שׂ`, is its own letter in a root key and stands for one sound.) The
sound is what the cognates show, not a reconstructed root, and says nothing of
which language came first. Two senses showing different sounds behind a letter
are two older roots in one spelling; two senses showing the same sound are
evidence that their meanings belong together. `cargo run -- db
lexicon-splits` lists each entry's sounds by letter, splits first.

### Greek

The Greek words that connect the New Testament to this Hebrew word: `lemma`
(`ψυχή`), `gloss`, and `evidence`: New Testament verses that quote or expound
a passage using the Hebrew word (scripture citations), and the Septuagint's
rendering as a source citation. The Septuagint here is the bridge to the New
Testament's vocabulary, not evidence of the Hebrew's meaning; that rests on the
senses' verses and the cognates.

A Greek word is either kind, as its `relation` says:

- `rendering` (the default): the Greek the Septuagint renders the Hebrew by,
  as above.
- `loan`: the Hebrew or Aramaic word itself, borrowed into Greek (ἀρραβών
  from עֵרָבוֹן, σάκκος from שַׂק, σάββατον from Aramaic שַׁבָּתָא). The
  New Testament's use of a loan is scripture's use of the word, so its
  verses are evidence of the word's sense, and a loan must cite at least one.
  Cite the Greek lexicon's etymology as a source too. A loan's use in
  classical Greek is weaker evidence than a Semitic cognate (a borrowed word
  can drift); record it, if at all, as a `grc` cognate with `relation:
  "loan"`.

Greek is not related to Hebrew by descent, so only a loan a Greek lexicon
derives from the Hebrew or Aramaic counts: never a Greek word that only looks
or sounds like a Hebrew one.

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
   the Peshitta's cognate root, the Greek words Abbott-Smith or Thayer derive
   from the root's words (loans) with their New Testament verses, and the
   other lexicons' text, as leads only.
3. Write `<id>.json` from the brief, following the evidence back to the verses
   and to the cognate languages' own dictionaries.
4. `cargo run -- db check-lexicon` checks it, listing every fault.
5. Leave it `draft` until the user has reviewed it.

## Reviewing in the app

The app's **Lexicon review** page (reader menu, in admin mode) shows each
entry as readers will see it, with every cited verse's Hebrew or Greek and
English a tap away, and can edit its text and mark it reviewed. Run the
desktop app with this directory named:

```sh
HAQOR_LEXICON_DIR=$HOME/base/haqor-core/data/lexicon flutter run -d linux
```

A save is checked first (schema, verses, sources, across the whole lexicon)
and nothing is written if it fails. Files are kept in the layout `jq .`
writes, which is also the layout the page saves in, so a save's diff shows
only what changed.
