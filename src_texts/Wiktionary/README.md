# Wiktionary: the Semitic languages

Entries from the English Wiktionary for Hebrew, the other Semitic languages
(Akkadian, Ugaritic, Phoenician and Punic, the Aramaic dialects, Classical
Syriac, Mandaic, Arabic, Old South Arabian, Ge'ez and its daughters, and the
Proto-Semitic reconstructions), Egyptian, Coptic and Sumerian, as extracted by
[wiktextract](https://github.com/tatuylonen/wiktextract) and published by
[kaikki.org](https://kaikki.org/dictionary/rawdata.html). Prepared by
`scripts/fetch-wiktionary-semitic.sh`; `SOURCE` records the dump and the
number of entries per language.

The Haqor lexicon's briefs use it to find a root's cognates (ADR 7): an entry
links a word to its cognates, borrowings and inheritances in other languages,
and a Proto-Semitic page lists its descendants. It is a lead to the evidence,
which an entry cites from each language's own dictionary.

`semitic.jsonl` has one line per entry:

- `word`, `lang` (Wiktionary's language code), `pos`, `etym` (the etymology
  number on a page with several), `roman` (romanisation).
- `roots`: the roots Wiktionary files the word under.
- `glosses`: its senses' glosses, up to twelve.
- `etymology`: the etymology section's text.
- `links`: `[relation, language, word, transliteration]` from the etymology's
  templates: `cog` cognate, `inh` inherited, `bor` borrowed, `der` derived,
  and their variants; `root` gives the root.
- `descendants`: `[language, word, romanisation]`, for reconstructions.

Arabic keeps its lemmas only, not the pages that are inflected forms.

## Licence

Wiktionary's text is "dual-licensed to the public under both the Creative
Commons Attribution-ShareAlike 4.0 International License (CC-BY-SA) and the
GNU Free Documentation License (GFDL)"
(<https://en.wiktionary.org/wiki/Wiktionary:Copyrights>). The entries are the
work of Wiktionary's contributors; each is found at
`https://en.wiktionary.org/wiki/<word>`, under its language's heading.

kaikki.org asks that the extraction be cited as: Tatu Ylonen, "Wiktextract:
Wiktionary as Machine-Readable Structured Data", *Proceedings of the 13th
Conference on Language Resources and Evaluation (LREC)*, pp. 1317–1325,
Marseille, 2022.
