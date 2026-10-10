# Events of the Bible

The app's Bible timeline shows the events of Theographic Bible Metadata
(`src_texts/Theographic`, CC BY-SA 4.0), dated from the creation in 4004 BC
after Floyd Nolen Jones's *The Chronology of the Old Testament*, and in Acts
after Todd Atwood. `db gen-names` (or `db gen-runtime`) builds them into
`haqor.db` with the names.

The build:

- writes each year BC negative with no year 0. Theographic counts a year 0
  (its `-4003` is 4004 BC), and gives a few New Testament events a day of the
  Roman calendar, of which only the year is kept.
- orders the events by year, keeping Theographic's order within one.
- re-numbers the verses onto the Hebrew text, as the Treasury of Scripture
  Knowledge's are, and joins verses running on into passages.
- finds each person and place among TIPNR's records: one of its kind, called
  by its name (or a word of it, `Peter` of `Simon Peter`, or another name it
  goes by), that TIPNR first finds in a verse naming it or that Haqor's text
  names there. Where none is found, or two are as likely, it is left off.
- leaves Jesus out of every event before his birth. Theographic has him take
  part in the creation.

Edit `event_overrides.json` to correct Theographic, then run `db gen-names`:

- `events` corrects an event, by Theographic's `id`. `was` is its title as
  Theographic gives it, checked so a changed id cannot correct another event.
  `title`, `year` (BC negative, no year 0), `duration` (`40Y`, `3M`, `1W`,
  `7D`) and `note` (shown with the event) replace Theographic's; `drop: true`
  leaves the event out. `why` is for whoever edits the file.
- `names` gives a person or place, by Theographic's id (`mary_1943`), the
  TIPNR record it is (`key`, as in `src_texts/STEPBible-Data/names.jsonl`), or
  `null` to leave it off every event.

A build fails if an override names no event, its `was` differs, a key matches
no record, or a date, duration or verse cannot be read. Run it with
`RUST_LOG=haqor_db_gen::events=debug` to list the people and places it found
no record for.
