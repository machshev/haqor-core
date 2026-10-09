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

A build fails if an entry's key matches no place record, or its name differs.

## The exodus

The exodus places follow the reading that the Israelites crossed the Gulf of
Aqaba from the Nuweiba beach into Midian, and that Mount Sinai is Jabal al-Lawz
(Ron Wyatt; Lennart Möller, *The Exodus Case*). OpenBible.info follows most
scholarship in placing the crossing by the Nile delta or the Gulf of Suez, and
the mountain in the south of the Sinai peninsula. Haqor departs from it here.
Stations the reading gives no site for (Succoth, Etham, Migdol, Shur, Marah,
the wilderness of Sin, Dophkah, Alush) keep OpenBible.info's.

## The cities of the plain

Sodom, Gomorrah, Admah, Zeboiim and Zoar follow Joel Kramer (Expedition Bible),
who takes the five Early Bronze sites Walter Rast and Thomas Schaub surveyed
south-east of the Dead Sea for the five cities: Bab edh-Dhra, Numeira, Feifa
and Khanazir burned and were abandoned, and es-Safi, Zoar, lived on. Zoar's
entry carries Lot's cave (Genesis 19:30), which has no record of its own, as a
second location labelled as the cave.
