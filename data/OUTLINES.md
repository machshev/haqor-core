# Book outlines

`outlines.json` gives the structure of a book, or a stretch of books, as a
tree of sections: the Psalms' five books, their collections and each psalm
with its title and author; the collections of the Proverbs; the weekly
portions of the Torah; the structure of the law of Moses. `db gen-outlines`
(or `db gen-runtime`) builds them into `haqor.db`, and the app shows each as a
tree, as an index filtered by its facets, and beside the chapter being read.

They are Haqor's own, written from the text. A psalm's author and kind are
what its title in the Hebrew text says; where the title names none, the
author is `Unnamed`, unless the New Testament names one (Psalm 2 by Acts 4:25,
Psalm 95 by Hebrews 4:7), which the note then gives.

## References

Passages are written in STEP Bible's book abbreviations, as
`prophecies.json`'s are:

- a whole chapter, `Psa.51`, or a run of chapters, `Psa.1-41`. A whole
  chapter is the Hebrew text's chapter, so a psalm takes in its title, which
  the Hebrew counts as its first verse or two. Name whole chapters only where
  the Hebrew's chapters are the English Bible's: Malachi 4, for one, is
  Malachi 3:19-24 in the Hebrew.
- a verse, `Gen.3.15`, or a run of verses within a chapter, `Pro.24.23-34`,
  across chapters, `Gen.6.9-11.32`, or across books, `Exo.20.1-Num.10.10`.
  Verses are **in the English Bible's numbering**, and the build renumbers OT
  verses onto the Hebrew text through STEP Bible's TAHOT data: Leviticus
  6:1-7 is stored as 5:20-26, Deuteronomy 29:1 as 28:69.

## Fields

`outlines` lists the outlines, in the order the app offers them. Each has:

- `id`, unique, and a `title`.
- `note`, saying what it outlines and how; shown above it.
- `facets` (optional), the facts its sections are indexed by, each an `id`
  and a `title`: the Psalms' `author`, `kind` (a `Psalm`, `Song`, `Maskil`
  and so on, as the title calls it) and `occasion` (the time of David's life
  a title names).
- `contiguous` (optional, default false): whether its sections leave no gap.
  Then each list of sections must run on without a verse missing between
  them, and a section's own sections must run from its first verse to its
  last. The Psalms, the Proverbs and the Torah portions are; the law, which
  leaves out the narrative between, is not.
- `sections`, the top of its tree.

Each section has:

- `title`, and `passage`, its verses. A section lies within its parent, and
  after the one before it, without overlapping it.
- `subtitle` (optional), a line under the title: a psalm's theme, a
  portion's name in English.
- `superscription` (optional), the heading the text itself gives it, in
  English: a psalm's title, the heading of a collection of proverbs.
- `note` (optional), a sentence or two shown with it.
- `facts` (optional), its values of the outline's facets, by facet id: one
  value, or a list of them (`"kind": ["Song", "Psalm"]`).
- `sections` (optional), its own sections, in order.
