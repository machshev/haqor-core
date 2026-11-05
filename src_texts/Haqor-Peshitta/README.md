# Word-linked English of the Peshitta

`translation.tsv` contains Haqor's English renderings in Peshitta
word order, using James Murdock's public-domain translation as a reference.
The unmodified reference remains in `src_texts/Murdock/murdock.tsv`.
The goal is reviewed, word-linked English for all 7,958 Peshitta verses.
This is an incremental translation project: only the verses in the TSV have
word links. Other verses retain the existing Murdock translation and select
whole verses in the English reader.

Current coverage: Matthew 1-28, Mark 1-9, 2 John, 3 John, and Jude
(1,497 of 7,958 Peshitta verses; 6,461 remain). Mark 10 is next.
The unlinked Murdock fallback is temporary coverage, not a completed
translation or word alignment.

Each non-comment row is `book<TAB>chapter:verse<TAB>translation`, using
Haqor's one-based book numbers (Matthew = 40), Peshitta verse numbering, and
zero-based source positions. `[English phrase|0]` links a phrase to one
Aramaic word. `[English phrase|0,1]` renders a contiguous idiom together.
Keep the links in source order. Prefixes and suffixes belong to their
source word; a phrase may therefore translate more than one English word.
Separate linked phrases with spaces where English words need separation;
markup does not insert spaces between spans. Review the running English as
well as each individual source-to-English mapping.

Both `db gen-runtime` and `db gen-translation` load this tracked source
from the supplied `--src-texts` directory on every build. The source file
is required: a missing file fails the build instead of dropping the links.
The runtime `haqor.db` is generated output.

The generator rejects missing or duplicated source positions, reversed or
cross-verse links, duplicate verse entries, empty renderings, unlinked
lexical text, and malformed markup. These checks validate structure;
they cannot establish translation accuracy. Every addition needs comparison
against the actual source forms, their grammar, and Murdock in context.
Do not populate this file by concatenating dictionary glosses or by assigning
positions to remembered English verses.

To prepare a review batch, run `scripts/export-peshitta-translation.py` with
`--book`, `--first-chapter`, `--last-chapter`, and `--out`. The JSONL includes
Murdock, exact source positions and forms, morphology, and all SEDRA gloss
alternatives. Translate each verse against these records. Independently
review the mappings before adding rows, paying attention to negation,
subjects, suffixes, idioms, and differences from familiar Greek numbering.
Work in book and chapter order.

Keep working context limited to one chapter: export its records to a file,
review and append that chapter, validate, sync, and commit it before starting
the next. Between chapters retain only the goal, coverage and next chapter,
translation conventions, verification commands, and unresolved questions.
Use this README and the committed TSV as the checkpoint; do not reload
completed chapter records routinely. Reopen a specific earlier verse only
when needed to resolve a consistency or accuracy question.

Use chapter subagents to keep the main session limited to the checkpoint.
If needed, split a chapter into short verse batches in separate files and
cross-review each batch with another subagent. Merge and validate the complete
chapter before syncing and committing it.

Rebuild with `cargo run --release -- db gen-translation`, then copy the
runtime database to the paired Flutter app's `assets/db/haqor.db` and update
its `assets/db/version.txt`. The reader already supports these links;
no public binding change is required. Run the generator translation tests
and the Flutter `verse_row_selection_test.dart` widget tests.

Source checks for Matthew 7-12:

- Matthew 8:15 has a singular object: she ministered to him, rather than
  Murdock's "to them".
- Matthew 9:20 has twelve years, rather than the local Murdock reference
  file's "fourteen years".
- Matthew 10:8 has no source words for Murdock's supplied "raise the dead";
  those words are not added or linked to unrelated source positions.
- Matthew 11:14 explicitly names Elijah, although the local Murdock
  reference omits his name.

Source checks for Matthew 13-16:

- The final clause of Murdock 13:12 (taking away from the one who does not
  have) belongs to Peshitta 13:13 and is linked there.
- Matthew 13:21 distinguishes the word (feminine: it has no root in him)
  from its hearer (masculine: he is temporary). In 13:22 the word becomes
  fruitless; the final verb is feminine.
- Matthew 13:37 says "Son of man", rather than the local Murdock
  reference file's "Son of God".
- Matthew 15:39 names Magdu; the reference renders the place as Magdala.

Source checks for Matthew 17-20:

- Matthew 17:15 positions 8-9 form the idiom bar egara, rendered "epilepsy"
  as a linked phrase. Brock and Kiraz record the expression under
  [egara in SEDRA](https://sedra.bethmardutho.org/lexeme/get/27).
- Matthew 18:29 has no equivalent of Murdock's added "the whole".
- Matthew 19:5 says "one flesh"; 19:6 uses the distinct word "body".

Source check for Matthew 21: the praise quotation divided across Murdock
21:16-17 is entirely in Peshitta 21:16 and is linked there.

Source check for Matthew 22: 22:37 explicitly includes strength as well as
heart, soul, and mind; all four are retained and linked to their source words.

Source checks for Matthew 23:

- The two-word face-taking expression is rendered as the single linked idiom
  "hypocrites", rather than adding a separate appearance phrase.
- In 23:37, the participial descriptions of Jerusalem and the third-person
  "to her" are retained; Murdock changes the latter to direct address. Compare
  the [Peshitta interlinear, page 5](https://www.atour.com/media/files/religion/peshitta/interlinear-nt/Mattai/Mattich23.pdf).
- In 23:39, the negated future is a prediction that they will not see him,
  rather than a prohibition on seeing him.

Source checks for Matthew 24:

- The feminine form in 24:33 is rendered with neutral "it", rather than
  personal masculine "he".
- In 24:36 the source goes directly from the angels of heaven to the Father
  alone; no intervening person is supplied.
- The masculine and feminine pairs in 24:40-41 are expressed as two men and
  two women, respectively.

Source checks for Matthew 25:

- In 25:1 both the bridegroom and the bride are explicit source words and
  are retained.
- The differently pointed past auxiliary is first person in 25:24 ("I knew")
  and second person in 25:26 ("you knew"). It is linked with the knowing
  predicate, rather than with the following description or reaping predicate.
- In 25:41 the king speaks to those on his left; the imperative with its
  reflexive dative is linked separately from "from me". The false-accuser
  title follows the established rendering used in earlier chapters.

Source checks for Matthew 26:

- In 26:39 the cup petition is volitional, rather than a bare infinitive.
  The repeated first-person forms belong to the willing predicate; they do
  not require a detached extra English "I".
- In 26:69 the past auxiliary belongs to the assertion that Peter was with
  Jesus. It is not part of the following "Nazarene" epithet.
- In 26:75 the cock-crow clause is future-relative, not past: "before the
  cock crows" is followed by "you will deny me".

Source checks for Matthew 27:

- In 27:9 the source names only "the prophet"; no individual name is added.
- In 27:38 the two robbers are crucified: the verb is passive. In 27:52,
  by contrast, the bodies arise; the past auxiliary belongs to the sleeping
  description rather than making the rising verb passive.
- In 27:47 the past auxiliary belongs to "were saying". The explicit
  demonstrative then begins their claim: "This one calls for Elijah."

Source checks for Matthew 28:

- In 28:1 the evening wording and the first-day-of-the-week idiom are
  retained, without replacing the source timing with a different formulation.
- In 28:10 the possessive in "my brothers" is first person; the morphology
  dataset uses person code 3 for first person.
- The distinct verbs in 28:19 and 28:20 are rendered "instruct" and "teach",
  respectively.

Source checks for Mark 1:

- In 1:2 Isaiah is explicitly named and is retained.
- In 1:31 the woman ministers to "them", preserving the plural source object
  rather than copying the singular object of Matthew 8:15.
- In 1:21-22 the teaching predicates retain their subjects and auxiliaries;
  the postpositive explanatory particle stays with the second teaching clause.

Source checks for Mark 2:

- In 2:2 the gathering is intransitive, and the singular containing predicate
  refers to the available space, not to the assembled people.
- In 2:4 the ascent includes a reflexive dative. The lowering verb anticipates
  the following bed noun; no separate human object is added.
- In 2:17 the illness expression is one idiom, not an additional action of
  laboring.
- In 2:26 Abiathar is retained as named; no other priest is substituted.

Source checks for Mark 3:

- In 3:8 the crowd expression is plural: "great multitudes".
- In 3:21 the out-of-his-mind expression remains the kinsmen's reported
  claim, with the explanatory particle linked to "they were saying".
- In 3:29 the source specifies eternal judgment; no different object is
  substituted. The first-person family possessives in 3:35 are retained.

Source checks for Mark 4:

- In 4:5 the two-word hour expression means "immediately", not a literal
  duration of an hour.
- In 4:6 the heat predicate is retained despite the morphology dataset's
  unrelated noun gloss; compare "it became hot" in
  [Etheridge's Syriac translation](https://studybible.info/Etheridge/Mark%204%3A6).
- In 4:27 the explicit knowing subject is the man, not the seed. In 4:28
  the object suffix anticipates the fruit noun; no extra recipient is added.
- In 4:30 the question asks to what the kingdom should be likened; the
  kingdom remains the object of comparison.

Source checks for Mark 5:

- In 5:9 the name suffix is plural: "our name". In 5:12 the speakers are
  explicitly identified as those demons, not left as an unnamed pronoun.
- In 5:30 the knowledge clause says that power had gone out from Jesus;
  the clause-introducing prefix is linked with the power noun.
- In 5:41 the command is simply "Girl, arise"; no explanatory wording from
  another textual tradition is supplied.
- In 5:42 the first past auxiliary belongs to the walking predicate. The
  following existential clause explains that she was twelve years old.

Source checks for Mark 6:

- In 6:20 the adverb "gladly" belongs to hearing John, following the
  separate statement that Herod listened and acted in many matters.
- In 6:21 the linked compound means "birthday", rather than a literal
  house of birth; compare the
  [CAL entry for beth yalda](https://cal.huc.edu/oneentry.php?cits=all&lemma=byt%40yld%29+N).
- In 6:41 the plural verb says that they divided the two fish. In 6:43
  the twelve full baskets contain fragments and fish; fragments are not
  omitted or assigned to the fish noun.
- In 6:45 Bethsaida is explicitly named as the destination. In 6:52
  the demonstrative specifies "that bread"; in 6:56 the streets are
  explicitly retained as the place where the sick were being laid.

Source checks for Mark 7:

- In 7:11 the second-person pronoun completes "you say" before the
  conditional begins. The offering noun retains its first-person suffix.
- In 7:19 the feminine cleansing predicate remains with the physical
  elimination process. The source noun has cleansing and excrement senses;
  compare the [dictionary entry for tadkitha](https://www.assyrianlanguages.org/sureth/dosearch.php?language=id&searchkey=46502).
- In 7:25 the masculine suffix in the hearing phrase refers to Jesus:
  the woman heard "about him". The following daughter clause is kept
  distinct, with the unclean spirit as what her daughter had.
- In 7:26 the woman is a Gentile from Phoenicia of Syria. In 7:31 the
  final location is in the territory of Decapolis.
- In 7:28 the source has plural "tables", matching its vocalisation;
  compare the [Peshitta text](https://peshitta.eu/nt/mark/7.html). In 7:36
  the hearers are warned to tell no one; the comparative clause begins
  with the following "And the more".

Source checks for Mark 8:

- The dismissal clause belongs to source 8:10, after the population count
  in 8:9; it is not duplicated from Murdock's earlier verse boundary.
- In 8:14 the exception precedes the negative: apart from one loaf, they
  had no bread. The connective has an exception sense in this construction;
  compare the [CAL entry for ella](https://cal.huc.edu/oneentry.php?cits=all&lemma=%29l%29+c).
- In 8:20 the breaking action is elliptical after 8:19. English supplies
  the repeated action to express the question grammatically.
- In 8:28 both occurrences of "others" are plural and retain their
  conjunctions. In 8:29 the Son of the living God is explicitly included.
- In 8:35 the gospel has a first-person possessive, "my gospel". In 8:38
  the suffix in "Son of man" anticipates the following genitive, while
  "his Father" and "his holy angels" retain independent possessives.

Source checks for Mark 9:

- In 9:5 there are three booths, one each for Jesus, Moses, and Elijah.
  The following "one" belongs to each preceding recipient phrase.
- In 9:22 the requests are singular: "help me" and "have compassion on me".
  In 9:23 belief is explicit; 9:24 retains "my Lord" and "my faith";
  9:29 explicitly includes both fasting and prayer.
- In 9:25 the crowd is both running and gathering. The final prohibition
  concerns entering the boy again, following the command to come out.
- In 9:20 the past auxiliary belongs with writhing. In 9:32 it belongs
  with understanding; the object suffix anticipates the following saying.
- In 9:41 the name/belonging construction is rendered as belonging to
  Messiah. The repeated worm/fire clauses in 9:44, 9:46, and 9:48 remain,
  as do both the fire and sacrifice clauses in 9:49.
