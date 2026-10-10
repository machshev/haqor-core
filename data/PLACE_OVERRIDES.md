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

A build fails if an entry's key matches no place record, or its name differs.

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
the way from Jabal al-Lawz to Kadesh.

Succoth and Etham keep OpenBible.info's, by the delta, on the way into the
Sinai peninsula. So do Migdol, whose record is also Jeremiah's and Ezekiel's
Egyptian city; Shur, also the region east of Egypt of Genesis and 1 Samuel;
and Kadesh, whose record carries the southern border of the land as well.

## The cities of the plain

Sodom, Gomorrah, Admah, Zeboiim and Zoar follow Joel Kramer (Expedition Bible),
who takes the five Early Bronze sites Walter Rast and Thomas Schaub surveyed
south-east of the Dead Sea for the five cities: Bab edh-Dhra, Numeira, Feifa
and Khanazir burned and were abandoned, and es-Safi, Zoar, lived on. Zoar's
entry carries Lot's cave (Genesis 19:30), which has no record of its own, as a
second location labelled as the cave.
