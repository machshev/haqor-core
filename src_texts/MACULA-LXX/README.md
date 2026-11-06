# MACULA Hebrew: Septuagint alignment

For each Hebrew word that MACULA Hebrew aligns to the Septuagint, the Greek
word that renders it. Prepared by `scripts/fetch-macula-lxx.sh` (`db
import-lxx`) from the same MACULA commit as `src_texts/MACULA-Hebrew`; `SOURCE`
records it.

The Haqor lexicon uses it as the bridge from the New Testament's Greek back to
the Hebrew word (ADR 7). A translation is no evidence of what the Hebrew means:
that rests on scripture's use of the word and its Semitic cognates.

`lxx.tsv` has one line per aligned Hebrew word, in MACULA's order:

1. the verse, as MACULA names it (`GEN 2:7`);
2. MACULA's word number in the verse (prefixes share their word's number);
3. the Hebrew lemma;
4. the Hebrew word as written;
5. the Greek word as the Septuagint has it;
6. its Greek lemma, named from Strong's Greek dictionary by the Strong's
   number MACULA gives (the number itself is not kept). Where MACULA gives
   none, the Greek word stands as its own lemma.

## Licence and credit

"MACULA Hebrew Linguistic Datasets, available at
https://github.com/Clear-Bible/macula-hebrew/", © 2022-2024 by Biblica, Inc,
licensed CC BY 4.0. Its licence lists among the datasets "Greek equivalents
drawn from the Septuagint" and "Strong's numbers for both Hebrew and Greek
equivalents".

The Greek lemmas are named from Open Scriptures' JSON edition of Strong's
*Dictionary of Greek Words* (James Strong, 1890; XML edition by Ulrik Petersen,
2006), which states "Copyright 2009, Open Scriptures. CC-BY-SA."
