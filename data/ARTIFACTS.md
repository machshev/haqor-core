# Archaeological finds

`artifacts.json` lists finds that bear on the Bible: inscriptions naming its
people and places, sites it describes, early copies of its text, and ancient
writings outside it that name them (Josephus, Tacitus, Herodotus). Each find
says what it attests and how strongly, and cites where the identification
comes from. It builds into the runtime database (`db gen-artifacts`).

A find corroborates a person, a place or an event. It does not prove the Bible
as a whole, and finds are not all equally certain, so every link carries its
own strength rather than the find carrying one.

## Sources

`sources` lists the works links cite, by `id`:

- `mykytiuk-2014` and `mykytiuk-2017`: Lawrence Mykytiuk's lists, in Biblical
  Archaeology Review, of Bible people identified in inscriptions of known
  authenticity. He counts a person only when the inscription matches the
  Bible's in name and in at least one of father's name and title, and in time.
  The file records the identifications he makes, in Haqor's own words, not his
  text.
- `wikipedia`: the find's Wikipedia article, for facts Mykytiuk does not cover
  (most passages, and finds that name no one).

## A find

- `id`: Haqor's own, kebab case, never reused.
- `name`, and `other_names` it is known by.
- `kind`: stele, inscription, relief, prism, cylinder, tablet, seal, bulla,
  coin, ostracon, papyrus, amulet, ossuary, manuscript, temple, site, or
  text: an ancient writing known from its copies rather than found, which
  has no `found`.
- `date` as a reader would write it, and `year` (negative for BC) to sort by.
- `script`: the language or script, and what it is written on where that is
  notable.
- `found`: `site`, `year` of discovery, and `key`, the TIPNR place record of
  the site where there is one.
- `museum`: where it is now, or "In place". Left out where not confirmed.
- `wikipedia` (the article title) and `wikidata`, where the find has them.
- `summary`: what it is and what it says, a sentence or two, for the app.
- `attests`: the people and places it names (below).
- `passages`: the passages it bears on (below).
- `note`: for whoever edits the file, not shown in the app. Notes flag
  what a reviewer should check.

## Links

Each of `attests` and `passages` has:

- `strength`:
  - `strong`: the find names the person in a way that can only be the Bible's
    (Mykytiuk's test), or matches the passage directly.
  - `reasonable`: likely, but the reading is restored or damaged, the
    identification is debated, or the match is to the circumstances rather
    than the words.
  - `possible`: argued for by some, doubted by many.
- `how`: in a sentence, what the find says and how it matches.
- `source`: the `sources` id it comes from.

An attestation has `key` and `name`, the TIPNR person or place record as in
`src_texts/STEPBible-Data/names.jsonl`, checked against each other so a changed
key cannot move a link. A passage has `from` and, for more than one verse,
`to`, in STEP Bible's references and Haqor's (Hebrew) numbering, as
`journeys.json`: Daniel 4:30 in English Bibles is `Dan.4.27`.

## In a chapter

A chapter's finds are those with a passage in it and those attesting a person
it names. The first are about the chapter; the second about someone in it, so
the app should keep them apart. A famous name reaches far: David is named in
256 chapters, and the Tel Dan Stele would be in all of them.

## Known gaps

- TIPNR files Sargon under Sennacherib's record, so Sargon II's annals link by
  passage only.
- TIPNR has no record for Israel as a people, so the Merneptah Stele links
  only to Ashkelon and Gezer, which it also names.

## Places

A find naming a place shows on the place's page with those found there, but
not in each chapter naming the place: Jerusalem is named in some 700
chapters. A find bears on a chapter through a place only by a passage.
