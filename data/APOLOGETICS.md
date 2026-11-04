# Apologetics

`apologetics.json` holds Haqor's own apologetics: the evidence for the
Bible and the resurrection, the Bible's teaching on the subjects it is most
often misread on, and answers on the passages most often brought against
that teaching. It serves two uses: a quick reference on a subject, for
use in a conversation, and notes on the passages being read, shown in the
reader beside the verses they concern.

## Reading

The file gives one reasoned reading, not a survey of views, written from
the text: God is one, Jesus is his Son, man is mortal, death is a sleep
ended by resurrection, the adversary is not a fallen angel, and the
kingdom will be on the earth. It is candid where a favourite argument
is weak: an entry says so and rests on the stronger ones.

It follows Haqor's renderings: Yahweh, the grave (Sheol and Hades), the
valley of Hinnom (Gehenna), adversary and false accuser, living soul,
ecclesia.

## Sources

Arguments are written in Haqor's own words and rest on Scripture: its
passages are their references. Other sources are cited only for what
Scripture cannot show, such as historical records and archaeology
(Tacitus on Christus, Josephus on James). The `works` map names each
such work once, with its `licence`:

- `public-domain`: may be quoted in a source's `quote`.
- `copyright`: cited by `loc` only.

Public-domain apologetics kept for reference, cited or not, are in
`src_texts/Apologetics` (see its `README.md`).

## References

Passages are written in STEP Bible's book abbreviations (`Gen.3.15`,
`Jhn.8.58`, `1Co.15.3-8`) in the English Bible's numbering, as in
`prophecies.json`: a verse, a run within a chapter, or a run across
chapters (`Isa.52.13-53.12`).

## Fields

`subjects` lists the subjects in the order a reader is shown them, each
with an `id`, a `title`, and a `note` of a sentence.

`works` maps each work's id to its `title`, `author`, `year` where known,
its `licence`, a `url` where it can be read, and the `file` in
`src_texts` that holds it.

Each entry of `entries` has:

- `id`, unique, in kebab case.
- `subject`, one of the subjects' ids.
- `kind`: `evidence` (for the Bible's truth: the resurrection, the
  records, fulfilled prophecy), `doctrine` (what the Bible teaches on a
  subject, from its plain passages) or `passage` (an answer on a passage
  brought against the doctrine).
- `title`, a line naming it: the claim for `evidence` and `doctrine`,
  the passage and its common use for `passage`.
- `summary`, the answer in a few sentences, for a conversation.
- `passages`, the verses the reader marks with the entry. For a
  `passage` entry, the passage it answers on; for the others, the key
  texts.
- `points`, the argument in steps, each a `claim` with the `passages`
  that show it and the `sources` that make it, each a `work` (an id in
  `works`), a `loc` where in it, and for a public-domain work a `quote`.
- `objections`, if any: each an `objection` and its `answer`, with
  `passages`.
- `related`, other entries' ids.

## Building

`db gen-apologetics` (or `db gen-runtime`) builds the file into
`haqor.db`: `apologetic_subject`, `apologetic` (each entry, its points,
objections and related entries as JSON in `detail`) and
`apologetic_passage`, the passages the reader marks, renumbered onto the
Hebrew text as `prophecies.json`'s are. After an edit, rebuild with
`cargo run --release -- db gen-apologetics` and sync the database to the
app with its `tool/sync-dbs.sh`.

The reader marks a verse where one of an entry's `passages` starts (or
the first verse of a chapter one runs into), and lists the entries with
a passage in a chapter in its chapter context.

A build fails, naming every fault, on a passage that cannot be read or
is outside the corpus, an entry with no passages, an unknown subject,
kind, work, licence or related entry, a repeated id, or a `quote` from a
work under copyright.
