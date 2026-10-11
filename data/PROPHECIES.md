# Prophecies

`prophecies.json` lists prophecies of the Old Testament, concerning Christ and
in general, with the passages that fulfil them where the Bible records any.
They are Haqor's own, written from the text; `db gen-prophecy` (or
`db gen-runtime`) builds them into `haqor.db`, together with links it finds
from the New Testament's citation formulas.

## Reading

The file gives one reasoned reading, not a survey of views. A prophecy speaks
first to its hearers in their own day and echoes forward: Isaiah 7:14 is a sign
to Ahaz before it is fulfilled in Jesus. The Olivet discourse (Matthew 24,
Mark 13, Luke 21), and much of Christ's teaching on judgment, concerns AD 70;
Revelation concerns his return. Jerusalem is overrun from the north three
times — Babylon, Rome, and Gog (Russia) at Christ's return — and after the
thousand years a final Gog rises and is destroyed, and Christ delivers up the
kingdom to the Father (1 Corinthians 15:24-28).

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
- `stages`, the times it is fulfilled that its fulfilments do not show: where
  the Bible records no fulfilment (Daniel 2:44 at the return), or none at that
  time. A prophecy's stages are these and its fulfilments'. The stages, in
  order: `history` (its own day and the history after, before Christ),
  `first-advent`, `ad70`, `return`, and `millennium-end`.
- `note`, a sentence or two shown with it.
- `fulfilments`, which may be empty or absent: each with `ref`, the fulfilling
  passage (NT or OT); `ot`, the verses of the prophecy it takes up, if not all
  of them; `kind`, one of `quotation`, `allusion`, `type` (a person or event
  that prefigures, like the passover lamb) or `event` (a narrative of the
  thing happening, like Ezra 1 for Cyrus); `stage`, when it happens, which
  defaults to `history` for an OT passage, `return` for Revelation, and
  `first-advent` for the rest of the NT; and `note`. The stage is when the
  thing happens, not when it is quoted: Acts 7:6-7 quotes Genesis 15:13-14 of
  the exodus, `history`.

`patterns` lists series of prophecies repeating one pattern, like the
invasions from the north. Each has a `title`, a `note`, and `members` in
order, each a prophecy (`ot`, its passage exactly as the prophecy writes it),
the `stage` it stands for, and a `label`. A prophecy may appear more than once
at different stages: Ezekiel 38-39 is Gog at the return and after the thousand
years.

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
outside the corpus, an unknown theme, stage or kind, a fulfilment whose `ot`
lies outside its prophecy, or a pattern member naming no prophecy.
