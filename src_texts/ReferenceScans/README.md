# Local reference scans

Local copies of four works already cited in `data/lexicon/sources.json`:

| Directory | Work | Download host |
|---|---|---|
| `fisher-1969` | Milton Charles Fisher, *The Lexical Relationship Between Ugaritic and Ethiopic* (Brandeis dissertation, 1969) | [Internet Archive](https://archive.org/details/bim_early-english-books-1641-1700_the-lexical-relationship_fisher-milton-charles_1969-04) |
| `segert-1984` | Stanislav Segert, *A Basic Grammar of the Ugaritic Language: With Selected Texts and Glossary* (University of California Press, 1984) | [Internet Archive](https://archive.org/details/segert-a-basic-grammar-of-the-ugaritic-language-1984) |
| `krahmalkov-2000` | Charles R. Krahmalkov, *Phoenician-Punic Dictionary* (Peeters, 2000) | [Digital Thought PDF](https://digitalthought.info/Phoenician-Punic-Dictionary.pdf) |
| `biella-1982` | Joan Copeland Biella, *Dictionary of Old South Arabic: Sabaean Dialect* (Harvard Semitic Studies 25; Scholars Press, 1982) | [Internet Archive](https://archive.org/details/DOOSA) |

Biella's scan is the Eisenbrauns 2004 unchanged paperback reprint of the
1982 edition, with the original pagination.

## Fetching and searching

From the repository root:

```sh
scripts/fetch-reference-scans.sh
rg -n -i 'pasture|administer' src_texts/ReferenceScans/segert-1984/source.txt
```

The script requires `curl`, `jq` and `sha256sum`. Install Poppler's
`pdftotext` for Krahmalkov's search text; the other three works have downloadable
Archive.org OCR. An optional destination argument changes the cache location;
the paths in this document use the default directory.

Each directory contains `source.pdf` and `source.txt`. The Archive.org works
also have `pages.json`, the host's mapping between scan leaves and printed page
labels (which can be incomplete or misidentified by OCR). `files.json` maps
each source ID to its local files and records their exact download URLs and
SHA-256 checksums. Krahmalkov's text is extracted locally with
`pdftotext -layout`; its bytes depend on the Poppler version.

Use the text to find passages, then check the PDF before citing a reading.
OCR can lose or confuse Semitic letters, diacritics, superscripts and columns.
Cite the work's printed page, not the PDF viewer's page index. Segert is a
grammar with a glossary, and Fisher is a comparative dissertation; neither
should be treated as a comprehensive Ugaritic dictionary.

## Availability and rights

The files were fetched from public download links on 2026-11-08 without a
login. Availability is not a redistribution licence: no open reuse licence
was found in the Archive.org metadata or accompanying the Digital Thought
download. These are local consultation copies. Their directories (including
PDFs, OCR and page maps) are ignored by Git and are not bundled into the
runtime database or app. Only this documentation, the download manifest and
the fetch script are tracked. These consultation copies are optional: the
registry's `local` fields stay empty because nonempty paths are required by
the lexicon validator and database generator, including on fresh checkouts.
Database builds and app packaging do not need to download these works.
