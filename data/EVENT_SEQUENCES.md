# Event sequences

Some stretches of the Bible's events are known better by their order than by
their dates: Theographic gives most of the Gospels' events a year and no more,
so a year's dozens of events fall on one mark of the timeline. A sequence in
`event_sequences.json` orders such a stretch by what the text says came before
what, and the app draws it as a graph instead of on the timeline.

Each sequence has:

- `id`, unique, and a `title`.
- `note`, saying what it orders and how; shown above the graph.
- `expands` (optional), the title of the event whose inside it tells, such as
  `Holy Week`. Where that event is drawn, it opens the sequence. A sequence
  that expands none stands on the Bible's timeline for itself, spanning the
  years of its events, which are taken off the timeline.
- `chains`, each a `basis` and a list of event titles (as `bible_event` has
  them, after `event_overrides.json`) in the order the basis puts them: each
  event before the next. A basis is a witness — `Mark's order` — or a verse
  that says so — `Matthew 14:13: when Jesus heard of it, he departed`. A chain
  of one event makes it a member without ordering it.
- `groupings` (optional), ways of dividing the events into groups that the
  app folds and unfolds: Robertson's parts, the stretches between John's
  feasts. Each has an `id`, a `title`, a `note` and its `groups`, each with a
  `title`, a `note` (its dates or sections) and either `events`, by title, or
  `groups` of its own. Every event of the sequence is in one group of each
  grouping, and the groups holding events are in order: no chain puts an
  event of a later group before one of an earlier.
- `milestones` (optional), the feasts and other marks of time an event falls
  at, drawn across the sequence: its `event`, a `label` (`Passover`, `A feast
  of the Jews`), the `ref` saying so (`Jhn.2.13`, in STEP Bible's
  abbreviations as `journeys.json` writes them), and the `feast` as the app's
  moʿadim know it (`pesach`, `sukkot`), or another key or none for one they
  do not.
- `events` (optional), by title, what kind each event is and where it
  happens. `kinds` lists the chief first: `healing`, `exorcism`, `raising`
  (of the dead), `sign` (over nature: the water made wine, the loaves),
  `teaching`, `parable`, `journey`, `calling`, `encounter`, `conflict`,
  `revelation` (of who Jesus is: the baptism, the confession, the
  transfiguration, his foretelling of his death), `feast` and
  `john-baptist`. `region` is one of `north` (Phoenicia, Caesarea Philippi
  and Hermon), `galilee`, `east` (of the lake: the Decapolis and Bethsaida
  Julias), `samaria`, `perea` and `judea`. `places` lists the places it
  happens at, the chief first, for the map: TIPNR keys as in
  `journeys.json`, or `{"key", "label"}` where the place is called otherwise
  than its record (Cana for TIPNR's `Kanah@Jos.19.28`). A place must have a
  position; a place no one has found (Gerasa, Ephraim) is left out.

The chains join into one order. Two events no run of chains joins are of
unknown order to one another, and are drawn so. Where the witnesses disagree —
Mark and Luke on when the fishermen were called — keep the order of one and
leave out the other's, saying so in the `note`: an order that goes round in a
circle fails the build.

An event is in one sequence at most; one that `expands` another event holds
the events inside it, which its parent leaves out. The build (`db gen-names`,
or `db gen-runtime`) fails on a title naming no event or several, an event in
two sequences, an empty chain or basis, a sequence inside itself, or a circle;
and on a grouping that leaves out an event, puts one in two groups or runs
against the chains, a milestone or detail of an event not in the sequence, an
unknown kind or region, an unreadable reference, or a place with no record or
position.

## The ministry's places

The events' own places (Theographic's) are too loose for a map: they put the
Sermon on the Mount in Jerusalem. The ministry's `places` are Haqor's own,
from the text, and an event the text does not place (the leper's healing,
"in a certain city") has none. Its regions follow where Robertson's harmony
sets each event where the Gospels do not say.

A harmony of the Gospels is a witness like any other, its chain giving its
order of the events and its `basis` naming it: `Robertson's harmony (1922)`
(A. T. Robertson, *A Harmony of the Gospels for Students of the Life of
Christ*, public domain; Project Gutenberg #36264). Where it agrees with the
Gospels' own order, an ordering rests on both; where it orders what they
leave open, the ordering rests on it alone. An event the harmony places in a
way the Gospels' chains contradict, or tells only in passing, is left out of
its chain, and the sequence's `note` says so.
