# Lane's Arabic-English Lexicon

Edward William Lane, *An Arabic-English Lexicon* (London: Williams and
Norgate, 1863–93), as digitised by the Perseus Digital Library and corrected by
the [laneslexicon](https://github.com/laneslexicon/lexicon_xml) project.
Prepared by `scripts/fetch-lane.sh`; `SOURCE` records the commit.

`lane.jsonl` has one line per entry, in printed page order:

- `root`: the root in Arabic script; `root_key` the same with every hamza form
  as ء and alif maqṣūra as ي, the key the Haqor lexicon's cognate search
  matches on; `root_bw` the root as the XML writes it (Buckwalter).
- `id` and `key`: the entry's id and key in the XML.
- `headword`: the entry's first form, in Arabic script.
- `page`: the printed page the entry begins on, the page it is cited by
  (`Lane p. 1427`).
- `text`: the entry's text, with the Arabic converted from Buckwalter to
  Unicode.

Lane's own vocalisation of quotations was partly lost in digitisation, and
volumes 6–8 (from ق) were completed from his notes after his death and are
thinner.

## Availability

The Perseus TEI files carry this availability statement, reproduced here as
they give it:

> This text may be freely distributed, subject to the following restrictions:
>
> - You credit Perseus, as follows, whenever you use the document: "Text
>   provided by Perseus Digital Library, with funding from The U.S. Department
>   of Education and The Max Planck Society."
> - You leave this availability statement intact.
> - You offer Perseus any modifications you make.

Perseus's download page (http://www.perseus.tufts.edu/hopper/opensource/download)
states: "Texts are licensed under the Creative Commons ShareAlike 3.0".

Text provided by Perseus Digital Library, with funding from The U.S.
Department of Education and The Max Planck Society.
