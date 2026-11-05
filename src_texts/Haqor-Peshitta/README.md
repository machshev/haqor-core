# Word-linked English of the Peshitta

`translation.tsv` contains Haqor's English renderings in Peshitta
word order, using James Murdock's public-domain translation as a reference.
The unmodified reference remains in `src_texts/Murdock/murdock.tsv`.
The goal is reviewed, word-linked English for all 7,958 Peshitta verses.
This is an incremental translation project: only the verses in the TSV have
word links. Other verses retain the existing Murdock translation and select
whole verses in the English reader.

Current coverage: Matthew 1-28, Mark 1-16, Luke 1-24, John 1-21, Acts 1-7, 2 John, 3 John, and Jude
(4,085 of 7,958 Peshitta verses; 3,873 remain). Acts 8 is next.
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

Source checks for Mark 10:

- In 10:13 Jesus is the singular subject of touching the children. In
  10:14 the dative belongs to his being displeased, rather than identifying
  someone he was offended at; the kingdom belongs to those like the children.
- In 10:24 "my children" and "their possessions" retain their distinct
  possessors. In 10:27 both occurrences of "with God" and the explanatory
  "for" are retained, with the positive predicate supplied in the ellipsis.
- The lists in 10:29 and 10:30 remain distinct: the first includes a wife,
  father, and fields; the second includes mothers and fields, without a
  repeated wife or father. The first-person "my gospel" is retained.
- In 10:38 the repeated first-person pronoun completes the drinking
  predicate; it is not a possessive attached to the immersion noun.
- In 10:46 the source names Timaeus and then son of Timaeus. In 10:52 the
  imperative is "See", and the ending describes going along the way;
  no following-after-him phrase is supplied from the reference translation.

Source checks for Mark 11:

- In 11:2 the final word includes the command to bring the colt. In 11:3
  the title is "Our Lord", and "immediately" belongs to sending it here.
- In 11:6 the plural pronoun is the object of commanding: Jesus commanded
  them. In 11:22 the possession construction expresses having faith in God.
- In 11:23 both predicates are retained: believing that what is said will
  happen, and having whatever is said. The mountain command ends at the sea.
- In 11:25 the grievance is something the hearers have against someone.
  The negative conditional and its consequence in 11:26 remain in full.

Source checks for Mark 12:

- In 12:13 the source names scribes and the household of Herod; no
  different group is substituted from another textual tradition.
- In 12:19 the surviving brother is the subject of taking the wife.
  In 12:24 Jesus addresses them; in 12:29 he addresses the individual scribe.
- Both 12:30 and 12:33 retain heart, soul, mind, and strength, in their
  respective source orders.
- In 12:42 the two coins are explicitly explained as small coins. The
  rendering keeps that explanation without assigning a specific value;
  compare the broader weight/value gloss in
  [Brock and Kiraz's entry for manya](https://sedra.bethmardutho.org/lexeme/get/1803).
- In 12:41 the past auxiliary belongs with casting. In 12:44 it belongs
  with what the widow had; the feminine demonstrative begins the contrast
  with the other contributors, and her entire property is retained.

Source checks for Mark 13:

- In 13:6 the two first-person forms express one claim, "I am he".
  In 13:9 the explanatory "for" belongs to delivering the hearers to judges;
  the following synagogues retain their possessive.
- In 13:10 the first-person possessive is retained: "my good news".
  In 13:11 the delivery verb explicitly includes its "you" object.
- In 13:14 Daniel is explicitly named. In 13:32 the Son is explicitly
  included before the exception for the Father.
- In 13:34 the journey, leaving, giving, and commanding verbs are past.
  In 13:37 the two saying clauses remain distinct: the first addresses
  "you", and the second addresses "you all".

Source checks for Mark 14:

- In 14:5 the masculine suffix in vocalized `bēh` is rendered "at it",
  rather than the reference's "at her"; compare the
  [vocalized Peshitta text](https://aramaicnewtestament.org/gospel/mark_14.htm).
- The first-person possessives in 14:9 ("my gospel") and 14:14
  ("Our Rabbi") are retained. In 14:13 the man will meet the hearers;
  carrying the water jar is a separate description.
- In 14:23 "all" describes the people drinking, rather than how much
  was drunk. In 14:31 "my Lord" is explicit, and the past auxiliary
  belongs to the emphatic saying predicate.
- In 14:42 the first-person object belongs to handing over, rather
  than adding a second "to me" after drawing near.
- In 14:53 Caiaphas is explicitly named. In 14:56 the testimonies are
  plural and do not agree; 14:59 instead has singular collective testimony.
- In 14:70 "also" qualifies the Galilean claim, not the preceding
  statement that Cephas is one of them. The twice/thrice counts in
  14:30 and 14:72 are retained.

Source checks for Mark 15:

- In 15:32 the title is "king of Israel", rather than the local Murdock
  reference's "king of the Jews"; the descent clause is third-person
  volitional, rendered "Let Messiah ... descend".
- In 15:34 the quoted cry and its explicit English interpretation remain
  distinct. In 15:35 the past auxiliary belongs to "they were saying";
  calling for Elijah is a separate past predicate.
- In 15:38 the curtain expression appay tar'a is rendered as an idiom;
  compare [Brock and Kiraz's dictionary entry](https://sedra.bethmardutho.org/lexeme/get/6121).
- In 15:39 "this man" is explicit in the centurion's statement. In 15:43
  taking courage and going in to Pilate remain separate actions.
- In 15:46 Joseph buys a linen sheet, without an added quality adjective;
  the lowering, wrapping, placing, and rolling predicates remain distinct.

Source checks for Mark 16:

- In 16:1 Mary the mother of James and Salome remain separate members
  of the three-woman list. In 16:10 the first past auxiliary belongs to
  those who had been with Jesus, and the second to their mourning.
- In 16:11 the feminine plural saying, recipient, and object forms are
  retained as the reporting women; no individual women are identified
  beyond what the source states. In 16:13 the additive negative is
  rendered "did not believe them either".
- In 16:15 "my tidings" is explicit. In 16:16 the living and condemnation
  predicates retain the source's present formulation.
- In 16:19 and 16:20 "our Lord" retains its first-person possessive.
  In 16:20 the Lord was helping and confirming their words; the signs
  were being performed by the plural actors.

Source checks for Luke 1:

- In 1:2 the ministers serve the word; the anticipatory feminine suffix
  refers forward to that noun. In 1:3 both "all of them" and "everything"
  are retained, as is the distinct ordering phrase "in its order".
- In 1:8 the source names God, rather than the local Murdock reference's
  Lord. In 1:15 the Holy Spirit is explicit, and the temporal phrase is
  "while he is in his mother's womb".
- In 1:28 "Our Lord" and in 1:43 "my Lord" retain their distinct possessives.
- In 1:39 the adverb is rendered "with diligence", following the careful
  and diligent senses in [SEDRA's lexical entry](https://sedra.bethmardutho.org/lexeme/get/368).
- In 1:72 the holy covenants are plural, as are the oaths in 1:73.
- In 1:77 the forgiveness phrase is introduced by "through", rather than
  an added conjunction. In 1:78 the visitation is future and explicitly
  includes its first-person object: "will visit us".

Source checks for Luke 2:

- In 2:9 the angel is explicitly "of God", while the following glory is
  "of the Lord". In 2:10 the announced joy is for all the world.
- In 2:14 the source says "good hope", which is retained alongside peace.
- In 2:22 the purification has a plural possessor, "their". In 2:27 the
  parents are explicit, while 2:41 uses the distinct expression "his people".
- In 2:29 the release request is imperative. In 2:30 the object is
  "your mercy"; in 2:35 the piercing weapon is a spear.
- In 2:36 the seven years with Anna's husband are explicit. In 2:37 the
  eighty-four-year wording is retained without resolving its age/duration
  ambiguity; both fasting and prayer are linked separately.
- In 2:43 and 2:45 the dative following "returned" is idiomatic, without
  an added external recipient. In 2:49 the Father's house is explicit,
  and 2:52 retains both "his stature" and "his wisdom".

Source checks for Luke 3:

- In 3:1 the repeated head/fourth expression is the tetrarch title.
  In 3:4 both the plain and the first-person plural "our God" are explicit.
- In 3:9 the tree must produce good fruits; the negated clause retains
  both the fruit noun and its adjective.
- In 3:14 the second prohibition is rendered "oppress no one"; compare
  [CAL's entry for the verb](https://cal.huc.edu/oneentry.php?cits=all&lemma=%28%24q+V),
  which cites this verse under the oppression sense.
- In 3:16 the source has no separate "after me" phrase. The straps and
  sandals are plural, as are the threshing floors in 3:17; the burning
  predicate in 3:17 is future.
- In 3:22 the final address is second person, "in you", rather than the
  reference's "in whom". In 3:23 no beginning-of-ministry wording is added.
- The genealogy retains John in 3:27 and Aram in 3:33, rather than the
  local reference's Joanna and Ram. The final relation in 3:38 is
  "from God", not a further explicit "son of" phrase.

Source checks for Luke 4:

- In 4:4 the source includes every word of God. In 4:5 Satan is explicit;
  no separate dismissal of Satan is supplied in 4:8. In 4:6 "all" qualifies
  the authority, followed by its glory.
- In 4:18 the final verb is rendered "strengthen", following the
  [dictionary entry for sharrar](https://sedra.bethmardutho.org/lexeme/get/3306).
  The brokenhearted, release for captives, sight for the blind, and the
  final forgiveness phrase are all retained; no sins noun is added.
- In 4:27 the negative and exception mean none was cleansed except Naaman.
  In 4:29 "their city" and the relative clause ending with "on it" stay
  together, before the distinct purpose of casting him from the cliff.
- In 4:34 "Leave me alone" is singular, but the following question uses
  plural "we". In 4:36 the feminine command verb agrees with the word,
  which is its grammatical subject.
- In 4:41 "also" and both Messiah and Son of God are retained. The final
  clause gives the reason for silencing the demons: they knew he was
  Messiah; compare [Etheridge's translation](https://www.studylight.org/bible/eng/jet/luke/4.html).
- In 4:43 the sending verb is first person, "I was sent", and the preceding
  copula emphasizes the purpose. In 4:44 the source names Galilee.

Source checks for Luke 5:

- In 5:3 the boat belongs to Simon Cephas; both the ownership and the
  second name are explicit. The net is singular in 5:4 and 5:6, while
  the fishermen's nets in 5:2 are plural.
- In 5:5 "at your word" introduces Simon's decision to cast the net,
  rather than qualifying the unsuccessful catch. In 5:6 the final
  quantity word intensifies "many" fish, without calling them good fish.
- In 5:13 the two-word hour expression means "immediately", not an address
  to a son. The explicit final statement that he was cleansed is retained.
- In 5:14 the priests are plural. In 5:17 the healing object is "them";
  in 5:20 "Man" is a direct address to the paralytic. In 5:25 glorifying
  God accompanies the journey home, rather than following it.
- In 5:24 the Son of man has authority to forgive sins; the suffix in
  the title anticipates the following genitive, without a separate "his".
- In 5:34 the wedding guests expression and the causative "make fast"
  remain under the negative. In 5:36 the feminine final predicate agrees
  with the patch: it does not fit the old garment. In 5:39 the old wine
  is pleasant, without an added comparative.

Source checks for Luke 6:

- In 6:1 the disciples rub the ears in their hands. In 6:4 the source
  names the bread of the Lord's table; its possessive suffix anticipates
  the Lord, without a separate "his".
- In 6:6 the past auxiliary belongs with teaching before the separate
  man/hand clause. The right hand is explicit. In 6:13 Jesus actively
  names the apostles; 6:14 names Cephas, and 6:16 says Jude son of James.
- In 6:18 people came to hear his word and to be healed; the purpose
  clause is distinct from the following statement that healing was ongoing.
  In 6:26 the source says people, without an added "all".
- In 6:35 the lending command is followed by "do not cut off anyone's
  hope". The kindness clause addresses the evil and then the ungrateful.
- In 6:38 the measure is pressed down and overflowing, without a separate
  shaking action; the recipients' laps are plural. In 6:41 the beam is
  the subject of the passive seeing clause.
- In 6:45 both stores of treasures are plural, and the lips speak from
  the heart's abundance. In 6:46 both titles retain their first-person
  possessive: "My Lord, my Lord".
- In 6:48 foundations are initially plural, while the final foundation is
  singular. In 6:49 the river beats against the house, which immediately
  falls; the two-word hour expression is not part of the river noun.

Source checks for Luke 7:

- In 7:1 all the words are completed in the people's hearing. In 7:6–8
  the centurion's first-person speech retains "My Lord", "my roof",
  "my servant", his own coming to Jesus, and his command over soldiers.
- In 7:12 the dead man is his mother's only son, and his mother is a
  widow. In 7:15 he sits up and begins speaking; Jesus gives him to his
  mother, without a change of actor.
- In 7:18 John's disciples tell him what happened. In 7:26 prophets are
  plural. In 7:27 "my messenger" has a first-person suffix, while "the way"
  has no possessive suffix. The prophet and Baptizer titles in 7:28 and
  the tax collectors in 7:29 are explicit.
- In 7:36 the Pharisee invites Jesus; Jesus then enters and reclines.
  In 7:37 the feminine demonstrative describes "that city", and the
  masculine demonstrative describes "that Pharisee".
- In 7:41 one debtor owes five hundred denarii and the other fifty.
  In 7:42 both debts are forgiven.
- In 7:44 Jesus says he entered the house; in 7:45 the entry verb is
  feminine, referring to the woman. In 7:46 oil for his head and perfumed
  oil for his feet remain distinct.
- In 7:47 the loving verb is perfect, "she has loved much", with the
  explicit causal connector retained. In 7:50 the causative life verb
  is rendered "has given you life".

Source checks for Luke 8:

- In 8:1 the twelve are explicitly his; in 8:3 the women serve "them",
  plural. In 8:4 people come from all the cities, and parables are plural.
- In 8:10 the mystery is singular. In 8:12 the enemy takes the word from
  their heart so that they may not believe and live. In 8:13 their faith
  is temporary; in 8:15 holding the word and producing fruits are distinct.
- In 8:19 the family cannot speak with him because of the crowd. In 8:24
  both calls retain "Our Rabbi". The winds, storms, and sea in 8:25 are
  distinct objects of the command.
- In 8:26 the place name is Gadarenes. In 8:28 the initial question uses
  "we", while the plea ends "do not torment me". In 8:29 the man is bound
  and breaks his bonds; the demon leads him into the wilderness.
- In 8:36 those who saw the healing report it to the visitors. In 8:38
  Jesus dismisses the man before speaking. In 8:39 the command names God,
  while the man's proclamation names Jesus.
- In 8:40 the crowd receives Jesus. In 8:44 the blood flow stops; the
  feminine verb does not describe the woman standing up. In 8:45 both
  Simon Cephas and those with him speak, addressing Jesus as "Our Rabbi".
- In 8:48 "Take courage, my daughter" is explicit, and the causative life
  verb is rendered "has given you life". In 8:50 Jesus addresses the girl's
  father and says she will live. In 8:56 he warns the parents to tell no
  one what happened.

Source checks for Luke 9:

- In 9:1 the twelve are his, while the suffix on "all" anticipates the
  demons. In 9:11–12, 9:16, and 9:18 the crowds are plural. In 9:12 the
  villages are around "us", and the disciples say "we are" in the wilderness.
- In 9:9 Herod says he cut off John's head; the following hearing clause
  remains first person. In 9:20 the title is the Messiah of God, without
  a separate "his" before the genitive.
- In 9:23 both following phrases say "after me". In 9:24 the motive is
  "for my sake"; the following demonstrative resumes the person who loses
  his life. In 9:25 his life is explicit. In 9:26 the Son of man is the
  subject of the second ashamed predicate.
- In 9:31 Moses and Elijah speak about his departure and its completion
  in Jerusalem. In 9:34 they are explicitly named as entering the cloud.
  In 9:35 the Son is beloved.
- In 9:39 the feminine spirit seizes the child; the masculine predicates
  describe the child crying out, gnashing his teeth, and foaming. The
  feminine subject returns for the spirit's departure and crushing him.
- In 9:48 the final demonstrative resumes the least person: "this one
  will be great". In 9:49 the address is "Our Rabbi", followed by "your
  name", "with us", and "after you". In 9:50 both final suffixes are "you".
- In 9:51 the days being fulfilled are distinct from Jesus setting his
  face toward Jerusalem. In 9:52 messengers are plural, without a numeral.
  In 9:54 the address is "Our Lord", and Elijah is explicitly named.
- In 9:56 souls are not to be destroyed, and the causative life verb is
  rendered "to give life". In 9:58 his head is explicit. In 9:61 the
  household idiom does not identify its members as biological sons.

Source checks for Luke 10:

- In 10:1 Jesus sets apart another seventy from his disciples and sends
  them two by two. In 10:3 the disciples are the object of his sending,
  rather than recipients of a separate message.
- In 10:6 their peace rests on the son of peace or returns to them.
  In 10:7 the worker is worthy of his wage; these nouns remain distinct.
- In 10:12 and 10:14 the judgment expressions are comparative: more
  tolerable for one city than another. In 10:13 the conditional context
  gives the time expression its "long ago" sense.
- In 10:16 rejecting the disciples means rejecting Jesus, and rejecting
  him means rejecting the one who sent him. In 10:17 the address is
  "Our Lord". In 10:18 Satan is named; 10:19 instead says the enemy.
- In 10:21 the Holy Spirit is explicit and both addresses say "my Father".
  In 10:23 the eyes have no "your" suffix. In 10:28 "this" is the object
  of "do", followed by "you will live".
- In 10:30 the robbers fall upon the man, and little life remains in him.
  In 10:34 wine precedes oil, and his donkey is explicit. In 10:35 "when"
  begins the return clause after spending anything more; in 10:36 the
  hands of the robbers are explicit.
- In 10:39 Mary listens to the words of "our Lord", while Martha says
  "My Lord" in 10:40. In 10:42 Mary's portion is good, without an added
  comparative, and it will not be taken from her.

Source checks for Luke 11:

- In 11:1 the address is "Our Lord". In 11:2 the prayer explicitly has
  "Our Father in heaven" and the petition for his will on earth as in
  heaven. In 11:3 the bread is "of our need"; in 11:4 sins are forgiven,
  while the following clause names everyone indebted to us.
- In 11:12 `bartha` means egg in this context, despite the incomplete
  supplied glosses. The [Comprehensive Aramaic Lexicon's egg sense](https://cal.huc.edu/cal_entry_web.php?lemma=brh+N)
  gives Syriac textual examples, including a stork egg and a bird from
  its egg. The source word is retained without substituting another form.
- In 11:15 the demonstrative means "this one", the subject casting out
  demons. In 11:19 the sons cast them out "by what means", and those sons
  become the judges. In 11:20 the finger of God is explicit.
- In 11:25 the return is conditional, "if". In 11:26 both comparisons
  retain "worse than". In 11:27 the woman speaks to him, and in 11:28
  he speaks to her. In 11:29 Jonah's prophet title is explicit.
- In 11:34 the eye is upright/simple, paired with evil. In 11:36 the
  second illumination predicate begins after the dark-part clause.
- In 11:37 Jesus enters and reclines after the invitation. In 11:38 the
  washing precedes the meal. In 11:41 what is available is given as alms,
  and the final clean predicate is present.
- In 11:44 scribes, Pharisees, and hypocrites are explicit. In 11:46 one
  of their fingers remains part of the accusation. In 11:49 persecution
  precedes killing in the text; in 11:51 temple precedes altar.
- In 11:52 keys are plural, and taking, entering, and hindering are past.
  In 11:53 Jesus is actively saying these things; Pael `tks` means
  contradict/rebut, as documented in the [CAL entry](https://cal.huc.edu/cal_entry_web.php?lemma=tks+V).
  In 11:54 the final two-word idiom means accusing him, rather than eating.

Source checks for Luke 12:

- In 12:2 both covered and hidden things are explicit. In 12:3 speech
  in darkness is heard in the light, while whispering in chambers and
  ears is proclaimed on rooftops. In 12:4 the speaker says "I" and
  addresses "my friends".
- In 12:7 the sparrows belong to the value comparison, not the fear
  prohibition. In 12:10 both forgiveness clauses retain their recipient.
  In 12:11 the defense expression is rendered as a coherent idiom.
- In 12:15 Jesus explicitly addresses his disciples. In 12:16 the land
  yields produce. In 12:19 the rich man's speech remains first person,
  "I will say to my soul"; the requirement of his soul in 12:20 does not
  identify the unspecified requiring agents.
- In 12:25 the measure is a cubit of stature. In 12:28 the grass falls
  into the oven. In 12:30 "you too need" preserves the source's emphasis
  on the disciples' need, rather than adding "also" to the Father's knowing.
- In 12:35 the loins are girded and the lamps shine. In 12:36 the source
  specifies a feast. In 12:37 the master girds himself, has the servants
  recline, and serves them. In 12:39 the final past auxiliary belongs to
  not allowing the break-in.
- In 12:41 Simon Cephas says "Our Lord" and asks whether the parable is
  for "us". In 12:47 readiness is according to the master's will; no
  separate doing verb is supplied. In 12:49 the fire's kindling is wished
  for, while 12:50 retains the baptism and pressure until completion.
- In 12:53 all six family relationships retain their possessives,
  including mother-in-law and daughter-in-law. In 12:54 the crowds are
  plural and the cloud rises in the west; in 12:55 the wind is southern.
  In 12:57 the judgment concerns truth.
- In 12:58 "make a settlement" is a contextual legal rendering of the
  commercial expression; [CAL lists its noun as merchandise/business dealings](https://cal.huc.edu/browseSKEYheaders.php?first3=%22t%22).
  The judge and officer remain distinct actors. In 12:59 the speaker
  says "I", and the payment ends with the last coin.

Source checks for Luke 13:

- In 13:4 the comparison uses "sinners". In 13:3 and 13:5 the warnings
  address the whole audience; 13:5 explicitly says
  "like them".
- In 13:6–9 the fig tree belongs to a man and is planted in his vineyard.
  The three years, cultivation, and manure remain explicit. The positive
  fruit condition in 13:9 is elliptical; "next year" belongs to the
  following cutting-down clause.
- In 13:11 the infirmity has lasted eighteen years. In 13:13 his hand
  is singular. In 13:15 the address is the singular "Hypocrite", and the
  animal-release example is a negative rhetorical question. In 13:16
  the source names the Accuser and repeats the eighteen years.
- In 13:17 the opponents are ashamed, while all the people rejoice at
  the wonders. In 13:19 the bird is singular and the tree is large.
  In 13:21 all three measures of flour are leavened.
- In 13:23 the asking verb has an explicit "him" suffix. In 13:25 both
  calls say "Our Lord"; the master's answer is followed by his own
  first-person "I tell you". In 13:26 the visitors ate and drank, but
  the master is the subject of "you taught".
- In 13:27 the phrase is "workers of falsehood". In 13:29 the direction
  order is east, west, south, north. In 13:32 the fox is "this"; the
  completion predicate does not supply an additional work or death noun.
- In 13:33 today's and tomorrow's activity is distinct from going on
  another day. In 13:34 the sent ones go "to her", before the address
  shifts to "your children". In 13:35 the house is left "to you", and
  the final coming is in the name of the Lord.

Source checks for Luke 14:

- In 14:1 the leaders and Pharisees are plural. In 14:2 the gathered-water
  expression describes fluid retention. In 14:5 the rescue example says
  "son or ox", and both drawing out and bringing up remain explicit.
- In 14:6 the answer belongs to the give-him clause. In 14:7 the seeing
  clause retains its past auxiliary. In 14:8 the invitation is to a feast;
  no wedding is specified. In 14:9 the place has no possessive modifier.
- In 14:10 the address is "My friend", and moving higher is followed by
  reclining. In 14:12 the invitation warning includes "not even" the
  brothers, and the final demonstrative refers to the repayment.
- In 14:15 one of those reclining hears these things and speaks to him.
  In 14:17 everything is ready "for you". In 14:18 all begin "as one";
  the field's buyer is obliged to go, rather than the field compelling him.
- In 14:19 the purchase is five pairs of oxen. In 14:20 taking a wife is
  the marriage idiom. In 14:22 the command has been fulfilled. In 14:23
  the route goes between the hedges. In 14:24 the negative covers every
  previously invited man, rather than only one member of the group.
- In 14:26 "hate" is retained, and the family order is father, mother,
  brothers, sisters, wife, children, then the person's own life. In 14:28
  the builder calculates its cost and considers resources for completion.
- In 14:29 the introductory connector is "lest"; the future mocking
  predicate supplies no beginning verb. In 14:31 the opposing ruler is
  a fellow king, with ten thousand facing twenty thousand. In 14:33 all
  the person's possessions remain explicit.
- In 14:35 the first predicate concerns suitability for soil or manure;
  "outside" belongs to throwing it away. The hearing exhortation retains
  both having ears and hearing.

Source checks for Luke 15:

- In 15:1 the approach is ongoing. In 15:2 scribes precede Pharisees,
  and the murmuring has a past auxiliary; the demonstrative identifies
  the one receiving sinners, rather than making a sinner the subject.
- In 15:4 the question retains a hundred sheep, one lost, ninety-nine
  left in the open country, and seeking until finding. In 15:6 the
  speaker calls his friends and neighbors concerning his own lost sheep.
- In 15:7 the comparison concerns one repenting sinner and ninety-nine
  righteous people who need no repentance. In 15:8 the woman has ten
  coins, lights a lamp, sweeps, and searches carefully. In 15:9 her friends
  and neighbors are grammatically feminine; 15:10 names the angels of God.
- In 15:11 Jesus speaks to them again. In 15:12 the portion comes
  "from your house", and the father divides his property among them.
  In 15:13 the younger son is the subject gathering what fell to him,
  not an object gathered by someone else.
- In 15:15 the sons-of-the-city expression means citizens; the employer
  sends the son to tend pigs; "the latter" clarifies that subject change.
  In 15:16 the food is carob pods; the longing, pigs' eating, and
  absence of giving all have past auxiliaries.
- In 15:17 "now", "here", and the speaker's hunger remain explicit.
  In 15:19 he says "no longer worthy"; in 15:21 he says "not worthy"
  without repeating the petition to become a hired servant. In 15:20
  the father sees him, has compassion on him, runs, embraces, and kisses him.
- In 15:22 the robe is the best; no quickly adverb is supplied. In 15:23,
  15:27, and 15:30 the animal is a fattened ox. In 15:25 the elder son
  hears many singing, with no dancing term in this source.
- In 15:29 both never clauses remain, together with the labor/service
  expression. In 15:31 the first clause concerns the son always being
  with his father; the second says everything the father owns is the son's.
  In 15:32 the vocalized suitability predicate means it was right for
  them to rejoice, rather than being a negation.

Source checks for Luke 16:

- In 16:1 the accusations are brought to the rich man against the
  steward; the two pronominal references remain distinct. In 16:2 he
  can no longer be the master's steward. In 16:4 leaving the stewardship
  and being received into their houses remain separate clauses.
- In 16:6 the oil debt is a hundred measures, changed to fifty, with
  sitting down quickly explicit. In 16:7 the wheat debt is a hundred
  cors, changed to eighty; sitting is repeated without the quickly adverb.
- In 16:8 the subject is explicitly "our Lord". The generation phrase
  retains both "this" and "theirs". In 16:9 it is the mammon that comes
  to an end, and the eternal shelters belong to the receiving group.
- In 16:11 truth is what is entrusted, and the disciples are recipients.
  In 16:12 the first property is "not yours", distinct from the final
  "your own"; the negative about being found faithful is also retained.
- In 16:14 both loving money and mocking have past auxiliaries. In 16:15
  hearts are plural. In 16:16 everyone presses to enter the kingdom.
  In 16:17 the measure is one letter, and `pshiq` means easy in the
  comparative construction, despite the supplied gloss list; see
  [CAL's easy sense and adjectival reference](https://cal.huc.edu/cal_entry_web.php?lemma=p%24q+V).
- In 16:19 the garments are linen and purple, in that order. In 16:20
  Lazarus lies at that rich man's gate, afflicted with ulcers. In 16:21
  the longing and dogs' coming retain their past auxiliaries, and the
  new clause begins "but even" the dogs.
- In 16:23 the place is Sheol. In 16:24 the request concerns the tip of
  Lazarus's finger and the speaker's tongue. In 16:25 the good and bad
  things retain their possessors, followed by "now", "behold", and "here".
- In 16:26 the chasm has been placed between us and you, and passage is
  denied in both directions. In 16:27 "I ask you" includes the recipient;
  in 16:28 the five brothers are warned lest they too come. In 16:30
  the visitor comes from the dead; in 16:31 the final clause explicitly
  says they will not believe "him", preserving its pronominal complement.

Source checks for Luke 17:

- In 17:1 both negatives are retained in the impossibility of offenses not
  coming. In 17:2 the millstone is associated with a donkey. In 17:4 the
  quoted "I repent" ends before the command to forgive; both seven-times
  expressions and both day references remain explicit.
- In 17:5 the apostles address "our Lord". In 17:6 the tree is a mulberry,
  and the condition compares faith to a grain of mustard. In 17:7 the
  servant drives a plough or tends a flock. In 17:8 he prepares supper,
  girds his loins, and serves before he too eats and drinks.
- In 17:9 `qbl` with the favor/thanks noun is an idiom meaning thank;
  [CAL documents this Syriac combination under D 2(c)](https://cal.huc.edu/getlex.php?coord=5100104111&word=8).
  The question concerns a master thanking the servant.
  In 17:10 the final clause states the obligation without adding "only".
- In 17:11 the ethnic term is plural Samaritans, followed by the directional
  prefix on Galilee; the rendering follows the among-Samaritans route
  toward Galilee. In 17:12 ten leprous men meet him and stand at a distance.
- In 17:15 the man turns back and "was praising God"; the dative after
  turning is rendered naturally without adding a receiver. In 17:16
  "this man" was a Samaritan. In 17:17 both ten and nine remain explicit.
- In 17:18 `prsh` has the departure sense, as in
  [CAL's Syriac examples under G 3](https://cal.huc.edu/cal_entry_web.php?lemma=pr%24+V).
  The question follows Murdock's contextual gone-away/without-coming
  construction, with this man from a foreign nation as the exception.
  The opening `lma` introduces the question; it does not require "why"
  here, as discussed for this [interrogative particle](https://eprints.glos.ac.uk/3339/1/292355.pdf),
  p. 147, citing Payne Smith and Nöldeke. In 17:19 the faith has given him life.
- In 17:20 the Pharisees ask Jesus, and the kingdom does not come with
  observation. In 17:21 it is "within you". In 17:22 the requested sight
  is one of the days; no plural object is supplied to the final seeing
  verb. In 17:23 no following-them command is added. In 17:24 lightning
  flashes from heaven and illuminates all beneath heaven.
- In 17:27 taking wives and giving women to husbands preserve the active
  marriage expressions. In 17:28 planting has its own past auxiliary.
  In 17:29 the Lord rains fire and sulphur, destroying them all. In 17:31
  both prohibitions remain, along with his belongings and taking them.
- In 17:33 both life nouns and their following object suffixes remain.
  In 17:34 the time is night and the bed is one; 17:35 has two women,
  and the field scene in 17:36 is present in this Peshitta source.
  In 17:37 the address is "our Lord", and the eagles gather at the body.

Source checks for Luke 18:

- In 18:1 the parable is also told to them, without an added demonstrative;
  the final dative belongs to growing weary, not being weary of them.
  In 18:3 the widow was coming to him in that city. In 18:5 tiring the
  judge and subsequently hurting him use distinct verbs.
- In 18:6 the subject is "our Lord", and the judge "said" the preceding
  words. In 18:7 the greater vindication, the chosen ones calling by day
  and by night, and God's patience with them remain distinct. The patience
  expression is the long-spirit idiom. In 18:8 vindication is quick,
  and the faith question retains the emphatic question particle.
- In 18:9 he was telling this parable against people confident in themselves.
  In 18:11 the Pharisee was standing and was praying; his claim is
  "I have not been" like others. In 18:12 he fasts twice a week and
  tithes everything he acquires. In 18:13 the tax collector was beating
  his own breast, and his prayer names himself as a sinner.
- In 18:14 the comparative explicitly has more/than, with justified
  belonging to the tax collector's descent to his house. The comparison
  names that Pharisee. The two following maxims retain their reflexive
  subjects and opposite humbling/exalting results.
- In 18:15 they were also bringing infants for Jesus to touch them;
  the disciples are his. In 18:16 the kingdom is of heaven, while in
  18:17 it is of God. In 18:18 one of the chiefs asks him, then speaks
  to him. In 18:20 killing precedes adultery, stealing, false testimony,
  and honoring father and mother.
- In 18:21 the youth is the speaker's own. In 18:22 he is to sell all,
  give to the poor, and come after Jesus; no cross clause is supplied.
  In 18:23 he is sorrowful without an added departure; in 18:24 Jesus
  sees his sorrow. In 18:25 the comparison uses a camel and a needle's
  eye. In 18:26 the question asks who can live; 18:28 names Simon Cephas.
- In 18:29 houses are plural, and the family order is parents, brothers,
  wife, children. In 18:30 the return is many times more in this time,
  and eternal life in the world to come. In 18:31 Jesus takes his twelve,
  and all that is written in the prophets concerning the Son of Man
  will be fulfilled.
- In 18:32 they spit in his face. In 18:33 scourging, treating with
  contempt, and killing each retain their object before his rising on
  the third day. In 18:34 not one of these things was understood; this matter
  was hidden from them, and they did not know what was spoken with them.
- In 18:35 the clause about nearness to Jericho has a singular subject.
  In 18:36 the asking has a past auxiliary. In 18:39 the people were going before
  Jesus and were rebuking the blind man, who was crying out all the more.
  In 18:40 Jesus commands him to be brought to him. In 18:41 the address
  is "My Lord"; in 18:42 his faith has given him life. In 18:43 immediate
  sight is followed by ongoing following, praise, and the people's giving
  glory to God.

Source checks for Luke 19:

- In 19:2 Zacchaeus is rich and chief of the tax collectors. In 19:3
  the stature clause repeats his name. In 19:4 he runs ahead of Jesus
  and climbs the sycamore fig tree to see him. In 19:5 the suitability
  predicate means he must be in Zacchaeus's house today, not a negative.
- In 19:7 they were murmuring; the final expression concerns entering
  to lodge with a sinful man. In 19:8 half of his possessions go to
  the poor, and the repayment is fourfold. The source's "one" and "four"
  belong to that multiplier, not an additional defrauded person.
  In 19:9 life comes to this house, and this one also is Abraham's son.
- In 19:10 seeking and giving life remain distinct. In 19:11 they were
  hearing and supposing; "at that hour" belongs to the expectation that
  the kingdom was about to be revealed, not to an hour being ready.
  In 19:12 the nobleman receives kingship for himself and returns.
- In 19:13 ten of his servants receive ten minas. In 19:15 the servants
  are called to him, and the inquiry concerns what each had gained by
  trading. In 19:16–19 the gains are ten and five minas, with authority
  over ten and five towns. The first servant was found faithful.
- In 19:20 the mina was with the servant, laid up in linen cloth;
  the form rendered "laid up" is a passive participle. In 19:22
  "You knew me" keeps its object. In 19:23 the table has the financial
  sense documented in [CAL's money changer's table entry](https://cal.huc.edu/getlex.php?coord=8100124004&word=2),
  and the demand includes its interest. In 19:25 the address is "our lord".
- In 19:24–26 possession is rendered naturally from the with-him and
  to-him constructions. In 19:26 the recipient after giving resumes
  "everyone who has"; no different receiver is introduced. In 19:27
  the enemies belong to the parable's ruler, who orders their execution
  before him. No additional "here" is supplied.
- In 19:29 the mountain is called the place of Olives, with two of his
  disciples sent. In 19:30 the village is opposite us, not you; the colt
  has never been ridden. In 19:31 and 19:34 the title is "our Lord".
  In 19:33 the colt's owners are plural; in 19:34 the reply is to them.
- In 19:35 the garments are theirs, and Jesus is set on the colt. In
  19:36 they were spreading the garments. In 19:37 the descent is of
  the Mount of Olives, the entire crowd of disciples begins to praise,
  and all the mighty deeds are retained. In 19:38 they were saying,
  with peace in heaven and glory in the heights. In 19:39 crowds are plural.
- In 19:42 both "this" and "your" belong to the day; the things pertaining
  to peace are hidden from the city's eyes. In 19:43 the enemies surround
  and press from every side, without an added rampart clause. In 19:44
  the children are within the city, and no stone upon stone is left in it;
  the time is that of its visitation.
- In 19:45 both buying and selling occur in the temple. In 19:46 the
  house is a house of prayer, without a verb meaning call. In 19:47
  the chief priests, scribes, and elders seek to destroy him. In 19:48
  both doing something to him and hearing him retain their object suffixes.

Source checks for Luke 20:

- In 20:1 the chief priests, scribes, and elders confront Jesus while
  he is teaching and preaching. The English passive preserves those roles
  while keeping the source links in order. In 20:4 John's baptism is
  contrasted as from heaven or from people.
- In 20:5 the leaders were reasoning among themselves. In 20:6 the
  whole people would stone them, because the people are persuaded that
  John was a prophet. In 20:8 Jesus retains the first-person refusal
  and the first-person doing of these things.
- In 20:9 the vineyard is leased to cultivators, and the owner goes
  away for a long time. In 20:10 and 20:11 the servants belong to him;
  the second is another servant of his. In 20:12 the cultivators, not
  the sender, wound the third servant and cast him out.
- In 20:13 the beloved son is the owner's own, and the hoped-for
  response is respect. In 20:14 the cultivators were reasoning, call
  the son the heir, and propose killing him so the inheritance will
  be theirs. In 20:16 the owner gives the vineyard to others.
- In 20:17 the rejected stone becomes the chief cornerstone. In 20:18
  falling on the stone and the stone falling on someone remain distinct;
  the final verb is rendered with its scatter/winnow sense. In 20:19
  the attempt is at that hour, and the parable is against the leaders.
- In 20:20 the spies feign righteousness and seek to catch him in a word,
  with delivery to the judge and the governor's authority as their aim.
  In 20:21 taking faces is the idiom for partiality. In 20:22 the question
  concerns the head tax. In 20:25 Caesar's and God's respective claims
  and recipients remain distinct; in 20:26 the answer is singular.
- In 20:28 the man who dies has the wife without children, and his brother
  is to take her and raise offspring for his brother. In 20:30 the second
  takes her for his wife, then this one dies. In 20:31 the third and all
  seven share the same outcome. In 20:33 all seven taking her belongs to
  the explanatory clause, not to the singular wife predicate.
- In 20:34-35 taking wives and wives being given to husbands render the
  source's marriage idioms. The resurrection is from among the dead.
  In 20:36 inability to die again, likeness to angels, children of God,
  and children of the resurrection remain distinct; the last copula is
  rendered in the surrounding present characterization.
- In 20:37 the dead rising is explicit, Moses shows it, and the Lord
  is named as the God of Abraham, Isaac, and Jacob. In 20:38 all live
  to him. In 20:41 the scribes' claim is that the Messiah is David's son.
  In 20:42 both speech verbs remain, with my Lord and my right hand;
  in 20:43 the speaker says I will place your enemies under your feet.
- In 20:44 David calls him my Lord. In 20:45 the disciples are his.
  In 20:46 robes have no added long adjective, while marketplaces,
  synagogues, and suppers are plural. In 20:47 widows' houses, the
  pretext of prolonging their prayers, and greater condemnation remain.

Source checks for Luke 21:

- In 21:1 the rich were putting their offerings into the treasury.
  In 21:3 the comparison concerns this poor widow. In 21:4 all these
  people contribute from their surplus, while this woman gives from
  her poverty everything she possessed; both demonstratives remain.
- In 21:5 some people were speaking about the adorned temple. In
  21:7 they were questioning him about the timing and sign. In 21:8
  the claim explicitly includes the Messiah, with my name and the
  warning not to follow them. In 21:9 these things happen first.
- In 21:11 the earthquakes, famines, plagues, terrors, fearful events,
  great signs seen from heaven, and great storms all remain. In
  21:12 the delivery to councils and prisons precedes being brought
  before kings and governors, because of my name.
- In 21:14 the answer/defense expression is a compound idiom; its
  component spirit word does not introduce a separate actor or means.
  In 21:15 the gift is a mouth and wisdom which none of their enemies
  can withstand. In 21:16 parents, brothers, relatives, and friends
  hand them over, and some of them are put to death.
- In 21:18 even a hair of their head will not perish. In 21:19 endurance
  preserves their lives. In 21:20 an army surrounds Jerusalem, and the
  impending desolation is the city's own. In 21:21 the mountain is
  singular, and those in the countryside must not enter the city.
- In 21:22 all that is written is to be fulfilled. In 21:23 the
  demonstrative belongs to those days, not to the nursing clause;
  distress in the land and wrath against this people remain distinct.
  In 21:24 captivity goes to every place, while Jerusalem is trampled
  by the Gentiles until their times are fulfilled.
- In 21:25 the distress of nations and the wringing of hands have
  distinct source spans. Amazement concerns the sea's roar; no waves
  clause is supplied. In 21:26 agitation drives out people's souls
  from fear of what is to come, and heaven's powers are shaken.
- In 21:27 the clouds are plural. In 21:28 deliverance belongs to the
  hearers. In 21:29 Jesus told the parable. In 21:30 the sprouting
  trees supply the inference about summer. In 21:31 all these things
  happening indicate the nearness of God's kingdom.
- In 21:33 the source has my words, plural, rather than Murdock's
  singular word; the possessive is first person. In 21:34 lest scopes
  both hearts being weighed down and that day arriving suddenly.
  In 21:35 the snare falls on all inhabitants of the whole earth.
- In 21:36 the prayer concerns being worthy to escape and standing
  before the Son of Man. In 21:37 the past auxiliaries accompany
  teaching and going out to lodge at the mountain called the Place
  of Olives. In 21:38 the people were coming early to hear his word,
  with an explicit singular object and third-person possessive.

Source checks for Luke 22:

- In 22:1 the feast of unleavened bread is called Passover. In 22:2
  the priests and scribes were seeking to kill him and feared the people.
  In 22:4 both scribes and temple commanders are named. In 22:6 Judas
  was seeking an opportunity to hand him over without the crowd.
- In 22:8 Jesus sends Cephas and John to prepare the Passover for us.
  In 22:11 the message says our Rabbi and my disciples. In 22:12 the
  furnished upper room is shown, with preparations to be made there.
  In 22:14 Jesus comes and reclines with the twelve apostles.
- In 22:15 the desire is emphatic and precedes suffering. In 22:16
  he will not eat it from now on until fulfillment in God's kingdom.
  In 22:17 taking the cup, thanksgiving, and division among the disciples
  are all present in the source. In 22:19 my body, given for you, and
  remembrance of me retain their possessive and beneficiary relations.
- In 22:20 the cup is explicitly the topic after they have eaten supper;
  the covenant is new, the blood is mine, and it is poured out for you.
  In 22:21 the betrayer's hand is on the table without an added with me.
  In 22:22 the appointed course and the man through whose hand the Son
  of Man is handed over remain distinct. In 22:24 the dispute is about
  which of them is greatest.
- In 22:25 the kings are lords over the nations, and those exercising
  authority are called benefactors. In 22:26 greatness is contrasted
  with being least and leadership with service. In 22:29 the kingdom
  is promised to you as my Father promised to me; in 22:30 eating and
  drinking, sitting on thrones, and judging twelve tribes all remain.
- In 22:31 Satan's sifting object is plural you, while the prayer,
  faith, turning, and strengthening command in 22:32 address Simon
  in the singular. The turning clause is imperative, not a subordinate
  when-you-have-turned clause. In 22:33 the address is my Lord; in
  22:34 Jesus names Simon, with three denials before today's crowing.
- In 22:35 purses, wallets, and sandals are plural. In 22:36 the person
  without a sword sells his own garment and buys himself a sword.
  In 22:37 I will be counted is first-person future; the final completion
  perfect is rendered prospectively in the context of the impending
  scriptural fulfillment. In 22:38 the address is our Lord, two swords
  are present, and the answer says they are sufficient.
- In 22:39 the mountain is of the Place of Olives, with his disciples
  following. In 22:41 kneeling precedes ongoing prayer. In 22:42 the
  direct address is Father, followed by my will and yours. In 22:44
  fear and earnest ongoing prayer precede the sweat like drops of blood
  falling to the ground. In 22:47 Judas's prior sign explicitly identifies
  whomever I kiss; in 22:48 the kiss belongs to its own source span.
- In 22:49 those with him are the people who see and ask our Lord about
  striking. In 22:51 the ear belongs to the person who was struck, while
  the feminine healing suffix refers to the ear: healed it. In 22:52
  chief priests, elders, and temple captains come against him. In 22:54
  Simon is the subject of ongoing following, and in 22:55 he too was
  sitting among the people around the fire.
- In 22:57 the denial uses present knowing, despite Murdock's perfect.
  In 22:58 Cephas belongs to the following speech clause. In 22:61 Jesus,
  Cephas, Simon, and our Lord's word retain their distinct references.
  In 22:63-65 mocking, blindfolding, striking his face, and uttering
  blasphemies retain their ongoing past forms. In 22:68 answering and
  releasing him are distinct. In 22:69 the right hand is of God's power;
  in 22:71 they have heard from his mouth.

Source checks for Luke 23:

- In 23:2 the accusation concerns our people, head-tax money for Caesar,
  and the claim to be king Messiah. The English forbidding construction
  absorbs the source negative without reversing the tax accusation.
  In 23:5 the accusers were crying out; Judea, Galilee, and here remain.
- In 23:6 Pilate hears the name Galilee. In 23:8 Herod's long desire,
  hearing many things, and hope of seeing a sign remain distinct.
  In 23:9 Herod was asking questions, while Jesus gives no answer.
  In 23:10 the priests and scribes were standing and accusing him.
- In 23:11 the source has plural scarlet garments, not Murdock's purple
  robe. In 23:13 Pilate calls the chief priests, rulers, and people;
  the people are coordinated with the rulers, not their genitive.
  In 23:15 Pilate says I sent him to him, and no death-deserving act
  has been done by Jesus, rather than an act being done to him.
- In 23:17 releasing one at the festival is customary. In 23:19 riot
  in the city and murder explain the imprisonment. In 23:23 both the
  crowd's and the chief priests' voice prevail; the source's ending
  is retained despite its absence from the supplied Murdock reference.
  In 23:25 the requested prisoner is released, while Jesus is handed
  over to their will; Jesus belongs to that second clause.
- In 23:26 Simon is the Cyrenian coming from the countryside, with
  the cross laid on him to carry after Jesus. In 23:27 the women were
  mourning and wailing over him. In 23:28 the daughters of Jerusalem
  are to weep for themselves and their children. In 23:30 the future
  verb addresses you in the plural: you will begin to say.
- In 23:32 two other wrongdoers accompany him to be killed, rather
  than importing Murdock's crucified into that verse. In 23:33 their
  actual crucifixion is at the place called Skull, one on each side.
  In 23:34 Jesus said Father, without a possessive suffix;
  his garments are divided, and a lot is cast over them. The perfect
  with its auxiliary is rendered as simple past, as also corrected
  in 21:29; see [Younansardaroud, section 11.6](https://d-nb.info/1184881278/34#page=107).
- In 23:35-37 the people watch while rulers and soldiers mock him;
  the saving verbs preserve their respective objects and reflexives.
  In 23:38 the inscription is above him in Greek, Latin, and Hebrew,
  and reads this is the king of the Jews. In 23:39 a single wrongdoer
  is reviling him among the plural men crucified with him.
- In 23:41 just punishment, deserved recompense, and the other man's
  innocence remain distinct. The passive participle with the l-dative
  permits the active perfect he has done; see [Younansardaroud, section 11.5](https://d-nb.info/1184881278/34#page=107).
  In 23:42 the address is my Lord, and the kingdom is yours. In 23:43
  today's promise places the addressee with Jesus in paradise.
- In 23:44 the darkness lasts from about the sixth to the ninth hour.
  In 23:45 the temple-door expression is the dictionary idiom for
  curtain, documented in [Brock and Kiraz's SEDRA entry](https://sedra.bethmardutho.org/lexeme/get/6121).
  In 23:46 the address is my Father, your hands are plural, and the
  committed spirit is mine. In 23:48 the crowds are plural; in 23:49
  acquaintances and women stand at a distance and were watching.
- In 23:50 Joseph is a councillor from Ramath, a city of Judea, and
  is good and righteous. In 23:51 he had not agreed to their decision
  and deed and was awaiting God's kingdom. In 23:53 the body is put
  in a hewn tomb where no one had yet been laid. In 23:55 the women
  see how his body is placed. In 23:56 spices and ointments remain
  distinct, followed by Sabbath rest as commanded.

Source checks for Luke 24:

- In 24:1 the day expression is the first day of the week, not a first
  Sabbath. Dawn and continuing darkness remain, with the prepared spices
  and other women accompanying them. The preparation auxiliary belongs
  to preparing the spices, before the other women are introduced.
- In 24:4 the two men stand opposite the women, with gleaming clothing.
  In 24:5 the masculine speakers address the frightened women, rather
  than the women asking the resurrection question. In 24:7 delivery
  into sinful men's hands, crucifixion, and third-day rising all remain.
- In 24:10 the named women include Mary Magdalene, Joanna, and Mary the
  mother of James, together with the others. In 24:11 the apostles do
  not believe them. In 24:12 the name is Simon, and the linen sheets
  are lying alone. In 24:13 Emmaus is sixty stades from Jerusalem;
  the source unit is retained without a modern distance conversion.
- In 24:14 the two were speaking with one another. In 24:15 Jesus
  reaches them and was walking with them. In 24:16 their held eyes
  prevent their recognition of him. In 24:18 Cleopas answers and then
  speaks; the Jerusalem stranger expression is rendered idiomatically
  in its conversational setting, retaining the question's addressee.
- In 24:19 Jesus is from Nazareth, a prophet mighty in word and deed
  before God and all the people. The suffix on all resumes the people;
  it does not introduce a possessive his people. In 24:20 the chief
  priests and elders deliver him to a death sentence and crucify him.
  In 24:21 they were hoping for Israel's deliverance, with three days
  elapsed since these things occurred.
- In 24:23 the women report angels and the statement that he is alive.
  In 24:24 the final masculine pronoun begins the following clause,
  but him they did not see; it is not a recipient of the women's report.
  In 24:25 deficient understanding and a heavy heart are distinct.
  In 24:27 he began from Moses and was explaining the Scriptures.
- In 24:28 he makes them think he is going to a distant place. In
  24:29 the day is inclining toward darkness. In 24:30 the blessing
  has no added object. In 24:31 recognition and withdrawal are distinct.
  In 24:32 the recorded source is `יקיר`, heavy, with singular our heart;
  see [CAL's adjective entry](https://cal.huc.edu/cal_entry_web.php?lemma=yqyr+A).
  The supplied Murdock reference uses burn. The English follows the
  actual recorded form, together with speaking and explaining to us.
- In 24:33-34 the assembled eleven and those with them are saying that
  our Lord has risen and appeared to Simon. In 24:35 the two describe
  his recognition at the breaking of bread. In 24:36 the full greeting
  includes peace, it is I, and the command not to fear.
- In 24:39 my hands, my feet, and I myself retain their first-person
  references, with flesh and bones in the comparison. In 24:42 broiled
  fish and some honeycomb are both present; honeycomb naturally renders
  the comb-of-honey expression. In 24:43 he eats before their eyes.
  In 24:44 the law of Moses, prophets, and psalms are all named; in
  24:45 mind is a singular collective and its opening is for understanding.
- In 24:46 fittingness has the past auxiliary, unlike the necessity
  expression in 24:44. In 24:47 repentance for forgiveness is proclaimed
  in his name to all nations, beginning from Jerusalem. In 24:49 I will
  send is future, my Father is possessive, and the city is Jerusalem.
  In 24:50 Bethany and his lifted hands remain; blessing, separation,
  ascent, worship, return, continual praise, and Amen complete 24:51-53.

Source checks for John 1:

- In 1:1 the Word's being with God and being God retain their distinct
  predicates. In 1:3 both finite predicates and the final relative about
  coming into being remain. Agency is expressed by the hand idiom. In
  1:4 life is in him and is humanity's light; in 1:5 the light shines,
  while the darkness does not comprehend it.
- In 1:6 John is sent from God. In 1:7 the source explicitly includes
  the purpose that everyone might believe through him, although this
  clause is absent from the supplied Murdock reference. In 1:8 John is
  not the light and comes to testify about it. In 1:9 the true light
  enlightens everyone coming into the world, with for and the past copula.
- In 1:10 the world comes through his hand and does not know him.
  In 1:11 his own do not receive him. In 1:12 those receiving him are
  given authority to become God's children. In 1:13 the plural birth
  is from God, contrasted with blood, flesh's desire, and man's desire;
  the birth predicate remains with its actual final source position.
- In 1:14 the Word becomes flesh and dwells among us; we see his glory.
  Only-begotten, from the Father, and full of grace and truth remain
  distinct. In 1:15 the coming-after, being-before, and prior-to-me
  expressions keep their respective first-person references. In 1:16
  we all receive from his fullness, and grace in place of grace.
- In 1:17 the source order is truth and grace, after the law given
  through Moses' hand; they come through Jesus the Messiah's hand.
  In 1:18 the recorded title is only-begotten God. He is in his Father's
  bosom and makes the unseen God known, with the understood object
  supplied naturally in English.
- In 1:19 the Jews from Jerusalem are the senders, priests and Levites
  the delegates, and John the recipient of their questions. The full
  sending clause is linked together to preserve those roles in English.
  In 1:20 confession and non-denial remain explicit. In 1:21 the two
  questions concern Elijah and a prophet, each with its own denial.
- In 1:23 the voice is calling in the wilderness and commands making
  the Lord's way smooth, as Isaiah said. In 1:24 the sent men are from
  the Pharisees. In 1:26 the standing one is among you but unknown to
  you. In 1:27 John is unworthy to loosen the straps of his sandals.
- In 1:28 Bethany is across the Jordan; the location phrase is present
  in the source despite its absence from the supplied Murdock reference.
  In 1:29 the Lamb bears the world's sin, singular. In 1:31 the purpose
  is that Jesus be made known to Israel, with John's coming to baptize
  as the means. In 1:32 the Spirit descends like a dove and remains on
  him; John is the witness, not the one who remains on Jesus.
- In 1:33 the sign is the person on whom the Spirit descends and remains,
  identifying the one baptizing with the Holy Spirit. In 1:34 John's
  seeing and testimony concern the Son of God. In 1:38 the address is
  our Rabbi. In 1:39 Jesus' lodging and the two disciples' being with
  him are separate clauses, followed by that day and the tenth hour.
- In 1:40 the hearing is from John and the following is after Jesus;
  Andrew is Simon's brother. In 1:41 the source verb is saw. In 1:42
  Simon is son of Jonah and will be called Cephas, without an added
  Greek name gloss. In 1:44 Bethsaida is the city of Andrew and Simon.
  In 1:45 we have found stays at its actual source position, with
  Jesus, Joseph, and Nazareth distinct. In 1:51 the future audience is
  plural, with God's angels ascending and descending toward the Son
  of Man; both introductory amens remain.

Source checks for John 2:

- John 2:1 renders משתותא as a wedding feast and preserves Cana as a city of Galilee.
- John 2:3 retains the source's explicit “they have no wine” negative and the mother's address to Jesus.
- John 2:4 preserves the question “What is that to me and to you, woman?” and the not-yet-arrived hour.
- John 2:6 preserves six stone water jars, their Jewish purification use, and the two-or-three-measure capacity.
- John 2:9 distinguishes the master’s ignorance of the wine's source from the attendants' knowledge and their having filled them with water, without adding a causal conjunction or an unlicensed “jars.”
- John 2:10 renders the source drunk predicate directly and retains the source “then” before the inferior wine.
- John 2:15 preserves the reflexive dative in “he made himself a whip,” the singular cord, the expulsion of all named groups, poured-out money, and overturned tables.
- John 2:19-21 distinguishes the Jews' literal temple from Jesus' temple-as-body explanation; “I will raise it up” has no added “again,” and the explanatory verb is simple past “he spoke.”
- John 2:20 preserves the source position order for forty-six years, the built temple, second-person “you,” three days, and the raising question.
- John 2:13 keeps the movement verb, Jerusalem, and Jesus together in one source-linked phrase, matching positions 4–6.
- John 2:22 retains both Scripture and the word spoken by Jesus after the disciples remember his saying.
- John 2:24-25 preserves Jesus' refusal to entrust himself, his knowledge of everyone, and the explicit claim that he needs no human testimony.

Source checks for John 3:

- John 3:3,7 renders מנ דריש idiomatically as “anew,” rather than mechanically translating its parts as “from the beginning.” Younansardaroud's Classical Syriac Course Book, p. 97, lists /men d-rēš/ as “again” (https://d-nb.info/1184881278/34#page=108).
- John 3:4 retains both תוב (“again”) and דתרתינ זבנינ (“a second time”), and the rhetorical possibility of entering the mother's womb, with “enter” linked to its actual source position.
- John 3:5-8 retains birth of water and the Spirit; רוחא is rendered “wind” in the blowing/sound comparison and “Spirit” in its birth application. Both meanings occur in the supplied lexicon.
- John 3:11 preserves the plural knowing, speaking, seeing, testimony, and reception clauses; John 3:12 retains the final first-person object suffix as “believe me,” without duplicating the heavenly-things condition.
- John 3:13 retains the final source clause “who is in heaven”; John 3:15-18 keeps each universal believer clause, the non-perishing/eternal-life contrast, and the already-judged negative explicitly linked.
- John 3:20 retains causal גיר (“for”) and the reproof of deeds. John 3:22-24 retains Judea, Aenon near Salim, abundant water, ongoing staying/baptizing and coming/being baptized, and John's not-yet-imprisoned chronology.
- John 3:25 identifies one of John's disciples and a certain Jew, rather than changing them into collective groups. John 3:26 retains “Our Rabbi,” testimony about the other baptizer, and “many” going to him.
- John 3:27 renders the own-will/self expression idiomatically as “on his own”; John 3:28-30 retains John's denial of being the Messiah, being sent ahead, the bridegroom/friend distinction, emphatically completed personal joy, and the increase/decrease contrast.
- John 3:31 preserves all three earth expressions and both “above all” predicates within their own linked phrases. John 3:34 keeps both causal links and “it was not by measure that God gave the Spirit”, without adding a recipient.
- John 3:36 renders מתטפיס contextually as “heed”; CAL's Syriac Ct entry explicitly includes “obey, consent” (https://cal.huc.edu/cal_entry_web.php?lemma=pys+V). The final future predicate remains “God's wrath will remain upon him.”

Source checks for John 4:

- John 4:1-4 retains Jesus as the named subject, the comparative baptizing clause, the explicit statement that his disciples rather than Jesus baptized, and the necessity of passing through the Samaritans with the discourse transition.
- John 4:5-8 retains Sychar, Jacob's gift to Joseph his son, the water well, Jesus' weariness from the journey, the sixth hour, and the causal explanation that his disciples had gone to buy food for themselves.
- John 4:9-12 preserves the Samaritan/Jewish contrast, God's gift, the hypothetical request and living-water answer, the missing bucket and deep well, and Jacob, his sons, and his flocks. The demonstrative at John 4:11,15 stays with “that woman,” distinct from the following address “my lord.”
- John 4:12,29,33 treats למא contextually as an interrogative, without mechanically importing causal “why.” CAL's general-interrogative entry directly cites near-identical Old Syriac John 4:33 as “has anyone brought him something to eat?” (https://cal.huc.edu/oneentry.php?cits=all&lemma=lmh+c).
- John 4:14-18 preserves both water/giver relative clauses, the spring welling up to eternal life, the woman's request, five husbands, and the separate present “not your husband” predicate.
- John 4:19-26 keeps the mountain/Jerusalem contrast, the plural worship/knowledge contrasts, salvation from the Jews, worship in spirit and truth, the Father's “also,” and the speaker's “I am he” self-identification.
- John 4:27-38 retains the disciples' amazement and unasked questions, the woman's testimony, “Our Rabbi,” the food/will/work imagery, four months, fields already ready for harvest, future shared rejoicing, and the causal “for others have labored.”
- John 4:39-44 preserves the woman's testimony and the Samaritans' own hearing as distinct grounds for belief. John 4:42 keeps the fronted “we” with the hearing clause and retains “the Messiah, the Life-Giver of the world.” John 4:41 says “many,” without adding a comparative “more”; both two-day references and the prophet's own-city statement remain explicit.
- John 4:45-50 retains the Galileans' reception after seeing the Jerusalem feast signs, Cana and Capernaum, the royal official's sick son, and the subject “This man” when he hears of Jesus' arrival. The request, signs-and-wonders condition, living-son declaration, belief, and departure remain distinct.
- John 4:51-54 retains the servants' meeting, announcement and saying, the seventh hour yesterday, the fever leaving, the father's recognition and household belief, and the second-sign conclusion after the journey from Judea to Galilee.

Source checks for John 5:

- John 5:1-5 retains the unnamed Jewish festival, Jerusalem, Bethesda's Hebrew name, five porticoes, the waiting sick, the angel's periodic descent and stirring, the first descent and healing, and the man's thirty-eight years of illness. The source's angel clause is retained in full.
- John 5:4,7 uses “pool” for מעמודיתא; CAL explicitly gives Syriac “basin, pool” alongside baptism/ablution (https://cal.huc.edu/getlex.php?coord=620441037&word=12). The auxiliary after the first descent stays with that predicate, followed by the explicit stirring “of the waters.”
- John 5:6-15 preserves the request to be healed, the lack of a helper and another person's prior descent, the bed/walk commands, the immediate healing, Sabbath objection, and later temple encounter. John 5:13 renders the withdrawal with “himself,” without treating the ethical/reflexive dative as a newly introduced recipient.
- John 5:16 retains the causal explanation and active-participle-plus-HWA aspect: the Jews “were persecuting” and “were seeking,” because Jesus “was doing these things” on the Sabbath. The initial ד can introduce a causal subordinate clause; see Younansardaroud §14.6 (https://d-nb.info/1184881278/34#page=140).
- John 5:17-19 distinguishes Jesus' short first answer, the narrative reaction, and the ensuing discourse. The reaction's יתיראית remains “all the more,” followed by the Sabbath and Father/equality claims.
- John 5:19-27 retains the Father/Son working, loving, showing, raising, life-giving, judgment, and honor clauses; the repeated “also” particles and the hearing/believing/passing-from-death clauses remain explicit. Authority “also to execute judgment” is retained at John 5:27.
- John 5:28 begins with the actual stored clause דברה הו דינ דאנשא, rendered “But as for his being the Son of Man,” followed by “do not marvel at this.” It is a clause about the subject, not a vocative addressed to the hearers. The clause stays linked to its actual John 5:28 positions despite the different boundary in the supplied reference; hearing and coming from the graves lead to both resurrection outcomes.
- John 5:30-38 keeps the own-will/sender's-will contrast and the distinct testimonies of the speaker, John, the works, and the Father. John 5:35 renders דתשתבהרונ as “take pride,” consistent with CAL's reflexive “boast, be proud” sense (https://cal.huc.edu/oneentry.php?cits=all&lemma=%24bhr+V).
- John 5:39 remains the imperative “Search the Scriptures,” with the relative “in which you think you have eternal life” and their testimony about Jesus. John 5:40-44 retains unwillingness to come, eternal life, human glory, God's love, the Father's name, another's own name, and glory from “the only God.”
- John 5:45 preserves the source's למא as a contextual question, “Do you think,” followed by Moses as accuser and the hope placed in him; CAL allows the general-interrogative use (https://cal.huc.edu/oneentry.php?cits=all&lemma=lmh+c). John 5:46-47 retains both Moses' writing about the speaker and the final writings/words belief question.

Source checks for John 6:

- John 6:1-6 retains Galilee/Tiberias, the mountain, the near Jewish Passover feast, Philip, and Jesus' testing question and prior knowledge. The feeding account keeps the two hundred denarii, five barley loaves, two fish, five thousand, and twelve baskets explicit.
- John 6:13 preserves the attachment of the fragments left for those who had eaten “from the five barley loaves”; the from-phrase belongs to the loaves, not to the people. John 6:14-15 keeps the prophet claim distinct from the crowd's intended seizure and kingship.
- John 6:19 retains the generic travel verb and “about twenty-five or thirty stadia,” rather than importing a rowing verb. John 6:20-21 renders the identity reply “It is I,” willingness to take him into the boat, and the time clause before the boat's arrival; willingness is distinct from gladness.
- John 6:22 retains the sole boat and disciples' boarding, with “except the one” as natural conditional-exception English. Jesus is explicitly the subject who had not entered the boat with his disciples. John 6:23-24 keeps the other boats from Tiberias, the bread/blessing location, and the crowd's movement to Capernaum.
- John 6:25,34 retains “Our Rabbi” and “Our Lord.” John 6:27 links “remains” to its food noun and the fronted demonstrative להנא to “For God the Father has sealed this one,” keeping the Son-of-Man/giver and sealing clauses distinct.
- John 6:32-40 preserves Moses' negative giving clause, the Father's true bread, hunger/thirst, the coming/believing promises, the sender's will and loss/raising contrasts, and the last-day raising. John 6:37 uses the plain negative “I will not cast out.”
- John 6:45 retains the singular בנביא, vocalised בַּנבִֿיָא and marked singular in the source, as “in the prophet,” with causal “For” and “Therefore.” “From him” stays with learning, followed by coming to Jesus. The discourse introduced at John 6:43 continues through John 6:51.
- John 6:50-58 preserves the recorded descent tense, the adjective חיא as “living” in “living bread,” the final active “I give for the life of the world,” and the body/give/us roles in the Jews' question. The body/blood, abiding, life, and last-day clauses remain explicit; John 6:58 retains manna and the fathers' death rather than inserting a second bread noun.
- John 6:59-66 distinguishes synagogue narration, disciples' reaction, Jesus' reply and the narrator's knowledge explanation. John 6:60 takes מנ תלמידוהי as the partitive with “many,” yielding “many of his disciples, on hearing this,” rather than inventing outsiders hearing from them. John 6:62 retains the conditional question without supplying an apodosis; the Spirit/body/words and nonbelief clauses remain explicit.
- John 6:67 retains the contextual yes/no question “Do you also want to leave?” CAL records למא as a general interrogative in Syriac direct speech (https://cal.huc.edu/oneentry.php?cits=all&lemma=lmh+c). John 6:68-71 consistently uses Simon Cephas, retains “the Messiah, the Son of the living God,” the adversary predicate, Judas son of Simon Iscariot, and “one of the twelve.”

Source checks for John 7:

- John 7:1-10 preserves Galilee/Judea, the festival of Booths, the brothers' request and nonbelief, and the time/readiness contrasts. John 7:8 retains “this festival” in the first command and “to this festival now” in Jesus' response; the later secret ascent remains distinct.
- John 7:11-15 keeps the Jews' search, crowd murmuring because of Jesus, the contrasting good/deceiver reports, fear, the festival's halfway point, temple teaching, and the book/learning question. The explanatory גיר in John 7:12 remains “For.”
- John 7:16-18 preserves teaching from the sender and the question of the speaker's own initiative. John 7:18's second relative clause has its own subject: “whoever seeks the glory of the one who sent him” is true, contrasting with the person seeking his own glory.
- John 7:19-24 retains the negative in “Did not Moses give you the law?”, the crowd's demon accusation, the one deed and amazement, circumcision's origin parenthesis, the whole man's Sabbath healing, and righteous judgment. John 7:22's מטל הנא is “For this reason,” distinct from the circumcision noun.
- John 7:23 retains the purpose clause “so that Moses' law is not set aside.” CAL explicitly lists meṭṭol with a preformative as “in order to, so that” (https://cal.huc.edu/oneentry.php?cits=all&lemma=m%2Bwl+c). The following עלי, “at me,” belongs to the grumbling clause, not to Moses' law.
- John 7:25-31 keeps the Jerusalem speakers, “our elders” from the first-person plural suffix, the known/unknown origin contrast, the sender and hearers' knowledge, the still-unarrived hour, and the more-signs question. John 7:28's fronted ולי begins “You know me”; it is distinct from the preceding speech verb and is not a new recipient “to me.”
- John 7:32-36 preserves the chief-priest/Pharisee officers, the speaker's departure and search promises, and the crowd's repeated question. The future/go predicate in John 7:35 stays in its own linked clause about going to the regions of the Gentiles, followed by teaching the Gentiles.
- John 7:37-43 distinguishes Jesus' invitation and living-water saying from the narrator's explicit Spirit explanation: the Spirit had not yet been given, because Jesus had not yet been glorified. The prophet/Messiah reports, David/Bethlehem origin question, and division because of Jesus remain explicit.
- John 7:44-49 preserves attempted seizure, untouched hands, the officers' return and reply, and the Pharisees' questions. The source's plural accursed predicate is rendered with natural English agreement: “these people ... do not know ... they are accursed.”
- John 7:50-53 retains Nicodemus' membership and previous nighttime visit, “our law,” hearing before judgment, the Galilee challenge, “Search and see for yourself,” the no-prophet claim, and each person's departure to his own house. Speaker changes and nested quotations are punctuated across the actual verse boundaries.

Source checks for John 8:

- John 8:1-11 retains the Mount of Olives, dawn return, accused woman, Moses-law challenge, writing on the ground, first-stone command, departures, and the condemnation/sin exchange. John 8:5 keeps “You therefore” with the question; the generic stoning command is rendered without adding an execution actor. The act/preposition and first/cast/stone/her links stay in their own source phrases.
- John 8:12-20 preserves light, darkness, life, self-testimony, origin/destination, the Father, two witnesses, the law, treasury, and the still-unarrived hour. John 8:15 renders the bodily criterion as “according to the body”; the repeated second-person pronoun stays with that clause. John 8:20's implicit subject is “He,” and its full negated hour/coming clause is kept together.
- John 8:21-27 retains the going/seeking/death clauses, the Jews' self-killing question, below/above and world contrasts, the speaker's identity, the concessive beginning-to-speak clause, and the narrator's Father explanation. John 8:25 retains the explicit “Jews” subject.
- John 8:24 keeps “unless you believe.” CAL explicitly lists אלא as “except, unless,” including Syriac usage; an additional separate לא is not required for that conditional sense (https://cal.huc.edu/oneentry.php?cits=all&lemma=%29l%29+c).
- John 8:28-32 retains the Son-of-Man lifting, recognition, own-initiative and Father-taught speech, the sender's presence, pleasing actions, believers, remaining in the word, truth, and liberation.
- John 8:33-40 preserves Abrahamic descent, slavery, sin's slave, house/son permanence, freedom, intended killing, the speaker's word, and what each side has seen with its father. בני חארא is rendered idiomatically as “free people”; CAL defines the compound as “free individual” and records plural free-person usage (https://cal.huc.edu/oneentry.php?cits=all&lemma=br%40xyryn+N).
- John 8:41-43 retains the denial of prostitution origin, the one Father identified as God, coming out from God, being sent, and both occurrences of “my word.” The compound מטל מנא is the opening “Why”; the following על דלא introduces the causal explanation “Because you cannot hear.”
- John 8:44-47 keeps the accuser, the father's desire, the murderer-from-the-beginning predicate, standing “in the truth,” truth's absence, speaking a lie/from himself, the liar and its father, the truth/unbelief contrast, conviction concerning sin, and hearing God's words. The lie's possessive is retained in “its father.”
- John 8:48-56 preserves the Samaritan/demon accusation, the denial, honor and glory, the death/word promise and repeated challenge, Abraham and the prophets, the Father's glorifying, knowing and keeping his word, and Abraham's desire and rejoicing. John 8:54 retains the actual first-person plural “our God” in the reported claim.
- John 8:57-59 retains the fifty-year comparison, Abraham question, “before Abraham was, I am,” and the full final movements: stones taken up, Jesus hiding, leaving the temple, passing among them, and going away. The named Jesus and hiding verb remain in the same linked phrase.

Source checks for John 9:

- John 9:1-7 retains blindness from the mother's womb, “Our Rabbi,” the parents/sin question, the works-of-God purpose clause, the speaker's singular obligation, day/night and world/light imagery, clay made from his spittle, and the Siloam washing. Jesus' answer continues through John 9:5. John 9:7 names the pool of Siloam without adding a supplied name etymology; John 9:11 explicitly says “the waters of Siloam.”
- John 9:8-14 preserves the neighbors' identification debate, ongoing begging, the man's identification and named-Jesus account, the nested washing command, ignorance of Jesus' location, the Pharisee referral, and the Sabbath setting. The subject and auxiliary in John 9:9 remain with the man's speaking, separate from his quoted answer.
- John 9:15-19 keeps the restored-sight question and reply, the Pharisees' Sabbath objection and disagreement, the man's prophet declaration, the Jews' disbelief, and the parents' questioning. The subject at John 9:15 belongs to “But he said,” and the auxiliary at John 9:16 belongs to “others were saying.” The object “them” in John 9:19 stays with “asked.”
- John 9:20-24 retains “our son,” blindness at birth, the parents' uncertainty about present sight, their separate assertion that he has come of age, his ability to speak for himself, fear of the Jews, the prior expulsion decision, the second summons, and the causal “for we know.” The age statement is not turned into another uncertainty clause.
- John 9:25-30 preserves the one-thing contrast, present sight, repeated questioning, the invitation to become disciples, the insult, the Moses/Jesus disciple contrast, known divine speech to Moses, unknown origin, and the man's astonishment and “my own eyes.” John 9:28's anticipatory possessive stays with “that man's disciple”; גיר is rendered emphatically as “indeed,” an attested Syriac sense in [CAL](https://cal.huc.edu/oneentry.php?cits=all&lemma=gyr+X).
- John 9:31-34 retains the voice of sinners, fear of God and doing his will, the hearing response, the unprecedented opening of a man born blind's eyes, the from-God counterfactual, the accusation of being born wholly in sins, and the expulsion. The quoted testimony runs through John 9:33.
- John 9:35-41 retains the actual “Son of God,” the healed man's explicit subject, both “My Lord” addresses, seeing/speaking identification, falling and worshiping, “the judgment of this world,” the seeing/blind reversal, and the final conditional and present sin. John 9:41's מטל הנא is the causal “for this reason,” without adding a demonstrative to “your sin.”

Source checks for John 10:

- John 10:1-6 preserves the door/fold contrast, thief and robber, shepherd, doorkeeper, names and voice, leading out, following, stranger, and the hearers' failure to understand. John 10:3's first door noun forms “doorkeeper,” while its second is the door opened; the initial demonstrative is the recipient, not a modifier of the guard.
- John 10:7-14 keeps the renewed introduction, door-of-the-flock image, entry and pasture, theft/killing/destruction, life and “something more,” laying down life for his flock, the hired worker's lack of ownership and concern, the wolf's actions, and knowing/being known. John 10:8 has no source phrase “before me”; John 10:11 retains the flock's possessive suffix.
- John 10:15-18 retains “my Father,” the other sheep, this fold, bringing and hearing, the whole flock becoming one with one shepherd, the Father's love, voluntary laying down and taking up life, both authority clauses, and the Father's command. The object at John 10:16 position 17 belongs to bringing, followed by hearing at positions 18-19; the whole-flock quantifier remains explicit.
- John 10:19-24 preserves the division among the Jews, demon/madness accusation and reply, the Feast of Dedication in Jerusalem during winter, Solomon's portico, the surrounding Jews, the suspense question, and the request for a plain Messiah declaration. The spoken accusations remain distinct from the narrator's statements.
- John 10:25-30 retains the works in the Father's name as witnesses, unbelief and sheep membership, “my own sheep,” hearing/knowing/following, eternal life, never perishing, the Father's gift and greatness, and “I and my Father are one.” John 10:28's hand noun is plural in the actual source (“my hands”); John 10:29's Father's hand is singular.
- John 10:31-36 preserves the stones, good-works challenge, blasphemy accusation, human-being/God contrast, the law's plural “gods,” the word of God being with its recipients, Scripture's permanence, the Father's sanctifying and sending, and “I am the Son of God.” In John 10:36 the plural “you say” and singular “you blaspheme” have separate linked clauses; על ד introduces “because I said.”
- John 10:37 uses “Unless I do my Father's works,” followed by “do not believe me.” [CAL](https://cal.huc.edu/oneentry.php?cits=all&lemma=%29l%29+c) records אלא as “except, unless,” including Syriac usage. John 10:38 retains the works-directed pronoun with “believe the works,” the knowing/believing purpose, and both “my Father” phrases.
- John 10:39-42 retains the renewed seizure attempt, escape, journey across the Jordan to John's former baptizing place, staying there, the statement that John performed not even one sign, the truth of everything he said about Jesus, and the many believers. Quotation boundaries distinguish the two shepherd discourses, the later temple exchange, and the final crowd testimony.

Source checks for John 11:

- John 11:1-6 preserves Lazarus's Bethany origin, Mary and Martha as his sisters, the anointing identification, the sisters' message, the death/glory contrast, Jesus' love for all three, and the two-day delay. In John 11:5 the named Jesus stays with the loving predicate rather than being moved into a neighboring span.
- John 11:7-16 keeps Judea, “Our Rabbi” and “Our Lord,” the twelve-hour/day/night illustration, “Lazarus our friend,” the sleep/death misunderstanding, the explicit death announcement, joy for the disciples' sake and belief, and Thomas's Twin designation and invitation to die with Jesus. The speech verb in John 11:16 remains with Thomas's introduction.
- John 11:17-24 retains the four days in the tomb, Bethany's distance of about fifteen stadia from Jerusalem, the visitors' comforting, Martha's meeting and Mary's remaining seated, Martha's request, and the last-day resurrection reply. The distance clause is spatial, without an invented traveler; “your brother will rise” keeps its predicate and subject together.
- John 11:25-27 uses “resurrection” for נוחמא, matching the exported lexical alternatives and [CAL's Syriac entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=nwxm+N), which directly cites John 11:24. The supplied Murdock's “consolation” is not copied into this passage. The life/belief/death promise, the belief question, Messiah, Son of God, and coming into the world remain explicit; a repeated first-person pronoun is not turned into an extra affirmation verb.
- John 11:28-37 preserves “Our Rabbi,” Mary's quick response, the visitors' following and supposition that she was going to the tomb to weep, Mary's address, the Jews' weeping, strong inner agitation, the grave-location question, Jesus' tears, and the blind-man comparison. John 11:35 retains tears as the grammatical subject; John 11:37's second “this man” belongs to the clause about Lazarus not dying.
- John 11:38 keeps בינוהי ולה together as “within himself.” [CAL](https://cal.huc.edu/oneentry.php?cits=all&lemma=byn+p) records the identical-suffix idiom and a near-identical Old Syriac John 11:38 example. The second word belongs to that expression rather than to the following coming verb; the cave and stone over its entrance remain explicit.
- John 11:39-44 retains “this stone,” Martha as the dead man's sister, the four-day explanation, the glory reminder, the stone's removal, Jesus' raised eyes and prayer, the crowd-directed belief purpose, the loud Lazarus command, bound hands and feet, the face cloth, and the release command. The bound predicate stays with the limbs and bandages in its own linked phrase.
- John 11:45-52 distinguishes belief from reporting to the Pharisees, the chief priests' and Pharisees' assembly, signs, Roman threat to “our place” and “our nation,” Caiaphas's year as high priest, and his unwitting prophecy. John 11:50 retains “for us”; John 11:51's negation, speaking, own initiative, priesthood, and prophecy clauses stay with their actual predicates. The final gathering of God's scattered children remains part of the prophecy.
- John 11:53-57 keeps the death plot, Jesus' withdrawal to Ephraim near the wilderness, his stay with the disciples, the approaching Jewish Passover, purification, the temple question, and the command to report his location for seizure. The singular ethical dative accompanying his departure does not introduce another recipient “from them.”

Source checks for John 12:

- John 12:1-6 retains the six days before Passover, Lazarus's Bethany setting and raising by Jesus, the supper, Martha's serving, Mary's alabaster vessel of choice nard, Jesus' feet and her hair, the fragrance, Judas Iscariot's identity and betrayal, three hundred denarii, and the money-bag explanation. The source has an alabaster vessel rather than an added weight measure. The speech verb stays in John 12:4; the next verse begins with “Why,” not another speech verb.
- John 12:7-17 preserves the burial-day explanation, the poor/Jesus contrast, the visitors' hearing and Lazarus purpose, the chief priests' plan, belief, palms and Hosanna, the donkey and Zion quotation, and the disciples' later understanding. John 12:14 says “donkey” without adding “young”; John 12:15 retains the female donkey. The crowd's hearing and testifying verbs remain with their actual source positions.
- John 12:18-22 keeps the sign as the reason for the crowds' meeting, the Pharisees' complaint, Gentile men among those who had gone up to worship, Philip's Bethsaida/Galilee origin, “My lord,” and the Philip/Andrew message sequence. The source names the nations/Gentiles without an explicit Greek-name word; the people and partitive clauses remain in their own linked phrases.
- John 12:23-28 retains the Son-of-Man hour, the grain of wheat's unless/fall/die contrast, much fruit, life/loss and eternal life, service and following, the Father's honor, the troubled soul, “My Father,” the hour's purpose, and the name-glorifying request and heavenly voice. John 12:26 preserves the source future “he will come after me”; Jesus' speech continues through the request in John 12:28 before the heavenly reply begins.
- John 12:29-36 preserves thunder/angel reports, the voice's purpose for the hearers, judgment and the ruler's casting out, lifting from the earth and drawing everyone, the death explanation, the crowd's Messiah/Son-of-Man question, and the light/darkness commands. The second light noun in John 12:35 belongs to “Walk while you have the light,” followed by the darkness clause.
- John 12:37-41 keeps the signs/unbelief contrast, fulfillment of Isaiah's word, “My Lord,” “our report,” the Lord's arm, the repeated Isaiah explanation, and his seeing glory and speaking about the subject. John 12:40's two actual predicates are third-person plural: “They have blinded their eyes and darkened their hearts.” The source is not normalized to a singular actor; the final healing predicate remains first person.
- John 12:42-50 distinguishes the leaders' belief from their non-acknowledgment because of the Pharisees, with the purpose of avoiding synagogue expulsion expressed without a double negative. Human/divine praise, believing/seeing the sender, light, keeping words, life-giving rather than judging, the rejecting hearer's judge, the last day, and the Father's command remain explicit. John 12:48's דמללת is a first-person perfect (“I have spoken”), not a present participle; its resumptive pronoun stays with the judging predicate. The Father's saying and the speaker's speaking in John 12:50 form one natural correlative clause.

Source checks for John 13:

- John 13:1-5 preserves the Passover setting, departure to his Father, love for his own to the end, supper taking place, Satan's influence on Judas, the Father's gift, coming from God and going to God, and the garments, linen cloth, basin, washing and wiping. John 13:3 says “from God” rather than the local Murdock's “from the Father”; its causal knowing clause continues into Jesus' rising in John 13:4.
- John 13:6-13 retains Simon Cephas, “my Lord,” not knowing now but knowing later, never washing, no part without washing, hands/head as well as feet, bathing and cleanness, and the betrayer explanation. John 13:10 explicitly retains both “all of you are clean” and “but not all of you.” John 13:13 keeps “Our Rabbi” and “Our Lord”; the repeated second-person pronoun belongs to “you speak well,” followed by “for that is what I am.”
- John 13:14-20 preserves “your Lord and your Rabbi,” the reciprocal washing obligation, the example, servant/master and messenger/sender contrasts, knowing/doing and blessedness, chosen recipients, the bread/heel quotation, foretelling before the event and belief when it happens, and the chain of receiving the sender. In John 13:18 the speaking, knowing and Scripture clauses remain with their actual source predicates. In John 13:20 the two consecutive receiving predicates have separate linked clauses.
- John 13:21-26 keeps Jesus' agitation and testimony, one betrayer, the disciples' uncertainty, the unnamed beloved disciple's reclining, Simon Cephas's signal, the disciple's question, and the dipped bread given to Judas son of Simon Iscariot. The unnamed disciple is not given an inferred name; the dipping/giving clause preserves the bread, speaker and recipient together.
- John 13:27-30 retains Satan's entry after the bread, the prompt-action command, the table companions' lack of understanding, their suppositions about Judas's money bag and festival/poor provisions, Judas's immediate departure, and the nighttime statement. John 13:28 renders על מנא as “why,” an attested compound in [CAL](https://cal.huc.edu/oneentry.php?cits=all&lemma=%28l%40m%29+c). The ethical dative accompanying departure is not rendered as a new “from it” object or an added house.
- John 13:31-35 preserves the Son-of-Man/God glorification clauses, immediate future glorifying, “My children,” remaining briefly, seeking, the statement to the Jews and its repetition to the disciples, and the new commandment to love one another as the speaker loved them. The final love clause is grammatical English while retaining the reciprocal source expression.
- John 13:36-38 distinguishes “Our Lord” from “My Lord,” retains where/now/later following, the laying-down-life offer and reply, both “Truly” words, and the rooster's not crowing until three denials. The speaking predicates stay linked with Simon Cephas and Jesus in their respective introductions.

Source checks for John 14:

- John 14:1-7 retains the untroubled-heart commands, belief in God and the speaker, the Father's house and dwellings, preparation and return, knowing the destination and way, Thomas's “Our Lord,” and the way/truth/life and Father-knowing statements. John 14:2 keeps the conditional and going/preparing clauses; the causal reading of the latter follows the supplied Murdock without imposing a remembered Greek question.
- John 14:8-12 preserves Philip's request, seeing the speaker and the Father, the mutual indwelling, words and works, the grounds for belief, and the believer's doing the works and greater things. The repeated first-person pronoun in John 14:12 belongs to “the works that I do,” followed by the separate “he too will do.” Its final Father noun is unsuffixed “the Father,” not the reference's supplied “my Father.”
- John 14:13-18 retains asking in the speaker's name, “ask me,” the Father's glorification in his Son, loving and keeping commandments, asking the Father, another Advocate, the Spirit of truth, and knowing, seeing, receiving and dwelling. John 14:17 distinguishes “with you” from “in you”; the final “in a little while” remains in John 14:18 at the actual source boundary.
- John 14:16,26 consistently uses “Advocate” for פרקלטא. [CAL](https://cal.huc.edu/oneentry.php?cits=all&lemma=prqly%2B+N) records this Syriac lexical sense and the Paraclete/Holy Spirit use. John 14:26 retains “my Father” from the actual possessive suffix and the present “all that I say to you.”
- John 14:19-24 retains the world's not seeing, the disciples' seeing and living, mutual indwelling, commandments and love, the self-revelation promise, Judas explicitly distinguished from Iscariot, and the Father's love, coming and dwelling. John 14:19 uses the source's simple “not,” without adding “no longer.” The speaking and love/coming predicates in John 14:22-23 remain with their own source words; the singular word heard in John 14:24 belongs to the Father who sent the speaker.
- John 14:25-29 preserves the present being-with-the-hearers clause, teaching and remembering, peace and its distinct giving, the heart's not being troubled or afraid, the repeated going/coming report, love and rejoicing, the Father's greatness, and the foretelling/belief sequence. John 14:29 keeps both “before it happens” and “when it happens” in their respective linked clauses.
- John 14:30-31 retains limited further speech, the coming ruler, “he has nothing in me,” the world's knowing the speaker's love for his Father, the Father's command and the speaker's doing, and the rise/depart commands. The ruler's possessive relation is retained rather than reduced to a statement that nothing is in the speaker. Quote boundaries keep Judas's question separate from Jesus' preceding and following discourses.

Source checks for John 15:

- John 15:1-6 preserves the true vine and cultivator, unfruitful and fruitful branches, taking away and cleansing, cleanness through the word, mutual remaining, bearing fruit, and inability without the speaker. Both “unless” clauses in John 15:4 remain explicit. John 15:6 begins with אלא, rendered “Unless”; a separate לא is not needed for this attested Syriac conditional sense. The gathering and throwing predicates are both retained, followed by the fire and burning clause.
- John 15:7-12 retains remaining words, the request and its result, the Father's glory and much fruit, discipleship, the Father's and speaker's love, commandments and remaining in love, the speaker's and hearers' joy, and the reciprocal-love command. Repeated pronouns and possessives are rendered naturally; “my Father's commandments” does not become an extra owner or “his commandments of my Father.”
- John 15:13-18 keeps life laid down for friends, friendship and obedience, the servant/master distinction, disclosure of everything heard from the Father, the speaker's choice and appointment, going and bearing enduring fruit, asking the Father in the speaker's name, the love command, and the world's prior hatred of the speaker. The full chain of purpose clauses in John 15:16 remains explicit.
- John 15:19-24 preserves belonging to or being chosen from the world, hatred, the remembered servant saying, persecution and keeping words, the name and sender, the coming/speaking and unique-works counterfactuals, sin and excuse, and hatred of the speaker and his Father. John 15:22 retains singular “their sin”; John 15:24 keeps the works being done “in their sight.”
- John 15:25 uses “without cause” for מגנ. [CAL](https://cal.huc.edu/oneentry.php?cits=all&lemma=mgn+X) records the Syriac “for no reason” sense and a Peshitta Psalm 69:5 example, distinct from the word's gratis/free-of-charge sense. The quoted hatred remains part of the law-word fulfillment clause.
- John 15:26 retains “Advocate,” consistent with John 14, and both actual “my Father” phrases. נפק is vocalised as a present participle, supporting “proceeds.” John 15:27 keeps the hearers' predicative “you also are witnesses” and their being with the speaker from the beginning. The chapter remains one continuous speech, with nested quotations in John 15:20 and 15:25.

Source checks for John 16:

- John 16:1-4 retains the purpose of preventing stumbling, expulsion from their synagogues, the killing/offering supposition, ignorance of the speaker and his Father, and remembering the warning when its time arrives. The second “these things” belongs to the explanation about not speaking from the beginning, rather than being moved into the preceding warning.
- John 16:5-7 preserves going to the sender, the unasked destination question, sorrow filling the hearers' hearts, truthful speech, the benefit of departure, both conditional branches, and the sending of the Advocate. The repeated first-person pronoun after the going predicate remains with that predicate; it does not become the Advocate's subject.
- John 16:8-11 retains conviction of the world concerning sin, righteousness and judgment, with the explanatory clauses about unbelief, going to my Father and no longer being seen, and the judgment of this world's ruler. The d-prefixed explanations remain distinct from the discourse particle דינ.
- John 16:12-15 keeps the still-ungraspable teaching, the Spirit of truth's guidance, not speaking from his own mind, speaking what he hears, making future things known, glorifying the speaker, and receiving and showing what is his. John 16:15 explicitly retains “my Father.”
- John 16:16-19 preserves the two little-while sayings and the speakers' questions. The actual Father noun is unsuffixed in John 16:16, “the Father,” but possessive in John 16:17, “my Father.” John 16:19 ends with seeing again; the Father-going continuation in the supplied Murdock reference is absent from this source verse and is not added.
- John 16:20-24 distinguishes the hearers' weeping and sorrow from the world's rejoicing, sorrow becoming joy, the woman's day of childbirth, a son and the human born into the world, the speaker's seeing the hearers again, and joy no one takes away. Asking the speaker and asking his Father in his name remain distinct, followed by the ask/receive command and complete joy.
- John 16:25-28 keeps parables and future plain speech, asking in the speaker's name, the Father's own love, and departure and return. עינ בגלא is rendered together as “plainly,” an adverbial idiom recorded by [CAL](https://cal.huc.edu/cal_entry_web.php?lemma=%28yn%40bgl%29+N), rather than inventing a separate revealing predicate. John 16:27 says “God,” unlike the supplied Murdock's “Father”; John 16:28's first-person ethical לי is absorbed naturally into “I go,” without introducing a third-person recipient.
- John 16:29-33 preserves plain speaking, not one parable, knowing everything, belief in coming from God, scattering each to his place, the Father with the speaker, peace, affliction, courage, and overcoming the world. John 16:31's actual הַימֵנו is tagged as a second-person plural imperative and is rendered “Believe!” rather than the reference's question. Quotation boundaries distinguish the disciples' speech from Jesus' preceding and final replies.

Source checks for John 17:

- John 17:1-3 keeps Jesus' raised eyes and prayer, “my Father,” the arrived hour, mutual glorification, authority over all flesh, the gift of eternal life, and knowing the only true God and the sent Jesus Messiah. The name and title remain with their actual source positions.
- John 17:4-5 preserves glorifying on earth, completing the assigned work, and the glory before the world's existence. John 17:5 retains both actual לותכ relations: glorifying the speaker with the Father and the glory he had with the Father. The compound מנ קדמ is rendered “before,” an attested Syriac sense in [CAL](https://cal.huc.edu/cal_entry_web.php?lemma=mn%40qwdm+p); it also occurs before the foundation phrase in John 17:24.
- John 17:6-8 retains disclosure of the Father's name to the people given from the world, their belonging to the Father, giving them to the speaker, keeping the word, receiving the words, certain knowledge, and belief in the sender. John 17:7's actual ידעת is first-person perfect, rendered resultatively as “Now I know,” with the same first-person subject as the supplied Murdock reference; it is not changed to a third-person plural reading.
- John 17:9-12 distinguishes prayer for those given from prayer for the world, the reciprocal possessions and glorification, no longer being in the world, the disciples' remaining, coming to the Father, and keeping them united. In John 17:11 the given antecedent is “your name”; John 17:12 separately speaks of keeping those given and the exception of the son of destruction, with Scripture's fulfillment.
- John 17:13-17 keeps the present coming and speaking, complete joy, the world's hatred, not belonging to the world, the request for protection rather than removal, and sanctification in the Father's truth. The causal גיר in John 17:16 and explanatory d-prefix in John 17:17 remain explicit; “your word” naturally renders the repeated possessive without inventing another owner.
- John 17:18-20 preserves the paired sending into the world, sanctifying himself on their behalf, their sanctification in truth, and prayer also for those believing through their word. The source's on-behalf expressions are rendered idiomatically rather than as literal faces; the believer clause remains attached to the recipients of prayer.
- John 17:21-23 retains all being one, “my Father,” mutual indwelling, unity in the speaker and Father, the world's believing and knowing the sender, the glory given and passed on, perfected unity, and the comparison of the Father's love for them and for the speaker. Each knowing, sending and loving predicate stays in its own linked clause.
- John 17:24-26 keeps the desire for those given to be with the speaker and see his glory, love before the world's foundation, “my righteous Father,” the world's lack of knowledge, the speaker's knowledge and the disciples' knowledge, disclosure of the name, and love and the speaker being in them. Bare “Father” in John 17:24 remains distinct from possessive “my Father” in John 17:25. The prayer forms one continuous quotation after its narrative introduction.

Source checks for John 18:

- John 18:1-4 retains Jesus' departure with his disciples, the Kidron brook and garden, Judas's knowledge of that place and the repeated gatherings, the cohort and guards led from the chief priests and Pharisees, lanterns, lamps and weapons, and Jesus' foreknowledge and question. The source's two leading predicates in John 18:3 remain separate rather than being collapsed into one action.
- John 18:5-9 preserves the explicit Jesus names, Judas standing with the group, the repeated seeking question and identification, backward movement and falling, the command to let the others go, and the purpose of fulfilling the speaker's word. Repeated first-person forms are rendered naturally as “I am”; the fulfillment quotation retains “not even one of them” in its own linked phrase.
- John 18:10-14 keeps Simon Cephas, the sword, the servant's right ear, Malchus, the sheath and cup question, “my Father,” seizure and binding, Annas first, Caiaphas's father-in-law relation, and Caiaphas's year as high priest and prior counsel. The stored morphology marks John 18:12's commanders as plural; the translation preserves that number rather than adopting the supplied reference's singular commander. The English relative clause in John 18:13 attaches the year-specific priesthood to Caiaphas.
- John 18:15-18 retains the other disciple, Simon outside the door, the doorkeeper's question and denial, and the servants' and guards' fire, cold and warming. John 18:15-16 reads the l-marked disciple as a fronted object resumed by לה, giving “was known to the high priest”; the supplied Murdock's “knew the high priest” is an alternative syntactic reading. This is an editorial grammatical judgment, supported by the object-marking, word-order and pendens constructions described in Younansardaroud, [sections 7.2, 14.3 and 15.5](https://d-nb.info/1184881278/34), rather than a change to the stored source.
- John 18:19-24 keeps the questions about disciples and teaching, public speaking and teaching in synagogue and temple, the hearers' knowledge of everything said, the blow on Jesus' cheek, the conditional reply, and Annas sending Jesus while bound to Caiaphas. עינ בגלא in John 18:20 stays together as “openly”; the actual speaking predicate is in that same phrase. Jesus' answer continues across John 18:20-21 without an artificial quotation break.
- John 18:25-27 preserves Simon's standing and warming, the disciple question and denial, the high-priest servant's kinship to the man whose ear Simon cut off, his seeing question, the repeated denial and the rooster's immediate crowing. John 18:26 groups the speech introduction and its intervening subject/relative clause together so the speech verb, kinship and ear-cutting roles stay intact in natural English.
- John 18:28-35 keeps the movement from Caiaphas to the praetorium, morning, avoiding defilement before eating Passover, Pilate's accusation question, the evildoer counterfactual, the law and killing restriction, the death-word fulfillment, and the king question and reply. למא is used as a general yes/no interrogative in John 18:17,25,35; [CAL's Syriac sense](https://cal.huc.edu/oneentry.php?cits=all&lemma=lmh+c) explicitly cites Peshitta John 18:35, “am I a Judean?” The repeated first-person form in that question does not introduce an extra “I say.”
- John 18:36-40 retains the kingdom's origin, the counterfactual servants fighting, the handing-over purpose, the king reply, birth and coming into the world, testimony and hearing the voice, the truth question, no charge, the release custom and “this king,” and the choice of Barabbas and his being a robber. The adjacent first-person forms in John 18:37 remain with their respective king and birth clauses. The ethical לה accompanying departure in John 18:38 is absorbed naturally, without an invented “away from him”; its later repeated first-person pronoun belongs to finding no charge. Pilate's speech continues from John 18:38 through 18:39.

Source checks for John 19:

- John 19:1-6 retains Pilate's scourging of Jesus, the soldiers' thorn crown and purple garments, the king greeting and blows, repeated bringing outside, the man declaration, repeated crucifixion demands, and Pilate's no-charge statements. The charge nouns remain with their actual predicates, and the explicit “not even one” in John 19:4 is retained rather than moved into a neighbouring span.
- John 19:7-11 preserves the law and death-liability claim, making himself the Son of God, Pilate's increased fear, the origin question, Jesus' silence, the two authority clauses, the counterfactual gift from above, and the greater-sin comparison. John 19:9's second Jesus name is the subject of giving no answer, not a vocative added to Pilate's question. John 19:11 explicitly ends “than yours.”
- John 19:12-15 keeps Pilate's desire to release Jesus, the Caesar-friend and rival-king claims, the tribunal and Stone Pavement, Gabbatha in Hebrew, Passover Preparation, about the sixth hour, the king declaration, the repeated take-away/crucify commands, and the chief priests' “no king except Caesar.” The negative existential belongs to the no-king clause, not to its speech introduction.
- John 19:16-22 retains handing Jesus over so that they might crucify him, taking him and bringing him out as distinct actions, his carrying the cross to Skull/Golgotha, two others and Jesus in the middle, Pilate's tablet and its wording, the languages, and the requested alteration and refusal. John 19:20 says “near the city,” without importing the supplied Murdock's explicit Jerusalem. The actual language order is Hebrew, Greek and Latin; the tablet includes “This is Jesus the Nazarene.”
- John 19:23-24 keeps four portions and a portion for each soldier, the seamless tunic woven throughout from the top, the decision against tearing it, casting lots, the Scripture quotation and the soldiers' doing these things. The tunic's lack of a seam is not reduced to a claim that it was merely not sewn from the top. Both the fulfilled Scripture and its saying predicate remain explicit.
- John 19:25-28 preserves the mother/sister/Mary list and its conjunctions, Mary of Cleophas without an added wife relation, Mary Magdalene, the loved disciple standing, the woman/son and mother declarations, taking her to himself from that hour, and the knowing/fulfillment/thirst sequence. No house or home is added to John 19:27. The quoted speech and nested quotations throughout the chapter have explicit speaker boundaries.
- John 19:29-37 keeps the vinegar, sponge, hyssop and mouth, the finished declaration, bowed head and spirit, Preparation and Sabbath reasons, the leg-breaking request and soldiers' actions, Jesus already dead, the side struck with a spear, blood and water, the witness's true testimony and knowledge, belief, the bone quotation and the pierced-one quotation. John 19:31 preserves plural “their crosses.” Its saying predicate remains in the introduction and its request keeps leg-breaking distinct from taking down. John 19:36 retains the actual “which says” before the quotation.
- John 19:38-42 retains Joseph from Arimathea, his request and disciple/fear explanation, Pilate's permission, both taking-body clauses, Nicodemus's earlier nighttime visit, myrrh and aloe, about a hundred Roman pounds, linen and spices, Jewish burial custom, the garden and unused new tomb, and the Sabbath and nearness reasons for laying Jesus there. The weight is the source's ancient Roman-pound unit, without a modern conversion. The causal explanation of Joseph's request is parenthetical so the request to take Jesus' body remains clear; the tomb's existential clause does not acquire an extra standing predicate.

Source checks for John 20:

- John 20:1-4 retains the first day of the week, morning while still dark, Mary Magdalene, the removed stone, Simon Cephas and the other disciple whom Jesus loved, the taken-away Lord, Mary's singular “I do not know,” both disciples running and the other disciple arriving first. John 20:2 explicitly says “our Lord,” without replacing it with “my Lord.”
- John 20:5-10 keeps looking in without entering, Simon's entry, the lying linen cloths, the separate head cloth folded and set aside in one place, the other disciple's entry, seeing and believing, the Scripture explanation, resurrection from the dead, and returning to their place. The single going predicate in John 20:3 is not expanded into two independent going/coming actions. No original undisturbed position is added to the cloths, and no house is added to “their place.”
- John 20:11-14 preserves Mary's weeping and looking, two angels in white, the head/feet positions and Jesus' laid body, the question, “my Lord,” her turning and seeing Jesus standing without recognizing him. The headrest/feet expressions are rendered idiomatically “at his head” and “at his feet”; [CAL's rgl entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=rgl+N) quotes the exact Peshitta John 20:12 phrase with that rendering. The negative knowing clause keeps its own auxiliary before the separate Jesus identification.
- John 20:15-17 retains both questions, the gardener supposition, the respectful address, taking/placing/telling and fetching, Mary's name, the Hebrew reply and its explanation, not coming near, not yet ascending, and the message to the brothers. The actual title רבולי is kept as “Rabbuli,” with the source's “Teacher” explanation; [CAL](https://cal.huc.edu/cal_entry_web.php?lemma=rbwl%29+N) records this Syriac variant alongside the rbwn/rabbouni form. Its first-person ending is retained in the transliteration without an added second “My.” All four Father/God possessive expressions in John 20:17 remain explicit.
- John 20:18-23 keeps Mary Magdalene's announcement, “our Lord,” the evening of that first day, shut doors and fear, Jesus' coming and standing among them, peace, hands and side, the disciples' joy, the repeated peace and sending, breathing, receiving the Holy Spirit and the forgiveness/retention conditions. John 20:18 and John 20:20 retain “our Lord.” The initial “And when” in John 20:22 renders actual וכד; its “these things” and the next verse form one continuous saying.
- John 20:24-26 preserves Thomas as one of the twelve and called Twin, his absence when Jesus came, the disciples' report, the nail places, his fingers and hand, his refusal to believe, eight days, the disciples inside, Thomas with them, shut doors and renewed peace. John 20:25's הו begins “he, however, said”; it does not become “our Lord himself” in the preceding report. The source's plural “my fingers” remains plural, and John 20:26 does not add a possessive to “the disciples.”
- John 20:27-29 keeps singular “your finger,” Jesus' hands and side, the contrast between unbelieving and believing, Thomas's reply “My Lord and my God,” and the seen/believed and unseen/believed contrasts. John 20:29 retains “Now” and the perfect believing forms as a declaration, rather than importing a remembered question. Speech and nested-message quotation marks follow the actual changes of speaker.
- John 20:30-31 preserves many other signs before the disciples, their not being written in this book, the purpose of believing Jesus is the Messiah and Son of God, the temporal “when you believe,” and life in his name. “Eternal” is explicit in the final source phrase and is retained. The discourse particle דינ is rendered contextually, including “Then” in John 20:21, without forcing an adversative meaning into every occurrence.

Source checks for John 21:

- John 21:1-6 retains the Sea of Tiberias setting, Simon Cephas, Thomas called the Twin, Nathanael from Cana of Galilee, Zebedee's sons and two others of the disciples, fishing without a catch that night, Jesus on the shore, the food question, the right side of the boat, and the net's heavy catch. The question in John 21:5 is a yes/no food question; למא does not force an added “why.”
- John 21:7-8 keeps the loved disciple's saying to Cephas, both “our Lord” declarations, Simon's tunic and girding, his nakedness, entering the sea to come to Jesus, the other disciples' boat, the net and fish, and “not very far,” about two hundred cubits. The speech verb stays in its own subject/relative-clause span; the distance explanation is parenthetical and the source unit is retained without a modern conversion.
- John 21:9-14 preserves coals laid out, fish laid on them, bread separately, bringing some newly caught fish, Simon Cephas going up and drawing the net ashore, 153 large fish, all that weight and the untorn net, the meal invitation, knowing “our Lord,” Jesus giving the food to them, and the third appearance after rising from the dead. No boat noun is added to Simon's going-up verb and no extra disciples noun is added to the giving clause.
- John 21:15-17 retains Simon son of Jonah, the first comparison “more than these,” all three love questions and replies, “my Lord,” and the feed-for-me requests. The actual lambs/sheep terms are kept distinct in their source links; the final נקותי is rendered “my sheep,” a sense recorded alongside “ewe” by [CAL](https://cal.huc.edu/oneentry.php?cits=all&lemma=nqy+N), without inferring a restricted audience from grammatical gender. John 21:17 has both understanding everything and knowing the speaker's love; repeated second-person pronouns remain with their respective predicates.
- John 21:18-19 preserves the doubled “Truly,” the contrast between young self-girding and future old age, stretched hands, another's girding and leading, the unwanted destination, the death/glorifying-God explanation, and “Come after me.” The perfect in the future temporal old-age clause is rendered naturally “when you grow old,” rather than treating the predicted age as already past.
- John 21:20-21 keeps Simon Cephas turning, the loved disciple coming after him, the recollection of supper and Jesus' breast, “my Lord,” the betrayal question, and the question about this other disciple. There is no added “called” predicate between Simon and Cephas, and the demonstrative is rendered “this one.”
- John 21:22-23 preserves “this one” remaining, coming, the personal question and the emphatic “You come after me,” then distinguishes the brothers' report from what Jesus actually said. הנא refers to the disciple and is not an added locative “here.” The first לכ is read with coming (“come to you”), while the second remains in the question “what is that to you?” Both forms are preserved in the repeated saying. The negative saying clause keeps the actual speech predicate before the contrasting conditional quotation.
- John 21:24-25 retains the disciple's testimony about all these things, his also writing them, plural “we know” and true testimony, many other things Jesus did, “one by one,” the world's insufficiency for the books, singular “I suppose,” and the further writing clause. The negative remains with “not even the world itself.” The final relative/anaphoric link is rendered through two natural English sentences, retaining its reference as “they” in the conditional.

Source checks for Acts 1:

- Acts 1:1-9 retains the first book, Theophilus, our Lord Jesus Messiah, the apostles and Holy Spirit, forty days, eating bread together, the Father's promise, water and Spirit baptism, the kingdom question, the Father's authority, witnesses among the Samaritans, and both the cloud receiving Jesus and his being hidden. Acts 1:7 says “the time or times,” without replacing the actual terms with a remembered Greek formulation.
- Acts 1:10-14 keeps the two men in white, the ascension and return, the mountain called House of Olives, about seven stadia from Jerusalem, the upper room, all eleven names, women, Mary the mother of Jesus, and his brothers. The distance retains the source unit; no Sabbath-day journey is substituted. Judas is explicitly “son of James,” rather than “brother of James.”
- Acts 1:15-19 retains Simon Cephas, about one hundred twenty, “our brothers,” Scripture, the Holy Spirit speaking through David, Judas's guide role and ministry share, and the field, wages of sin, fall, bursting and entrails. The narrator's parenthesis in 1:18-19 is separate from Peter's speech, which resumes in 1:20. The field's name is preserved as “Haqel Dama,” followed by its explicit “Field of Blood” explanation and the source's “language of the country.”
- Acts 1:20-22 preserves both Psalm sayings and the requirement for a witness of the resurrection from among those who were with the group during Jesus' activity. The initial co-verb in 1:22 is rendered “beginning from the baptism,” continuing the preceding description of Jesus' activity; [CAL's nqp entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=nqp%232+V), causative sense 3, cites this exact verse for “to begin to do something.”
- Acts 1:23-26 keeps Joseph, Barsabbas, Justus and Matthias, the prayer to the Lord who knows all hearts, present “you choose,” ministry and apostleship, Judas's departure to his own place, casting lots, the lot coming up for Matthias and his being counted with the eleven apostles. No different tense or downward lot motion is imported from the reference translation.

Source checks for Acts 2:

- Acts 2:1-4 retains the completed days of Pentecost, the gathered people, sudden sound from heaven like a mighty wind, the filled house, divided tongues like fire resting on each, the Holy Spirit and speaking in various languages. The co-verb in 2:4 is rendered “began”; [CAL's nqp entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=nqp%232+V) records this causative co-verb sense explicitly.
- Acts 2:5-11 preserves the listeners' own languages and every group and region in the list. The actual אלניא is retained as “Alans,” without asserting a modern geographic identity or substituting the supplied reference's “Elamites.” The source has plural “Jews” and “Cappadocians” in 2:9 and “Arabs” in 2:11. The question about native languages continues through the ethnic list; “Look, we hear” then begins a declaration within the same speech.
- Acts 2:12-21 retains the separate astonishment and mockery, drinking new wine and becoming drunk, Simon Cephas and the eleven, the raised voice and address, three elapsed hours, Joel, sons and daughters prophesying, young men's visions, elders' dreams, servants and handmaids, signs and wonders, the great and terrible day, and calling on the Lord's name to live. The cardinal “three hours” is preserved rather than replaced by a modern clock time.
- Acts 2:22-24 keeps Jesus the Nazarene, manifestation by God, deeds of power, signs and wonders, foreknowledge and will, appointment, delivery to the wicked, crucifixion, killing, God's raising and the pains of Sheol. The passive agent in 2:22 may be introduced by מנ; [CAL's mn entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=mn+p) explicitly records this Syriac use. The appointment in 2:23 is a passive participle with a past auxiliary and anticipatory dative, without an additional invented male subject.
- Acts 2:25-32 retains David's foreseeing and right-hand declaration, my glory, the body resting in hope, the soul and holy one, Sheol and corruption, singular “path of life,” open speech about the patriarch and his tomb, the oath and fruit of the body, first-person “I will seat,” the Messiah's resurrection and all the witnesses. The impersonal permission expression in 2:29 is rendered naturally “I may speak.” The Psalm quotation continues through 2:28; the oath in 2:30 remains direct speech.
- Acts 2:33-40 preserves exaltation by God's right hand, the Father's promise concerning the Holy Spirit, the poured-out gift, seeing and hearing, David's non-ascension and Psalm quotation, Jesus as Lord and Messiah, the listeners' hearts, Simon and the apostles, repentance, baptism in the Lord Jesus' name, forgiveness, the Spirit's gift, the promise's recipients, God's call, testimony and the imperative to live free from the crooked generation. Both speech verbs in 2:34 retain their own source links, and the final pronoun in 2:39 is absorbed as a resumptive rather than duplicated as an extra object.
- Acts 2:41-47 retains readily receiving, believing and baptism, about three thousand souls, apostolic teaching, prayer and the explicit Eucharist, signs and wonders in Jerusalem, common property, selling and distributing according to need, temple and house, bread and food, gladness, simplicity of heart, praising God and our Lord's daily addition to the assembly. In 2:47 the passive יהיבינ is rendered “they were held in favor,” preserving the believers as recipients of favor rather than changing them into givers of mercy. The final living expression remains “those who were coming to life.”

Source checks for Acts 3:

- Acts 3:1-3 retains Simon Cephas and John, going up together, the prayer time and ninth hour, the man lame from his mother's womb, people carrying him, their habitual bringing and placing him at the Beautiful gate, his request for alms and the temple entrants. Carrying, bringing and placing remain distinct source predicates. The opening “when” clause continues into the second verse.
- Acts 3:4-9 preserves both apostles' looking and speaking, the man's expectation, gold and silver, the gift, Jesus Messiah the Nazarene, rising and walking, the right hand, immediate strengthening of feet and heels, leaping, standing, entering and praising God. Acts 3:6 has no source words for the supplied reference's “our Lord,” so that title is not added.
- Acts 3:10-14 keeps recognition of the beggar, his daily sitting and asking, amazement and wonder, holding Simon and John, the people running to Solomon's portico, Simon's reply, the power/authority question, Abraham, Isaac, Jacob, our fathers, his Son Jesus, delivery and denial before Pilate, and Pilate's judgment concerning release. Both “Holy” and “Righteous” remain explicit in 3:14, and its final request retains “be given to you.”
- Acts 3:15-18 retains the Prince of life, killing, God's raising from among the dead, all the witnesses, faith in his name, the explicit strengthening and healing, faith's gift of soundness, “my brothers,” acting “in error,” the leaders, God's earlier proclamation through all the prophets and its fulfillment. Acts 3:18 explicitly says “his Messiah,” without dropping the possessive. The two occurrences of “this” in 3:12 distinguish the deed from the man who walks.
- Acts 3:19-21 keeps repentance and turning back, blotted-out sins, times of rest coming from the Lord's presence, sending the one prepared for the hearers, Jesus Messiah, heaven receiving him and the fulfillment of the times of all the things God spoke through his holy prophets from ancient times. The rest clause belongs to source 3:19, despite its placement in the supplied reference's 3:20. No different restoration formula is substituted for the actual fulfillment/times wording.
- Acts 3:22-25 preserves Moses' prophet “like me,” the Lord's raising from among the brothers, listening, the person perishing from their people, Samuel and those who were after him, the proclaimed days, children of the prophets and of the covenant, “our fathers,” Abraham, his seed and all the earth's families. The noun in 3:23 is read as “people” in context rather than the exported homograph's base gloss “with”; its feminine possessive resumes the source's feminine “soul/person.” The Moses quotation continues through 3:23.
- Acts 3:26 retains “to you first,” God's raising and sending his Son, the blessing clause and the explicit condition “if you turn back and repent from your evil deeds.” Both turning and repenting remain present; the sentence is not changed into an unconditional statement that he turns each hearer from evil.

Source checks for Acts 4:

- Acts 4:1-7 retains speaking to the people, the priests and Sadducees, plural temple rulers, anger, teaching and preaching in the Messiah about the resurrection from among the dead, arrest, evening, about five thousand men, the assembly, Annas, Caiaphas, John and Alexander, and the high-priestly family. The resurrection noun and its “about” preposition remain together; the phrase is not reduced to “the Messiah's resurrection.” The belief auxiliary in 4:4 stays with believing, before the separate count predicate.
- Acts 4:8-12 preserves Simon Cephas, the Holy Spirit, the rulers and elders of the house of Israel, examination for the good deed and healing, Jesus Messiah the Nazarene, crucifixion and God's raising, the man's standing in sound health, the rejected stone and head of the corner, and salvation in no one else. The stone quotation is nested within Peter's speech. The modal in 4:12 is rendered “by which we are to live,” retaining its fitting/proper sense rather than treating it as bare ability; compare [CAL's wālē entry](https://cal.huc.edu/cal_entry_web.php?lemma=wly+X).
- Acts 4:13-18 keeps hearing Simon and John's openly spoken message, their being unlearned in books and ordinary men, recognition that they had gone about with Jesus, the healed lame man, inability to answer, removal from the council, the manifest sign known throughout Jerusalem, the inability to deny it, threats and the prohibition on speaking or teaching in Jesus' name. The speech's “eye/open” idiom is translated naturally rather than made into an anatomical claim.
- Acts 4:19-24 retains Simon Cephas and John, the choice of whom to obey before God, the single “cannot keep from speaking” assertion about what they saw and heard, renewed threats, release, grounds for punishment, the people praising God, the man's age of more than forty, the report to their brothers, and the united prayer to God who made heaven, earth, plural seas and everything in them. The complete age clause in 4:22 is one linked grammatical span so its copulas and age remain together while the English reads naturally.
- Acts 4:25-30 preserves the Holy Spirit, David as “your servant,” the Psalm's past verbs, the kings and rulers, the Lord and his Anointed One, the holy Son Jesus, Herod and Pilate with the nations and Israel's assembly, God's hand and will, looking and seeing the threats, servants' open proclamation, healings, wonders and signs. Herod and Pilate are the people who gathered, not additional objects of anointing. The Psalm quotation closes at 4:26; the community's prayer continues through 4:30.
- Acts 4:31-35 retains praying and supplication, the shaken gathering place, filling with the Holy Spirit and openly speaking God's word, one soul and one mind, possessions and common ownership, the apostles' resurrection testimony, great grace “with them all,” no one in need, selling fields and houses, bringing the proceeds, laying them at the apostles' feet and distribution according to need. Acts 4:32 says “mind,” without substituting “heart”; 4:33 does not add “Lord” to Jesus Messiah; 4:34 retains “and” between fields and houses.
- Acts 4:36-37 names Joseph, the apostles' naming him Barnabas, the source explanation “Son of Encouragement,” his being a Levite from Cyprus, his field, sale, price and laying it before the apostles' feet. No different personal name or additional birthplace claim is supplied.

Source checks for Acts 5:

- Acts 5:1-4 keeps Ananias and his wife Sapphira, the field sale, taking and hiding part of the price, the wife's knowledge, bringing money to the apostles' feet, Satan filling the heart, lying to the Holy Spirit and God, and the owner's continuing authority over the sale money. The questions and statements continue as one speech through 5:4. The text's knowing predicate is not expanded into a separate claim that the wife expressly consented.
- Acts 5:5-11 preserves falling and dying, the young men's gathering and carrying the body and burial, three hours, the wife's lack of knowledge, Simon's price question and her answer, agreement to test the Lord's Spirit, the buriers at the door, her falling before plural “their feet,” and burial beside her husband. Acts 5:9 begins with the causal “Because you agreed,” rather than a substituted “Why?” question. The same-hour idiom in 5:10 is rendered “at that moment.”
- Acts 5:12-16 retains many signs and wonders, gathering in Solomon's portico, the others' reluctance to approach, the people's honor, added believers in the Lord, multitudes of men and women, the sick in the streets on beds, Simon's coming, his shadow, the surrounding cities of Jerusalem, unclean spirits and all being healed. Acts 5:15 renders the focus before “his shadow” as “at least”; [CAL's ˀpn entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=%29pn+c) records the Syriac “if only” use before nouns. The Lord in 5:14 belongs with believing, rather than being reassigned as the target of adding.
- Acts 5:17-23 keeps the jealous high priest and those of the Sadducees' teaching, laying hands on the apostles, seizing and binding them, the prison, the Lord's angel at night, plural doors, bringing them out, going and standing in the temple, all the words of life, departure at daybreak, entry and teaching. The prison report retains secure locking, guards at the doors, opening and finding nobody inside; its introductory אמרינ is a third-person plural participle, “saying.”
- Acts 5:24-28 preserves the chief priests and temple rulers hearing and wondering, the messenger's report, the confined men now standing and teaching in the temple, bringing them without force for fear of the people, the whole council and the high priest beginning to speak. His opening is a strict-command question, followed by Jerusalem filled with the apostles' teaching and the desire to bring this man's blood upon the authorities. The beginning co-verb and the following saying infinitive keep their own source links.
- Acts 5:29-34 retains Simon with the apostles, obedience to God more than human beings, the God of our fathers, Jesus raised and killed by hanging on a tree, God's establishing him as leader and giver of life and also exalting him, repentance and forgiveness for Israel, the witnesses and Holy Spirit, anger and considering killing, and Gamaliel's honored standing and order to remove the apostles briefly. Acts 5:32 says God “gave” the Spirit to those who “believe in him,” without substituting a different tense or the familiar “obey him” reading.
- Acts 5:35-39 keeps caution, consideration, Theudas' claim, about four hundred men, his killing and the followers' scattering and coming to nothing, Judas the Galilean, registration for the head tax, leading many astray, his death and all his followers' scattering. Both “this plan” and “this work,” their dissolving and passing away, human versus divine origin, inability to overthrow and the danger of standing against God remain explicit. No external dates or chronological harmonization are supplied.
- Acts 5:40-42 retains agreeing with Gamaliel, calling the apostles, scourging, the speaking prohibition in Jesus' name, release, rejoicing in worthiness to be dishonored for the name and continuing daily teaching in the temple and at home. The assent clause belongs to source 5:40, despite the supplied reference's placing it at the end of 5:39. The final preaching clause explicitly includes “our Lord Jesus Messiah.”

Source checks for Acts 6:

- Acts 6:1-4 retains the increasing disciples, Hellenist disciples' complaint against the Hebrews, neglected widows and daily service, the twelve apostles, the whole assembly, the word of God and serving tables. The opening resumptive pronoun belongs to “those days,” without adding a separate “among them” group. The speech preserves searching and choosing seven, “my brothers,” attestation, fullness of the Spirit of the Lord and wisdom, appointment over the matter, and continued prayer and ministry of the word.
- Acts 6:5-6 keeps Stephen, Philip, Prochorus, Nicanor, Timon, Parmenas and Nicolas, the latter described as an Antiochian proselyte. Stephen is full of faith and the Holy Spirit. The selected men “stood” before the apostles: the source's intransitive verb is retained rather than replaced by a different actor presenting them. Prayer and laying a hand on them remain distinct predicates.
- Acts 6:7-10 distinguishes God's word growing from the number of disciples increasing greatly in Jerusalem; each past auxiliary stays with its own predicate. The final group in 6:7 is many people “from among the Jews,” without substituting “priests.” Acts 6:8 has “grace and power,” and the synagogue/group list retains the Freedmen, Cyrenians, Alexandrians, Cilicia and Asia. In 6:10 the Spirit “was speaking through him,” preserving the source speaker rather than reversing the roles into Stephen speaking by the Spirit.
- Acts 6:11-14 retains sending men and instructing them to make their allegation, rather than supplying a different inducement verb; stirring up the people, elders and scribes; coming, rising against, seizing and bringing Stephen before the council; and false witnesses speaking against the law and this holy place. Their second statement continues into 6:14, including hearing him say that this Jesus the Nazarene will destroy the place and change the customs Moses delivered “to you,” not “to us.” These remain the witnesses' quoted allegations.
- Acts 6:15 preserves all those sitting in the council looking at him and seeing his face “like the face of an angel.” The simile is retained without adding a claim that he became an angel.

Source checks for Acts 7:

- Acts 7:1-7 preserves the high priest's question, Stephen's address to “our brothers and our fathers,” the God of glory, Abraham, Mesopotamia and Haran, departure from land and kindred, the promise without an existing son or even a footstep of inheritance, four hundred years of foreign sojourn and mistreatment, judging the enslaving nation and serving God in this place. The God quotation continues through 7:7 inside Stephen's continuing speech.
- Acts 7:8-16 retains circumcision on the eighth day, Isaac, Jacob and “our twelve fathers,” jealousy of Joseph, Egypt, Pharaoh's appointment, the famine and Canaan, the first and second journeys, Joseph actively making himself known to his brothers, and seventy-five persons. In 7:16 both transport and deposition are singular, and the text names Shechem, Abraham as buyer and Hamor's sons. The wording is retained without harmonizing the burial account with another passage.
- Acts 7:17-22 keeps the sworn promise, the people's increase and strength, the king who did not know Joseph, mistreated fathers, the infant order, Moses beloved by God, three months in his father's house, his casting out “by his mother,” Pharaoh's daughter and his Egyptian education. The actual adjective in 7:22 is rendered “prepared” in words and deeds; [CAL's ʿtyd entry](https://cal.huc.edu/cal_entry_web.php?lemma=%28tyd+A) records the prepared/made-ready sense rather than requiring a different remembered adjective.
- Acts 7:23-30 retains forty years of age, the thought entering Moses' heart, visiting his brothers, the oppressed kinsman, avenging and doing justice, killing the Egyptian, his expectation that the brothers understood God's deliverance through his hand, the peace appeal and its rejection, yesterday's killing, flight to Midian and two sons. The next forty-year period is explicitly spent “there”; the angel of the Lord appears in the wilderness of Mount Sinai in fire in a bush.
- Acts 7:31-38 preserves seeing and approaching the sight, the Lord speaking in a voice, the God of Abraham, Isaac and Jacob, Moses' trembling, sandals and holy ground, seeing oppression and hearing groaning, descending to deliver and sending Moses to Egypt. God sends the rejected Moses as ruler and deliverer through the angel; signs, wonders and mighty deeds, Egypt, the Sea of Reeds and forty wilderness years remain explicit. The prophet saying includes “Lord God,” “from among your brothers,” “like me” and listening to him. The final living words are received “to give to us.”
- Acts 7:39-43 retains the fathers' refusal to submit, turning back to Egypt in their hearts, Aaron's requested gods, uncertainty about Moses, making the calf, sacrifices to idols and rejoicing in their own handiwork. The prophetic question keeps forty years, a slaughtered animal or a sacrifice, and Israel. The actual names are retained as Malchum and Rephan, with the images explicitly made “to worship them” and the removal beyond Babylon.
- Acts 7:44-50 keeps our fathers' tabernacle of testimony, the command and pattern shown to Moses, the fathers bringing it in with Joshua, the inherited land, expelled nations and the tabernacle being carried until David's days. David's request is for the God of Jacob, and Solomon builds the house. The Most High's refusal to dwell in what is made by hands, heaven as throne, earth as footstool, the house/rest-place questions and God's own hand remain in the quoted prophecy.
- Acts 7:51-53 retains the stiff-neck and uncircumcised-heart/hearing descriptions, the third-person possessives rendered naturally through “whose,” then the direct “you” standing against the Holy Spirit. The causal question retains both persecuting and killing, those announcing the Righteous One's coming, the hearers' handing him over and killing him, the law through angels' command and failure to keep it. Stephen's outer speech closes only after 7:53; the prophetic quotations remain nested within it.
- Acts 7:54-58 preserves anger in the hearers' souls and gnashing, Stephen explicitly full of faith and the Holy Spirit, his looking and seeing God's glory, Jesus standing at God's right hand, the Son of Man, the loud cry and stopped ears, rushing, seizure, removal outside the city and stoning. Those who testified against him lay their garments at the feet of the young man Saul; no additional action by Saul is inserted here.
- Acts 7:59-60 retains stoning Stephen while he prays and speaks, “Our Lord Jesus, receive my spirit,” kneeling and the loud cry, “Our Lord” and the request not to hold this sin against them, and falling asleep after speaking. Both first-person plural Lord possessives remain explicit.
