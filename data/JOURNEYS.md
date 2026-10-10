# Journeys

`journeys.json` lists journeys the Bible narrates, each as the places it
passes in order, for the app to draw on its map and list with the places.
They are Haqor's own, written from the text; `db gen-names` (or
`db gen-runtime`) builds them into `haqor.db` with the places.

Each journey has a `name`, a `summary` of a sentence or two, and its `stops`:

- `key` is the TIPNR record's key, as in `src_texts/STEPBible-Data/names.jsonl`
  (`Haran@Gen.11.31`), and `name` its name, checked against the record as in
  `place_overrides.json`. The stop is drawn where the place is: OpenBible.info's
  likeliest identification, or Haqor's own (see `PLACE_OVERRIDES.md`).
- `ref` is the verse that takes the journey there, in STEP Bible's book
  abbreviations (`Gen.12.6`, `1Sa.24.1`, `Act.13.4`) and Haqor's numbering,
  which in the Old Testament is the Hebrew Bible's: David reaches En-gedi at
  1 Samuel 24:1, which English Bibles number 23:29.
- `label`, if given, is what the stop is called in place of the record's name:
  `Ptolemais` for Acco, `Mount Hor` for TIPNR's `Hor Mount`.
- `by` is `sea` for a stop reached by ship (or, for the Red Sea, through the
  sea); otherwise it is reached by land. `via` gives points, longitude then
  latitude, that a sea leg passes on its way, so that it is not drawn across
  the land: round Cyprus, along the south coast of Crete.
- `drawn: false` lists a stop without drawing it, for a station Haqor gives no
  site: its line runs from the stop before to the stop after.
- `note` is a short line shown with the stop.

The legs between stops are straight lines, not roads: the map shows the order
of a journey, not its way. A build fails if a stop's key matches no place
record, its name differs, the place has no position, or its `ref` cannot be
read. An Old Testament stop's verse must name the place itself, by TIPNR's tag
on its words, which catches a slip in the numbering; New Testament words are
not tagged, so those stops are checked by hand.

## The exodus

The exodus and the wilderness journeys follow Haqor's own reading of the
crossing (see `PLACE_OVERRIDES.md`): through the Gulf of Aqaba from Nuweiba,
to Mount Sinai at Jabal al-Lawz in Midian. The stations that reading gives no
site for keep OpenBible.info's, which lie on the traditional route through the
Sinai peninsula, across the gulf; drawn, they would send the line back and forth
across it. They are listed but not drawn: Shur, Marah, the wilderness of Sin,
Dophkah and Alush on the way to Sinai, and Kibroth-hattaavah and Hazeroth
after it.

## Left out

- Ezra's return from Babylon: TIPNR's tags are missing from the words of Ezra,
  so its stops could not be checked against the text.
- Numbers 33's stations that no one has placed, and Jesus' ministry, which the
  Gospels do not give in a single order.
