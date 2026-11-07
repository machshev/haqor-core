# Building a lexicon entry, step by step

How a Haqor lexicon entry is written, from choosing the root to marking it
reviewed. The schema is in [the lexicon README](../../data/lexicon/README.md)
and the decisions behind it in [ADR 7](../adr/0007-haqor-lexicon.md). The two
background pages, [the origin of the Hebrew alphabet](alphabet-origin.md) and
[an evaluation of Jeff Benner's letter-meaning approach](benner-evaluation.md),
explain why the method rests on scripture's use and on sound correspondences,
not on the shapes of the letters.

## What the lexicon assumes

- **Scripture's Hebrew has a systematic root system** that is consistent
  within the text. The project's morphology generator, which builds verb
  forms from a three-letter root by rule, reproduces about 98% of the Open
  Scriptures Hebrew Bible's verb analyses: a rule-governed system behaves
  like that.
- **Scripture's Aramaic records the same roots** in its speakers' dialect:
  Daniel, Ezra, Jeremiah 10:11, Genesis 31:47, and the Peshitta, whose New
  Testament may be the original of some books and is at least a translation
  from the apostles' own time.
- **Every word goes back to a root**, even where no one has yet worked out how
  its meaning does.

## How evidence is weighed

1. **Scripture's own use** decides a word's meaning: the verses whose use
   fixes the sense, its parallels and opposites, the New Testament where it
   quotes or expounds the passage, the Peshitta's use of the word, and the New
   Testament's use of a Hebrew word borrowed into Greek (ἀρραβών, σάββατον).
2. **Cognates** in the other Semitic languages come next, each cited from its
   own language's dictionary or texts.
3. **A loan's use in classical Greek** is weaker than a Semitic cognate: a
   borrowed word can drift.
4. **The Septuagint** is a translation, no evidence of the Hebrew's meaning.
   It is kept only as the bridge from the New Testament's Greek back to the
   Hebrew word.

Other lexicons (BDB, Gesenius, Klein, Jastrow, Payne Smith, Thayer…) are
never cited. They are leads: follow each claim back to the verse, the
cognate's own dictionary or the inscription, and cite that.

## Before you start

The tools read the runtime database and the vendored sources:

- `data/haqor.db`, built by the generator.
- `src_texts/`, filled by the `scripts/fetch-*.sh` scripts: Lane's Arabic
  lexicon, Wiktionary's Semitic data, Oracc's Akkadian and Sumerian
  glossaries, the Proto-Semitic root list, the Egyptian and Coptic lexicons,
  MACULA's Septuagint alignment, and STEPBible's Greek lexicons. A brief
  section whose source is missing says so and names the script.

## 1. Choose the root

```sh
cargo run -- db lexicon-rank -n 50
```

lists the Old Testament's roots by how many tokens they have, with the id of
any entry already written. Take the most frequent root with no entry. Proper
names are left out of the ranking, since names get entries of their own.

## 2. Gather the evidence

```sh
cargo run -- db lexicon-brief <root> > brief.md
```

The root is a root key: bare consonants, no final forms, sin written `שׂ` and
shin a bare `ש` (`נפש`, `מלכ`, `עשׂה`). Use `-e N` for more example verses per
word and stem, and `--min-score` to list weaker New Testament quotations. The
brief's sections:

| Section | What it gives |
|---|---|
| **Words** | Every word BDB files under the root, with part of speech, gloss and token count. |
| **Each word** | Its tokens by stem, how the English renders them, examples spread across the canon, and tokens filed here by spelling whose own analysis is another root's verb. |
| **New Testament quotations** | New Testament verses quoting or alluding to verses that use the root, aligned through the Peshitta. |
| **Peshitta cognate root** | The root's Syriac words in the Peshitta. |
| **Cognates** | Candidates from Lane, Wiktionary, Oracc and the Proto-Semitic list: stated links, matching root sets, and sound correspondences ranked by gloss overlap. |
| **Greek loans** | New Testament Greek words that Abbott-Smith or Thayer derive from the root's words, with their verses. |
| **Septuagint renderings** | The Greek words the Septuagint uses for each word, with their New Testament counts: the bridge, not evidence. |
| **Other lexicons** | BDB, Klein, Jastrow and the rest, as leads only. |

## 3. Decide which words belong

- **One spelling, one entry.** Keep every word of the root together, even
  where a dictionary files them in separate sections.
- **Keep words whose link is unclear**, and say in the word's `summary` that
  the link to the root's sense is not yet understood, with whatever evidence
  there is.
- **Leave a word out** only when the evidence puts it under another root: a
  loan from another language (Assyria, from Akkadian Aššur), or cognates that
  point elsewhere. Say why in the entry's `note`.
- **Leave proper names out.** They get their own entries, which link back to
  the root.
- **Add scripture's Aramaic words** of the root as words of the entry, with
  `lang` (`arc` Biblical Aramaic, `syc` the Peshitta) and their own `root`
  where the spelling differs (Aramaic דְּהַב under Hebrew זָהָב, אֲתַר under
  אשר).
- **Check the brief's "filed here by spelling" tokens**: the brief groups
  tokens by spelling, so some belong to another root's verb.

## 4. Write the senses from scripture

For each word (each stem, for a verb):

1. Read the examples and the full reference list. Group the uses into senses.
2. For each sense, find the verses whose use **fixes** it: a definition in
   the text, a parallel, an opposite, an unambiguous context. Mark those
   `"role": "defining"`.
3. Write the `definition` from what those verses show, in plain words.
4. Add parallels, opposites and fixed phrases under `usage`, each with its
   verses.
5. Where the New Testament quotes or expounds a verse using the word, or the
   Peshitta uses the root, add those verses to the sense.

A sense cites scripture only; the validator rejects anything else.

## 5. Follow the cognates

1. Take each candidate from the brief's Cognates section and look it up in its
   own language's dictionary (vendored in `src_texts` where its licence
   allows, otherwise by page in a scan; `data/lexicon/sources.json` lists the
   works and where they are).
2. Record it with its form in its own script, a transliteration, its gloss,
   and `relation`: `cognate`, `possible` (argued but uncertain) or `loan`.
3. **Merged letters.** If the root has ח, ע, ש, ז or צ, check which older
   sound the cognate shows behind it (Arabic, Ugaritic and Ge'ez keep most of
   them apart) and record it in `sounds`: `{"ע": "ġ"}`.
4. Give the cognate an `id` and name it in `cognate_ids` on the words, stems
   or senses it goes with.
   - Two senses showing **different** sounds behind one letter are two older
     roots in one spelling (ערב "evening", ġ; "pledge", ʿ). Keep them in the
     one entry; the tags make the split visible.
   - Two senses showing the **same** sound are evidence that their meanings
     belong together (זכר "remember" and "male", both ḏkr).
5. **Name a cognate only where it shares the meaning.** Naming it in
   `cognate_ids` says the word or sense shares its meaning; leaving it out says
   it does not, and `db lexicon-support` counts it against that word, twice
   over. A cognate that means something else entirely is the strongest
   evidence the word may belong to another root, or have moved far from it.
6. **Kin.** Where a sense shares its meaning with a sibling word's sense (the
   noun אֶשֶׁר "happy is…!" and the verb's "call happy"), give the sibling
   sense an `id` and name it in `kin`. The sibling's uses and cognates then
   count for it, and its cognates no longer count against it.

## 6. Record the Greek

- **Renderings** (`relation` left out): the Greek the Septuagint renders the
  word by, with the New Testament verses that use it. The bridge only.
- **Loans** (`"relation": "loan"`): the Hebrew or Aramaic word borrowed into
  Greek, from the brief's Greek loans section once its etymology checks out.
  Cite the New Testament verses that use it (required) and the Greek lexicon's
  etymology. Their use counts toward the word's sense.

Never link a Greek word that only looks or sounds like the Hebrew one.

## 7. Write the doctrine, if any

Where the word's use bears on doctrine, write each point under a kebab case
`topic`, **evidence first**: what the verses show in their own terms, then
what follows from them. Scripture citations only. Use the text's Hebrew names
(יַהְוֶה, its own month and feast names), with any translation second.

## 8. Distil

Write last what is read first:

- `summary` for the root: two to four sentences on its core meaning and how
  scripture uses it.
- `gloss`: a few words, as the word sheet and the tutor will show it.

Someone who reads only these two should come away with the right idea.

## 9. Leave notes for later

The entry's `note` is for editors, never shown to readers. Record what was
left out and why, sources not yet consulted, leads not yet followed to verses
(for example a lexicon's senses for a Peshitta word), and open questions.

## 10. Check it

```sh
cargo run -- db check-lexicon     # schema, verses, sources, across the lexicon
cargo run -- db lexicon-splits    # each root's sounds behind merged letters
cargo run -- db lexicon-support   # each word's uses, verses, cognates for and against, kin
```

`check-lexicon` lists every fault at once. `lexicon-support` counts each word's
evidence: give each word its `bdb` keys so its uses in scripture are counted.
A word with few uses, few defining verses, or a negative balance (cognates of
another meaning outnumbering those sharing it, which count double) is one
whose meaning rests on little, and its `summary` should say so. `lexicon-splits` shows whether the
new entry adds to a pattern across roots: the same merger explaining two
senses again and again.

## 11. Review it

The entry stays `"status": "draft"` until the user has reviewed it. Run the
desktop app against the lexicon directory:

```sh
HAQOR_LEXICON_DIR=$HOME/base/haqor-core/data/lexicon flutter run -d linux
```

and open **Lexicon review** from the reader menu (admin mode). Every cited
verse opens with its Hebrew or Greek, the Peshitta for New Testament verses,
and the English. Texts can be edited in place. A save checks the whole lexicon
first and writes nothing if it fails. The reviewer marks the entry reviewed.

## 12. Commit

Files are kept in the layout `jq .` writes (the review page saves the same
way), so a diff shows only what changed. Commit each entry as it is drafted,
and again when it is reviewed.
