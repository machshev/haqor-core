# Gospel journeys: coverage and reconstruction

This catalogue combines Matthew, Mark, Luke and John, using
[A. T. Robertson's 1922 harmony](https://www.gutenberg.org/files/36264/36264-h/36264-h.htm)
as a comparison and a proposed order where a single Gospel does not connect
scenes. The Gospel passages, not the harmony's dates, are the primary evidence.
The [public-domain KJV text](https://www.gutenberg.org/files/10/10-h/10-h.htm)
and the repository's Gospel passage catalogue provide the text references.

This is a record of narrated movements and attested places, with estimates where
requested, not a claim to recover every journey Jesus actually made. John 20:30
and 21:25 explicitly describe a selective record. Unnamed towns on preaching
circuits cannot be enumerated. A region anchor represents a circuit, not one
known visit at its centre. Repeated visits remain separate stops.

## Coverage ledger

| Period / itinerary | Gospel evidence | Handling |
| --- | --- | --- |
| Nazareth, Bethlehem, presentation in Jerusalem | Luke 2:1-39 | Existing infancy journey; Luke's compressed return kept distinct from Matthew's Egypt account |
| Bethlehem, Egypt, Nazareth | Matthew 2:1-23 | Existing journey; eastern Nile delta chosen as an explicit regional estimate, no invented Egyptian town list |
| Annual Passovers; visit at twelve, return to Nazareth | Luke 2:41-52 | Added journey; recurrent visits acknowledged without invented individual dates |
| Nazareth to Jordan; wilderness, temple pinnacle, high mountain | Mark 1:9-13; Matthew 3-4; Luke 3-4 | Baptism departure added; existing temptation interpretation retained, exact sites now explicitly estimated |
| Bethany beyond Jordan, Cana, Capernaum | John 1:28-2:12 | Added separate early journey |
| Jerusalem, Judean baptizing country, Sychar, Cana | John 2:13-4:54 | Added countryside event and route; official's son healed remotely, no invented Capernaum visit |
| Nazareth, Capernaum, solitary place, Galilean circuit, Capernaum | Luke 4:16-44; Mark 1:14-2:17 | Return visits retained; unknown circuit represented by regional estimates |
| Jerusalem feast, Bethesda, temple, return to Galilee | John 5:1-6:1 | Feast unnamed; placement among Synoptic episodes not treated as certain |
| Mountain/level place, Capernaum, Nain | Matthew 5-8; Luke 6-7 | Estimated northwestern teaching slopes; Nain explicit |
| Chorazin | Matthew 11:20-21; Luke 10:13 | Retrospectively attested mighty works; node has no invented dated edge or itinerary |
| Western lake, eastern tombs, return, Nazareth, village circuit | Mark 4:35-6:13; Matthew 9:1 | Eastern landing estimated near Kursi, not inland Jerash |
| Bethsaida retreat, mountain, water crossing, Gennesaret, Capernaum | Luke 9:10; Mark 6:30-56; John 6 | Feeding site outside town; Mark's intended Bethsaida destination not a second asserted landing |
| Tyre region, Sidon, Decapolis lakeshore, Magadan/Dalmanutha | Mark 7:24-8:10; Matthew 15:21-39 | Sidon transit added as an event; critical-text/KJV difference noted; daughter healed remotely |
| Crossing, Bethsaida, Caesarea Philippi villages, mountain, Galilee, Capernaum | Mark 8:13-9:33; Matthew 16-18 | Southern Hermon estimate chosen; no Gospel names the transfiguration mountain |
| Samaria villages, Tabernacles in Jerusalem | Luke 9:51-56; John 7 | Northern Samaria village estimated; joining these accounts is a harmony choice |
| Temple, Mount of Olives, treasury, Martha/Mary, Dedication | John 7-10; Luke 10:38-42 | Bethany inferred from John 11:1; John 7:53-8:11 textual issue retained in note |
| Beyond Jordan, Bethany, Ephraim | John 10:40-11:54 | Separate return event; Ephraim near et-Taiyibeh is estimated |
| Border of Galilee/Samaria, beyond Jordan, Jericho, Bethany | Luke 17:11-19:28; Mark 10; John 12:1 | Connection from Ephraim is conjectural; Luke's approaching/Mark's leaving Jericho healing noted |
| Bethany, Bethphage, Olives, Jerusalem and evening returns | Mark 11-13; Luke 19:29-21:37 | Repeated temple visits and estimated fig-tree road; widow's treasury location retained in route |
| Supper, Olives/Kidron, Gethsemane, Annas, Caiaphas, council, Pilate, Herod, Pilate, Golgotha, tomb | Matthew 26-27; Mark 14-15; Luke 22-23; John 18-19 | Detailed journey and expandable hearing sequences; city buildings separately estimated |
| Tomb appearances, Simon, Emmaus, gathered disciples, Thomas | Matthew 28; Luke 24; John 20 | Expanded appearances; Emmaus return in Luke 24:33 belongs to disciples, not a stated walk by Jesus |
| Sea of Tiberias and mountain in Galilee | John 21; Matthew 28:16-20 | John 21 added to source coverage; lake/mountain appearances unordered in graph |
| Jerusalem gathering, toward Bethany / Mount of Olives, ascension | Luke 24:50-53; Acts 1:3,9-12 | Estimated eastern Olives vicinity; forty days, not resurrection evening |

The teaching and miracle events already present in the ministry sequence remain.
New events fill travel omissions and unpack compressed resurrection/trial nodes;
this is not an attempt to split every saying into its own event. An appearance to
Simon is known only from its report. The Mark 16:9-20 longer ending is retained
in the source parent passages, but no new location depends solely on it.

## Avoiding false movements

Siloam is the healed man's destination (John 9:7), not a recorded visit by Jesus.
Aenon belongs to John the Baptist (John 3:23), not Jesus' Judean baptizing site.
The priests' council in John 11:47-53 is not on Jesus' track. The official's son
and Syrophoenician daughter are healed without a journey to the patient. Tyre
and Sidon city-centre coordinates stand for the named regions, not proof Jesus
entered their centres. The places in parables, people visiting Jesus from other
regions, and Herod's palace in Galilee are not automatically Jesus' stops.

## Estimates and sources

`place_overrides.json` has standalone `added` narrative sites as well as existing
place overrides. Every standalone site has an explanatory summary, valid
coordinates, and `estimated: true`. These have no fabricated word/name tags.
Estimates follow the surrounding scenes and the existing harmony, including
broad Galilean and Judean circuit anchors. Their apparent coordinate precision
is only for drawing. The app marks these sites as low certainty.

Ephraim is placed near et-Taiyibeh, a possible wilderness-edge retreat northeast
of Jerusalem. Gerasa's Gospel *district* is represented near Kursi's eastern
shore to fit the landing and steep bank, not at inland Jerash. These are working
identifications, not archaeological findings. Compare
[the NET note on John 11:54](https://classic.net.bible.org/passage.php?passage=Joh+11%3A54&theme=wiki)
and [the Gerasa/Gadara/Gergesa discussion](https://wartburgproject.org/faqs/2025/07/gerasagadaragergesa-kursi).
[OpenBible's Dalmanutha alternatives](https://www.openbible.info/geo/ancient/ae8d09d/dalmanutha)
explain why one approximate Magadan/Dalmanutha district is used rather than two
successive landings. Each uncertain choice also carries a local note.

The frontend's `doc/jerusalem-map.md` describes the independent AD 30 city-context
layer, its archaeological sources, approximate walls and alternatives. Event
coordinates remain in this core catalogue and flow through the existing shared
bindings; no public interface change is needed.

Emmaus uses El Qubeibeh as a working estimate based on the sixty-stadia reading
of Luke 24:13; Qalunya and Nicopolis remain alternative locations. Compare the
[Franciscan account of the tradition and distance](https://www.custodia.org/en/sanctuaries/emmaus-el-qubeibeh/)
and [OpenBible's competing identifications](https://www.openbible.info/geo/ancient/ae7274b/emmaus).
