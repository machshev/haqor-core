# Prophecies

`prophecies.json` lists prophecies of the Old Testament, concerning Christ and
in general, with the passages that fulfil them where the Bible records any.
They are Haqor's own, written from the text; `db gen-prophecy` (or
`db gen-runtime`) builds them into `haqor.db`, together with links it finds
from the New Testament's citation formulas.

## References

Passages are written in STEP Bible's book abbreviations (`Gen.3.15`,
`1Sa.24.1`, `Act.13.4`) **in the English Bible's numbering**, as a reader
looking them up in the King James Version finds them. The build renumbers OT
verses onto the Hebrew text through STEP Bible's TAHOT data, as it does the
Treasury of Scripture Knowledge: Psalm 22:18 is stored as 22:19, Micah 5:2 as
5:1, Malachi 4:5 as 3:23. A passage may be a verse (`Mic.5.2`), a run within a
chapter (`Isa.42.1-4`) or across chapters (`Isa.52.13-53.12`).

## Fields

`themes` maps each theme to the OT passages it covers. A curated prophecy names
its own theme; the map gives one to a prophecy the build finds on its own.

Each entry of `prophecies` has:

- `ot`, the prophecy's passage, and `title`, a line naming it, in the King
  James Version's words where they serve.
- `theme`, if it has one: `seed-of-woman`, `abrahamic`, `davidic`, `servant`,
  `kingdom` and `priest` for the promises concerning Christ; `israel`,
  `nations` and `empires` for prophecy in general.
- `messianic`: whether it concerns Christ.
- `horizon`, if known: `first-advent`, `second-advent`, `both`, or `history`
  for a prophecy fulfilled in the history of Israel or the nations.
- `note`, a sentence or two shown with it.
- `fulfilments`, which may be empty or absent: each with `ref`, the fulfilling
  passage (NT or OT); `ot`, the verses of the prophecy it takes up, if not all
  of them; `kind`, one of `quotation`, `allusion`, `type` (a person or event
  that prefigures, like the passover lamb) or `event` (a narrative of the
  thing happening, like Ezra 1 for Cyrus); and `note`.

`rejected` lists links the build found that are wrong, as `ref` (the NT verse)
and `ot`, with `why`.

## Found links

The build looks in the Greek New Testament for citation formulas: "that it
might be fulfilled" (a passive of πληρόω, τελέω or τελειόω, with the scripture,
a prophet, or what was spoken or written), "that which was spoken" (τὸ ῥηθέν),
"it is written" (γέγραπται) and "the scripture" (γραφή). It links each to the
OT verse it quotes by the `quotation` table's root alignment and the TSK's
references in the formula's verse and the two after it, preferring the book of
a prophet the formula names.

A found link is used only where curation does not already speak:

- Inside a curated prophecy, it confirms the curated fulfilment it overlaps
  (recording the formula) or is added as an unreviewed one.
- If its NT passage is curated as fulfilling another prophecy, it is dropped:
  the curation says which prophecy the passage quotes.
- Outside every curated prophecy, a strong link after a fulfilment formula
  starts an untitled prophecy of its own, for review.

To review, rebuild with `cargo run --release -- db gen-prophecy`, which logs how
many links confirmed, were added or started prophecies, and look at the rows
with `reviewed = 0` or `curated = 0`. Curate a right one (give it a title and
add its fulfilment), or reject a wrong one.

A build fails, naming every fault, on a passage that cannot be read or is
outside the corpus, an unknown theme, horizon or kind, or a fulfilment whose
`ot` lies outside its prophecy.
