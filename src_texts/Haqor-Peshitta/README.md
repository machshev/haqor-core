# Word-linked English of the Peshitta

`translation.tsv` contains Haqor's English renderings in Peshitta
word order, using James Murdock's public-domain translation as a reference.
The unmodified reference remains in `src_texts/Murdock/murdock.tsv`.
The goal is reviewed, word-linked English for all 7,958 Peshitta verses.
This is an incremental translation project: only the verses in the TSV have
word links. Other verses retain the existing Murdock translation and select
whole verses in the English reader.

Current coverage: Matthew 1-28, Mark 1-16, Luke 1-24, John 1-21, Acts 1-28, Romans 1-16, 1 Corinthians 1-16, 2 Corinthians 1-6, 2 John, 3 John, and Jude
(5,825 of 7,958 Peshitta verses; 2,133 remain). 2 Corinthians 7 is next.
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
Decode numeric morphology with `src_texts/SEDRA/SEDRA3.README.TXT`; a lemma
gloss is not an inflected translation. Check causal particles, relative
prefixes, and copular pronouns explicitly in the final linked text.
Use the generator's established “ecclesia/ecclesias” terminology for
congregations so the reviewed source and runtime text match exactly.
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

Source checks for Acts 8:

- Acts 8:1-4 retains Saul's approval and participation in Stephen's killing, that day's persecution of the ecclesia in Jerusalem, scattering through Judea's villages and among the Samaritans except for the apostles, Stephen's gathering and burial by believing men, and Saul's persecution of God's ecclesia. His entry into houses, dragging away men and women and delivery to prison remain distinct from the scattered believers' proclamation.
- Acts 8:5-8 keeps Philip's descent to a Samaritan city, proclamation concerning the Messiah, the people's listening and conviction because of the signs, and the city's great joy. In 8:7 the masculine human criers are distinguished from the feminine-plural spirits that come out of them; the translation does not silently substitute a different subject throughout the sentence.
- Acts 8:9-13 preserves Simon's long residence, sorceries, misleading the Samaritan people, self-exaltation, the people's description of him as God's great power, and their subsequent belief and baptism. Philip proclaims God's kingdom in the name of “our Lord Jesus Messiah”; Simon's belief, baptism, attachment to Philip and astonishment remain explicit.
- Acts 8:14-19 names Simon Cephas and John as the apostles sent from Jerusalem, distinguishes receiving God's word from receiving the Holy Spirit, and retains baptism in the name of “our Lord Jesus.” The laying on of a hand, reception of the Spirit and Simon's offer of money and request for authority remain separate actions.
- Acts 8:20-24 retains Simon Cephas' condemnation, God's gift and worldly wealth, no portion in “this faith,” the heart not upright before God, repentance, prayer, possible forgiveness of the heart's deceit, bitter gall and bonds of iniquity. Simon's answer asks the apostles to pray to God for him; it does not add an unexpressed resolution of his story.
- Acts 8:25-31 preserves Simon and John's testimony, teaching, return and preaching in Samaritan villages, the Lord's angel, the southward desert road from Jerusalem to Gaza, and the eunuch from Cush serving Candace, queen of the Cushites, in charge of her treasury. Verse 8:29 says “the Spirit,” without adding “Holy.” The eunuch's question, need for instruction and invitation into the chariot remain explicit.
- Acts 8:32-35 keeps the lamb, slaughter, sheep, shearer, silence, unopened mouth, humiliation, imprisonment and judgment, generation question and life taken from the earth. The humiliation phrase belongs to source 8:33, although the supplied reference places it in 8:32. The Scripture quotation continues across both verses; Philip begins from this Scripture to proclaim “our Lord Jesus.”
- Acts 8:36-40 translates verse 37's actual dataset words, including wholehearted belief and the confession that Jesus Messiah is God's Son. Both men descend to the water, Philip baptizes the eunuch, the Lord's Spirit snatches Philip away, and the eunuch continues rejoicing. Azotus, preaching in all the cities and arrival at Caesarea are retained without adding a route or place identification to the source.

Source checks for Acts 9:

- Acts 9:1-6 retains Saul's threats and murderous fury against the disciples of “our Lord,” plural letters requested from the high priest for the synagogues at Damascus, men or women following this way, binding and bringing them to Jerusalem, and the sudden heavenly light. The goads saying occurs in source 9:4, and Jesus identifies himself as “the Nazarene” in 9:5. The instruction to enter the city includes what Saul ought to do, rather than a statement of mere ability. The singular chief-of-priests title in 9:2 is kept distinct from the plural chief priests in 9:14; compare [CAL’s compound entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=rb%40khnyn+N).
- Acts 9:7-12 keeps the companions' hearing only the voice while seeing no one, Saul's open eyes without sight, leading him by his hands, three days without sight, food or drink, and Ananias' vision in Damascus. The Lord's instructions retain the street called Straight, Judas' house and Saul from the city of Tarsus. The temporal clause “while he was praying” begins in source 9:11 and leads into his seeing Ananias in 9:12; the vision includes laying a hand on him so that his eyes open.
- Acts 9:13-16 preserves Ananias' report about harm to the Lord's saints in Jerusalem, authority from the chief priests and binding all who call on the Lord's name. The chosen vessel, carrying the name among the nations, kings and Israel's sons, and suffering on account of the name remain in the Lord's reply, which continues through 9:16.
- Acts 9:17-22 keeps “Saul, my brother,” “our Lord Jesus,” his appearance on Saul's journey, Ananias' sending, opened eyes and filling with the Holy Spirit. The falling material is compared with plural scales; baptism, food, renewed strength, time with the Damascus disciples, proclamation in the Jews' synagogues and Jesus as God's Son remain explicit. The hearers recall both Saul's earlier persecution and his being sent to bind and bring believers to the chief priests.
- Acts 9:23-30 retains the plot, guarding the city gates by day and night, lowering Saul from the wall in a basket at night, the Jerusalem disciples' fear, and Barnabas bringing him to the apostles. In 9:27 Saul “saw the Lord,” rather than substituting the supplied reference's “the Lord appeared to him.” His open proclamation, going in and out, disputes with Jews who knew Greek, the death threat and the brothers' nighttime journey to Caesarea and sending to Tarsus are retained.
- Acts 9:31-35 preserves singular “ecclesia,” all Judea, Galilee and Samaria, peace, being built up, walking in the fear of God and increase in the Holy Spirit's encouragement. Simon visits the saints at Lydda; Aeneas' paralysis and eight years in bed, Jesus Messiah's healing, the bed instruction and immediate rising remain distinct. The residents of Lydda and Saron turn to God; “Lord” is not substituted here.
- Acts 9:36-39 names Tabitha as a female disciple in Joppa and retains her good deeds and almsgiving without adding an absent Dorcas/gazelle explanation. Her illness, death, washing, upper room and the two men sent for Simon remain explicit. The request in 9:38 is that he not hesitate to come; [CAL's mʾn entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=m%29n+V) documents this impersonal feminine verb with a dative and following action. In 9:39 the widows gather around “him,” not the supplied reference's “her,” and show garments Tabitha had given them, without substituting a making verb.
- Acts 9:40-43 retains putting all the people outside, kneeling, praying, turning to the corpse and the command “Tabitha, rise.” Her opening her eyes, seeing Simon and sitting up, his outstretched hand and raising her, and presenting her alive to the saints and widows remain separate actions. Many believe in “our Lord,” and Simon's stay in Joppa at the house of Simon the tanner is retained without adding a duration or address absent from the source.

Source checks for Acts 10:

- Acts 10:1-8 names Cornelius, Caesarea and the Italian cohort, retains his being a centurion, righteousness, fear of God with his whole household, almsgiving and continual prayer. The vision is about the ninth hour of the day; the temporal preposition is not rendered as an additional person “before him.” His prayers and alms, the memorial before God, sending to Joppa for Simon called Cephas and lodging with Simon the tanner by the sea remain in the angel's continuous speech. Two household members and one soldier who feared God and obeyed Cornelius remain distinct.
- Acts 10:9-16 preserves the next day's journey, the roof and sixth-hour prayer, hunger and food preparation, the trance, open heavens, a vessel fastened at four corners and resembling a great linen sheet, and its descent to earth. Four-footed creatures, creeping things and birds, the slaughter/eat command, “my Lord,” Simon's refusal and never having eaten defiled and unclean food remain explicit. The second utterance and the event occurring three times are retained without converting the source's hours to a modern clock.
- Acts 10:17-23 keeps Simon's pondering, the men sent by Cornelius, the house inquiry, the courtyard gate, Simon called Cephas, and the Spirit's explicit three men. Its command to go without doubting continues through 10:20. The messengers describe Cornelius as upright and God-fearing, retain the Jewish people's testimony and the holy angel's instructions, and ask Simon to come so Cornelius may hear a message. The following day's rising and departure and some Joppa brothers accompanying “him” are retained.
- Acts 10:24-29 has singular “he entered” Caesarea, rather than the supplied reference's plural arrival, and retains Cornelius' relatives and close friends, meeting Simon, falling at his feet, Simon's raising him and “I too am a human being.” The statement about association with a foreigner is kept within Simon's speech, including the stranger not being of his lineage, God's showing him not to call anyone unclean or defiled, his coming readily and his question about the summons.
- Acts 10:30-33 preserves four days up to the present, the fasting reference with “since,” ninth-hour prayer in Cornelius' own house, the man in white, heard prayer and remembered alms. It does not add a continuous four-day fast or convert the hours to modern times. Cornelius is the speaker of 10:30; the angel addresses him by name in 10:31. The angel's nested quotation ends in 10:32; Cornelius continues through 10:33, including “you did well to come,” “we are all before you” and wanting to hear everything God commanded.
- Acts 10:34-38 retains God's impartiality, all nations, fearing him and practicing righteousness, the message sent to Israel's sons, and both peace and tranquility. The message begins from Galilee after John's baptism; [CAL's nqp entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=nqp%232+V) attests the beginning use, including a closely parallel beginning-from-baptism clause. Jesus of Nazareth, God's anointing with the Holy Spirit and power, healing and God being with him remain explicit. “The Evil One” in 10:38 follows [CAL's byš entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=by%24+A), which cites this exact verse under that substantival sense. No absent “doing good” clause is supplied from another text.
- Acts 10:39-43 keeps the witnesses, Judea and Jerusalem, hanging on a tree and killing, God's raising on the third day, and being seen openly. The open-eye expression is rendered idiomatically, as documented in [CAL's ˁyn bglˀ entry](https://cal.huc.edu/cal_entry_web.php?lemma=%28yn%40bgl%29+N). God chose the witnesses, who actually ate and drank with Jesus after his resurrection; the meal is not changed into a future purpose of their selection. The command to proclaim and testify, appointment as judge of living and dead, prophetic testimony and forgiveness for everyone who believes in his name remain in Simon's speech through 10:43.
- Acts 10:44-48 preserves the Holy Spirit descending on those hearing, the circumcised brothers' amazement and astonishment, the gift poured out on the Gentiles also, speaking in various languages and magnifying God. Simon's speech introduction belongs to source 10:46, although the supplied reference places it in 10:47. His water/baptism question retains reception of the Holy Spirit “just as we have”; the subsequent command uses the full name “our Lord Jesus Messiah.” The request to stay for days adds no numerical duration.

Source checks for Acts 11:

- Acts 11:1-4 keeps the apostles and brothers in Judea hearing that the Gentiles too received God's word, Simon's ascent to Jerusalem, the circumcision group's dispute and the allegation about entering among uncircumcised men and eating with them. The allegation's actual verbs are third person, so it remains indirect “he,” without importing a second-person accusation. Simon begins his account step by step; [CAL's nqp entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=nqp%232+V) explicitly cites 11:4 under the beginning/resuming construction.
- Acts 11:5-10 preserves prayer in Joppa, a descending vessel like a linen sheet, fastening at its four corners, its lowering from heaven and coming right to Simon, four-footed animals, creeping things and birds. The voice addresses Simon, the slaughter/eat command and “my Lord” remain explicit, and the heavenly reply retains what God cleansed and the prohibition on defiling it. Three repetitions and “everything” being taken up are retained. These inner utterances remain inside Simon's outer account, which continues beyond 11:10.
- Acts 11:11-14 names Cornelius and Caesarea, the three men, the courtyard gate and Simon's lodging; passive-agent “by Cornelius” is distinct from origin “from Caesarea.” The Spirit's instruction, the six accompanying brothers and entering the man's house remain explicit. The angel's command names Joppa, Simon and Cephas; the promised words are those by which “you and all your household will live,” retaining the source's active life verb rather than mechanically substituting a different passive formulation.
- Acts 11:15-18 keeps Simon beginning to speak “there,” the Holy Spirit descending on them as on the original witnesses, remembering “our Lord,” John's active baptizing with water and the promised baptism with the Holy Spirit. The equal gift is expressly given to the Gentiles who believed in “our Lord Jesus Messiah,” as to the witnesses; the believing clause is not reassigned to a different group. Their response retains quieting, glorifying God and repentance leading to life. The ambiguous kbar is read contextually as “already,” with the past giving verb; [CAL records both already and perhaps in Syriac](https://cal.huc.edu/cal_entry_web.php?lemma=kbr+X). No exclusive lexical claim is made for the alternative.
- Acts 11:19-21 retains scattering because of the oppression concerning Stephen, Phoenicia, the region of Cyprus and Antioch, the initial restriction to Jews alone and the subsequent people from Cyprus and Cyrene speaking with Greeks and preaching about “our Lord Jesus.” No unexpressed ethnic qualification is added to “Greeks.” The Lord's hand, belief and turning to the Lord remain distinct actions.
- Acts 11:22-24 keeps the news reaching the Jerusalem ecclesia and sending Barnabas to Antioch, his arrival, seeing God's grace, rejoicing and urging wholehearted devotion to “our Lord.” The sons-of-the-ecclesia community idiom is rendered naturally as its members. Barnabas remains a good man filled with the Holy Spirit and faith, and the many people are added to “our Lord”; neither first-person plural Lord possessive is dropped.
- Acts 11:25-26 retains the journey to Tarsus to seek Saul, finding him and bringing him along to Antioch, a whole year assembled together in the ecclesia, teaching many people and the source's “from then on” and “first” naming of the disciples as Christians at Antioch. No additional duration, journey membership or reason for the name is supplied.
- Acts 11:27-30 preserves prophets arriving from Jerusalem, Agabus standing, making the famine known by the Spirit, the great famine throughout the land and its occurrence in the days of Claudius Caesar. No modern date or wider historical extent is inferred. The disciples' contribution follows what each had, with aid sent for the brothers in Judea and delivery through Barnabas and Saul to the elders there.

Source checks for Acts 12:

- Acts 12:1-4 keeps Herod the king's explicit name Agrippa, seizure of people of the ecclesia, intended mistreatment, killing James the brother of John with the sword, and proceeding to seize Simon Cephas when this pleased the Jews. The days of Unleavened Bread, prison, sixteen soldiers and guarding remain explicit. “After Passover” governs the planned handing over to the Jewish people, rather than postponing the guarding.
- Acts 12:5-9 preserves constant prayer to God by the ecclesia on Simon's behalf, the night before the planned morning handover, two soldiers, two chains and other guards at plural prison doors. The angel stands over Simon, light fills the building, and the angel strikes his side, wakes him and speaks. The belt, sandals, cloak, following, falling chains from plural hands and uncertainty about reality versus a vision remain separate actions. The causal “for” in 12:9 is retained.
- Acts 12:10-11 retains the first and second watches, iron gate opening of its own accord, passage along one street and the angel's departure. Simon's realization and statement name “the Lord” as sender of his angel, rather than the supplied reference's “God,” and retain release from Herod the king and the Jews' plotting. Agrippa is not added again where the actual verse says only Herod.
- Acts 12:12-14 names Mary as the mother of John called Mark, the many brothers gathered and praying, the courtyard gate and the maid Rhoda coming out to answer. Her recognition of Simon's voice, joy, failure to open for him, running back and announcement that he is standing at the courtyard gate remain explicit. Her actions are not reassigned to Simon.
- Acts 12:15-17 preserves the group's disbelief, Rhoda's insistence, and their speculation “perhaps it is his angel,” without replacing the actual angel noun with the supplied reference's “ghost.” Simon continues knocking; the group comes out and sees him. His hand signal, silence, entering and reporting the Lord's release, telling James and “our brothers,” and departing to another place remain distinct. No destination for his departure is supplied.
- Acts 12:18-19 retains morning, the soldiers' great uproar about what happened to Simon, Herod's search, failure to find him, judgment of the guards and order that they die. The departure is from Judea and the stay is in Caesarea; an ethical dative is not expanded into an additional “from him” destination or relationship.
- Acts 12:20-23 keeps the causal anger toward the Tyrians and Sidonians, their assembled approach, persuading Blastus the king's chamberlain, the peace request and dependence on supplies from Herod's kingdom. The certain day, royal clothing, tribunal and address to the assembly remain explicit. The crowd's plural utterances and contrast with human beings are preserved; [CAL's compound entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=brt%40ql+N) records the voice/utterance expression as a lexical unit. The striking angel is “of the Lord,” not the supplied reference's “of God”; lack of glory to God, the same hour, worm infestation and death remain explicit.
- Acts 12:24-25 calls the proclaimed and growing message God's “good news,” retaining its actual noun rather than substituting “word.” Barnabas and Saul return explicitly from Jerusalem to Antioch after completing their service, and take John called Mark with them. No different travel direction, additional relative or historical date is supplied.

Source checks for Acts 13:

- Acts 13:1 names the prophets and teachers in Antioch's ecclesia: Barnabas, Simeon called Niger, Lucius from Cyrene, Manael and Saul. “Manael” follows the actual stored form מנאיל, ending in lamed, rather than silently replacing it with the supplied gloss's “Manaen.” The relationship is son of the plural guardians/rearers of Herod the tetrarch, without substituting the familiar foster-brother wording.
- Acts 13:2-5 retains fasting and supplication to God, the Holy Spirit's command to set apart “Saul and Barnabas” in that order, their assigned work, fasting, prayer and laying a hand on them. Sending by the Spirit, Seleucia, travel by sea to Cyprus, Salamis, the Jewish synagogues, the word of “our Lord” and John's service remain explicit.
- Acts 13:6-12 preserves the whole island, Paphos, the Jewish sorcerer and false prophet named Barshuma, his association with the wise proconsul Sergius Paulus, the proconsul's summons to Saul and Barnabas and request for God's word. The name-interpretation clause gives Elymas; neither name is replaced by an unexpressed Bar-Jesus. Saul called Paul, filling with the Holy Spirit, the accusation, the Lord's straight ways and hand, temporary blindness, mist/darkness, seeking a guide and the proconsul's belief remain distinct.
- Acts 13:13-17 expressly names Paul and Barnabas on the sea journey from Paphos to Perga in Pamphylia, John's separation and journey to Jerusalem, and their arrival in Pisidian Antioch. The synagogue, Sabbath, Law and Prophets, elders' “our brothers” address, encouragement request and Paul's hand gesture are preserved. God's choice concerns “our fathers”; both exalting and making them great remain, without forcing a numerical multiplication gloss.
- Acts 13:18-22 retains nourishing the people forty years in the wilderness, seven nations in Canaan and the land inheritance. Four hundred and fifty years belongs with the judges until Samuel, without harmonizing it to another chronology. Saul is Kish's son from Benjamin and is given for forty years. David's raising as their king, Jesse, God's heart and all his desires remain in the testimony, with its quotation nested inside Paul's speech.
- Acts 13:23-28 preserves God's raising Jesus as a deliverer from this man's seed for Israel as promised, and explicitly sending John before his coming. John completes his ministry, denies the mistaken identity and speaks of the coming one's sandal straps. The address has “our brothers,” Abraham's line and those fearing God “with you”; the word of life is sent “to you.” The feminine “it” in 13:27 resumes the message, and the text names prophetic writings read every Sabbath. The plural killing verb in 13:28 is retained as asking Pilate to let them kill him.
- Acts 13:29-35 keeps completion of what was written, taking Jesus down from the cross, burial, God's raising from among the dead, appearance for many days and the present witnesses who accompanied him from Galilee to Jerusalem. God fulfills the fathers' promise for “us, their children.” The second Psalm's “my Son” belongs with “you”; the following “I” belongs to today's begetting, not the preceding clause. The singular sure grace of David and the past negative “You did not allow your Holy One to see decay” remain in their respective quotations.
- Acts 13:36-41 retains David serving God's will in his generation, sleeping, addition to his fathers and seeing decay, contrasted with the one God raised. “My brothers,” forgiveness, being justified in this one and the law of Moses remain explicit without adding a theological agent or qualification absent from the source. The prophetic warning is nested inside Paul's speech, and both close at 13:41.
- Acts 13:42-49 does not insert ethnic names into the pronoun-based request for the next Sabbath. Jews and God-fearing proselytes follow, and God's grace remains the object of their urged devotion. The whole city gathers, without adding “almost.” Opposition, open speech, the word first to the hearers, pushing it away and judging themselves unworthy of eternal life remain explicit. The command is from “our Lord,” and its quoted mission has light and life to the earth's ends. The Gentiles praise God; those appointed to eternal life believe, without an added agent or explanation of appointment. The Lord's word is spoken throughout the region.
- Acts 13:50-52 retains the city's chiefs and wealthy women who feared God “with them,” rather than moving God into a different span or treating the phrase as a separate co-presence predicate. Persecution targets Paul and Barnabas, followed by expulsion from their borders. The dust of the travelers' feet is shaken off against them; arrival is at Iconium, and the disciples are filled with joy and the Holy Spirit.

Source checks for Acts 14:

- Acts 14:1-5 keeps entry into the Jewish synagogue, many Jews and Greeks believing, the unpersuaded Jews' incitement, mistreatment of the brothers, and the divided city crowd. The Lord testifies to the word of his grace through signs and wonders performed through their hands. The eye/revelation expression is rendered idiomatically as “openly,” consistently with earlier occurrences. The Gentiles, Jews and their chiefs remain coordinated in the assault and intended insult and stoning.
- Acts 14:6-9 retains both departure and taking refuge, the Lycaonian cities Lystra and Derbe and surrounding villages, and continued preaching. The man is sitting in Lystra, afflicted in his feet, lame from his mother's womb and never having walked. His faith is rendered in its healing context as “faith to be healed”; [CAL's ḥyy entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=xyy+V) attests healing within the G-stem's live/revive senses. The clause continues into Paul's command in 14:10.
- Acts 14:10-13 retains the loud voice, “I say to you,” the full name “our Lord Jesus Messiah,” rising on his feet and all three ensuing actions: leaping, standing and walking. The crowd speaks in the region's language; no specific language name is inserted. Barnabas is called “the lord of the gods,” without replacing this actual title with Zeus; Paul is Hermes because he began speaking. The priest, location outside the city, oxen, wreaths, courtyard gate, lodging and intended sacrifice remain explicit.
- Acts 14:14-17 preserves Barnabas and Paul's hearing, torn garments, leaping and going out toward the crowd, and their cry. Their speech continues through 14:17 and retains their shared human susceptibility to suffering, the command to turn from useless things to the living God, creation of heaven, earth and plural seas, and everything in them. Former generations and all nations following their own ways remain distinct from God's testimony through good gifts from heaven, rain, fruits growing in their seasons, and hearts filled with food and gladness. The beneficiaries in 14:17 are third-person plural, not an imported “you” or “us.”
- Acts 14:18-20 retains the difficulty of restraining the people so that no one would sacrifice to them. Jews arrive from Iconium and Antioch, stir up the people against them, stone Paul and drag him outside the city because they think he has died. The disciples gather around him; he rises, enters the city and departs the next day with Barnabas for Derbe. No assertion about his actual death is added.
- Acts 14:21-23 preserves preaching to the city's inhabitants and teaching many, the return to Lystra, Iconium and Antioch, strengthening the disciples' souls, remaining in the faith and necessary entry into God's kingdom through much tribulation. Appointment of elders is in every ecclesia, with fasting together and prayer. Entrusting is to “our Lord,” in whom they believed. Ethical datives are absorbed into natural English without inventing an additional recipient on the return journey.
- Acts 14:24-26 retains the region of Pisidia, Pamphylia, proclamation of the Lord's word in the city of Perga, descent to Attalia and travel by sea to Antioch. The reason for the return is their prior entrusting from there to the Lord's grace for the work now completed; it is not changed into a new commission.
- Acts 14:27-28 keeps gathering the whole ecclesia and recounting everything God had done with them. God remains the singular subject who opened a door of faith to the Gentiles. Their long stay is with the disciples; no numerical duration is added.

Source checks for Acts 15:

- Acts 15:1-4 retains men coming down from Judea, their teaching that circumcision according to the law's custom is required for life, and Paul and Barnabas's trouble and debate with them. Their ascent is to the apostles and elders in Jerusalem because of this question. The ecclesia escorts and sends them; all Phoenicia and the Samaritans are retained without adding towns. The receiving groups are the ecclesia, apostles and elders, and the report concerns all God did with them.
- Acts 15:5-11 describes believers from the Pharisees' teaching, their rising and explicit demand to circumcise the Gentiles and command observance of Moses's law. Simon's speech preserves the early days, his own mouth, the Gentiles hearing the good news and believing, God's knowledge of hearts, the Holy Spirit given as to the speakers, no distinction and cleansing through faith. The yoke is on the disciples' necks and was unbearable to both our fathers and us. Life through the grace of “our Lord Jesus Messiah” retains the active live verb rather than automatically importing a passive saved formula.
- Acts 15:7 and 15:13 distinguish the actual vocalisation and suffix information of the brothers address: 15:7 has aḥīn without a possessive suffix; James's 15:13 has aḥayn and the explicit first-person plural suffix, rendered “our brothers.” The whole assembly's silence, listening to Paul and Barnabas and all the signs and wonders God did through their hands among the Gentiles remain before James's address.
- Acts 15:14-18 preserves Simon's report of God's beginning to choose a people from the Gentiles for his name, and the prophets' agreement. The nested citation retains return, setting up David's fallen tabernacle, rebuilding what fell from it and raising it, the remaining humans and all Gentiles seeking the Lord, and the name called upon them. The setting-up form is read with the surrounding first-person future verbs, without changing the quoted speaker because of ambiguous morphological tagging. God's works being known from eternity adds no unexpressed “to God” recipient.
- Acts 15:19-21 preserves James's “I say” without turning its repeated pronoun/copula into a second declaration. The not-troubling predicate is third-person plural, and the Gentiles are turning to God. The proposed message retains pollution of sacrificed meat, sexual immorality, strangled meat and blood, without adding an explicit idol noun. Moses has heralds from former generations in every city, in synagogues, with reading every Sabbath. James's speech continues through 15:21.
- Acts 15:22-27 retains the apostles and elders with the whole ecclesia, selected delegates, Antioch, Paul and Barnabas, Judas called Barsabbas and Silas, and their leadership among the brothers. The letter addresses Gentile brothers in Antioch, Syria and Cilicia and includes the troubling circumcision-and-law demand, which the writers had not commanded. Deliberation, choosing and sending remain finite actions, and “our beloved Paul and Barnabas” retains the possessive designation. Their surrendered lives are for the full name “our Lord Jesus Messiah”; Judas and Silas are sent with them to convey the same matters orally.
- Acts 15:28-34 keeps the Holy Spirit and the writers, no additional burden beyond necessary things, all four abstentions and the final “Be steadfast in our Lord.” The requiring/necessity form is third-person feminine plural, not a first-person “we require.” The letter closes at 15:29. The delegates come to Antioch, gather all the people and deliver the letter; no absent descent verb is supplied. Rejoicing and encouragement are distinct. The d-bayt expression in 15:32 is rendered as Judas and Silas's company, following the supplied reference's associates sense, without inventing a particular household or relocating them as objects. The brothers dismiss them in peace to the apostles, and the stored verse 15:34 about Silas remaining is retained.
- Acts 15:35-41 preserves Paul and Barnabas remaining in Antioch, teaching and proclaiming God's word with many others, and Paul's proposal to revisit the brothers in every city. John called Mark, his earlier departure in Pamphylia and failure to accompany them remain the reason for Paul's refusal. Separation is because of this dispute, without adding an unexpressed sharpness description. Barnabas takes Mark and they travel by sea to Cyprus; Paul chooses Silas for himself and departs commended by the brothers to God's grace. His route is Syria and Cilicia, strengthening plural ecclesias.

Source checks for Acts 16:

- Acts 16:1-5 preserves Derbe and Lystra, the disciple Timothy, his believing Jewish mother and his father described as a Gentile, rather than replacing that description with an unexpressed Greek identity. Testimony comes from all the disciples in Lystra and Iconium. Paul takes and circumcises him because the Jews know about his father. Both preaching and teaching remain in the instructions to observe the commands written by the apostles and elders in Jerusalem. The ecclesias are established in faith and increase daily in number.
- Acts 16:6-12 keeps the Phrygian and Galatian regions, the Holy Spirit's prohibition on speaking God's word in Asia, Mysia, intended travel to Bithynia and the Spirit of Jesus's refusal. The night apparition appears to Paul like a Macedonian man, standing and pleading; its request ends “help me.” The first-person singular suffix follows the actual source and [Dukhrana's SEDRA analysis](https://dukhrana.com/peshitta/analyze_verse.php?font=Serto+Jerusalem&size=125%25&source=&verse=Acts+16%3A9), without harmonizing it to the supplied reference's plural. The subsequent first-person travel account retains “our Lord,” the call to preach, Troas, Samothrace, Neapolis, Philippi's chief-city and colony descriptions and the stay for days, without adding a district to qualify Macedonia.
- Acts 16:13-15 preserves Sabbath departure outside the city gate, the river, the house of prayer's seen/visible construction and conversation with the gathered women. It adds no “we thought” predicate. Lydia sells purple, fears God and comes from Thyatira; “our Lord” opens her heart, and she listens to Paul. Her baptism and household, invitation to lodge, and strong urging remain distinct. The condition is “you are truly confident that I have believed in our Lord”: the confidence clause is second-person plural, while the believing verb is first-person singular. The source's person codes are not interchanged.
- Acts 16:16-18 retains the young woman, her spirit of divination, meeting the speakers and producing much gain for her masters through divination. No explicit slavery term is added to the source noun. Her following Paul and the speakers, crying out, God Most High and the way of life remain. Her repeated action lasts many days. Paul is provoked and speaks to the spirit in Jesus Messiah's name; no unexpressed turning action is inserted. The spirit's departure retains its source time expression.
- Acts 16:19-26 keeps lost hope of gain, seizure of Paul and Silas, dragging to the marketplace and appearance before the prefects and city officials. The allegation about Jewish men, the city's disturbance, customs and Roman permission to receive or practise them remains continuous through 16:21. The great crowd, torn garments, ordered beating, severe scourging, prison, careful custody, inner confinement and stocks remain separate. No rods are added to the beating clause. Midnight prayer and praise, the listening prisoners, sudden great earthquake, shaken foundations, immediate opening of all doors and release of everyone's bonds are retained.
- Acts 16:27-34 preserves the jailer's waking, open doors, drawn sword, intended self-harm and mistaken flight assumption; Paul's loud prohibition says all are present. The jailer lights a lamp for himself, leaps and enters trembling, falls at Paul and Silas's feet and brings them outside. His question and their response retain the active life verb, the full “our Lord Jesus Messiah” name and his household. Speaking the Lord's word, the same hour of night, washing wounds, immediate baptism of him and all his household, entry into his house, the table and household joy in faith in God remain explicit.
- Acts 16:35-38 distinguishes the prefects from the rod-bearers they send. The release message, jailer's entry and report to Paul, and departure in peace remain. Paul speaks to the singular jailer, claims flogging without fault in public and Roman identity, and contrasts imprisonment with secret release. He demands that the officials themselves come and bring them out. No unexpressed trial, verdict or legal procedure is added. The rod-bearers report to the prefects, whose hearing of Roman identity leads to fear.
- Acts 16:39-40 retains the officials coming and pleading, asking both that the prisoners come out and that they depart from the city. It does not suppress the first action or add a formal apology. Departure from prison, entry to where Lydia was, seeing and encouraging the brothers and subsequent departure remain in the source's order.

Source checks for Acts 17:

- Acts 17:1-3 retains the cities Amphipolis and Apollonia, arrival at Thessalonica and the Jewish synagogue. Paul enters according to his custom and speaks from the Scriptures for three Sabbaths. His explaining and showing concern the Messiah's suffering and rising from among the dead; the min-beth expression does not require a literal house-building in English. The following claim is Paul's first-person “this Jesus is the Messiah, whom I proclaim to you,” not an unmarked continuation about what someone else proclaimed.
- Acts 17:4-11 preserves believing and joining Paul and Silas, many God-fearing Greeks and not a few notable women. Jealousy, evil men from the city's marketplaces, the crowd, disturbance, attack on Jason's house, bringing the men out and handing them to the crowd remain distinct. The accusation continues through 17:7, including Jason as host, Caesar's decrees and another king, Jesus. The additive “here also” does not invent an earlier visit. Security is taken from Jason and the brothers before release. Sending Paul and Silas to Berea is immediate and on the same night. The Jews there are contrasted with Thessalonica's Jews; they gladly hear the word from the speakers daily and discern from the Scriptures whether these things are so.
- Acts 17:12-17 retains many believers, Greek men and notable women, Thessalonica Jews hearing of God's word proclaimed by Paul in the city of Berea, their arrival and unceasing agitation. The brothers send Paul toward the sea while Silas and Timothy remain. His escorts reach Athens and receive an actual letter from him to Silas and Timothy for their urgent coming; the letter is not replaced by oral instructions. Paul's remaining in Athens has no added “for them” object. His spirit is grieved at the whole city's idols. Synagogue speech is with Jews and God-fearers; the marketplace audience is those present every day.
- Acts 17:18-22 names philosophers from Epicurus's teaching and others called Stoics, their dispute with Paul, and the question about what the collector of words wants. The foreign-gods remark ends before the narrator's reason: Paul proclaimed Jesus and his resurrection to them. The court is called Areopagus, and the connected question and statement retain the new teaching, strange words sown into “our hearing” and the wish to know their meaning. Athenians and visiting foreigners care for saying and hearing something new. Paul's address retains the explicit reverence/fear of demons rather than replacing it with a generic statement of religiosity.
- Acts 17:23-25 keeps Paul's walking, viewing the place of worship and finding a certain altar with the nested inscription “To the unknown God.” The stored בית is singular; “your place of worship” does not import plural houses from the supplied reference. The knowing/worshipping clauses and emphatic identification of the one proclaimed remain correctly related. God made the world and everything in it, is Lord of heaven and earth and does not dwell in hand-made temples. He is not served by human hands and needs nothing; his gifts here are life and breath to everyone, without an added “everything.”
- Acts 17:26-29 retains one blood, all humankind, dwelling on the whole earth, seasons appointed by God's command and bounds of human habitation. Seeking and investigating lead to finding God through his plural “creations”; no absent groping clause is supplied. He is not far from each of us. Living, moving and existing in him are followed by plural wise men among the hearers who “have said,” not one poet or sage. Their nested quotation says “Our descent is from him.” The next comparison has the Godhead as its grammatical subject and gold, silver and carved stone as its comparison objects; the clause is reordered within one linked grammatical unit to preserve that relation naturally.
- Acts 17:30-31 retains causal “For” and times of “error.” The causative אעבר is read with its temporal object as God making those times pass away, consistent with the make-pass sense recorded in [CAL's ˁbr entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=%28br+V), rather than importing a different ignorance/overlooking formula. The present command names all people, each person and every place. The appointed day, judgment of the whole earth in righteousness and the appointed man remain explicit. The source says God turned everyone to faith in him through raising him from among the dead; the actual turning and faith words are not replaced by an assurance formula.
- Acts 17:32-34 preserves hearing of resurrection from among the dead, some mocking and others saying “At another time we will hear you about this.” The time phrase stays with its own words, without an added “again.” Paul departs from among them. Some join him and believe; Dionysius is from among the judges of the Areopagus, and the woman Damaris and other adherents remain in the concluding account. Paul's speech closes at 17:31 before this narration.

Source checks for Acts 18:

- Acts 18:1-4 retains Paul's departure from Athens and arrival at Corinth, Aquila as a Jewish man from the region of Pontus, his arrival from Italy at that time with his wife Priscilla, and Claudius Caesar's command for all Jews to leave Rome. Paul's shared trade, lodging and work with them and their tentmaking remain distinct; the craft noun is not lost in a generic reference to trade. Sabbath synagogue speech and persuasion concern both Jews and Gentiles.
- Acts 18:5-8 keeps Silas and Timothy arriving from Macedonia, Paul's constraint in speech and the explicit cause: the Jews stood against him and blasphemed while he testified that Jesus is the Messiah. Shaking his garments is followed by “From now on I am clean; I am going to the Gentiles.” The subsequent householder is Titus, whose house adjoins the synagogue. Crispus is the synagogue leader and believes in “our Lord,” he and all his household. Many Corinthians hear, believe in God and are baptized.
- Acts 18:9-11 keeps the Lord's vision and speech to Paul, the prohibition on fear, the command to speak and not be silent, the Lord being with him, inability to harm him and the many people in this city. The quotation continues through 18:10. Paul's stay is explicitly one year and six months in Corinth, with teaching them God's word. No night-time detail is added to the vision.
- Acts 18:12-17 names Gallio as proconsul of Achaia, the Jews' assembly against Paul and his appearance at the tribunal. The accusation concerns persuading people to fear God contrary to the law. Paul's attempt to open his mouth and speak precedes Gallio's reply. His counterfactual names wrong, deceit and a hateful act, while the following questions concern speech, names and the hearers' own law. The internal knowledge clause and refusal to judge those matters remain. He drives them from his tribunal. The assailants are explicitly all the Gentiles, and Sosthenes is an elder of the synagogue; Gallio disregards the beating before the tribunal.
- Acts 18:18 preserves Paul's many days there, farewell to the brothers, sea journey toward Syria, and Priscilla followed by Aquila accompanying him. The shaving at Cenchrea and the vow use an unnamed masculine subject, rendered “he”; the source does not explicitly name the man in these clauses, so the English does not force a choice between possible antecedents. The passive participle with a dative in the vow clause is rendered naturally as his having made a vow, without inventing a recipient to whom a vow was owed.
- Acts 18:19-23 keeps arrival at Ephesus, Paul's synagogue speech, the request to stay and his refusal to be persuaded. His stated obligation concerns the approaching feast in Jerusalem, followed by a God-willing return. Leaving Aquila and Priscilla in Ephesus belongs to the actual final clause of 18:21, although the supplied reference places it at the start of 18:22. The following sea journey reaches Caesarea, goes up and greets the members of the ecclesia, then goes to Antioch; no unexpressed city is named for that ascent. The next route names Galatia before Phrygia, preserving the stored order, with strengthening of all disciples.
- Acts 18:24-26 describes Apollos as a Jew with origins in Alexandria, trained in speech and well versed in the Scriptures. The way of the Lord is singular. His fervent spirit, speaking and teaching fully about Jesus, and knowledge limited to John's baptism remain explicit. The except construction is not turned into a new hypothetical condition. The eye/revelation expression is rendered idiomatically as “openly.” Aquila and then Priscilla hear him, bring him to their house and fully explain the Lord's way to him.
- Acts 18:27-28 keeps the intended journey to Achaia, the brothers encouraging him and writing to the disciples to receive him. Grace modifies the helping: through grace he greatly assists all the believers. The final causal “For” introduces his powerful argument against the Jews before plural assemblies and his demonstration from the Scriptures concerning Jesus that he is the Messiah.

Source checks for Acts 19:

- Acts 19:1-7 keeps Apollos at Corinth while Paul travels through the upper regions to Ephesus. The inquiry at the end of 19:1 introduces the question in 19:2. The disciples have not even heard whether there is a Holy Spirit; John's baptism and his instruction to believe in the one coming after him remain. The baptismal name is “our Lord Jesus Messiah.” Paul lays his singular hand on them, the Holy Spirit comes upon them, and their speaking in various tongues and prophesying remain separate. The total is exactly twelve persons, without an added approximation.
- Acts 19:8-12 preserves three months of open synagogue speech, persuasion about God's kingdom, hardening and opposition, reviling God's way before the assembly, Paul's withdrawal and separation of the disciples, and daily speech in Tyrannus's school. No hours are added. The subsequent duration is two years, and all who dwell in Asia, Jews and Gentiles, hear the Lord's word. God's great works are through Paul's singular hand. Cloths or patches from his clothing are brought and laid on the sick; diseases leave them and demons go out. The alternative “or” is retained.
- Acts 19:13-17 distinguishes the Jews' existing itinerant adjuration of demons from their desire to pronounce an adjuration over people with unclean spirits. Both adjuration predicates and both occurrences of “in the name” remain; the narrative has “our Lord Jesus,” and the quotation names Jesus whom Paul proclaims. Sceva is a Jewish chief of the priests with seven sons. The demon's recognition of Jesus and knowledge of Paul precede its question to them. The man with the evil spirit overpowers and throws them down; naked and wounded describes those fleeing from that house. The episode becomes known to all Jews and Gentiles in Ephesus, fear falls on all, and the full name “our Lord Jesus Messiah” is exalted.
- Acts 19:18-23 retains believers recounting their sins and confessing their deeds, sorcerers gathering and bringing their books for public burning, and the computed value of five myriads, rendered fifty thousand in silver without naming a coin. The source says the “faith of God” grows strong and increases, rather than replacing it with a word/Lord formula. Paul resolves in his mind to travel through all Macedonia and Achaia to Jerusalem and then see Rome. Two assistants, Timothy and Erastus, are sent to Macedonia while he remains in Asia. The disturbance concerns the way of God.
- Acts 19:24-27 names Demetrius, his silver shrines for Artemis and the craftsmen's great profit. He gathers all the craftsmen of his trade and those working with them; “all” is not transferred from craftsmen to the second group. His speech says all their business comes from this work. The hear/see, persuasion, turning away and saying predicates remain distinct. Gods made by human hands are denied. The business's disgrace and cessation, the temple being reckoned as nothing and the goddess of all Asia being despised remain explicit. The public-disgrace sense of מתפרסיא follows [CAL's prsy, Qt sense 2](https://cal.huc.edu/oneentry.php?cits=all&lemma=prsy+V), which cites this verse directly. The name remains Artemis throughout the chapter.
- Acts 19:28-34 retains the city's uproar, the crowd's running together and entering the theatre, and Gaius and Aristarchus as Macedonian members of Paul's company. בני לויתה דפולוס is read as a company-membership construction, without adding a first-person owner from an ambiguous morphological tag. The Classical Syriac construct plural bnay is illustrated in chapter 3, page 52 of [Attributive constructions in North-Eastern Neo-Aramaic](https://langsci-press.org/catalog/view/123/302/1034-1); the construction is applied here in context. The disciples restrain Paul; the chiefs of Asia, his friends, send and plead with him. The confused crowds shout different things because many do not know why they assembled. The Jewish people bring Alexander forward, and he wants to make a defense. The repeated cry lasts about two hours, preserving the approximation and the single voice.
- Acts 19:35-37 calls the official the city's chief. Ephesus is the “priestess” of great Artemis and her image, not a temple substituted for כומרתא. Descending from heaven modifies the image; no additional divine name is introduced. The instruction to remain quiet and do nothing rash is followed by the explicit grounds that the brought men have neither plundered temples nor reviled “our goddess.” The chief's speech continues through 19:40.
- Acts 19:38-41 preserves the singular proconsul in the city and the separate plural statement that they are craftsmen, followed by approaching and litigating against one another. Other matters are settled in the place appointed by law for assembly, keeping the passive appointment. The closing danger and inability to account for this day's assembly use first-person plural forms, with both purposeless gathering and disturbance without cause. נפוק רוחא is idiomatic in the defense at 19:33 and giving an account at 19:40, consistent with [CAL's npq, G sense 5 with rūḥā](https://cal.huc.edu/oneentry.php?cits=all&lemma=npq+V); no additional spirit actor is created. After the speech, the chief dismisses the assembly.

Source checks for Acts 20:

- Acts 20:1-6 retains the subsiding riot, Paul's calling, encouragement and kissing of the disciples, departure to Macedonia, regional travel and encouragement, Greece, three months there, the Jews' plot, intended travel to Syria and contemplated return to Macedonia. The companions include Sopater from Berea, Aristarchus and Secundus from Thessalonica, Gaius from Derbe, Timothy explicitly from Lystra, and Tychicus and Trophimus from Asia. They go ahead and await the speakers at Troas. The departure from Philippi follows the days of unleavened bread; the sea journey takes five days and the stay at Troas seven. The “Philip” lexical tag at 20:6 is not treated as a different person or an instruction to rename the city: its city context and the same vocalised name in [Dukhrana's analysis of Acts 16:12](https://www.dukhrana.com/peshitta/analyze_verse.php?font=Estrangelo+Edessa&size=125%25&source=&verse=Acts+16%3A12) support the conventional English Philippi.
- Acts 20:7-12 keeps the first day of the week and the explicit אוכרסטיא, “Eucharist.” The gathering is first-person plural, while Paul's speaking is “with them,” preserving the third-person plural suffix. His intended next-day departure and speech until midnight remain. Many burning lamps are in the upper room. Eutychus is sitting in the window and listening before sinking into deep sleep during Paul's prolonged speech. His fall from the third floor and being picked up dead are retained without an added apparent-death qualification. Paul goes down, falls upon and embraces him, and says his soul is in him. Going up, breaking bread, tasting it, speaking until daybreak and departure by land belong to the subsequent narrative. The young man is taken away alive and they rejoice greatly over him.
- Acts 20:13-17 preserves first-person travel to the ship, the appointed meeting place at Thasos, the plan to pick up Paul from there and his instructions while he travels overland. The source's Thasos name is retained without replacing it with a different route label. Receiving him from there and taking him aboard remain separate actions. The route includes Chios, Samos, the explicit stay at Trogyllium and Miletus, with the successive day expressions. Paul's decision to pass Ephesus retains causal “For,” the concern about delay, his haste and the conditional possibility of celebrating Pentecost in Jerusalem. The decision's passive participle plus dative is expressed naturally as his having decided. From Miletus he sends for and brings the Ephesian ecclesia's elders.
- Acts 20:18-21 opens the speech to the elders and preserves their knowledge of how Paul was with them throughout the time since his first day in Asia. That temporal/how clause is reordered within one linked grammatical unit. Service to God, great humility, tears and trials through Jewish plots remain distinct. The source speaks of what benefited “your souls” and teaching in streets and houses. Testimony concerns Jews and Gentiles, repentance toward God and faith in the full “our Lord Jesus Messiah.” The Gentile sense of ארמיא is retained, rather than restricting the audience to an inferred ethnic nationality.
- Acts 20:22-25 keeps being bound “in spirit” distinct from the explicitly Holy Spirit in the next verse, without resolving the first phrase's referent by an added name. Paul goes to Jerusalem without knowing what will happen to him there. The Spirit's city-by-city testimony includes a nested quotation addressing Paul as “you,” not an unmarked switch to “me.” His life counts for nothing to him in relation to finishing his course and the ministry received from “our Lord Jesus,” with testimony to the gospel of God's grace. The repeated first-person pronoun in 20:25 belongs to “I know”; the following seeing clause is second-person plural, “you will not see my face again.” The final source noun is only “the kingdom,” without the supplied reference's added “of God.”
- Acts 20:26-30 retains the causal “For this reason,” testimony today and purity from the blood of all the hearers. The source's אשתאלת at 20:27 is read in its refuse/refrain sense, supported by [CAL's šʾl, Gt sense 1](https://cal.huc.edu/oneentry.php?cits=all&lemma=%24%29l+V), rather than the exported base gloss “ask.” Paul has made known God's whole will. The warning concerns themselves and the entire flock, the Holy Spirit's appointment of overseers and tending God's ecclesia, acquired with his blood. The blood clause is not emended to a different possessor or divine title. Fierce wolves enter after Paul's departure and do not spare the flock; men from among the hearers speak perverse things to turn the disciples into following them. The disciples noun stays with its own turning predicate in the links.
- Acts 20:31-35 preserves the call to stay awake and remember three years of unceasing admonition, night and day, with tears for each hearer. Paul entrusts them to God and the word of his grace; the feminine relative and building/giving predicates are retained with the word clause. The inheritance is among all the holy ones. Silver, gold and clothing are not coveted. “These hands of mine” retains the explicit first-person hand suffix and service for Paul's needs and those with him. The instruction to labor and care for the weak includes remembering the words of “our Lord Jesus.” In the nested quotation the comparative word belongs with “More blessed is the one who gives,” followed by “than the one who receives.”
- Acts 20:36-38 resumes narration after the speech and nested quotation close in 20:35. Kneeling on his knees and prayer with all those present are followed by everyone's great weeping, embracing and continued kissing. Their particular distress concerns his statement that they would not see his face again. They accompany him as far as the ship. No neck-embrace or other unexpressed detail is imported into these actions.

Source checks for Acts 21:

- Acts 21:1-7 preserves the straight course to Cos, the following day's Rhodes arrival, Patara, the ship for Phoenicia, approach to Cyprus and leaving it on the left, Syria and Tyre with the ship's cargo unloading. The stay with the disciples is seven days; their Spirit-mediated warning to Paul is explicitly every day, retaining כליומ omitted from the supplied reference. The escort includes the disciples, their wives and children. They, not the first-person travellers, kneel and pray by the shore. The subsequent farewell is an actual mutual kiss, followed by boarding and the others' return home. The next city is the source's Acco, with greeting and a one-day stay; no different place-name is substituted.
- Acts 21:8-11 keeps Caesarea, lodging with Philip the evangelist, his membership among the seven and four virgin daughters who prophesy. Agabus comes down from Judea after many days. The strap taken from Paul is explicitly associated with his waist, and Agabus binds his own feet and hands. The prediction retains the owner of this strap and third-person plural נֵאסרֻונֵה, “they will bind him,” rendered naturally as the man being bound by the Jews in Jerusalem. This is not a second-person address to the Jews; the object suffix is retained through the passive patient's role. Delivery into the hands of the Gentiles follows. The same source form appears in [Dukhrana's Peshitta text of Acts 21:11](https://dukhrana.com/peshitta/analyze_verse.php?font=XenoType+SY+Darmasuq&lang=en&size=125%25&source=ubs&verse=Acts+21%3A11).
- Acts 21:12-16 preserves the travellers and local people pleading with Paul, his question about weeping and breaking his heart, and readiness not only for bonds but also death in Jerusalem for the name of “our Lord Jesus.” No Messiah title is added where the stored source lacks it. The travellers fall silent when he is not persuaded and invoke our Lord's will. After preparation and ascent to Jerusalem, Caesarean disciples accompany them with Mnason, a brother from among the earlier disciples and from Cyprus. His receiving them at his house is a purpose or anticipated hospitality, not an already completed reception asserted during the journey.
- Acts 21:17-21 keeps the brothers' joyful reception, the next day's visit with Paul to James and all the elders, first-person “we” greeting them and Paul recounting God's work among the Gentiles through his ministry. The praise is to God. “Our brother” retains its own first-person plural suffix, and the believing myriads are in Judea, without an unexpressed exact number. The elders' speech continues through 21:25. The reported accusation concerns all Jews among the Gentiles departing from Moses, non-circumcision of their children and not following the law's customs. The Moses/name and teaching/departure predicates remain in their own linked grammatical clause.
- Acts 21:22-27 retains the causal report of Paul's arrival, the instruction continuing into 21:23, four men and their purification vow, taking them, purifying with them, paying their expenses and shaving their heads. The expected public knowledge concerns the false report and Paul's fulfilling and observing the law. For believing Gentiles, “we wrote” and the four prohibitions remain: sacrificed meat, fornication, what is strangled and blood. No additional idol noun is inserted. Paul's taking these men on the next day, purification, temple entry, notification of the purification days' completion and offering for each person remain distinct. The actual seventh day arrives in 21:27; it is not replaced by an imported near-completion formula. The Asian Jews' sighting, incitement and seizure lead into the following cry.
- Acts 21:28-33 preserves the help request to Israel's sons, teaching everywhere against “our people,” the law and this place, and the accusation of bringing Gentiles and defiling the holy place. The law and people nouns are not moved into each other's spans. The narrated basis is having seen Trophimus the Ephesian with Paul in the city and supposing that Trophimus entered the temple with Paul; this is not rewritten as a causative claim that Paul brought him. The whole city's agitation, assembled people, seizure, dragging outside the temple and immediate gate closure remain. The cohort commander's intervention names one centurion and many soldiers, followed by plural “they ran.” The crowd stops beating Paul. The binding command specifies two chains, with questions about his identity and deeds.
- Acts 21:34-38 keeps contradictory shouting, inability to learn what was true because of that shouting, and the order to take Paul to the barracks. The soldiers carry him on reaching the stairs because of the crowd's violence; the following crowd cries “Away with him!” Paul's approach to the barracks includes the complete conditional “If you permit me, I will speak with you,” preserving אנ and the first-person future speaking verb; the if sense is recorded in [CAL's ˀn, sense 1](https://cal.huc.edu/oneentry.php?cits=all&lemma=%29n+c). The commander's Greek-language question continues into the negative question about the Egyptian, rather than becoming a declarative denial. Four thousand men “doing evil” remain explicit, without replacing the broad source wording with an unexpressed group name.
- Acts 21:39-40 retains Paul's Jewish identity, Tarsus in Cilicia, a notable city, his birth there and request to speak to the people. Subject and enclitic first-person forms are naturally grouped as “I am a Jewish man,” without forcing duplicate English declarations. Permission, standing on the stairs, the singular hand gesture and the people's quiet precede his address. The source label עבראית is rendered “in Hebrew”; the supplied lexical alternatives also permit Aramaic, so the English label is not offered as an independent historical resolution of which language variety was spoken. The chapter ends with the introduction to his words; the actual quotation begins with the following chapter's speech.

Source checks for Acts 22:

- Acts 22:1-3 opens Paul's defense after the introduction in 21:40, closes for the narrative interruption in 22:2 and resumes the speech in 22:3. מפק ברוח is a defense idiom, consistent with [CAL's npq, G sense 5 with rūḥā](https://cal.huc.edu/oneentry.php?cits=all&lemma=npq+V); no separate spirit actor is introduced. The listeners hear him speaking with them in Hebrew and become quieter. The Hebrew label follows 21:40 without claiming an independent historical resolution of the language variety. Paul's Jewish identity, birth in Tarsus of Cilicia, upbringing in this city at Gamaliel's feet, full instruction in the law of our fathers, past zeal for God and comparison with all the hearers remain. No unexpressed “today” is added.
- Acts 22:4-5 preserves persecution of this way to death, binding and handing men and women over to prison. The high priest and all the elders testify about Paul; the witness predicate remains with its subjects rather than being moved into the letters clause. The elders are not given an unexpressed first-person owner. Letters, travel to the brothers in Damascus, bringing those there to Jerusalem in bonds and their receiving punishment remain distinct. מסם ברשא is the lexical compound “punishment,” supported by [CAL's mɘsām brēšā entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=msm%40bry%24%29+N); the English does not make execution explicit merely from its literal head/placing components or the supplied reference's capital-punishment wording. The testimony clause continues the preceding persecution sentence.
- Acts 22:6-10 keeps travel, beginning to approach Damascus and midday. מן תחית שליא is rendered idiomatically “suddenly,” consistent with the compound usage in [CAL's šly entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=%24ly%232+N), whose citations include the same construction at Acts 9:3. “From heaven” belongs to the light's origin and is not made the possessor of quietness. The light shines upon Paul, who falls and hears the repeated “Saul, Saul.” Jesus identifies himself as the Nazarene. The men see the light but do not hear the voice of the one speaking with Paul; this is not changed to an understanding formula to harmonize another account. “My Lord” in Paul's question and “our Lord” in the narrator's next clause remain distinct, as do the rise/go commands and subsequent instructions at Damascus.
- Acts 22:11-13 retains inability to see because of the light's glory, the companions taking Paul by the plural hands and Paul's own entry into Damascus. The copular word at 22:11:13 belongs to the companions, not the next entry predicate. Ananias is upright according to the law, as all the local Jews testify. The noun phrase in 22:12 carries into the actual coming predicate at the start of 22:13; no coming verb is imported into the previous verse. His command to Saul is “open your eyes.” That nested quotation ends before Paul narrates the opening of his own eyes at that time and looking at Ananias.
- Acts 22:14-18 resumes Ananias's words with the God of our fathers, appointment to know his will, seeing the Righteous One and hearing a voice from his mouth. Witness to all people concerns everything seen and heard. The commands to rise, be baptized and be cleansed from sins, while invoking his name, remain through 22:16. The next narration keeps returning, coming here to Jerusalem and praying in the temple. The subsequent sight is explicitly a vision; no separate trance is added. The command to hurry and leave Jerusalem retains the stated cause: they will not receive his testimony about the speaker.
- Acts 22:19-21 preserves “my Lord,” the hearers' knowledge, delivery to prison and beating believers in all the synagogues. Stephen is “your witness”; his blood being shed, Paul's standing with the killers, consent to their will and guarding the garments of those stoning him remain separate. The final command sends Paul far away to preach to the Gentiles. The nested words and Paul's outer speech both close before the crowd narrative begins.
- Acts 22:22-27 explicitly names Paul in the hearing clause, the crowd's raised voice and cry for such a man to be removed from the earth because he ought not to live. Shouting, throwing garments and tossing dust into the sky continue into the commander's response. Questioning is by scourging, intended to discover why they shout against Paul. The binding apparatus is plural leather straps, not an unexamined rope substitution. Paul's question includes the actual flogging predicate with the Roman and uncondemned man. The centurion asks “What are you doing?” and identifies the man as Roman. The commander's “Tell me, are you a Roman?” is one question; a repeated subject/copular pronoun is not made into another utterance. Paul's answer is yes.
- Acts 22:28-30 keeps the large monetary acquisition of Roman citizenship and Paul's contrasting birth into it, with the even/also particle retained. Those intending to scourge him withdraw; the departure verb is not forced into a flight claim. The commander fears because he has bound a Roman, not because the source suddenly substitutes a stretching verb. The next day brings inquiry into the actual accusation, unbinding, an order for the chief priests and the whole assembly of their leaders to come, then taking Paul, bringing him down and standing him among them. Each of these final motion and placement predicates stays with its own source positions.

Source checks for Acts 23:

- Acts 23:1-5 retains Paul's look at their assembly, “Men, my brothers,” conduct in all good conscience before God and the explicit “until today.” Ananias is called the priest; no additional high-priest title is inserted into the stored wording. His command names Paul and the mouth. Paul's reply retains God's impending blow, the whitewashed-wall address, sitting in judgment according to law while transgressing it, and ordering others to strike him. The bystanders ask about insulting God's priest. Paul's lack of knowledge, “my brothers,” priest identification and the written prohibition on cursing the ruler of your people remain. The writing predicate belongs to positions 9-11, not the priest clause, and the scriptural quotation is nested within Paul's reply.
- Acts 23:6-10 preserves the Sadducee and Pharisee groups, Paul's Pharisee identity and “a son of Pharisees,” with the actual plural at 23:6:18. Hope concerns resurrection of the dead. Their falling upon one another and the divided people precede the explanation: no resurrection, plural angels or a singular spirit, while the Pharisees confess all these things rather than an imported “both.” Scribes from the Pharisees' party contend with the others, find nothing evil in this man and retain the final “what of that?” question after the spirit-or-angel possibility. No extra fighting-against-God clause is supplied. The commander fears Paul's being torn apart and sends for the explicitly named Romans to come, snatch him from their midst and bring him into the barracks; no unexpressed descent is added here.
- Acts 23:11-15 keeps night, the appearance of “our Lord” to Paul, “Be strong,” the causal comparison with Jerusalem testimony and the prospective testimony in Rome. Readiness alone does not replace that future assertion. The gathered men put themselves under a curse not to eat or drink until killing Paul. The pact with oaths involves more than forty men. They approach priests and elders, not an added set of chief priests, and their words continue through 23:15. The imperative “ask” stays with the requesters, the hearers and council chiefs; the request concerns bringing Paul to them under the pretext of examining his affair. Their readiness to kill precedes his arrival.
- Acts 23:15:19 נמטא is rendered “he reaches,” referring to Paul. Although the exported tag selects first-person plural, the form can also be third-person masculine singular. [Younansardaroud's Classical Syriac Course Book, section 8.1, printed pages 55-56](https://d-nb.info/1184881278/34#page=64) gives the shared third-singular and first-plural imperfect forms and discusses contextual interpretation. The preceding “him” and destination “you” support Paul's arrival here; no letters are changed to force the reading. The next account retains the son of Paul's sister, hearing the plot, entering the barracks and reporting to Paul. Paul sends for and calls one of the centurions, who brings the young man to the commander. The prisoner title, requested transport and message recipients remain distinct.
- Acts 23:19-22 explicitly retains the young man, the commander's taking him by the singular hand, leading him aside and asking what he has to tell. The reply identifies the Jewish planners, tomorrow, bringing Paul down to their assembly and the pretense of learning something more. The warning not to be persuaded retains more than forty men from among them, the ambush, their self-imprecatory curse, both food and drink prohibitions, the killing condition, readiness, waiting and “your promise.” The commander dismisses the young man with a command that no one know of his disclosure.
- Acts 23:23-26 keeps two centurions, two hundred Romans, seventy horsemen and two hundred described as throwing with the right hand. The wording does not specify a weapon for that last group. Departure is three hours into the night, retaining the cardinal hours expression without converting it to a modern clock time. The following command continues the same quotation and calls for a singular beast of burden to carry Paul safely to Felix the governor. The letter is written and given to them; the narrator's introduction precedes a continuous quoted letter from 23:26 through 23:30. Its address names Claudius Lysias, Felix and the excellent governor's title and greeting.
- Acts 23:27-30 remains the commander's first-person account, including his assertion of rescue when he learned of Roman identity; it is not rewritten to reconcile his account with the preceding narrator's chronology. The Jews' seizure has the stated purpose of killing. The reason for accusation, descent to their assembly, questions of their own law and the absence of a charge worthy of bonds or death remain. Negation stays within its own complete linked clause, not in a preceding positive noun span. The letter names a treacherous ambush prepared against Paul, immediate sending to Felix, orders to the accusers to come and speak with him before the governor, and the final farewell.
- Acts 23:31-35 resumes narration with the commanded night journey to the city of Antipatris. The source has the horsemen dismissing their companions, the foot soldiers, so that they may return to the barracks; that direction of dismissal is not reversed. Arrival at Caesarea, delivery of the letter and presenting Paul precede reading, the province question and learning of Cilicia. The final speech promises a hearing when the accusers arrive. The separate guard command belongs to positions 8-9; the following location is Herod's praetorium. No unexpressed Herodian personal identification is inferred from a lexical name tag.

Source checks for Acts 24:

- Acts 24:1-4 explicitly calls Ananias the chief priest and retains five days, the elders, Tertullus the orator and their report concerning Paul. Tertullus's speech starts in the actual words of 24:2, with abundant peace because of Felix and many good things through his care. All of us, every place and your favor remain in 24:3. In 24:4, נלאיכ is first-person plural with a second-person object: “we do not weary you,” followed by the singular “I ask” and first-plural “our humble plea.” It is not changed to Felix's gentleness or a singular subject in the wearying clause.
- Acts 24:5-8 keeps the accusation of being a corrupter and stirring an uproar among all the Jews throughout the whole land. The final noun is teaching, specifically the teaching of the Nazarenes, without importing a sect noun. Our temple, our law and our hands remain the accusers' first-plural claims. Lysias comes as commander, takes Paul with great force and sends him to the governor. The accusers' ordered arrival is followed by the governor's ability to learn when questioning him. Both occurrences of “him” remain pronouns; no disputed antecedent is resolved by inserting a name. Tertullus's quotation closes after 24:8.
- Acts 24:9-11 preserves the Jews' outcry against Paul rather than reducing it to agreement with Tertullus. The governor beckons to Paul, who answers and begins a continuous quotation. The many-years, knowledge and judge clauses are reordered together within one linked grammatical clause. נפק ... רוחא is rendered as making a defense, using the defense idiom recorded in [CAL's npq entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=npq+V), rather than adding a separate spirit participant. על אפי נפשי is naturally “on my own behalf.” The next verse retains the ability to learn, no more than twelve days, the ascent to Jerusalem and worship.
- Acts 24:12-15 keeps speaking with anyone in the temple, gathering a crowd, the repeated negatives, their singular synagogue and the city. The inability to prove charges before the governor is rendered idiomatically without physical hands. Paul's acknowledgment names the teaching about which they speak and the God of my fathers; no “to you,” sect noun or calling predicate is supplied. All that is written in the law and prophets, hope in God, the other men's expectation and resurrection of the righteous and unrighteous remain. מנ בית מיתא is “from among the dead,” following the Syriac preposition documented in [CAL's bēṯ entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=byt+p), rather than requiring a physical house from the homophonous noun's base gloss. The dead noun stays with its own prepositional phrase.
- Acts 24:16-19 retains the effort to have a pure conscience continually before God and people, the many-year interval, coming to his own people, alms and presenting an offering. The temple discovery occurs when Paul is purified; מדַֿכַּי is rendered as a resultant state without forcing an unexpressed cleansing agent. No crowd and not even an uproar remain distinct. The Jewish men from Asia belong to 24:19 in the actual source, with the conditional “if,” their disturbance, their obligation to stand with Paul before the governor and their own accusation. These clauses are not moved across the verse boundary to imitate the supplied reference.
- Acts 24:20-23 keeps the alternative invitation to these men to identify wrongdoing before their assembly and the exception for the one cry about resurrection. That earlier cry is nested within Paul's speech, which closes at 24:21. Felix's knowledge of this way is explicitly causal, followed by his deferral and promise to hear when the commander comes. The title does not assert a separate headcount. The centurion's guard of Paul at rest and the prohibition on hindering acquaintances from serving him remain; the rest expression is not made a release from custody.
- Acts 24:24-26 preserves both sending and calling for Paul, Felix and Drusilla, her description as his Jewish wife, their plural hearing and faith in the Messiah. The discourse concerns righteousness, holiness and the coming judgment; holiness is not replaced with a Greek-derived self-control formula. Felix is filled with fear and promises to send when he has opportunity. The next verse explicitly names a bribe from Paul, its causal “For,” the consequent repeated sending, bringing and conversation. No release clause or bargain absent from the actual source is added.
- Acts 24:27 retains completion of two years, another governor replacing him and the full name Porcius Festus. The two-year phrase does not assert Felix's entire tenure. Felix's purpose of doing the Jews a favor and leaving Paul bound remain separate from the replacement governor's arrival. Every verse uses contiguous source spans with each position represented once, and both the continuous speeches and nested resurrection cry have balanced quotation marks.

Source checks for Acts 25:

- Acts 25:1-3 retains Festus arriving at Caesarea, three days and the ascent to Jerusalem. The chief priests and Jewish leaders inform him about Paul and petition him; the request continues into the next verse. Both sending and having Paul brought to Jerusalem remain, followed by the plural ambushes on the road and the purpose of killing him. The report predicate stays with its own subject clause rather than being assigned to the later Paul phrase.
- Acts 25:4-7 keeps Festus's answer as one quotation through 25:5: Paul is kept at Caesarea, Festus is hurrying to travel, and those able among the hearers should go down with us and accuse the man. The ability expression is idiomatic rather than a statement about physical hands. The return follows eight or ten days, without importing an absent “not more than.” The tribunal session is on the next day. Jews who came down from Jerusalem surround Paul and bring many serious charges they cannot prove. רשא is rendered as legal heads of accusation in context, rather than an extra group of leaders being brought; [CAL's rēš entry, sense 7](https://cal.huc.edu/oneentry.php?cits=all&lemma=ry%24+N) records the broader main-point sense supporting this contextual use.
- Acts 25:8-10 preserves Paul's defense as an indirect statement that he committed no offense against the Jews' law, the temple or Caesar. נפק ... רוחא is the established defense idiom, not an extra spirit participant. Festus's desire to grant the Jews a favor explicitly causes his question about going to Jerusalem and being judged before him. Paul's answer retains Caesar's tribunal, standing before it, here as the proper place for judgment, no wrong to the Jews and the governor's own knowledge; no unexpressed “well” is added. Paul's continuous quotation runs through 25:11.
- Acts 25:11-12 keeps the conditional wrongdoing and death-worthy act, willingness not to refuse death, the conditional absence of the accusers' charges and the prohibition on handing Paul over to them as a gift. The passive participle with first-person dative in the first condition means wrongdoing committed by Paul, not wrongdoing done to him. “I do not refuse to die” follows the refuse/abstain-with-min usage in [CAL's šʾl entry, Gt sense 1](https://cal.huc.edu/oneentry.php?cits=all&lemma=%24%29l+V). The appeal to Caesar ends Paul's quotation. Festus speaks with his counselors and answers with two distinct clauses: the appeal to Caesar and the journey to Caesar. The second occurrence of Caesar stays with the going clause, not with the preceding appeal.
- Acts 25:13-16 preserves King Agrippa and Bernice coming down to Caesarea to greet Festus, their days with him and Festus relating Paul's case to the king. Felix leaves a certain man as a prisoner; the passive agent is naturally “by Felix,” without an obligatory physical-hands description. The Jerusalem report names chief priests and elders and explicitly asks for judgment against Paul for them. Festus's reply quotes the Roman custom: handing a person over as a gift to be killed is forbidden until the singular accuser comes, charges him to his face and he receives opportunity to make a defense. The killing clause is present in the actual source and is retained. Opportunity does not require a physical place, and the defense compound does not add a spiritual actor. The embedded quotation closes before 25:17 while Festus's outer report continues.
- Acts 25:17-21 keeps arrival here, no delay, the next-day tribunal session and the order to bring the man to Festus. The accusers stand with him but cannot substantiate the expected charge of wrongdoing. Their disputes are with Paul about their religion and about Jesus, a man who died whom Paul said was alive. The religion sense of דחלתהונ follows [CAL's deḥləṯā entry, sense 2](https://cal.huc.edu/cal_entry_web.php?lemma=dxlh+N), rather than requiring ordinary fear from the base gloss. Festus's lack of clarity concerns the inquiry into these matters, followed by his quoted question about Jerusalem and judgment there. קאם with על has the ascertain/understand sense recorded in [CAL's qwm entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=qwm+V), without requiring a physical-standing or literal-establishment phrase. Paul's request for Caesar's judgment, the custody command and sending to Caesar close Festus's report at 25:21.
- Acts 25:22-25 distinguishes Agrippa's wish to hear this man from Festus's tomorrow reply. The next-day arrival names Agrippa and Bernice, great pomp, the courthouse, plural commanders and city leaders. Festus gives the order and Paul comes. The new speech addresses King Agrippa and all the men with us. כלה in 25:24 belongs to all the Jewish people petitioning Festus, not to the preceding “you see.” Their crying retains its third-person plural subject. Festus's finding is that Paul has done nothing deserving death; the passive-participle/dative construction is not reversed into a claim that nothing was done to Paul. The requested Caesar hearing leads to the sending order.
- Acts 25:26-27 keeps both occurrences of what to write, Caesar as recipient, all the hearers versus the singular King Agrippa and the purpose of examining the case. אשתאל is a passive interrogation form, with the case as its subject; it is not turned into a new first-person act of asking. אשכח stays with its own linked “I may find,” before the final what-to-write span. Its contextual first-person imperfect reading follows the first-person speaker and purpose clause, despite the exported homograph tag; [Younansardaroud's Classical Syriac Course Book, section 8.1, printed page 55](https://d-nb.info/1184881278/34#page=64) gives the first-singular prefix, and section 14.2, printed page 128, describes the irregular e-a forms of this verb. No letters are changed to force a new subject. The final causal “For,” explicit impropriety and first-plural sending and writing remain, with the prisoner's offense. Festus's speech closes at 25:27; the chapter's speeches and embedded quotations are balanced.

Source checks for Acts 26:

- Acts 26:1-3 retains Agrippa's permission for Paul to speak on his own behalf, Paul's singular hand and the introduction of his defense. נפק ... רוחא is the legal-defense idiom, also used in 26:2 and 26:24; [CAL's npq entry, G sense 5](https://cal.huc.edu/oneentry.php?cits=all&lemma=npq+V) directly cites 26:24. No separate spirit participant is introduced. Paul considers himself fortunate to defend himself before King Agrippa today and asks for patient hearing. מפס with ב is rendered as being well acquainted with the Jewish questions and laws, following the attested participial construction in [CAL's pys entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=pys+V), rather than requiring a generic convinced predicate from the base gloss.
- Acts 26:4-7 keeps causal “For,” the Jews themselves, their conditional willingness to testify and Paul's way of life from youth among my people and in Jerusalem. The same acquainted-with construction in 26:5 concerns their long familiarity with Paul. רשיא describes the foremost teaching of the Pharisees; it is not replaced by an unexpressed strictest formula. The promise is to our fathers from God. Standing and being judged remain separate predicates. The twelve tribes, earnest plural prayers, day and night, repeated hope and accusation by the Jews all remain. No first-plural possessive is added to the actual tribe noun merely because the supplied reference says “our.”
- Acts 26:8 preserves the source's two questions: how the hearers judge, and whether it is not right that we believe God raises the dead. The first-plural belief predicate is not replaced by a familiar incredibility question. Acts 26:9 retains causal “For,” Paul's former resolve, his own mind, many hostile acts and the name of Jesus the Nazarene.
- Acts 26:10-11 keeps “also,” Jerusalem, many saints, prison and the authority received from the chief priests. The plural suffix in מנהונ is retained as “by them” with the passive killing predicate, an agent use of מן recorded in [CAL's mn entry, sense 3](https://cal.huc.edu/oneentry.php?cits=all&lemma=mn+p). A partitive reading is possible; the chosen English follows the supplied reference without deleting the phrase or naming an unexpressed executor. Paul's participation with those condemning them remains distinct. Every synagogue, torture, pressure to become blasphemers of Jesus' name, great anger against them and travel to other cities to persecute them all remain. Paul's quotation continues beyond this range.
- Acts 26:12-15 retains the purpose of the Damascus journey, authority and permission from the chief priests, midday and the road. The delayed light object, heaven as its origin, the king address, greater brightness than the sun and shining on Paul and all with him are reordered within one linked grammatical clause. This avoids placing Paul in heaven as the observer. All of us fall to the ground; the voice speaks to Paul in Hebrew, calls Saul twice, asks why he persecutes the speaker and compares resistance with kicking plural goads. Paul's question says my Lord; the narrator says our Lord. The answer explicitly names Jesus the Nazarene. The voice, question and reply have separate nested quotations.
- Acts 26:16-18 keeps the renewed “he said to me” outside Jesus' embedded speech, followed by standing on your feet, the reason for appearing and appointment as servant and witness. The subordinate appointment verb is prospective purpose, naturally “to appoint you,” rather than an invented past indicative. Both seeing clauses have you as subject and me as object: Paul has seen Jesus and is yet to see him. Neither is changed to “I will show you.” Deliverance concerns the Jewish people and other nations, to whom Paul is sent. Eye-opening, darkness to light, Satan's dominion to God, forgiveness of sins, a share with the saints and faith in me retain their actual actors. No sanctification predicate absent from the stored clause is added. This embedded quotation closes at 26:18.
- Acts 26:19-23 preserves King Agrippa, Paul's refusal to oppose the heavenly vision, both preaching predicates, Damascus, Jerusalem, all the villages of Judea and the Gentiles. Repentance, turning to God and works worthy of repentance remain distinct. The temple seizure and desired killing precede God's help until this day and Paul's testimony to small and great. The negative saying clause includes Moses and the prophets within its own linked span, before the separate contrast about what they predicted. Messiah's suffering, becoming the firstfruits of resurrection from among the dead and future proclamation of light to the people and Gentiles remain. The becoming verb stays with firstfruits, not with the preceding suffering phrase. Paul's long quotation closes at 26:23.
- Acts 26:24-27 retains Festus's interruption, loud voice and allegation that Paul has gone mad. ספרא סגיאא is rendered “much learning,” using the Syriac learning sense documented in [CAL's spr entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=spr+N), rather than requiring a physical book. Paul's denial addresses excellent Festus and names truth and uprightness. Agrippa's knowledge leads to open speech before him; עינ בגלא is the lexical adverb documented in [CAL's compound entry](https://cal.huc.edu/cal_entry_web.php?lemma=%28yn%40bgl%29+N), not an added claim about eyes. Paul's belief that no single matter has escaped Agrippa and the explanation that these things were not done secretly remain. His direct question about believing the prophets and assertion that Agrippa believes close at 26:27.
- Acts 26:28-32 keeps Agrippa's reply with the small-quantity expression and becoming a Christian. Paul's answer echoes little and much while asking God that not only the singular hearer but all who hear him today become like him, except for these bonds. The English leaves the measure unspecified rather than inserting time units or making Agrippa's conversion an established event; [CAL's qlyl entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=qlyl+A) records the underlying small-amount sense. The actual only word remains with not only you, before but also. The king, governor, Bernice and those seated with them rise. Their reciprocal conversation, finding nothing worthy of death or bonds and Agrippa's conditional release statement to Festus retain all names, negatives and the appeal to Caesar. All speeches and embedded quotations have balanced boundaries.

Source checks for Acts 27:

- Acts 27:1-3 explicitly names Festus ordering the sending to Caesar in Italy and handing Paul and other prisoners to Julius, a centurion of the Augustan cohort. The boarding predicate remains at its own positions in 27:2, with Adramyttium as the ship's origin and Asia as its destination. Aristarchus boards with us and is a Macedonian from the city of Thessalonica. Sidon, the next day, the centurion's kindness, permission to visit Paul's friends and refreshment all remain.
- Acts 27:4-8 preserves Cyprus, Cilicia, Pamphylia, Myra and Lycia in route order. The ship's difficult progress precedes the many days and arrival opposite Cnidus. The stored source explicitly calls Cnidus an island and Salmone a city; these labels are not silently changed to match another geographical account. The wind's prohibition on a straight course and our going around Crete remain separate predicates in their own spans. Fair Havens and the nearby city Lasea retain the actual source names.
- Acts 27:9-14 keeps the long stay through the passing of the Jewish fast day and the danger of sea travel, without assigning a modern calendar date. Paul's warning concerns our voyage, hardship and great loss, the cargo of our ship and our own lives; ship is not made a separate threatened object in the cargo clause. The centurion listens to the shipmaster and owner more than Paul's words. Many of us want departure, not an unexpressed majority, with the conditional prospect of reaching Phenice in Crete for winter. The harbor faces south in the stored wording, without additional directions. The south wind, their hope of reaching their desired destination, their sailing around Crete and the tempestuous Euroclydon remain. The wind name follows the actual אורקלידונ rather than replacing it with another familiar name.
- Acts 27:15-19 keeps the ship as the feminine singular subject that cannot face the wind, followed by our yielding. Cauda is the actual island name; the reference's Cyra is not imported. After lifting the boat, we strengthen and secure the ship, rather than securing the boat to the ship. The first-plural verbs remain throughout 27:17-19. The sea's descent expression is rendered as a precipice in the sea, following the supplied reference and the topographic descent/incline sense in [CAL's maḥḥatṯā entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=mxth+N). The exact nautical hazard remains uncertain; no unexpressed Syrtis proper name or definite sandbank identification is inserted. Lowering the foresail and drifting remain. Equipment is thrown into the sea on the next day; the third day names the ship's own equipment and our hands.
- Acts 27:20-24 explicitly includes the moon with the sun and stars during the continuing storm. The hope of our lives is completely cut off. No one having eaten precedes Paul standing among them. His conditional addresses you, while both consequences are we: we would not have sailed from Crete and would have been spared the loss and distress. His counsel against distress distinguishes the loss of the ship from the survival of every person. The angel's appearance is this night; the God to whom Paul belongs and whom he serves remains. The angel addresses Paul, his future standing before Caesar and God's gift of all those sailing with him. The gift noun is retained rather than replaced only by an inferred promise of life. The angel's words are nested within Paul's speech.
- Acts 27:25-29 keeps courage, trust in God, the promised outcome and the necessary casting upon a certain island. The duration is fourteen days, followed by midnight in the Adriatic Sea; it is not replaced by a fourteen-night formula. The sounding operation and both twenty and fifteen fathoms are retained, using the contextually attested wording in [CAL's qaw/ōməṯā entry](https://cal.huc.edu/cal_entry_web.php?lemma=qwmh+N), which cites 27:28 directly as casting the sounding line and finding twenty fathoms. No modern distance conversion is added. Our fear of rocky places is followed by their casting four anchors from the stern and praying for daybreak; the first- and third-plural subjects are not flattened into one actor throughout.
- Acts 27:30-34 preserves the sailors' attempted flight, lowering the boat from the ship into the sea and the pretense of going in it to fasten the ship to land. An absent bow-anchor explanation is not substituted. Paul sees this before speaking to the centurion and soldiers; his conditional requires these men to remain aboard for the hearers' survival. The soldiers cut plural boat ropes and let it drift. Paul urges everyone to take food until daybreak. His statement explicitly links fourteen days without tasting anything to the danger; no waiting predicate is supplied from another reading. The passive-participle/dative construction concerns what you have not tasted. Food for sustenance of your lives and the assurance that no hair from any hearer's head will perish close this speech before the meal narration.
- Acts 27:35-40 retains taking bread, glorifying God before everyone, breaking it and beginning to eat. Encouragement and nourishment precede the explicit total of two hundred and seventy-six souls aboard. After eating, they lighten the ship, take the wheat and throw it into the sea; both taking and throwing remain. At daybreak the sailors do not recognize the land but see a bay along its edge and plan, if possible, to drive the ship there. They cut away the anchors, leave them in the sea, loosen the plural rudders' fastenings and hoist a small foresail to the blowing wind before heading toward land.
- Acts 27:41-44 keeps the raised place between two depths of the sea, the ship's sticking there, its immovable forepart and its rear breaking under the waves' force. The soldiers' intended killing of the prisoners has the purpose of preventing swimming escape; the centurion restrains them because he wants to save Paul. Able swimmers are ordered to swim first and cross to land. The final causative plural verb says the rest are carried across on planks and other ship timbers, without assigning a new singular agent. Everyone's safe arrival on land remains explicit. Every source position is owned once, and the speeches in 27:10, 27:21-26 with its angel quotation, 27:31 and 27:33-34 close before the following narration.

Source check for Acts 28: the chapter retains the island's name, the foreigners' kindness, the viper and the change from expecting Paul's death to calling him a god. Publius, his father's fever and bowel pain, the healing, the island's other sick people, honors and provisions remain explicit. The route preserves the three months, Alexandrian ship, sign of the Twins, Syracuse, Rhegium, the south wind, Puteoli, the seven-day stay and Rome, followed by the Appian Forum, Three Taverns, centurion, and soldier guarding Paul.

Paul's account to the Jewish leaders keeps the absence of an accusation worthy of death, his appeal to Caesar, the hope of Israel, the lack of letters or evil reports from Judea and Jerusalem, and the invitation to explain the teaching. The day-long explanation preserves the kingdom of God, Jesus, Moses, the prophets, and the responses of persuasion and refusal. The Isaiah quotation retains the continuous negative purpose concerning seeing, hearing, understanding, returning and forgiveness. The final statement keeps God's redemption being sent to the Gentiles: the source form is plural and its people/nation/Gentiles sense is resolved by the immediate contrast and Murdock context. Paul's two-year rented-house ministry preserves his own expense, reception of visitors, proclamation of God's kingdom, teaching about our Lord Jesus Messiah, openness and lack of hindrance. Quotation boundaries run continuously from Paul's speech in 28:17 through 28:20 and through the nested Isaiah quotation in 28:25-28.

Source check for Romans 1: the opening preserves Paul's servant and apostolic calling, the Gospel promised through the prophets, Jesus Messiah as God's Son, grace and apostleship among the nations, the Roman greeting, Paul's thanksgiving and prayer, and his desire to visit Rome. The chapter retains the explicit source order and predicates in the witness clause (God is Paul's witness), the adversative in 1:13, Paul's indebtedness to everyone in 1:14, the negation and Gospel power in 1:16, and the manifestation of God's knowledge in 1:19. The righteousness of God is revealed from faith to faith, and the repeated darkness and lack of understanding in 1:21 remain distinct.

Romans 1:23-32 retains the source's non-corruptible God, human and animal images, delivery over by God, unclean lust and shame, feared and served creation, their Creator, pain and shame, their nature, the male and female clauses, fitting recompense received in their persons, the delivered-over mind and improper acts, the full vice lists, the covenant and three separate negative clauses, and the final distinction between doing such things and being their companions. Source suffixes, repeated negatives, “these” demonstratives, judgment, death, condemnation, and the final “those who do these” clause remain explicit.

Source checks for Romans 2:

- Romans 2:1 retains the no-excuse construction, the neighbor being judged,
  the emphatic self-condemnation and the judge's own conduct. The related
  breath/escape expressions in 2:1 and 2:15 are rendered idiomatically as
  excuse, rather than adding an independent spiritual actor; compare
  [CAL’s mappaq b-ruha idiom](https://cal.huc.edu/cal_entry_web.php?lemma=mpq%40brwx%29+N).
- Romans 2:4 keeps the repeated preposition with the riches of goodness,
  patience and opportunity, and renders the insolent/daring predicate as
  “do you presume.” The repentance predicate retains its recipient.
  [CAL's mrḥ entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=mrx+V)
  records the daring sense. Romans 2:5 links the negative with repentance,
  and keeps both laying up and a store of wrath, the day and the revelation.
- Romans 2:7 retains glory, honor and incorruptibility in source order before
  seeking; the following recipient is explicit in “he gives them life
  forever.” Romans 2:9-10 retains the Jew/Gentile parallels. Romans 2:12
  keeps both without-law phrases in its first half, followed by the separate
  under-law and by-law judgment clauses.
- Romans 2:13 renders כאנינ as “who are righteous,” distinct from the next
  words, “before God.” The exported root's gloss alternatives do not exhaust
  the actual form's adjectival sense. [CAL's kʾn adjective entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=k%29n+A)
  defines just/honest and illustrates the righteous sense in Romans 3:10.
- Romans 2:23 keeps boasting, transgression against the law and dishonoring
  God himself distinct. Romans 2:25 retains present “is profitable,” the two
  conditional clauses, “however,” and the circumcision/uncircumcision
  contrast. Its second-person ethical dative belongs with “you depart,”
  without inventing an object “it”; the final dative is absorbed into
  “has become.” Romans 2:26 retains the question and renders the presentative
  particle as “then”; [CAL records its now-then use](https://cal.huc.edu/cal_entry_web.php?lemma=h%29+I). Romans 2:28-29 keeps the outward/hidden Jew and
  circumcision contrasts, with smaller meaningful source links.

Source checks for Romans 3:

- Romans 3:2 retains the passive entrusting of God's words. Romans 3:3 keeps
  the explanatory connector in its question about unbelief. Romans 3:8
  preserves the reported slander and repeated saying predicates rather than
  replacing them with a remembered parallel text.
- Romans 3:12 retains the passive/mediopassive sense of becoming worthless or
  rejected. Romans 3:13 renders חמתא as venom in the asp construction;
  [CAL's ḥmh/ḥmtʾ entry, sense 2](https://cal.huc.edu/oneentry.php?cits=all&lemma=xmh+N)
  explicitly cites this verse for venom. The abbreviated anger glosses in
  the export do not exhaust the source word's contextual senses.
- Romans 3:19 keeps the contrastive connector in “But we know,” the law's
  hearers, every mouth being stopped and the world being condemned before
  God. Romans 3:20 retains justification, works of the law and the knowledge
  of sin as separate predicates and constructions.
- Romans 3:22 renders the source's possessive-plus-genitive construction as
  “God's righteousness, however,” expressing one ownership relation without
  inventing two owners. Romans 3:23 preserves “all of them.” Romans 3:25
  retains propitiation, with its source faith and blood wording, consistent
  with the Syriac context and Murdock's reference rendering.
- Romans 3:26 renders במגרת רוחה idiomatically as “in his patience”; its
  spirit/breath word belongs to the patience construction and does not
  introduce a separate spiritual actor. Romans 3:31 retains the question
  about annulling the law and the contrasting assertion of establishing it,
  using grammatical English cleft constructions for the source copulas.

Source checks for Romans 4:

- Romans 4:1-5 retains Abraham's relation to the fathers, the flesh/works
  questions, boasting before God, the reckoning of faith as righteousness,
  and the contrast between grace and what is owed. Romans 4:6 links “also”
  and “David” to their own source words. Romans 4:7-8 keeps forgiveness,
  covering and the negated reckoning of sin distinct; the last clause uses
  one contiguous grammatical construction to preserve its participants.
- Romans 4:9-12 keeps the circumcision/uncircumcision questions and Abraham's
  faith as the object of reckoning. The source-fronted reckoning predicate
  and Abraham/faith construction are rendered together where English needs
  the grammatical relation. The sign, seal and fatherhood clauses remain
  explicit. Romans 4:13-16 preserves the law, promise, faith and grace
  contrasts, the conditionals and the source negatives.
- Romans 4:17-18 retains God's life-giving and calling, the things that are
  not, the repeated hope wording and the seed/nations promise.
- Romans 4:19 preserves “his faith,” consideration of the body as dead,
  “since he was,” and the bar-hundred-years age idiom. The second dead
  description belongs to Sarah's womb. The repeated prefixed object
  construction is rendered with “looking at” the body and “at the dead
  womb,” keeping the womb and its adjective with their actual source words.
- Romans 4:24 separates מטלתנ (“for our sake”) from דאפלנ, contextually
  d- plus af plus lan (“for also to us”). The compound's empty exported
  gloss is not evidence for an invented demonstrative. The later אילינ
  supplies “the ones,” followed by the believers; it is not the source of
  “to us.” The raising of our Lord Jesus Messiah from the dead remains
  explicit. Romans 4:25 preserves the passive delivery, our sins and the
  raising/justification parallel.

Source checks for Romans 5:

- Romans 5:1-2 retains justification by faith, peace, access to this grace
  and the hope of God's glory. The possessive-plus-genitive construction in
  “God's glory” expresses one ownership relation.
- Romans 5:3 renders the contiguous object/predicate construction as
  “perfects patience in us.” Romans 5:4 supplies the English elliptical
  progression predicates in “endurance produces proof” and “proof produces
  hope”; these clarify the source's nominal progression without inventing
  separate Syriac verbs. Romans 5:5 renders the negative and shame predicate
  as “does not bring shame,” without inserting an unexpressed object, and
  retains the love of God, our hearts and the given Spirit of holiness.
- Romans 5:8 keeps God's showing his love, our being sinners and the
  Messiah's death for us explicit. Romans 5:9 renders ובה contextually as
  “by him,” following Murdock's Syriac rendering. Its masculine suffix alone
  does not decide between the Messiah and the nearby blood; “by it” would
  commit to a different antecedent without sufficient contextual evidence.
- Romans 5:10-14 retains reconciliation, the Son's death and life, the
  source's death/sin sequence, the negatives about law and sin, and the
  Adam/type/coming contrast. The source's subjects and prepositional
  arguments are kept with their actual linked words.
- Romans 5:15-17 preserves fault versus gift, the comparative and
  conditional particles, the repeated one/many contrasts, grace, gift and
  righteousness. Romans 5:18-19 preserves condemnation versus
  justification and disobedience versus obedience. Romans 5:20-21 retains
  the law/sin increase and the parallel reign of sin in death and grace in
  righteousness unto eternal life through our Lord Jesus Messiah.

Source checks for Romans 6:

- Romans 6:1-5 preserves the rhetorical questions, remaining in sin versus
  abounding grace, death to sin, baptism into Jesus Messiah and his death,
  burial, new life, and the planting/likeness/resurrection relations.
  The repeated baptism predicate and its possessive death phrase are
  rendered as grammatical local constructions.
- Romans 6:6 keeps “For we know that,” our old human being crucified with
  him, and the purpose that the body of sin be annulled. The final דתוב לא
  predicate is contextual cessation, “so that we may no longer serve sin,”
  rather than a bare additive “furthermore.” Romans 6:7 keeps the causal
  connector and the dying/freedom-from-sin relation together.
- Romans 6:9-15 preserves the death/life predicates, the dead/alive contrast,
  the commands about the mortal body and members, sin's rule, and the
  law/grace contrast. Source pronouns remain with their actual predicates.
- Romans 6:16 retains “to whomever,” the purpose verb “obey,” its recipient,
  and bondage. The nominal predicate is “you are his own servants,” followed
  by the singular owner whom they obey. Its final hearing/ear construction
  is rendered as giving ear to righteousness. The source's sin/obedience
  alternatives do not contain an extra death clause.
- Romans 6:17-19 keeps the thanksgiving, heart/teaching relation, freedom
  from sin, service to righteousness, human analogy and bodily weakness.
  Romans 6:20 renders גיר as clause-level “For when you were servants,”
  separately from “of sin”; prepositional “for sin” would lose the causal
  connector's function. Romans 6:21-23 preserves fruit/end, death versus
  eternal life, the payment/gift contrast and our Lord Jesus Messiah.

Source checks for Romans 7:

- Romans 7:1-3 keeps the law's dominion, the living/dead conditions and the
  woman/husband roles distinct. The last condition in 7:3 says that she may
  be joined to another man; it does not say she becomes another man.
- Romans 7:4 keeps the relative “who” and being joined to the one raised
  from among the dead. Romans 7:5 preserves the causal connector, the law,
  passions and bearing fruit. Romans 7:6 links released/from/law to their
  actual words and renders the deictic reference neutrally as “to that
  which held us,” consistent with the law context and Murdock, without
  inventing a personal male actor from grammatical masculine gender.
- Romans 7:7-8 retains the knowledge/negation contrast, desire, commandment,
  occasion and sin as distinct source elements. The suffixed recipient in
  finding an occasion is retained. The accomplishment predicates are read
  in their actual desire/death contexts.
- Romans 7:9-16 preserves the came-to-life/died contrast, commandment, the
  finding predicate and “to me,” opportunity and its suffix, the good/death
  question, sin's role, fleshly/sold status, knowing, willing and doing.
  The final demonstrative and doing predicate are rendered as a grammatical
  local construction rather than dropping the first-person contrast.
- Romans 7:17-20 preserves the I/sin-in-me distinction and the exact negative
  scopes around dwelling, good, willingness, ability and doing. Necessary
  English copulas and local constructions clarify the source order without
  adding another actor. Romans 7:21-25 retains the conscience/law relation,
  members and mind, captivity, the deliverance question, thanksgiving and
  the final mind/flesh service contrast.

Source checks for Romans 8:

- Romans 8:1-3 retains the condemnation, law, Spirit/life, flesh and sin
  relations. In 8:3 the law is weak; the following weakness/flesh
  phrase expresses one proleptic genitive. מטלתה דחטיתא is one causal
  construction, “because of sin,” not two independent causes. The resumed
  object in דנחיביה לחטיתא is sin: “that he might condemn sin,” not
  “condemn it as sin.” The Son-sending and final flesh suffix remain explicit.
- Romans 8:5-10 keeps the flesh/Spirit contrasts and dwelling relations.
  Romans 8:7 preserves both the law's ל preposition and causal גיר in the
  fronted “for, to the law of God” phrase. Romans 8:10 keeps the conditional
  and contrastive opening.
- Romans 8:11-20 preserves the resurrection/life and debt/service clauses,
  the Spirit/children relation, the filial cry, inheritance and suffering,
  and creation's subjection. Romans 8:15 retains “again to fear,” with the
  source preposition. Adoption is a contextual rendering alongside the
  literal sonship sense. Romans 8:20 uses contextual “in hope,” consistent
  with Murdock's Syriac rendering.
- Romans 8:21-30 keeps creation's release, first-fruits, adoption and bodily
  redemption, Spirit help and prayer, God's will and intercession, and the
  knowing/calling/justifying/glorifying recipients. The 8:26 prayer question
  is split into smaller source-linked constructions: “what to pray for,”
  “as is fitting,” and “we do not know.” Romans 8:28 retains the all-things
  and beneficiaries relation; 8:30 retains the contrastive דינ.
- Romans 8:31-39 retains the rhetorical questions, delivery of the Son,
  death/resurrection/intercession, separation/love and the cited slaughter
  image. In 8:38 שולטנא remains the authority slot, while דקימנ and דעתידנ
  supply the present/future pair, rather than literal “standing/prepared”
  descriptions. Compare [CAL's qwm entry, including being present](https://cal.huc.edu/oneentry.php?cits=all&lemma=qwm+V)
  and [its ʿtyd analysis, including future things](https://cal.huc.edu/getlex.php?coord=620442623&word=7).

Source checks for Romans 9:

- Romans 9:1-5 keeps the truth/conscience/sorrow statements, kinship and
  Israel's sonship, glory, covenant, law, service and promise relations.
  The appearance of Messiah, the relative God-over-all clause and the
  recipient of praise/blessing remain with their actual source words.
- Romans 9:6-11 preserves the Israel and Abraham/seed distinctions, the
  flesh-children/promise-children contrast, the promise cleft, Sarah and
  Rebecca, and the one-partner/Isaac construction. The pre-birth condition,
  good/evil acts, election being known beforehand, its remaining, works and
  the one who calls are kept distinct. Local grammatical constructions
  express the source's copulas and ethical datives without adding owners.
- Romans 9:12-22 retains the causal “For it was said,” Jacob/Esau love/hate
  contrast, Moses and Pharaoh as recipients, and the mercy/compassion,
  will/power/name and potter/clay/vessel relations. Romans 9:19 keeps the
  conjunction in “And perhaps,” its causal connector and the will suffix.
- Romans 9:25 preserves two separate people expressions: עמי (“my people”)
  and עמא דילי (“my own people”). The two identical אתרחמת forms are
  passive/third-feminine forms in the parallel naming construction:
  the one not pitied is called “pitied.” They are not rewritten as an
  independent first-person future “I will have mercy.” The earlier calling
  predicate governs both naming pairs.
- Romans 9:26-29 preserves the quotation/calling relations, the remnant and
  Lord-of-hosts condition. The Sodom and Gomorrah comparisons are linked
  separately with grammatical counterfactual English; the comparative
  predicate is not left as an English postposition after Gomorrah.
- Romans 9:30 retains all three righteousness occurrences in their own
  links, including the appositional righteousness/faith construction.
  Romans 9:31-33 preserves the law/righteousness pursuit contrast, both
  causal elements, faith/works negation scope and the stumbling-stone
  quotation, without replacing source-specific words with a remembered
  parallel passage.

Source checks for Romans 10:

- Romans 10:1 retains the plural recipient in חלפיהונ, “for them.”
  Romans 10:2 preserves both causal גיר and the concerning-them relation
  in the local testimony construction. Romans 10:3 renders each God genitive
  as one ownership relation, “the righteousness of God,” distinct from
  “their own righteousness”; the proleptic suffix does not add another owner.
- Romans 10:4-7 preserves the law/Messiah and faith/righteousness relations,
  Moses and the source's life predicate, and the heart/heaven/abyss questions
  with their Messiah explanations. The quote roles and verb subjects are
  not replaced by a remembered parallel text.
- Romans 10:8-14 keeps word, mouth and heart, confession, belief, God and
  Jesus, resurrection and life/salvation, the Jew/Gentile distinction and
  the calling/hearing/believing questions. Romans 10:10 דמודא בה is rendered
  “that confesses him”: the Syriac verb's governed complement is expressed
  by English transitive confession, as in Murdock, rather than an unidiomatic
  locative “confesses in him.” The earlier believing-in-him phrase retains
  its own prepositional construction. Romans 10:12 keeps causal “For.”
- Romans 10:15-21 preserves the feet/herald sequence, sending and preaching,
  the negative scope in the good-news response, and the “our voice” suffix
  in Isaiah's question. Romans 10:17 retains the word-of-God reading supplied
  by the dataset. The rhetorical hearing/knowledge questions, Moses/Isaiah
  quote speakers and nation/Israel recipients, and the final outstretched
  hands and disobedience predicates remain source-linked.

Source checks for Romans 11:

- Romans 11:1-4 retains the rejection question, Paul's Israelite identity,
  the people known beforehand, Elijah's complaint and the answering
  revelation. Possessive and reflexive relations remain explicit without
  multiplying owners in the English.
- Romans 11:5-8 renders גביתא as contextual “election,” following the
  remnant/election contrast and Murdock rather than the supplied isolated
  alms-collection gloss. The grace/work conditional clauses preserve all
  repeated negations. Romans 11:8 renders מדערניתא “contentious,” following
  the Syriac lexical sense; Murdock's “stupid” is a different contextual
  choice. The seeing/hearing negatives and “until this day” remain explicit.
- Romans 11:9-16 preserves David's quotation, the stumbling/falling
  question and its emphatic denial, the jealousy purpose, and the
  Gentile/Israel comparison. Romans 11:15 retains בית in “the house of
  the dead”; the life-from-death image is not silently shortened.
- Romans 11:17 expresses מן סוכא as the partitive “some of the branches,”
  and the olive-from-the-wild phrase as one idiomatic description. The
  grafted recipient and the root/branch, kindness/severity and
  faith/disobedience contrasts retain their pronouns and negation.
  Romans 11:24 keeps the natural/contrary-to-nature comparison and its
  concluding grafting question; local groups avoid selecting the whole
  opening clause as one unit.
- Romans 11:25-29 retains blindness of heart, Israel, the Gentile fullness,
  the deliverer/Jacob and covenant/sin clauses, the Gospel/election
  contrast, and God's gift and calling. Romans 11:26 uses “will live,”
  matching the source life predicate and Murdock's wording.
- Romans 11:30-36 preserves the distinct you/they obedience and mercy
  relations, the purpose of mercy for all, the unsearched judgments and
  ways, and the questions about the Lord's mind, counsel and repayment.
  The final verse keeps the three all-things clauses, “through his hand,”
  praise, blessing and eternity supplied by the Syriac.

Source checks for Romans 12:

- Romans 12:1-3 preserves the appeal, bodies offered as a living sacrifice,
  the mercies of God, and the renewal/testing clause. The repeated thinking
  predicates and the beyond-what-is-proper contrast retain their subjects,
  negation and the distribution/measure-of-faith relation.
- Romans 12:4-8 keeps the body/member image and the distinct gift,
  prophecy, service, teaching, exhortation, giving, presiding and mercy
  clauses. Romans 12:7 treats אית לה as possession, “which he has,”
  within the existential service construction. Romans 12:8 retains the
  relative ד on each distinct gift-bearer: the one who gives, stands at
  the head, and has compassion. These are not three further acts with
  the comforter as their shared subject. “His comforting” names the
  consoling activity rather than comfort received.
- Romans 12:9-14 renders the deceit/love contrast as readable English,
  retains both reciprocal phrases in the brotherly-love/honor clause,
  and preserves the repeated exhortations in hope, endurance and prayer.
  Sharing in the saints' needs and loving strangers follow the Syriac
  constructions. Both commands to bless and the final prohibition of
  cursing are retained.
- Romans 12:15-18 preserves rejoicing/weeping and the two sets of people,
  the source's comparison of thinking about yourselves and your brothers,
  both negatives about pride/wisdom, and the evil-for-evil prohibition.
  The possible/from-you qualification limits the peace command as in the
  source rather than turning it into an unconditional assertion.
- Romans 12:19 keeps the prohibition of self-vengeance, the beloved
  addressees and room-for-wrath command. Causal גיר belongs to “For it is
  written” before the quotation. The quotation retains the Syriac-specific
  conditional “If you do not execute judgment for yourself, I will execute
  your judgment,” and its attribution to God.
- Romans 12:20 retains the enemy-feeding/drinking conditions and the
  source's further condition about doing these things, followed by the
  coals-of-fire/head image. Romans 12:21 preserves the negative and positive
  overcoming predicates, repeated evil and the final “with good.”

Source checks for Romans 13:

- Romans 13:1 retains plural governing authorities and the repeated
  source copulas. פקידין is a masculine plural passive participle:
  “are appointed,” agreeing with authorities; the lemma gloss “command”
  does not turn it into an active “he commanded.” The two הו tokens
  remain copular/emphatic rather than supplying a conjunction or a new
  acting subject. The second clause keeps appointment by God distinct
  from the relative clause about authorities existing.
- Romans 13:2-5 preserves the repeated standing-against constructions,
  their subjects and recipients, the sword image and both minister
  clauses. Romans 13:5 is impersonal necessity, not a new personal
  subject urging someone. All causal גיר occurrences remain represented.
- Romans 13:6-7 renders כסף רשא as “head-money,” retains קימין as standing,
  and keeps the repeated tax, fear and honor recipients. דמתחיב לה is
  “what is owed to him”; the obligation is not reversed into what he owes.
- Romans 13:8 retains the reciprocal חד לחד as “one another” and causal
  גיר in the explanation about loving one's neighbor and fulfilling the
  law. Romans 13:9 retains the repeated prohibitions and דתרחם as the
  complement “that you love,” not an added temporal “when.” Romans 13:10
  keeps love as the subject and the neighbor as its recipient.
- Romans 13:11-14 preserves our sleep/life, the approaching day and the
  works-of-darkness/armor-of-light contrast. “Our life” retains the
  source's possessive life predicate; salvation is a contextual alternative.
  Romans 13:13 uses contextual “revelry” for music/merriment in the vice
  list, rather than implying a general prohibition on singing. The
  impurity-of-the-bed phrase also retains the source bed noun. Our Lord,
  Messiah Jesus, the flesh and its lust relation remain source-linked.

All nine causal גיר occurrences were checked individually against the final
linked text. Numeric morphology was decoded with the SEDRA source codebook.

Source checks for Romans 14:

- Romans 14:1-3 preserves the weak-in-faith recipient, the giving-a-hand
  idiom, the addressees' thoughts, and the repeated eater/non-eater
  subjects and negatives. The believing and weak subjects in 14:2 are
  distinct; the causal God-receiving clause in 14:3 remains explicit.
- Romans 14:4 preserves the servant who does not belong to the addressee,
  his Lord, the repeated stand/fall clauses and the final establish-him
  object. מטא גיר באידי מרה is rendered as the local reach/power idiom
  “for it is within his Lord's power”; the causal and copular functions
  are not assigned to the wrong source words.
- Romans 14:5 renders כלהון יומתא as “all days,” not an invented possessor's
  “their days.” The day comparisons and each person's assurance in his own
  mind remain distinct. Romans 14:6 preserves the repeated day-assessment,
  eating/non-eating, Lord and thanksgiving clauses, including every negative.
- Romans 14:7-12 retains living/dying for oneself and for our Lord, the
  first-person-plural possessives, conditionals and “therefore,” the
  judgment-seat statement, and each person's account for himself to God.
  Local groups replace an unnecessary whole-verse link in 14:12.
- Romans 14:13-15 preserves both prohibitions about judgment/stumbling,
  the food/brother conditional and the Messiah-died relative clause.
  Romans 14:14 retains the negative existential and both self/unclean
  relations; the knowing and persuaded predicates remain separately linked.
- Romans 14:16 uses the source's passive/reflexive blasphemy construction
  with “our good,” not an active prohibition with the wrong subject.
  Romans 14:17-20 retains the kingdom's eating/drinking contrast, the
  righteousness/peace/joy triad, serving Messiah, God's approval, pursuit
  of peace and mutual building, and the food/work/purity/stumbling clauses.
- Romans 14:21 preserves all three coordinated negatives and “our brother.”
  Romans 14:22 keeps faith in the addressee, the imperative's object, the
  before-God relation and the reflexive judgment/approval clause.
- Romans 14:23 renders אתחיב לה as “is condemned,” absorbing the
  coreferential/ethical dative into its predicate. לֵה is third masculine
  singular; it does not introduce a separate causal “for it” referent.
  The following מטל already supplies the explicit reason, and the final
  not-from-faith/sin statement retains its own subject and negation.

All eleven גיר occurrences were checked individually against the final
linked English. Decoded morphology distinguishes finite and participial
forms, with first/second/third-person and passive/reflexive roles reviewed
against the source codebook and running context.

Source checks for Romans 15:

- Romans 15:1-3 preserves the strong/weak obligation, the infirmity being
  borne, both self-pleasing negatives, the neighbor's good and the quoted
  reproach/reproachers roles.
- Romans 15:4 retains the passive wording about things written beforehand
  and the instruction predicate. The purpose prefix in דבמסיברנותא is explicit in “so that
  through endurance”; endurance, Scripture's comfort and hope keep their
  distinct links. סברא נהוא לנ is the possessive existential “we may have
  hope,” rather than forcing a bare gloss sequence into the English.
- Romans 15:5-11 retains mutual thought, one mind and one mouth, God as
  the Father of our Lord, reciprocal receiving, the glory of God and the
  truth of God. Proleptic suffixes do not duplicate the explicit owners.
  The Gentiles/mercy clause and quoted confession, singing, rejoicing and
  praise retain their speakers, recipients, vocatives and imperatives.
- Romans 15:12-14 keeps Jesse's root, rising/chief roles and Gentile hope;
  “all joy” is one quantified phrase, not an extra “all of it.” The source
  Holy Spirit phrase does not add another God noun. Repeated emphatic
  first-person forms identify one persuaded speaker, not two separate
  persuasion predicates.
- Romans 15:15 retains the ב prefix in בטיבותא as the reason supplied by grace
  for the reminder; grace is not made the reminder's object. Romans
  15:16-18 keeps the minister/offering roles and passive acceptance,
  possession in “I have,” and Messiah as the actor who worked through
  Paul's hands. “My hands” does not become “Messiah's hands.”
- Romans 15:19-22 retains the Jerusalem/Illyricum extent, the single Spirit
  of God ownership chain, the participial preaching continuation, the statement about
  Messiah's name, the see/hear quotation and the repeated hindrance to coming.
- Romans 15:23-25 preserves the desire over many years and future route through
  the addressees to Spain, “a little” refreshment, “you will accompany me,”
  and the present Jerusalem journey to serve the saints.
- Romans 15:26-27 retains Macedonia/Achaia, fellowship with the needy saints
  in Jerusalem, the Gentiles' participation and the spiritual/physical
  service contrast. Both גיר tokens are causal discourse connectors,
  not prepositional “for those” or “for spiritual things.”
- Romans 15:28-33 keeps completion, sealing and fruit, the blessing of the Gospel of
  Messiah, the governed request “I ask you,” prayer for the speaker
  to God, deliverance and passive acceptance of the service, joy and
  refreshment. The closing “But the God of peace” retains the connector
  without duplicating God.

All 33 verses and 412 source words received source-to-English and running-
English review. All four גיר occurrences were checked individually in their
final English constructions.

Source checks for Romans 16:

- Romans 16:1-4 preserves the sister/minister and church relations, the
  receiving/support purpose, assistance to many and to Paul himself,
  Priscilla and Aquila's plural service, their necks and Paul's life, and
  the contrast between Paul's thanks and the Gentile churches' thanks.
- Romans 16:5-9 keeps the church in their house, Epaenetus's first-fruits
  relation to Achaia, Mary's work, and the distinct kinship, captivity,
  beloved and service relations. Urbanus's singular service is not confused
  with the plural description of Priscilla and Aquila.
- Romans 16:7 uses “Junia” following the supplied Murdock reference and
  source יוניא; SEDRA's alternate English gloss “Junias” does not decide
  historical gender. “My kinsfolk” adds no personal gender inference.
  וידיעין אנון is present “and are known,” distinct from past הוו in the
  captivity and earlier-in-Messiah clauses. Local groups preserve that
  tense distinction and the “among the apostles” relation.
- Romans 16:10-15 preserves the named companions, household owners,
  repeated workers and separate Persis clause, Rufus's mother relation,
  Nereus's sister, and brothers/saints with the named groups. The explicit
  household names each supply one owner, not duplicate proleptic owners.
- Romans 16:16 attaches “all” to the churches, not to the addressees.
  Romans 16:17 renders לבר מן as the local idiom “contrary to” the learned
  teaching; it does not require two independent English prepositions.
  Division/stumbling actors and the avoidance command remain distinct.
- Romans 16:18 retains causal גיר as “For,” negated service of our Lord
  versus their belly, and the speech/blessings/deception/hearts relations.
- Romans 16:19-23 preserves obedience and its addressees, wise/good versus
  guileless/evil, the peace/quickness/feet statement and separate grace
  blessing. The anticipatory object suffix and Satan form one object.
  Timothy's group, Tertius as letter writer, Gaius as host, Erastus's city
  office and the other brother retain their speakers and relationships.
- Romans 16:24-26 follows this dataset's doxology layout: mystery hidden
  and revealed, prophetic writings, the eternal God's command, the nations
  and faith's obedience, the one who alone is wise, Jesus Messiah mediation
  and glory. ובפוקדנה דאלהא דלעלם is one owner relation, “by the command of
  the eternal God.” אתידע stays passive “is made known”; God is not made
  a new active subject by misreading the genitive or proleptic suffix.
  Romans 16:27 remains the separate final grace blessing.

All 27 verses and 323 source words received source-to-English and running
English review. The single גיר occurrence was checked in its final causal
construction.

Romans 16 uses the established “ecclesia/ecclesias” vocabulary in the five
congregation spans. The generator already normalizes “church/churches” to
these terms; the tracked wording matches that runtime convention.

Source checks for 1 Corinthians 1:

- Verses 1-3 preserve Paul's call/apostleship, God's will, Sosthenes,
  the ecclesia of God in Corinth, sanctification in Jesus Messiah,
  calling and invocation of the Lord, including the place and possessive phrases. Grace, peace,
  God the Father and our Lord remain distinct without duplicate owners.
- Verses 4-9 retain thanksgiving, given grace, enrichment in word and
  knowledge, confirmed testimony, gifts and awaited revelation, the
  strengthening/blamelessness purpose and God's faithful call to fellowship.
- Verse 10 treats מנכון as the governed addressee of “I beseech you,”
  rather than turning the appeal into a spatial “among you.” The clauses about agreement,
  absence of divisions, one mind and one thought retain their subjects.
- Verse 11 keeps causal גיר attached to reporting, עליכון as “concerning
  you,” Paul as recipient and Chloe's house as the report's source. The
  impersonal plural reporting verb becomes grammatical English passive;
  the ד prefix introduces the reported contentions among the addressees.
- Verses 12-16 retain the four separate allegiance quotes, the division,
  crucifixion and baptism questions, name ownership, Crispus, Gaius and
  Stephanas's household, the baptism exceptions and what Paul does not know.
- Verses 17-20 state that Messiah did not send Paul to baptize but to preach.
  They retain the purpose that his cross not be emptied, the perishing/living
  contrast, God's power, the written quotation's first-person speaker, and
  the questions about the wise, scribe, disputer and the world's wisdom.
- Verse 21 keeps both causal functions in “For since ..., God was pleased
  ...”: explanatory גיר and the subordinate מטול clause. God/world/wisdom/
  knowing, preaching, saving and believing retain their actual roles.
- Verses 22-24 render ארמיא contextually as “Gentiles” in the contrast with
  Jews, following the supplied gloss alternative and Murdock. This does
  not restrict the clause to ethnic Arameans or substitute a Greek text.
  The broader gentile/pagan sense is also recorded in the
  [CAL lexical entry](https://cal.huc.edu/cal_entry_web.php?lemma=%29rmy%232+A).
  First-person preaching, crucified Messiah, stumbling/foolishness and
  calling/wisdom/power remain source-linked.
- Verses 25-31 preserve God's foolishness/weakness comparisons, the three
  “not many” clauses, God's chosen subjects and shamed/nullified objects,
  flesh not boasting, being in Messiah, wisdom from God and the distinct
  righteousness/sanctification/redemption terms. The final boasting quote
  retains the Lord as its reference.

All 31 verses and 382 source words received source-to-English and running
English review. All six גיר occurrences were checked individually for their
causal function; coverage alone was not used as proof of meaning.

Source checks for 1 Corinthians 2:

- Verses 1-5 preserve Paul's coming and speech, God's mystery, the negative
  self-judgment and exception concerning Jesus Messiah, and the passive
  crucifixion description. Weakness, fear, trembling, Spirit, power and
  human wisdom retain their separate roles. The purpose prefix in verse 5
  explicitly connects the addressees' faith with God's power rather than
  human wisdom.
- Verses 6-8 retain mature hearers, this world's rulers and their being
  annulled, hidden wisdom and God's appointment of it beforehand for our
  glory. The explicit God subject and wisdom object suffix remain active
  in “God appointed it beforehand.” The knowing/crucifixion condition is
  counterfactual; the Lord of glory has one owner relation, not an added
  “his Lord of glory.”
- Verse 9 keeps the three eye/ear/heart negatives and God's preparation
  for those who love him. Verse 10 retains both Spirit occurrences, God's
  revealing and the Spirit's active searching of everything and God's
  depths. The explanatory clause begins “for the Spirit searches.”
- Verse 11 preserves the analogy with the human spirit, both knowledge exclusions
  and “what is in God” for דבאלהא. The question's copular function is
  explicit. The Spirit of God does not acquire a second proleptic owner.
- Verse 12 retains the contrastive דין as “But we,” reception of the
  Spirit from God rather than the world's spirit, the knowing purpose and
  gifts given to us. The two explicit God occurrences belong to separate
  clauses and are both retained.
- Verse 13 retains speaking and the contrasting sources of teaching.
  We use the interpretation addressed to spiritual recipients, supported
  by the [CAL pḥm entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=pxm+V),
  which specifically cites this verse for the interpretive sense, and the
  [CAL rwḥn entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=rwxn+A).
  Murdock's comparison wording is an alternative. The construction and
  lexical attestation support this choice; gender alone would not settle it.
- Verses 14-15 describe the natural person once, retain the two
  causal גיר clauses and the further causal ד prefix, and preserve the
  negative reception/ability clauses. Judging and being judged remain
  distinct, including discernment by the Spirit and “by no man.”
- Verse 16 keeps knowing the Lord's mind and teaching him in their actual
  direction. The final אית לנ is possession, “is ours,” with the mind of
  Messiah as its subject; it is not an extra instruction predicate.

All 16 verses and 224 source words received source-to-English and running
English review. All six גיר occurrences were checked individually in the
final English clauses.

Source checks for 1 Corinthians 3:

- Verses 1-4 preserve the spiritual/fleshly and babe contrasts, milk versus
  food, former and current inability, envy/strife/divisions, and the
  distinct Paul/Apollos allegiance statements. The מָא construction in
  verse 4 is temporal “For when,” not an invented “What is it” question.
  Both גיר clauses in verse 3 are grammatical causal openings.
- Verses 5-8 retain the ministers through whom belief came, assigned
  service, planting and watering, and God as the one who gives growth. The
  “therefore neither ... nor ... but God” construction keeps both negatives.
  Shared labor does not erase each worker's individual reward.
- Verses 9-11 preserve working with God, the field/building relations,
  given grace, Paul's foundation and the other builder, and Jesus Messiah
  as the foundation. The “which is laid” qualifier in verse 11 is explicit
  within the foundation clause. The two דין markers in verse 10 retain
  their local contrasts, including “But let each one see.”
- Verses 12-15 keep all six building materials and distinguish the work,
  day, fire and person. Verse 13 has one day noun: “for the day will reveal
  it”; an added “on the day” is not retained as a second occurrence. Fire
  reveals/tests the work, while remaining or burning work leads to reward
  or loss. The person escapes; the source's comparison is “but in this way,
  as ... from fire,” retaining דין and מן rather than imposing “through.”
- Verses 16-17 keep plural “you” and singular “temple,” with God as its
  owner. They also retain God's Spirit dwelling in the addressees and
  the temple's holiness.
  The repeated חבל is rendered “ruins ... God will ruin him,” retaining
  the damaging/destructive sense and the temple/person actors. Murdock's
  repeated “mar” supports that sense; the wording does not suggest God
  causes moral corruption in the person.
- Verses 18-20 preserve self-deception, becoming foolish/wise, worldly
  wisdom in relation to God, and both scriptural quotations. The Lord
  knows the wise people's thoughts; the thoughts, not an invented new
  subject, are described as vain.
- Verses 21-23 retain the prohibition of boasting in people and every
  listed item: Paul, Apollos, Cephas, the world, life, death, the present
  and the future. Their relation to the addressees is followed by the
  addressees belonging to Messiah, and Messiah to God. Actual repeated
  nouns are not lost as supposed duplicate proleptic owners.

All 23 verses and 309 source words received source-to-English and running
English review. The twelve גיר, six דין and one הכיל occurrences were
checked individually against the final English clauses.

Source checks for 1 Corinthians 4:

- Verses 1-2 distinguish Messiah's ministers and household stewards of
  God's mysteries, retaining the requirement that each be found faithful.
  The compound for household stewards and the proleptic mysteries suffix
  do not introduce additional chiefs, houses, or owners.
- Verses 3-5 retain judgment by the addressees, judgment by any person,
  Paul's self-judgment, and his clear conscience without making that
  conscience a justification. Both causal clauses in verse 4 remain
  explicit, with the Lord as judge. The hidden things of darkness and
  thoughts of hearts each have one genitive relation. The Lord's coming,
  disclosure, and each person's praise from God remain distinct.
- Verses 6-7 retain Paul and Apollos, the purpose of learning within the
  written limit, the passive warning against being exalted above one's
  companion because of a person, and the received/not-received questions.
- Verse 8 uses contextual “By now” for the opening temporal idiom, rather
  than suggesting a new future claim with “From now.” The fullness,
  riches, reigning without the apostles, contrasting wish, and purpose
  that we also might reign with you retain their actors and connectors.
- Verses 9-14 keep God appointing the apostles last, the death comparison,
  all three spectacle audiences, every we/you contrast, and the hardship
  list. Work with their hands, blessing, endurance, and entreaty remain
  separate responses. The world and everyone are explicit in verse 13;
  verse 14 distinguishes shaming from admonishing beloved children.
- Verses 15-17 retain a singular myriad of tutors, not-many fathers,
  Paul's begetting through the Gospel in Jesus Messiah, the imitation
  appeal, and Timothy's sending to remind the addressees of Paul's ways.
  Teaching in all ecclesias uses the generator's canonical terminology.
- Verses 18-21 preserve the contrastive non-coming supposition, inflated
  people, the Lord-willing visit, their word versus their power, and
  God's kingdom in power rather than word. The final question contrasts
  the rod with love and a gentle disposition; it adds no separate actor.

All 21 verses and 301 source words received source-to-English and running
English review. The seven גיר, five דין and one הכיל occurrences were
checked individually against the final English clauses.

Source checks for 1 Corinthians 5:

- Verses 1-2 retain the report of sexual immorality, the contrast with
  what is not even named among the Gentiles, and the specific son/father's
  wife relation. The addressees' inflated state contrasts with mourning
  and the removal of the person who committed the deed.
- Verses 3-5 keep Paul's bodily distance and spiritual presence, his
  already-given judgment, the assembly in the name of our Lord Jesus
  Messiah, and the Lord's power. Repeated Lord/Jesus/Messiah names in
  distinct clauses remain explicit. Name, power and day suffixes followed
  by their explicit genitives do not add separate owners. The delivery
  verb's object suffix and following demonstrative name one offender;
  his body, life in spirit, and the day of our Lord remain distinct.
- Verses 6-7 retain the negative assessment of boasting, little leaven
  affecting the whole lump, purging old leaven, a new lump, and being
  unleavened. The causal Passover clause identifies Messiah, who was
  sacrificed for us; the copula does not introduce a second actor.
- Verse 8 has three actual occurrences of חמירא, including the final
  “leaven of purity and holiness.” This agrees with the supplied Murdock
  reference and is retained instead of replacing it with familiar
  “unleavened bread” wording. Both preceding negatives and the contrast
  remain explicit, as do wickedness and bitterness.
- Verses 9-11 distinguish the earlier letter's instruction from a blanket
  ban on association with people in this world. The conditional necessity
  of leaving the world is preserved. The person called a brother, every
  named vice, and the final not-even-eating-bread prohibition remain
  explicit; the final bread noun is not dropped into a generic meal.
- Verses 12-13 preserve the causal question about judging outsiders and
  the following imperative דונו, “judge,” addressed to the readers about
  those within. No negative rhetorical question is substituted for that
  imperative. God judges outsiders. The final removal command and its
  masculine singular object identify the wicked person; they do not
  introduce a second object or replace it with abstract wickedness.

All 13 verses and 200 source words received source-to-English and running
English review. The three גיר and four דין occurrences were checked
individually against the final English clauses.

Source checks for 1 Corinthians 6:

- Verses 1-4 retain the controversy with one's brother, the unjust versus
  holy judges, the holy ones judging the world, and the world being judged
  by the addressees. Judging angels and the lesser matters of this world
  have distinct objects. The appointment of people despised in the
  ecclesia remains an imperative concerning the addressees' own cases.
- Verses 5-7 preserve shame, the lack of even one wise person able to
  settle matters between brothers, and litigation before unbelievers.
  Verse 5's ability and settling predicates remain separately explicit.
  The opening temporal idiom in verse 7 is “already,” rather than the
  misleading concatenation “from ... it suffices.” Self-condemnation,
  litigation with one another, and the two questions about accepting wrong
  or defrauding retain their separate roles and connectors.
- Verses 8-10 retain doing wrong and defrauding even one's brothers, with
  no third predicate invented from a repeated pronoun. Every vice in both
  lists remains explicit, including distinct corrupt people
  and those who lie with males. The repeated kingdom-of-God clauses have
  one owner each, with the full negative inheritance predicate.
- Verse 11 retains “some of you,” washing, and the separately passive
  sanctified/justified predicates. The name belongs to our Lord Jesus
  Messiah and the Spirit to our God; proleptic suffixes do not add an
  extra owner or actor.
- Verses 12-14 preserve both lawful statements, the profitable/unprofitable
  qualification, and the final human subject who will not have dominion
  over Paul. That subject is not replaced with Murdock's supplied
  “none of them.” Food/belly and body/Lord relations remain reciprocal;
  God's doing away with both is distinct from raising our Lord and us.
- Verses 15-17 distinguish plural bodies/members from the singular member
  in the following question. The source's third-person actor is retained,
  rather than imposing a first-person question. The quoted “shall be”
  belongs to the two becoming one body; the explicit two, one body and
  one spirit are all retained.
- Verses 18-20 keep the body-related contrast between every sin and
  fornication, the singular body as temple with plural possessors, the
  Spirit dwelling in the addressees and received from God, and the
  denial of self-ownership. Feminine Syriac agreement does not supply a
  female identity or a new actor. Both body and spirit remain explicit
  in the final exhortation, followed by their belonging to God. The
  copula in verse 20 is retained with “Therefore be glorifying,” rather
  than swallowed into the preceding bought-with-a-price clause.

All 20 verses and 293 source words received source-to-English and running
English review. The three גיר, six דין and two הכיל occurrences were
checked individually against the final English clauses.

Source checks for 1 Corinthians 7:

- Verses 1-4 retain the written question, approaching a woman, and the
  reciprocal wife/husband obligations. Each spouse's body and the other
  spouse's authority remain distinct; no extra predicate is invented
  from a repeated copular pronoun.
- Verses 5-9 preserve mutual consent, the time devoted to fasting and
  prayer, returning together, and Satan's temptation because of bodily
  desire. The statement to weak people is distinguished from a command.
  Verse 7 explicitly retains purity, every person's gift from God, and
  the different gifts; enduring, marrying and burning with desire remain
  separate predicates.
- Verses 10-14 keep the speaker's distinction between his own instruction
  and his Lord's, separation, remaining without a husband, reconciliation,
  and not dismissing a spouse. The unbelieving husband and wife clauses
  remain reciprocal. Sanctified spouses and clean/unclean children use
  their actual separate source terms; their children are plural and the
  possessors are not silently changed to the addressees.
- Verses 15-20 retain the unbeliever's departure, brother or sister not
  being enslaved in such cases, God's calling to peace, and both spouses'
  questions about saving the other. Each person's distribution/calling
  and walking, the orders in all ecclesias, circumcision/uncircumcision,
  God's commands and remaining in one's calling remain explicit.
- Verse 21 follows the source's “choose for yourself to serve,” rather
  than importing Murdock's supplied preference for freedom over service.
  The choice verb plus ethical dative and a following ד clause has a
  directly comparable construction under the Syriac preference sense in
  [CAL's gby entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=gby+V),
  citing Ps.-Clementine Homilies 12.302:4 about choosing to remain a widow.
  This is a grammatical judgment from the actual source construction.
  [Lamsa's Syriac translation, verse 21](https://biblehub.com/lamsa/1_corinthians/7.htm)
  independently reads the service choice. The distinction from Murdock
  is recorded rather than hidden in a broad linked span.
- Verses 22-27 preserve God's freedman (the explicit owner is God),
  Messiah's servant, purchase with a price, and not becoming human
  servants. They also retain remaining with God, no command concerning
  virginity versus counsel, received mercy and faithfulness, the present
  necessity, and both bound/free-wife instructions.
- Verses 28-31 preserve both marrying without sin and bodily trouble,
  Paul's sparing the addressees, the shortened time, and every wives,
  weeping, rejoicing and buying comparison. Using this world is qualified
  by proper use; its passing fashion has its own causal clause.
- Verses 32-35 retain the unmarried/married and woman/maiden contrasts,
  their Lord/spouse/world concerns, holiness in both body and spirit,
  the addressees' benefit, no snare, and devotion to their Lord without
  worldly distraction. Qualifiers are not dropped into generic advice.
- Verses 36-38 retain the maiden's age, not having given her to a husband,
  the decision to give or keep her, firm judgment, absence of compulsion,
  and authority over one's will. “Daughter” is not inserted merely from
  Murdock's bracketed supplement. The good/better contrast keeps both
  repeated references to the man's maiden and their giving predicates.
- Verses 39-40 preserve the law-bound wife while her husband lives,
  his sleeping in death, her freedom and choice only in our Lord, and
  the final opinion about remaining so. The final Spirit belongs to God
  and is in Paul; it is not an extra owner or independent copular actor.

All 40 verses and 619 source words received source-to-English and running
English review. The eight גיר, nineteen דין and two הכיל occurrences
were checked individually against the final English clauses.

Source checks for 1 Corinthians 8:

- Verses 1-2 retain knowledge in all of us, knowledge puffing up and love
  building up, the person's claim to know, not yet knowing anything,
  and the standard of knowing as one ought. Postpositive discourse
  particles are grouped with their actual clauses rather than making
  “but thinks” mean merely thinking.
- Verse 3 retains the passive “is known by him” for the person who loves
  God. It is not changed to an active knowing of God. Verse 4 retains
  “an idol is nothing in the world”; reversing this into “nothing is an
  idol” would reverse the English subject and predicate. The food from
  sacrifices and the one-God exception are both explicit.
- Verses 5-6 preserve called gods, both heaven and earth, many gods and
  many lords, and the separate one God/Father and one Lord/Jesus Messiah
  clauses. The plural vocalized lords form is read in context rather
  than mechanically adopting the lemma's “dominion” gloss. All things
  from the Father, we in him, all things through the Lord and we also
  through him remain distinct; repeated quantities are not discarded.
- Verse 7 distinguishes knowledge not being in everyone from the next
  existential “For there are” clause. It retains conscience, until now,
  eating as a sacrifice to idols, weakness and passive defilement. The
  conscience noun is singular with plural possessors.
- Verse 8 preserves both eating conditions and their separate negative
  outcomes. The first negative governs abounding, while the second
  governs decreasing; neither is swallowed into the condition of not
  eating. Food remains the subject that does not bring us near to God.
- Verses 9-10 retain the addressees' authority becoming a stumbling block,
  the observer seeing one addressee who has knowledge reclining in the
  idol-house, and the observer's conscience being strengthened to eat.
  The weak-person clause and conscience predicate keep their distinct
  grammatical roles; the proleptic object marker adds no second person.
- Verses 11-12 keep the weak person perishing through the addressee's own
  knowledge, Messiah dying for that person, and sin against brothers and
  their weak consciences as sin against Messiah. An ethical dative does
  not introduce another cause or recipient; the explicit ב prefix on
  knowledge supplies the by/through relation. The plural weak consciences
  and possessors remain explicit.
- Verse 13 keeps food causing the brother to stumble, the speaker's
  future first-person refusal ever to eat flesh, and the final purpose
  of not causing the brother to stumble. The causative predicate is not
  replaced with a different actor or an omitted object.

All 13 verses and 190 source words received source-to-English and running
English review. The four גיר, six דין and one הכיל occurrences were
checked individually against the final English clauses.

Source checks for 1 Corinthians 9:

- Verses 1-2 retain every rhetorical negative, the free man/apostle
  distinction, seeing Jesus Messiah our Lord, the addressees as Paul's
  work in his Lord, others versus the addressees, and the seal of his
  apostleship. Distinct our/my possessors remain explicit.
- Verse 3 groups מפק ברוחי as the compound “my defense.” It does not add
  a separate “in my spirit” claim. The whole expression is independently
  attested as apology/defense in
  [CAL's compound entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=mpq%40brwx%29+N).
  The following judging predicate still has its actual object “me.”
- Verses 4-9 keep eating and drinking authority, a sister as wife,
  the other apostles, our Lord's brothers and Cephas, Paul and Barnabas,
  and the right not to work. Warfare/own expense, vineyard/fruit and
  shepherd/flock/milk remain separate examples. Human speech and the
  law's speech remain distinct; Moses, the threshing ox and God's care
  for oxen retain their actual source words and roles.
- Verse 10 has a first-person plural suffix in both “for our sake”
  phrases; Murdock's first “whose sake” is not substituted for it. The
  explanatory גיר is kept causal, separate from the sake phrase. Hope
  qualifies ploughing and threshing, with the produce noun retained.
- Verses 11-12 retain spiritual things sown and bodily things reaped,
  among/from the addressees, others' authority versus the apostles'
  greater claim, actual nonuse of this authority, enduring everything,
  and not hindering Messiah's Gospel in anything. The Spirit/body nouns
  are not dropped inside verb spans.
- Verses 13-14 retain both temple references, altar service and sharing,
  our Lord's command and both occurrences of his Gospel. The initial
  temple and Gospel nouns are not swallowed into generic “those” spans.
- Verses 15-18 keep nonuse, the purpose for writing, actually dying,
  nobody voiding Paul's boasting, both causal necessity clauses, and
  woe if he does not preach. Willing versus unwilling work and reward
  remain distinct. רבּת ביתותא is a stewardship compound, not a separate
  “great” modifier; being entrusted remains explicit. The reward answer
  retains cost-free proclamation and the given Gospel authority not used.
- Verses 19-23 preserve freedom from all, servitude to each, gaining
  many, and every repeated Jew/law/without-law/weak clause. God's law
  relation and Messiah's law have their actual owners. The final gain
  clause in verse 21 retains “also”; verse 22 aims at saving everyone,
  rather than replacing the source with familiar “some” wording. Verse
  23 is a purpose of becoming a Gospel participant, not an assertion
  that existing participation is the cause.
- Verses 24-25 retain all runners versus the one receiving victory,
  running so as to attain, the competitor's restraint, and perishing
  versus imperishable crowns. The explicit first-person plural חנן is
  “we,” not the dictionary's singular “I” gloss. The restraint phrase
  retains the source's mind/intentional-control concept.
- Verses 26-27 retain running toward no unknown object, fighting air,
  subduing the body and bringing it into servitude, preaching to others,
  and Paul's own possible rejection. The analytic participle/pronoun
  expresses one running actor; the impersonal לא הוא negative is not
  turned into a new first-person clause. The final rejection predicate
  absorbs its ethical dative rather than adding another recipient.

All 27 verses and 422 source words received source-to-English and running
English review. The seven גיר, six דין and two הכיל occurrences were
checked individually against the final English clauses.

Source checks for 1 Corinthians 10:

- Verses 1-4 keep every “all,” passing through the sea, baptism through
  Moses in cloud and sea, and the same spiritual food and drink. The
  second drinking clause has its own causal גיר. The accompanying rock
  and Messiah form one identification; copular pronouns do not create
  another actor.
- Verse 5 says God was not pleased with many of the people; it does not
  say their number or multitude displeased him. Verses 6-10 retain the
  example for us, not desiring evil, the quoted eating/drinking/rising,
  twenty-three thousand in one day, tempting Messiah, serpents destroying
  them and the destroyer. The exhortation in verse 10 addresses “you,”
  whereas the preceding “let us” clauses retain first-person plurality.
- Verse 11 keeps all the events happening to them as an example for us,
  and writing for our instruction. The final source has one singular
  end of plural ages; the suffix anticipates the explicit ages genitive,
  rather than adding “their” as a separate owner.
- Verses 12-13 preserve standing versus falling, the human trial and
  faithful God, no testing beyond the addressees' ability, the second
  explicit trial noun, an exit and ability to endure. The initial מכיל
  is logical “therefore” here. The exit is not detached from its actual
  trial or its endurance purpose.
- Verses 14-16 retain fleeing idolatry, speaking to wise people and their
  judging, the thanksgiving cup and blessed/broken elements, and both
  rhetorical communion questions. Blood and body each belong to Messiah
  once; no extra owner is made from their proleptic suffixes.
- Verse 17 predicates that the bread is one, rather than leaving the
  dangling phrase “one bread is.” Both all-of-us clauses, one body and
  the repeated one bread remain explicit, with causal “for we all.”
  Verse 18 makes the sacrifice-eaters the subject of the altar question,
  without inserting a second “they” actor.
- Verses 19-22 retain both idol/sacrifice questions and their explicit
  “No,” both actual sacrifice verbs, demons rather than God as recipients,
  the refusal of demonic partnership, both cup/table prohibitions and
  our Lord's ownership. Provoking jealousy and being stronger than the
  Lord are rhetorical questions; no dependent “lest” clause or normative
  demand to be stronger replaces them.
- Verses 23-28 keep both lawful statements and their distinct profit and
  building qualifications, own/fellow interests, the meat-market, and
  eating without inquiry about conscience. The earth belongs to the Lord
  in the earth's fullness. The invitation and willingness conditions,
  everything set before the addressees, the identified sacrificial meat,
  and not eating for the informant/conscience remain explicit.
- Verses 29-30 clarify the informant's conscience rather than the
  addressees' own, then retain the complete causal liberty question.
  The conscience noun is singular and its “others” possessor is plural.
  Paul partakes by grace, is reproached and gives thanks; repeated
  analytic pronouns do not add speakers or predicates.
- Verses 31-33 retain eating, drinking or doing anything as parallel
  conditions, followed by the command to do everything for God's glory.
  The exhortative copula/participle is not changed into a claim that the
  addressees already do everything for that glory. Jews, Gentiles and
  God's ecclesia remain distinct recipients of the no-offense command.
  Paul seeks not his own benefit but that of many, that they may live.

All 33 verses and 420 source words received source-to-English and running
English review. The five גיר, nine דין, two הכיל and one מכיל occurrences
were checked individually against the final English clauses.

Source checks for 1 Corinthians 11:

- Verses 1-3 preserve imitation of Paul as he imitates Messiah, both
  remembering Paul and holding the delivered precepts, and all three
  head relationships. The suffixes anticipating explicit possessors do
  not introduce another owner. Man and woman remain the source terms;
  the translation does not resolve each head reference into an
  unexpressed husband or another named person.
- Verses 4-7 retain praying or prophesying, covered versus uncovered
  heads, dishonoring one's head, and the distinct cutting and shaving
  expressions. The causal explanation remains explicit. Man is called
  God's likeness and glory, retaining both nouns, while woman is called
  man's glory.
- Verses 8-12 distinguish origin from man or woman, creation for the
  other, and the mutual not-without statements in our Lord. Verse 10
  says authority on her head because of angels; Murdock's bracketed
  supplement "mark" is not an additional source word. Verse 12 retains
  woman from man, man through woman, and everything from God.
- Verses 13-16 retain judgment among the addressees, nature teaching,
  the long-hair comparisons, and the woman's hair given to her as a
  covering. The ecclesia of God in verse 16 is singular in this source.
- Verses 17-19 retain the refusal to commend, progress versus descent
  into deterioration, divisions heard of among those assembling, and
  believing the report in part. The future necessity of contentions
  and the approved people becoming known remain distinct from an
  assertion that the contentions already exist.
- Verse 20 has the day of our Lord and both eating and drinking. It is
  not replaced with the familiar Greek wording about the Lord's supper,
  and the day is not identified with a particular weekday. Verses 21-22
  retain each person's supper beforehand, the hungry and drunken people,
  houses for eating and drinking, despising God's ecclesia as the object,
  and shaming those who have nothing. "In this" qualifies the final
  refusal to praise.
- Verses 23-25 distinguish Paul's receiving and handing on from Jesus
  being handed over. They retain taking bread, blessing, breaking,
  saying, taking and eating, the body broken for the addressees' sake,
  and doing thus in remembrance of Jesus. The cup is given after they
  have eaten supper and is the new covenant in his blood. Both commands
  to do thus are exhortations, not reports of past or future behavior.
- Verse 26 uses the source's commemorating or remembering of our Lord's
  death, rather than substituting the familiar "proclaim." Bread, cup,
  eating, drinking and the limit until his coming remain explicit.
  Verse 27 preserves the source order blood and then body, each with
  the Lord as owner, and the unworthy participant's guilt.
- Verses 28-30 retain examining oneself, then eating and drinking,
  both actual eating/drinking predicates in verse 29, condemnation upon
  oneself and not discerning the Lord's body. Both "many" quantities
  and the distinct weak, sick and sleeping descriptions remain.
- Verses 31-34 keep the first-person plural conditional about judging
  ourselves, being judged by our Lord, passive discipline, and the
  purpose of not being condemned with the world. The gathering command
  is to wait for one another. A hungry person is to eat at home so that
  the assembly is not for condemnation; directions about the rest await
  Paul's coming.

All 34 verses and 471 source words received source-to-English and running
English review. The twelve גיר, eleven דין, two הכיל and one מכיל
occurrences were checked individually against the final English clauses.

Source checks for 1 Corinthians 12:

- Verses 1-3 preserve the desire for the addressees to know, their former
  pagan condition, voiceless idols and being led without discernment.
  The source's positive desire to know is not replaced with the familiar
  wording about not wanting ignorance. Speaking by God's Spirit cannot
  declare Jesus accursed, and confessing Jesus as Lord requires the Holy
  Spirit; both speech predicates and both negative qualifications remain.
- Verses 4-6 retain the distinct gifts, services and powers, with one
  Spirit, one Lord and one God respectively. God works all in everyone.
  Verses 7-10 keep each recipient, the manifestation and its benefit,
  words of wisdom and knowledge, faith, healing gifts, miracles, prophecy,
  discerning spirits, kinds of tongues and their interpretation. Repeated
  Spirit references and the local discourse particles remain explicit.
- Verse 11 makes the one Spirit the worker and distributor of all these
  things according to the Spirit's will. Grammatical feminine agreement
  is not turned into a new actor or a claim about human sex. Repeated
  datives and anticipatory suffixes do not create extra recipients.
- Verses 12-14 retain one body, many members, all the members, and the
  analogy with Messiah. The first-person plural חנן is "we," not a
  singular dictionary gloss. All of us were baptized by one Spirit into
  one body, with Jews/Gentiles and slave/free distinctions; all drank one
  Spirit. The final drinking verb is active in the supplied Syriac.
- Verses 15-16 keep the foot/hand and ear/eye contrasts and the quoted
  claims not to belong to the body. Those claims do not actually remove
  the member from the body. The final rhetorical questions expect "No";
  an English "is it not ... not of the body?" would suggest the opposite
  answer. The repeated "from it/from the body"
  constructions name one relation rather than an additional body.
- Verses 17-21 retain both counterfactuals about eye, hearing and smell,
  God's placing every member as he willed, the question where the body
  would be if all were one member, and many members versus one body.
  The eye/hand and head/feet statements keep their distinct speakers,
  recipients, singular/plural forms and inability to declare the other
  unnecessary.
- Verses 22-25 retain the necessity of members considered weak, greater
  honor and decoration for the less regarded members, and no need of
  honor for the honored members in us. God tempered the body and gave
  greater honor to the lesser member. The purpose is no division, with
  all members caring equally for one another.
- Verse 26 keeps both shared suffering and shared honor. Its final
  predicate says all the members are honored, rather than substituting
  the familiar "rejoice." Verse 27 retains the addressees as Messiah's
  body and members "in your place."
- Verses 28-30 keep God's ecclesia and every listed role or gift:
  apostles, prophets, teachers, workers of miracles, healing gifts,
  helpers, leaders and kinds of tongues. The repeated "after them"
  sequence and every rhetorical "all" question remain explicit; healing,
  speaking in tongues and interpreting are separate questions.
- Verse 31 begins with the source's explicit condition: if the
  addressees are zealous for greater gifts, Paul further shows them a
  superior way. It is not changed into the familiar imperative to desire
  gifts.

All 31 verses and 425 source words received source-to-English and running
English review. The seven גיר and eighteen דין occurrences were checked
individually against the final English clauses.

Source checks for 1 Corinthians 13:

- Verses 1-3 retain every human tongue and that of angels, prophecy,
  all mysteries, all knowledge, all faith and moving mountains. The
  conditions repeatedly specify love not being in the speaker. The
  results are becoming resounding brass or a sounding cymbal, being
  nothing, and gaining nothing respectively. The ethical dative in
  verse 1 does not introduce another recipient.
- Verse 3 retains feeding the poor with all the speaker possesses,
  handing over his own body, and its being burned. The burning wording
  is not replaced with a Greek variant about boasting.
- Verse 4's long-spirit expression is an idiom for patience, not another
  personal spirit. All three explicit love nouns remain, along with
  kindness and the distinct negatives of envy, boisterous or agitated
  behavior, and puffing up. Verse 5 continues love as subject and keeps
  shameful action, seeking its own, provocation and thinking evil as
  separate negatives; thinking evil is not replaced with the familiar
  phrase about keeping a record of wrongs.
- Verses 6-7 retain rejoicing in truth rather than in iniquity and four
  distinct predicates with their own "all": bearing, believing, hoping
  and enduring. Each quantity is linked with its actual predicate,
  rather than borrowing the next predicate's all-word.
- Verse 8 keeps love never failing, plural prophecies ceasing, tongues
  becoming silent and knowledge ceasing. Its postpositive גיר is causal
  "For," rather than a substituted "but" or an omitted conjunction.
- Verses 9-10 distinguish present partial knowledge and prophecy from
  the coming of completeness, when the partial ceases. The source's
  "little of much" wording expresses the limited present knowledge and
  prophecy; it is not an assertion that the speaker knows everything.
  Completeness remains an abstract noun without an added identification.
- Verse 11 preserves all three childhood comparisons and their distinct
  speaking, thinking and reasoning predicates, followed by becoming a
  man and putting away these childhood things. The repeated perfect BE
  forms are auxiliaries, not three extra "I was" clauses.
- Verse 12 keeps the mirror and the source's comparison or similitude
  term, rather than importing "darkly." Both now/then contrasts remain,
  as do face to face, limited current knowledge and future knowing.
  The final known verb is passive; it is not changed into another active
  "I know" or supplemented with an unexpressed "fully."
- Verse 13 retains causal "For," the explicit three that remain, faith,
  hope and love, and love as greatest of these. The grammatical pronouns
  do not add another person to the list.

All 13 verses and 185 source words received source-to-English and running
English review. The three גיר and five דין occurrences were checked
individually against the final English clauses.

Source checks for 1 Corinthians 14:

- Verses 1-5 keep pursuing love, gifts of the Spirit and the preference
  for prophecy. Verse 2 retains both causal clauses, speaking to God
  rather than humans, no one understanding the utterance and speaking
  a mystery in spirit. Self-building and building the ecclesia remain
  distinct. Verse 5 keeps all the addressees, the greater prophet,
  the exception for interpretation and the explicit interpretation/
  ecclesia-building condition.
- Verses 6-9 retain the profit question and its exception for revelation,
  knowledge, prophecy or teaching. The lifeless instruments emit sound;
  pipe, harp, distinction between notes, sung/played sound, trumpet and
  preparation for battle remain explicit. Verse 9 keeps the need for
  interpretation and the comparison to speaking to the air.
- Verses 10-13 retain many kinds of languages in the world, not one
  lacking significance, and both sides of the foreigner comparison.
  The first foreigner noun is not swallowed by the following recipient
  phrase. Ethical datives do not create another actor. The command is
  to seek excellence for building the ecclesia, and the tongue-speaker
  is to pray to interpret.
- Verses 14-17 distinguish the speaker's spirit praying from his
  understanding lacking fruit. Verse 15 has four first-person prayer
  and singing intentions, not commands addressed to someone else. The
  unlearned hearer cannot say Amen because he does not know what the
  addressee says; the final verb is not reassigned to "you." Verse 17
  keeps blessing well and the neighbor not being built up.
- Verses 18-20 retain thanks to God, speaking in tongues more than all
  the addressees, five intelligible words versus ten thousand, and the
  purpose of also teaching others. The maturity instructions remain
  commands: not children in thought, infants toward evil, mature in
  thought. They do not become descriptions of existing behavior.
- Verses 21-22 retain the law quotation, foreign speech, another tongue,
  this people and still not listening to the Lord. The anticipatory
  suffix names this people once. Tongues are a sign for unbelievers,
  while prophecies have the contrasting believer/unbeliever scope.
- Verses 23-25 retain the whole ecclesia, all the speakers and both
  visitor categories. The visitor is examined and rebuked by all; he
  is not made the examiner of everyone. His heart's hidden things are
  revealed, he falls on his face, worships God and acknowledges God
  among the addressees. No extra recipient is created by the ethical
  dative in the quoted madness statement.
- Verses 26-31 keep every listed contribution, all for building, two
  speakers or at most three, one by one and one interpreter. Without
  an interpreter the tongue-speaker is silent in the ecclesia and
  speaks privately to himself and God. Prophets number two or three,
  with the rest judging. Revelation to another who is sitting makes
  the first speaker silent. All may prophesy in turn so everyone may
  learn and everyone be comforted.
- Verse 32 has singular spirit of the prophets, subject to the prophets,
  with causal "For" rather than a prepositional "for the prophets."
  Verse 33 keeps God of peace rather than tumult and all the ecclesias
  of the saints.
- Verses 34-35 retain the addressees' women, silence, the prohibition
  on speaking, subjection and the law. The actual permission-recipient
  suffix remains plural. The home phrase modifies asking their husbands,
  and the explanation retains causal "For" with shameful speaking in
  the ecclesia; it is not absorbed into a prepositional "for women."
- Verses 36-40 keep God's word coming forth from the addressees or
  reaching only them as questions, the conditional claims to prophetic
  or spiritual status, and recognition of the Lord's commands. Verse 38
  says if someone does not know, let him not know; it is not replaced
  with a different passive reading. The final commands retain zeal for
  prophecy, not forbidding tongues, and everything done decently and
  in order.

All 40 verses and 565 source words received source-to-English and running
English review. The eleven גיר, eleven דין and four הכיל occurrences
were checked individually against the final English clauses.

Source checks for 1 Corinthians 15:

- Verses 1-4 keep the proclaimed, received and sustaining gospel, the
  explicit remembrance condition and believing in vain. Messiah died
  for our sins, was buried and rose on the third day; both written-text
  references remain. The "on account of" idiom does not introduce faces
  as another object.
- Verses 5-9 retain Cephas, the twelve, more than five hundred brothers
  at once, many still living and some asleep, James and all the apostles.
  Paul is last of all, like an untimely birth, least among the apostles
  and unworthy to be called one because he persecuted God's ecclesia.
- Verse 10 retains God's grace, both later "his grace" possessors,
  grace in Paul versus with Paul, labor more than all the others and
  not Paul but grace as the explanation. Verse 11 retains I/they,
  our proclamation and the addressees' belief.
- Verses 12-19 preserve every resurrection condition and its resulting
  negation. Our preaching and the addressees' faith each have an actual
  empty predicate in verse 14. The false witnesses testify about God's
  raising Messiah; the source does not have the additional Greek clause
  sometimes supplied at the end of verse 15. Remaining in their sins,
  the Messiah-sleepers perishing and hope in this life only remain
  distinct consequences, with all the original persons and possessors.
  For כבר in verse 18, inferential "indeed" follows Murdock's "doubtless"
  within the preceding hypothetical argument. The [CAL entry for kbr](https://cal.huc.edu/oneentry.php?cits=all&lemma=kbr+X)
  records temporal and modal senses; the inferential reading is an
  interpretation of the context.
- Verses 20-23 keep risen Messiah as firstfruits, death through a human
  versus resurrection through a human, all humans in Adam and all in
  Messiah, each one's order and Messiah's own people at his coming.
- Verses 24-28 retain kingdom delivery to God the Father, every ruler,
  every authority and all powers. Verse 24's Pael perfect is active:
  he has abolished them. Verse 25 also has an active placing of all his
  enemies under his feet. Death is the last enemy abolished. The one
  subjecting all is distinguished from the one receiving that subjection,
  and the Son's own subjection leads to God all in all; agent and recipient
  are not reversed.
- Verses 29-32 retain both questions about baptism for the dead without
  adding an explanation of the practice, the failure-to-rise condition,
  our danger every hour, boasting in our Lord Jesus Messiah and dying
  daily. Verse 32 says Paul was cast to beasts at Ephesus, rather than
  substituting the familiar "fought with beasts." The final saying keeps
  eating, drinking and causal "for tomorrow we die."
- Verse 33 has evil stories or talk corrupting good minds, rather than
  replacing those nouns with familiar Greek company and morals. Verse 34
  explicitly has knowledge of God, despite the local Murdock reference's
  "love of God." Arousing the addressees' hearts righteously, not sinning,
  some people's lack of knowledge and speaking to their shame remain.
- Verses 35-38 keep both questions about the dead's rising and their
  body, seed not living unless it dies, and not sowing the future body.
  The naked grain examples explicitly include wheat, barley and other
  seeds; barley is not omitted to match familiar Greek wording. God gives
  each its own natural body as he wills.
- Verses 39-41 use the source's body noun for human, beast, bird and fish,
  then heavenly and earthly bodies with different glory. Sun, moon and
  stars retain their separate glory references and the comparison of
  one star with another.
- Verses 42-46 retain the sowing/rising contrasts of corruption versus
  incorruption, dishonor versus glory, weakness versus power and natural
  versus spiritual body. The first Adam becomes a living soul; the last
  Adam is a life-giving spirit. "Last" is not replaced with Murdock's
  "second." The natural comes first, then the spiritual.
- Verses 47-50 retain the first human of dust from earth, the second
  human explicitly the Lord from heaven, the corresponding earthly and
  heavenly people, and both likeness-wearing clauses. Verse 50 says
  kingdom of heaven, not the familiar Greek kingdom of God. Flesh and
  blood cannot inherit it, and corruption does not inherit incorruption.
- Verses 51-54 keep not all sleeping but all changing, the sudden
  eye-twinkle, last trumpet, incorruptible rising and our own changing.
  Both putting-on clauses, both corruptible/mortal conditions and the
  written word coming to pass remain: death is swallowed in victory.
- Verses 55-58 distinguish death's sting from the grave's victory,
  identify the sting with sin and sin's power with the law, and retain
  God's victory given to us through our Lord Jesus Messiah. The final
  steadfastness, not being shaken and always abounding are commands,
  with the assurance that the addressees' labor is not vain in the Lord.

All 58 verses and 703 source words received source-to-English and running
English review. The eleven גיר, fourteen דין, one הכיל and one מכיל
occurrences were checked individually against the final English clauses.

Source checks for 1 Corinthians 16:

- Verses 1-2 retain the collection for the saints, the Galatian ecclesias
  and the command to do likewise. Each first day of the week, each of
  the addressees is to put aside and keep at his home what comes into
  his hands. Both putting and keeping remain, with the purpose of no
  collections when Paul comes.
- Verses 3-4 distinguish the chosen envoys from the gift they carry.
  The anticipatory object suffix does not make the envoys a second set
  of recipients. Paul sends them with a letter to carry the addressees'
  gift to Jerusalem; if the matter warrants his going too, they go with
  him.
- Verses 5-7 retain passing from Macedonia and passing through Macedonia
  as separate clauses, the possible stay or wintering, accompanying Paul
  where he goes, and the hope of spending time together. The permitter
  is explicitly "my Lord," with a singular first-person possessor.
  Both causal clauses in verse 7 remain explicit.
- Verses 8-11 distinguish the discourse particle about remaining in
  Ephesus until Pentecost from the causal explanation of the opened
  great door, its activities and many opponents. Timothy is to be
  without fear, does the Lord's work as Paul does, and is not to be
  despised. The peaceful escort, coming to Paul and Paul's awaiting
  him with the brothers remain distinct.
- Verse 12 keeps Apollos, the earnest request, coming with the brothers,
  and the later opportunity. The modal "perhaps" retains כבר and leaves
  the willingness noun without an added possessor. The [published parallel
  translations](https://www.dukhrana.com/peshitta/analyze_verse.php?font=Estrangelo+Edessa&size=125%25&source=&verse=1Corinthians+16%3A12)
  also show Lamsa's modal reading; the categorical Murdock rendering is
  not treated as the only possible interpretation of this clause.
- Verses 13-16 keep all four vigilance/faith/courage/strength commands,
  all the addressees' affairs in love, and the request about Stephanas's
  household. The explanatory firstfruits/service parenthesis connects
  with the request that the addressees also obey such people and everyone
  working with us and helping. It is not a separate past-tense report.
- Verses 17-18 retain all three named arrivals, what the addressees
  lacked toward Paul, and what those visitors supplied. They refreshed
  Paul's own spirit and the addressees'; the conclusion is a command
  to acknowledge such people. The perfect BE/participle construction
  is not turned into an unrelated past statement.
- Verses 19-20 preserve both greetings, all the Asian ecclesias, much
  greeting in our Lord, Aquila and Priscilla, and the ecclesia in their
  house. Verse 20's brother noun has a first-person plural suffix:
  "our brothers." The second greeting is a reciprocal command with a
  holy kiss; its verb is not linked into the preceding greeting.
- Verse 21 identifies Paul's own handwriting with one named owner.
  Verse 22 retains our Lord Jesus Messiah, the negative love condition
  and the curse. Its final vocalized verb is perfect Peal, rendered
  "Our Lord has come." The [published SEDRA analysis](https://dukhrana.com/peshitta/analyze_verse.php?font=Estrangelo+Edessa&size=150&verse=1Corinthians+16%3A22)
  confirms that form and gives a matching Lamsa rendering, alongside
  Murdock's interpretive "cometh."
- Verses 23-24 retain grace from our Lord Jesus Messiah and Paul's love
  with all the addressees in Messiah Jesus. The supplied Syriac ends
  at Jesus; Murdock's final Amen has no source word here and is not added.

All 24 verses and 278 source words received source-to-English and running
English review. The seven גיר, nine דין and two הכיל occurrences were
checked individually against the final English clauses.

Source checks for 2 Corinthians 1:

- Verses 1-3 retain Paul and Timothy, Jesus Messiah, God's will, God's
  ecclesia in Corinth, all the saints and all Achaia. Grace and peace
  come from God our Father and our Lord Jesus Messiah. Father of our
  Lord, Father of mercies and God of all comfort remain distinct titles;
  anticipatory possessors do not create another father or owner.
- Verse 4 repeats the same possessively pointed affliction form in both
  places. This rendering retains "our" in both, following the chosen
  annotated source. The [published SEDRA analysis](https://dukhrana.com/peshitta/analyze_verse.php?font=Estrangelo+Edessa&size=125%25&source=&verse=2Corinthians+1%3A4)
  likewise assigns a first-person plural suffix to both occurrences,
  although its displayed English parallels make the second generic.
  God's comforting us, our ability also to comfort others, the same
  comfort and being comforted by God remain separate relations.
- Verses 5-7 retain Messiah's sufferings abounding in us and our comfort
  abounding through Messiah. Both actual afflicted predicates in verse 6
  remain, with the addressees' comfort and life, being comforted and
  diligence in them to endure the sufferings that we also suffer. The
  final object suffix does not introduce another beneficiary. Hope about
  the addressees is steadfast, with sharing suffering and comfort linked.
- Verses 8-10 retain the Asian affliction, its excess beyond our strength
  and our lives nearing destruction. The sentence of death is passed by
  us upon ourselves; it is not substituted with a passive Greek wording.
  The purpose is confidence in God who raises the dead rather than in
  ourselves. The rescue expression uses plural death forms, with rescue
  of us and hope of renewed rescue kept explicit.
- Verse 11 keeps the addressees' prayers on our behalf, his gift toward
  us, favor done on account of many, and many thanking him on our behalf.
  The two actual "many" quantities are not collapsed into one group.
- Verse 12 keeps our boast and the testimony of our conscience, simplicity,
  purity, God's grace, conduct in the world, rejection of fleshly wisdom
  and especially conduct toward the addressees. Verse 13 retains writing
  no other things, what they know and acknowledge, and the confidence
  that they will acknowledge to the end.
- Verses 14-16 preserve limited acknowledgment, we as the addressees'
  boast and they as ours, the day of our Lord Jesus Messiah, and the
  former intention to visit. The purpose is receiving grace doubly.
  Passing through them into Macedonia, returning from Macedonia and
  their accompanying Paul to Judea remain separate route relations.
- Verse 17 retains both rhetorical questions about the plan, rashness
  and fleshly purposes, with the modal clause and repeated "Yes, yes"
  and "No, no." The source locative is "in them," not "in me."
- Verse 18 calls God faithful; it does not replace that predicate with
  Murdock's interpretive witness wording. Verses 19-20 retain God's Son
  Jesus Messiah, proclamation through us and the named Paul, Silvanus
  and Timothy, not Yes and No but Yes in him, all God's promises in
  Messiah, and our Amen through him for God's glory.
- Verses 21-22 make God the establisher, anointer and sealer of us with
  the addressees in Messiah, giving the pledge of his Spirit in our
  hearts. The Spirit's possessor and the hearts' possessors remain
  different and explicit. No new actor is made from the repeated
  identifying pronouns.
- Verses 23-24 retain Paul's calling God as witness upon his own soul,
  sparing the addressees as the reason for not coming to Corinth, the
  denial of lordship over their faith and helping their joy instead.
  The final causal clause says they stand by faith.

All 24 verses and 356 source words received source-to-English and running
English review. The six גיר, five דין and one הכיל occurrences were
checked individually against the final English clauses.

Source checks for 2 Corinthians 2:

- Verses 1-2 retain Paul's decision within himself, not coming again
  in sadness, grieving the addressees and the question of who will
  gladden him except the one he made sad. The repeated first-person
  forms identify one actor rather than introducing another person.
- Verse 3 makes the others the subjects of "to gladden me": the actual
  verb has a third-person plural subject and first-person singular
  object. This differs from Murdock's "whom I ought to make joyful."
  Both occurrences of "all of you" remain, along with Paul's joy and
  their joy. Verse 4 retains both sources of distress, these things
  written with many tears, and the purposes of not causing grief but
  making known the exceeding love Paul has for the addressees.
- Verse 5 keeps Paul as the person grieved in the second grief clause,
  not as its cause. "Alone" expresses the contextual contrast with
  "all of you," as in Murdock's supplied "only"; it is not a separate
  Syriac word. The small degree and the actual noun "word" in the
  final clause remain explicit.
- Verse 6 treats the masculine demonstrative as the recipient of the
  rebuke, not as a modifier of the feminine rebuke noun. The sufficiency
  expression is rendered "sufficient" without requiring a separate
  temporal "already." Verses 7-8 preserve therefore, the contrary
  course of forgiving and comforting him, the passive swallowing of
  such a person in excessive grief, and confirmation to him of the
  addressees' love.
- Verse 9 retains both the causal particle and "for this reason," with
  also, writing, knowing by testing, and obedience in everything.
  Verse 10 keeps the shared forgiveness predicate, all three actual
  past "I forgave" forms, their relative objects, the causal particle,
  the addressees' sake and the presence of Messiah. It does not add a
  conditional "if" or turn repeated identifying pronouns into another
  actor. Verse 11 retains Satan's possible overreaching of us and the
  positive statement that we know his devices.
- Verses 12-13 retain Troas, the Gospel of Messiah, the opened door in
  the Lord, Paul's lack of rest in his spirit, not finding Titus his
  brother, taking leave of them and departure into Macedonia. Titus
  is not made a companion on that departure.
- Verse 14 retains the noun for sight or spectacle in "makes a
  triumphal display." "Triumphal" is a contextual interpretation:
  the [published Syriac analysis and English parallels](https://www.dukhrana.com/peshitta/analyze_verse.php?font=Estrangelo+Edessa&size=125%25&source=&verse=2Corinthians+2%3A14)
  show the spectacle noun and Etheridge's and Murdock's triumph
  renderings. No captive or victor role is added. The separate "for
  us" and "through us," Messiah, the odor of knowledge of him and
  every place remain explicit.
- Verses 15-16 retain "we," the sweet odor through Messiah to God,
  those living and those perishing, and the two distinct expressions
  "death to death" and "life to life." All four source death and life
  nouns remain, followed by the question of adequacy for these things.
- Verse 17 retains the negative comparison with others, diluting the
  plural "words of God," truth, being from God, speaking before God
  in Messiah and the actual speaking predicate. All three references
  to God remain; a possessive suffix anticipating the named owner
  does not introduce another owner.

All 17 verses and 230 source words received source-to-English and running
English review. The six גיר, seven דין and one מכיל occurrences were
checked individually against the final English clauses.

Source checks for 2 Corinthians 3:

- Verse 1 retains both beginning again and the from-the-beginning
  expression, showing the addressees what we are, and the alternatives
  about letters of recommendation. The letters are the subjects of
  being written to the addressees about us; the addressees are the
  subjects of the final writing and recommending. The interrogative
  particle is understood in the question, not as a purpose "lest."
- "Recommendation/recommend" in verse 1 is a contextual lexical choice
  following the supplied Syriac-derived Murdock reference. SEDRA's brief
  glosses list command/precept senses, but the broader
  [CAL verb entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=pqd+V)
  includes a Syriac Pael commendation sense, and the
  [Syriac dictionary entry](https://syriacdictionary.net/mobile/index.cgi?hidden=Search&word=%DC%A6%DC%A9%DC%B0%DC%95)
  includes recommendation for the noun. The abbreviated gloss list
  does not require replacing this contextual sense with literal commands.
- Verse 2 keeps "our own letter" as one emphasized owner, the
  addressees as the letter, writing in our heart and being known and
  read by everyone. Verse 3 explicitly says "you know," rather than
  substituting a familiar "you show." The letter belongs to Messiah
  and is ministered by us. Both writing contrasts retain their
  negations, ink, the Spirit of the living God, stone tablets and
  tablets of the heart of flesh.
- Verses 4-5 retain confidence in Messiah toward God, the denial of
  self-sufficiency to think anything as from ourselves, and the distinct
  power or strength that comes from God. Verse 6 keeps God's making
  us fit to be ministers of a new covenant. Writing and Spirit remain
  distinct, with the causal writing-kills clause and the contrasting
  Spirit-gives-life clause. The two writing nouns are not made three.
- Verses 7-9 preserve the ministry of death engraved in writing on
  stone, its glory, Israel's inability to look at Moses' face, and
  the glory of his face that ceased. The final relative refers to
  the glory, not Moses or the ministry. The Spirit's ministry retains
  the question's negation and future glory. Condemnation ministry
  having glory and righteousness ministry abounding in glory remain
  separate comparisons.
- Verses 10-11 distinguish the formerly glorified thing from its
  comparative lack of glory beside this excellent glory. "As though"
  and "not even" remain explicit. What ceased was in glory; what
  remains will be in glory. Neither comparison reverses which thing
  was glorified or which thing remains.
- Verse 12 retains both because and therefore, this hope possessed by
  us, exceeding openness and the actual conduct predicate. The open-eye
  construction is understood idiomatically rather than as a claim
  about physical eyesight. Verse 13 keeps Moses putting a physical
  veil on his face and the purpose that Israel's children not behold
  the end of what is being annulled.
- Verse 14 says they were blinded in their minds, rather than importing
  a different hardening predicate. The old covenant is read, the same
  veil remains upon them, and it is not revealed that in Messiah it
  is annulled. Verse 15 retains reading Moses and a veil laid upon
  their hearts. This is physical veil language, not atonement terminology.
- Verse 16 explicitly names a person from among them turning toward
  the Lord, with the veil taken from him. Verse 17 retains the Lord
  as the Spirit, the Spirit of the Lord and freedom where that Spirit
  is. The anticipating possessive does not create another owner.
- Verse 18 keeps all of us, uncovered faces, active beholding of the
  Lord's glory as in a mirror and passive transformation into the
  same likeness. The anticipating target suffix does not add an owner
  of the likeness. Both separate glory nouns in "from glory to glory"
  and the final relation to the Lord, the Spirit, remain explicit.

All 18 verses and 248 source words received source-to-English and running
English review. The six גיר, six דין and two הכיל occurrences were
checked individually against the final English clauses.

Source checks for 2 Corinthians 4:

- Verses 1-2 retain the service received, the mercies upon us, rejection
  of the hidden things of shame, and both negative clauses about walking
  in craftiness and dealing deceitfully with God's word. Manifestation
  of truth, showing ourselves, all human consciences and being before
  God remain distinct. Anticipating suffixes do not add another owner.
- Verse 3 makes the Gospel's being hidden the condition, with its being
  hidden to those perishing the result. Both hidden predicates remain.
  Verse 4 keeps the God-of-this-world expression, his blinding their
  minds, and the separate causal unbelief and negative dawning clauses.
  The causal "because they do not believe" follows the source's
  `al d-` construction, rather than Murdock's purpose wording; compare
  the conjunction discussion in [Classical Syriac Coursebook, lesson 14,
  page 126](https://refubium.fu-berlin.de/bitstream/fub188/15883/1/ClassicalxSyriacxCoursexBook-HY-NEU.pdf).
  Light, Gospel, Messiah's glory and his being God's likeness remain
  separate nouns and relations.
- Verses 5-6 retain the denial of preaching ourselves, Messiah Jesus
  our Lord, ourselves as the addressees' servants for Jesus' sake,
  God's saying that light arise from darkness, and his shining in
  our hearts. The purpose is explicitly that we may be illuminated
  with knowledge of God's glory on the face of Jesus Messiah; it is
  not replaced by a generic illumination noun.
- Verse 7 retains this treasure in a singular earthen vessel and the
  greatness of power coming from God rather than us. Verses 8-9 keep
  each separate condition and contrasting negation: constrained but
  not suffocated, beaten but not guilty, persecuted but not forsaken,
  and cast down but not perishing. The beaten/not-guilty pair is not
  replaced with a familiar perplexity/despair pair.
- Verses 10-11 distinguish both plural "our bodies" occurrences in
  verse 10 from the singular "this body of ours that dies" in verse 11.
  Bearing Jesus' death and manifestation of Jesus' life are different
  predicates. Manifestation remains passive and prospective; we are
  not made its active subject. The first purpose and the second
  conditional, being handed over alive to death, Jesus' sake, thus
  and both also particles remain explicit.
- Verse 12 has "now," without an additional therefore. Death is at
  work in us and life in the addressees. The contextual active sense
  is consistent with Murdock and the broader Ethpaal usage in the
  [CAL entry for hpt](https://cal.huc.edu/oneentry.php?cits=all&lemma=xp%2B+V),
  rather than mechanically substituting a Pael exhortation gloss.
- Verses 13-14 retain the same spirit of faith as one possession,
  therefore, the written quotation's "I believed" and "I also spoke,"
  our believing and also speaking, and both because-of-this clauses.
  The one who raised our Lord Jesus will also raise us through Jesus
  and bring us near with the addressees to himself. None of us, Jesus,
  the addressees or himself is collapsed into another participant.
- Verse 15 keeps causal "For" separately from the addressees' sake,
  everything, grace abounding through many, and the purpose that
  thanksgiving increase to God's glory. Verse 16 retains the denial
  of weariness, "for even if" the outer person is corrupted, the
  contrasting renewal of the inner person, and day by day. The inner
  person is the contextual counterpart of the explicit outer person,
  not an additional individual.
- Verse 17 makes the affliction of this time the subject preparing
  great glory for us, with very small, light, without end and for ever
  and ever retained. The initial possessive anticipates this time;
  it does not mean my affliction. No weight noun or double comparative
  is supplied. The object, its modifiers and the late predicate at
  positions 8-15 form one necessary larger dependency so that English
  keeps affliction as preparer and glory as its object.
- Verse 18 follows the annotated source's `hadinan` from `hdy` as
  "we rejoice," rather than substituting Murdock's "look." Compare the
  [CAL entry for hdy](https://cal.huc.edu/oneentry.php?cits=all&lemma=xdy+V).
  Both seen/not-seen pairs remain; the things seen are temporal and
  those not seen eternal. The final negative belongs to the not-seen
  subject, not to the temporal predicate.

All 18 verses and 259 source words received source-to-English and running
English review. The seven גיר, four דין and one הכיל occurrences were
checked individually against the final English clauses.

Source checks for 2 Corinthians 5:

- Verse 1 retains our earthly house and its bodily description,
  dissolution, the building from God and a house not made by hands,
  in heaven eternally. No tabernacle noun is substituted for the
  actual body noun. Verse 2 keeps causal For, also, groaning, desire
  to put on our house and its source from heaven.
- Verse 3 has no separate negative word before being found naked.
  The initial `ella` is understood conditionally as "Otherwise,"
  followed by even when clothed and the positive found-naked predicate.
  This contextual choice agrees with the conditional reading in
  [Lamsa's Syriac-derived rendering](https://biblehub.com/2_corinthians/5-3.htm),
  which begins "If not so." It differs from the supplied Murdock
  wording and does not move a supplied not into the final predicate.
- Verse 4 retains now, this house, groaning from its burden, our
  desire not to strip it off but to be clothed over it, and its
  mortality being swallowed up in life. The desire is first-person
  plural, rather than Murdock's "ye desire." The swallowing remains
  passive, with mortality as its subject and life as its destination.
- Verse 5 makes God the present preparer of us for this, and keeps
  his past giving of the pledge of his Spirit. The anticipating
  target suffix does not add another object. Verses 6-8 keep knowing
  and being persuaded, bodily lodging with absence from our Lord,
  walking by faith rather than sight, and the wish to depart from
  the body and be with our Lord.
- Verse 9 retains striving, the absent/dwelling alternatives and
  pleasing him. The target "to him" belongs to pleasing, not to
  dwelling. Verse 10 keeps all of us due to stand before Messiah's
  tribunal, each person's recompense in his body, what he has done,
  and both good and evil alternatives. The distributive person-person
  expression means each person; the final dative in the done-expression
  is not another recipient of the deeds.
- Verse 11 retains fear of our Lord, persuasion of people, being
  manifest to God, the singular I-hope clause, both discourse contrasts
  and the addressees' own minds. Manifestness to God is not changed
  into a passive revelation by God. Verse 12 keeps not again praising
  ourselves, giving the addressees an occasion to boast in us, and
  the face/appearance versus heart contrast of those who boast.
- Verse 13 keeps causal For separately from for-God and for-you,
  and both foolishness and soundness conditions. Verse 14 preserves
  Messiah's love constraining us to consider this: one died for
  everyone, and therefore everyone died. The final dative does not
  create an additional for-him beneficiary.
- Verse 15 repeats his dying for everyone, the purpose that the living
  not live for themselves, and the one who died on their behalf and
  rose. The on-their-behalf construction is an idiom, not physical
  faces. A separate no-longer expression is not supplied here.
- Verse 16 retains both actual body expressions, the first negative
  knowing, the conditional past knowing of Messiah and the final
  present negative knowing from now on. Body is not replaced with
  an absent flesh noun. Verse 17 ends with old things passing away;
  verse 18 explicitly begins with all things becoming new from God.
  Those verse boundaries and both old/new statements remain intact.
- Verses 18-19 keep God reconciling us to himself through Messiah, giving
  the service of reconciliation, reconciling the world with his
  majesty, not reckoning their sins to them, and placing in us the
  word of reconciliation. The actual greatness/majesty noun in
  verse 19 is not replaced by a generic to-himself expression.
- Verse 20 retains ambassadors for Messiah, both therefore particles,
  God beseeching the addressees through us, our beseeching on Messiah's
  behalf, and the imperative to be reconciled to God. Verse 21 keeps
  the one who had not known sin, God's making him sin for your sake,
  and the purpose that we become God's righteousness in him. Your
  is second-person plural, not us. Both sin nouns remain without
  expanding one into a doctrinal paraphrase. The final clause uses
  an English cleft to identify the sinless one as the object of God's
  making, while keeping causal For distinct from for-your-sake.

All 21 verses and 286 source words received source-to-English and running
English review. The nine גיר, five הכיל, two דין and one מכיל occurrences
were checked individually against the final English clauses.

Source checks for 2 Corinthians 6:

- Verse 1 keeps helpers, our entreaty to the addressees and the grace
  of God that they received. The relative belongs to grace, and the
  purpose is that this grace not be made void in them. Verse 2 keeps
  "he said" as the clause introduced by causal For, "I answered you,"
  "I helped you," and both acceptable-time/day-of-life statements
  with their separate behold and now markers.
- Verse 3 keeps the second-person plural giving predicate and the
  cause of stumbling in anything, with no blemish in our ministry
  as the purpose. The opening `lama` has a preventive reading here:
  the [CAL entry for lmh](https://cal.huc.edu/oneentry.php?cits=all&lemma=lmh+c)
  includes lest with an imperfect, also in Syriac. The brief why
  gloss alone does not require a question that reverses the warning.
  The preventive force is rendered as "Do not," following Murdock's
  contextual prohibition, without claiming another separate not-word.
- Verses 4-5 keep ourselves as God's ministers, much endurance,
  afflictions, necessity and confinement, then scourging, chains,
  tumults, labour, watching and fasting. Confinement and chains
  remain distinct entries. The identifying we does not introduce
  a second subject after ourselves.
- Verses 6-7 retain purity, knowledge, long-suffering of spirit,
  gentleness, the Holy Spirit and love without deceit. The first
  spirit completes the long-suffering expression; it is not another
  owner of the Holy Spirit. Word of truth, God's power, armour of
  righteousness and both right and left remain explicit.
- Verses 8-10 preserve each paired description: glory/dishonour,
  good report/reproach, deceivers/true, unknown/known, dead/alive,
  chastised/not dying, sorrowful/always rejoicing, poor/enriching
  many, and having nothing/holding everything. The final death verb
  in verse 9 is active, not a passive killed predicate. The dative
  in sorrow and possession identifies us; it does not create another
  person sorrowing for our benefit or an extra owner of our possessions.
- Verses 11-12 keep our mouth open toward the Corinthians and our
  heart expanded, then both constrained predicates: not in us, but
  in the addressees' own affections. The expanded-heart wording follows
  the actual form and context, not the abbreviated smell gloss.
  Verse 13 retains speaking as to children, repayment to me and
  enlargement of the addressees' love toward me; no my-children
  possessor is supplied.
- Verses 14-15 keep the son-of-yoke partnership idiom and those who
  do not believe, followed by all four questions: righteousness with
  iniquity, light with darkness, Messiah with Satan, and a believer
  with an unbeliever. Satan is the actual source name. There is no
  separate unequal adjective to add to the yoke expression.
- Verse 16 retains both discourse particles, God's temple and that
  of demons, the explicit you as the temple of the living God, and
  the full divine promise. The demons expression is not replaced
  with an absent idols noun. The repeated identifying you names one
  group. Dwelling among them, walking among them, being their God
  and their becoming my people remain four distinct relations;
  the and-they pronoun begins the people clause.
- Verses 17-18 retain coming out from among them, separation from
  them, not approaching the unclean and God's receiving the addressees.
  The first commands are plural, while the receiving speaker is
  singular. Father to you, sons and daughters to me, and the concluding
  divine utterance remain explicit. The all-holding expression is
  rendered as the established Lord Almighty title, not another actor.

All 18 verses and 221 source words received source-to-English and running
English review. The two גיר and four דין occurrences were checked
individually against the final English clauses.
