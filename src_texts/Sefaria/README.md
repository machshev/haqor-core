# Klein and Jastrow

Entries filtered from the `lexicon_entry` collection of
[Sefaria](https://www.sefaria.org)'s database dump by
`scripts/fetch-sefaria-lexicons.sh`. `SOURCE` records which dump they came
from. Each line is one entry exactly as Sefaria stores it, less the fields the
import drops (`_id`, `refs`, `parent_lexicon`, `prev_hw`, `next_hw`).

- `klein.jsonl` — Ernest Klein, *A Comprehensive Etymological Dictionary of the
  Hebrew Language for Readers of English* (Carta Jerusalem, 1987), digitised by
  Sefaria: <https://www.sefaria.org/Klein_Dictionary>. Licensed **CC BY-NC**:
  it may be redistributed with this credit, but not for commercial purposes.
- `jastrow.jsonl` — Marcus Jastrow, *A Dictionary of the Targumim, the Talmud
  Babli and Yerushalmi, and the Midrashic Literature* (Luzac, London, 1903),
  digitised by Sefaria: <https://www.sefaria.org/Jastrow>. Public domain.

An entry is kept when it belongs to the biblical layer (Klein's unmarked and
`BH` entries; Jastrow's `b. h.` and `ch.` entries) or when its headword or an
alternative spelling shares a consonant skeleton with a BDB entry or a SEDRA
lexeme or root.
