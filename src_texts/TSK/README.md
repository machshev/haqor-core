# Treasury of Scripture Knowledge

*The Treasury of Scripture Knowledge* (TSK): cross references attached to the
key words and phrases of each verse, first published by Samuel Bagster & Sons
in the 1830s and commonly credited to R. A. Torrey, who introduced its American
edition. Just Verses describes it as compiled by Thomas Scott around 1836,
mostly from the references of Scott's commentary, supplemented from the centre
column of the *English Polyglot Bible*; CrossWire credits its references to
"Canne, Browne, Blayney, Scott, and others".

The files here are the two members of `tsk.zip` from
[Just Verses](http://www.justverses.com/jv/app/downloadTSK.vm), byte for byte
as downloaded; `SOURCE` records the URL, date and checksums.

- `tskxref.txt` — Latin-1 text, one line per (verse, key phrase): book, chapter, verse,
  display order, the KJV word or phrase, and its `;`-separated references
  (63,682 lines, 305,945 reference groups).
- `readme.txt` — Just Verses' description of the columns and the book numbers
  and abbreviations the references use.

Just Verses describes the material it offers for download, the TSK among it, as
biblical information "in the public domain". The CrossWire SWORD project
distributes its own TSK module under the licence "Public Domain".

References are in the King James Version's versification. `db gen-runtime`
(and `db gen-tsk` on its own) re-number them onto the Hebrew Bible's through
STEP Bible's TAHOT data, which gives both numberings for every Hebrew word, and
store them in the runtime database's `thematic_reference` table.

The key phrases are the KJV's wording, except that the import writes the
divine name where the KJV substitutes a title for it: "the LORD" becomes
"Yahweh", "the LORD'S" "Yahweh's", "the Lord GOD" "the Lord Yahweh", "JAH"
"Yah" and "Jehovahjireh" "Yahweh-jireh" (`name_the_lord` in
`crates/haqor-db-gen/src/tsk.rs`). The file here is unchanged.
