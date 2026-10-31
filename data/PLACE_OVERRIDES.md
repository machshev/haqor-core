# Haqor's own place identifications

Edit `place_overrides.json` to place a TIPNR place where Haqor holds it was,
in place of OpenBible.info's identifications, then run `db gen-names` (or
`db gen-runtime`). Each entry replaces all of the place's locations:

- `key` is the TIPNR record's key, as in `src_texts/STEPBible-Data/names.jsonl`
  (`Sinai@Exo.3.1`), and `name` its name, checked against the record so a
  changed key cannot move another place.
- `note` says whose identification it is and where the position comes from.
  It is for whoever edits the file, not shown in the app.
- `locations` are drawn in order, the first as the likeliest. `kind` uses
  OpenBible.info's words (`mountain`, `spring`, `campsite`, …); `label` names
  the site. They carry no confidence: they are Haqor's choice, not a score.
- `estimated: true` marks a location as Haqor's estimate of a site no one
  has found, placed between the places either side of it on its journey. The
  app draws it faintly and marks it as of low certainty. Say in `note` how
  it was placed.
- `summary`, if given, replaces TIPNR's summary of the record, where it
  is wrong for the place as Haqor places it.

A build fails if an entry's key matches no place record, or its name differs.

## Splits

TIPNR sometimes gives two places one record. `splits` gives a new record for
one of them, taking from the record it splits:

- `from` and `from_name`, the record split, checked as an entry's `key` and
  `name` are;
- `id`, `key` and `name`, the new record's, which no TIPNR record may have:
  ids from 10001 up, keys in TIPNR's form;
- `forms`, the English names (each form's first) of the forms it takes;
- `refs`, the verses whose words of the old record name the new one instead,
  in STEP Bible's references and Haqor's numbering (as `journeys.json`);
- `summary` and `from_summary`, the two records' summaries, in TIPNR's words;
- `openbible`, the OpenBible.info place it is located by, unless an entry in
  `places` places it.

A build fails if a verse of `refs` has no word of the old record.

## The exodus

The exodus places follow the reading that the Israelites crossed the Gulf of
Aqaba from the Nuweiba beach into Midian, and that Mount Sinai is Jabal al-Lawz
(Ron Wyatt; Lennart Möller, *The Exodus Case*). OpenBible.info follows most
scholarship in placing the crossing by the Nile delta or the Gulf of Suez, and
the mountain in the south of the Sinai peninsula. Haqor departs from it here.
Massah and Meribah are at Rephidim (Exodus 17:7), and Mount Paran is taken as
another name for Mount Sinai (Deuteronomy 33:2), as OpenBible.info also does.

The reading gives no site for Marah, the wilderness of Sin, Dophkah and Alush,
or for Taberah, Kibroth-hattaavah and Hazeroth after Sinai. OpenBible.info's
sites for them are on the traditional route through the Sinai peninsula, so
Haqor places them as estimates between the places either side: Marah between
the shore at Baal-zephon and Elim; Sin, Dophkah and Alush evenly from Elim to
Rephidim; Taberah and Kibroth-hattaavah a third, and Hazeroth two thirds, of
the way from Jabal al-Lawz to Kadesh at Petra.

Succoth and Etham keep OpenBible.info's, by the delta, on the way into the
Sinai peninsula. So do Migdol, whose record is also Jeremiah's and Ezekiel's
Egyptian city; and Shur, also the region east of Egypt of Genesis and
1 Samuel.

Kadesh, where Israel came to the border of Edom (Numbers 20:16), is Petra:
the Targums call Kadesh Reqem, which Josephus gives as Petra's old name, and
Eusebius places it by Petra; Wadi Musa's spring is held to be where Moses
struck the rock. Mount Hor, where Aaron died, is Jebel Harun above it, where
his tomb is shown. TIPNR makes Kadesh-barnea the same record, but it stands on
the southern border of the land (Numbers 34:4; Joshua 15:3), so it is split
from Kadesh and keeps OpenBible.info's Ain el-Qudeirat. It takes the verses
naming Kadesh-barnea, and the Negev's Kadesh of Abraham and Hagar (Genesis
16:14; 20:1) and Judah's Kedesh (Joshua 15:23); the others are Kadesh's.

## The cities of the plain

Sodom, Gomorrah, Admah, Zeboiim and Zoar follow Joel Kramer (Expedition Bible),
who takes the five Early Bronze sites Walter Rast and Thomas Schaub surveyed
south-east of the Dead Sea for the five cities: Bab edh-Dhra, Numeira, Feifa
and Khanazir burned and were abandoned, and es-Safi, Zoar, lived on. Zoar's
entry carries Lot's cave (Genesis 19:30), which has no record of its own, as a
second location labelled as the cave.

## Estimated narrative sites

`added` creates a standalone place for an unnamed narrative setting, without
splitting or assigning a word tag in TIPNR. Each has a unique `id` from 10001,
a unique reference-shaped `key`, a nonempty `name` and `summary`. A matching
entry in `places` must give valid coordinates and mark **every** location
`estimated: true`. The generator rejects collisions and unmarked estimates.
These records have origin `Haqor estimate`, no forms and no invented occurrences.
The Gospel settings and the reasoning for their points are documented in
`GOSPEL_JOURNEYS.md` and each record's summary.
