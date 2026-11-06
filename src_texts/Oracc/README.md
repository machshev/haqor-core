# Oracc glossaries: Akkadian and Sumerian

The Akkadian (`akk`) and Sumerian (`sux`) glossaries of the projects of
[Oracc](https://oracc.museum.upenn.edu/), the Open Richly Annotated Cuneiform
Corpus, including the electronic Pennsylvania Sumerian Dictionary (ePSD2).
Fetched by `scripts/fetch-oracc-glossaries.sh` from each project's JSON
archive; `SOURCE` lists the projects and the date.

The Haqor lexicon's briefs search them for a root's Akkadian cognates and its
Sumerian and Akkadian loans (ADR 7).

`glossary.jsonl` has one line per lemma, merged across the projects that
attest it:

- `lang`, `cf` (citation form), `gw` (guide word), `pos`.
- `icount`: how many times the projects attest it.
- `meanings`: the senses' meanings.
- `norms`: its commonest normalised forms, up to eight.
- `periods`: the periods it is attested in.
- `entries`: `[project, id]` for each project's entry, most attested first;
  the entry is online at
  `https://oracc.museum.upenn.edu/<project>/cbd/<lang>/<id>.html`.

## Licence

Every glossary file states: "This data is released under the CC0 license",
with the licence URL https://creativecommons.org/publicdomain/zero/1.0/ and
the note "more-info": "http://oracc.org/doc/opendata/". (Oracc's site pages
describe its projects' default licence as Creative Commons
Attribution-ShareAlike; the fetch script checks that each file carries the
CC0 statement and fails otherwise.)

Oracc and its projects are credited as the sources of these glossaries: ePSD2
(The Pennsylvania Sumerian Dictionary Project) and the project teams of SAAo,
RINAP, RIBo, RIAo, CAMS, BLMS, DCCLT and the others `SOURCE` names.
