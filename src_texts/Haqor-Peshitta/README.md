# Word-linked English of the Peshitta

`translation.tsv` contains Haqor's English renderings in Peshitta
word order, using James Murdock's public-domain translation as a reference.
The unmodified reference remains in `src_texts/Murdock/murdock.tsv`.
The goal is reviewed, word-linked English for all 7,958 Peshitta verses.
This is an incremental translation project: only the verses in the TSV have
word links. Other verses retain the existing Murdock translation and select
whole verses in the English reader.

Current coverage: Matthew 1-28, Mark 1-16, Luke 1-21, 2 John, 3 John, and Jude
(2,773 of 7,958 Peshitta verses; 5,185 remain). Luke 22 is next.
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
  hearers. In 21:29 Jesus was telling the parable. In 21:30 the sprouting
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
