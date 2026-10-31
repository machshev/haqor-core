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
  `john-baptist`. Further kinds are `preaching`, `testimony`, `baptism`,
  `ecclesia`, `birth`, `childhood`, `covenant` (circumcision and presentation)
  and `letter`. `region` is one of `north` (Phoenicia, Caesarea Philippi
  and Hermon), `galilee`, `east` (of the lake: the Decapolis and Bethsaida
  Julias), `samaria`, `perea` and `judea`, or `egypt`, `syria`, `cyprus`,
  `asia-minor`, `macedonia`, `achaia`, `malta` or `italy`. `places` lists the places it
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

## Birth and childhood; Acts

`birth-childhood` keeps Matthew 1-2 and Luke 1-2 as separate chains, joining
at shared events. It does not choose whether the wise men's visit preceded
the presentation in the temple. Both accounts lead to the return to Nazareth.

`acts` groups the witnesses in Jerusalem, the spread to Samaria and the
Gentiles, Herod and the first journey, the council and further journeys,
and Paul's imprisonment. Four sequences expand the existing journey events:
Paul's first, second and third preaching journeys and his journey to Rome.
Peter's narrative and the scattered brethren are separate branches; chapter
order alone does not order these against all Paul's movements. The writing
of Galatians remains an unordered member because its date is disputed.

The notes use Christadelphian terminology: ecclesia for an assembly of
believers, the Holy Spirit as God's power poured out on the witnesses, and
the gospel of the Kingdom of God and the name of Jesus Christ. Event title
changes belong in `event_overrides.json`, not in display-only aliases.
The inherited chronology still supplies overview spans; the chains assert
only the order supported by their named passages.

All six newer sequences have episode groups and curated event kinds. Their
map keys reuse the place identifications in `journeys.json` where possible.
A journey summary may name several places; map lines join events, not every
stage of a reconstructed travel route. Multi-region events have no single
region lane. The unnamed hill-country town, the place of Joseph's dream,
the family's residence in Egypt, Samaria's unnamed city, the Gaza road and
the place of writing Galatians are deliberately unpinned. Bethlehem for
Jesus' circumcision follows Luke 2:1-22's setting before the journey to
Jerusalem; Nazareth is not assumed to be the location of Mary's betrothal.
Only explicit feast settings become milestones: Passover in Luke 2:41 and
Pentecost in Acts 2:1. The shipwreck is not dated to the fast already past in
Acts 27:9.

## Genesis through the restoration

Fourteen further sequences cover the remaining narrative, royal and prophetic
entries already in the source catalogue. `beginnings` opens the `flood`;
Abraham, Jacob, Joseph, Job, Moses and Joshua open from their existing lifetime
spans; Moses opens `sinai` through the wilderness span. `judges` stands alone.
The united, northern and southern kingdoms open their respective sequences.
`prophets` gathers the existing prophetic ministries through Malachi.

Overlapping lifetime spans and the long sojourn remain on the overview.
The source's identification of Job with Issachar's son is not endorsed by an
ordering edge: that birth remains outside the Job sequence. No missing episodes
are invented to fill gaps in the source catalogue. These are sequences of the
available events, not a claim that every biblical episode has been catalogued.

Reign and ministry nodes mark beginnings, not completed spans. Judah's accession
chain does not place each predecessor's death before his successor's accession.
Omri and Tibni branch as rival rulers. The later judges branch between Ammon
and Philistia. Genealogical deaths are not ordered by their position in a chapter.
Prophetic ministries are unordered except for Elijah's departure and Elisha's
succession. Group headings help navigation; only edges assert relative order.
No locations are inferred from the older events' loose place associations.

The additional event kinds are `creation`, `judgement`, `death`, `reign`,
`judgeship` and `prophecy`; the frontend provides corresponding icons and filters.
The Jehoash death override corrects the event attached to 2 Kings 13:13, distinct
from Jehoahaz's death in 13:9, so titles resolve unambiguously.

## Journey tracks

An event's optional `tracks` array names the people whose journey includes its
chief place (for example `["Jesus", "John the Baptist"]` at the baptism). These
are explicit location assignments, not everyone mentioned in a passage. Empty
or omitted tracks mean unassigned. Track names must be nonempty and unique per
event. Shared events can belong to several tracks. Ordering edges still express
chronology and synchronisation across tracks; they do not imply that one person
travels to another person's location. Unknown places remain unpinned.

The ministry separates John's activities from Jesus's. Paul's first journey
includes Barnabas; the second includes him at their separation only. No later
location for Barnabas is inferred from Paul's travels.
