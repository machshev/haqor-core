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

The chains join into one order. Two events no run of chains joins are of
unknown order to one another, and are drawn so. Where the witnesses disagree —
Mark and Luke on when the fishermen were called — keep the order of one and
leave out the other's, saying so in the `note`: an order that goes round in a
circle fails the build.

An event is in one sequence at most; one that `expands` another event holds
the events inside it, which its parent leaves out. The build (`db gen-names`,
or `db gen-runtime`) fails on a title naming no event or several, an event in
two sequences, an empty chain or basis, a sequence inside itself, or a circle.

A harmony of the Gospels is a witness like any other, its chain giving its
order of the events and its `basis` naming it: `Robertson's harmony (1922)`
(A. T. Robertson, *A Harmony of the Gospels for Students of the Life of
Christ*, public domain; Project Gutenberg #36264). Where it agrees with the
Gospels' own order, an ordering rests on both; where it orders what they
leave open, the ordering rests on it alone. An event the harmony places in a
way the Gospels' chains contradict, or tells only in passing, is left out of
its chain, and the sequence's `note` says so.
