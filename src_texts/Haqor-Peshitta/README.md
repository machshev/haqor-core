# Word-linked English of the Peshitta

`translation.tsv` contains Haqor's English renderings of the Peshitta,
using James Murdock's public-domain translation as a reference.
The unmodified reference remains in `src_texts/Murdock/murdock.tsv`.
All 7,958 Peshitta verses have reviewed English and complete word links.
Every source word is linked, and every Peshitta verse supports
word selection in the English reader.

Current coverage: Matthew 1-28, Mark 1-16, Luke 1-24, John 1-21, Acts 1-28, Romans 1-16, 1 Corinthians 1-16, 2 Corinthians 1-13, Galatians 1-6, Ephesians 1-6, Philippians 1-4, Colossians 1-4, 1 Thessalonians 1-5, 2 Thessalonians 1-3, 1 Timothy 1-6, 2 Timothy 1-4, Titus 1-3, Philemon, Hebrews 1-13, James 1-5, 1 Peter 1-5, 2 Peter 1-3, 1 John 1-5, 2 John, 3 John, Jude, and Revelation 1-22
(7,958 of 7,958 Peshitta verses; 0 remain). All Peshitta verses are translated and linked.
All Peshitta verses use the reviewed, linked source; none requires the
unlinked Murdock fallback.

Each non-comment row is `book<TAB>chapter:verse<TAB>translation`, using
Haqor's one-based book numbers (Matthew = 40), Peshitta verse numbering, and
zero-based source positions. `[English phrase|0]` links a phrase to one
Aramaic word. Use one source word per span by default. `[English phrase|0,1]`
is reserved for exceptional expressions that cannot reasonably be rendered
with separate spans, with a specific linguistic reason recorded in the
alignment review ledger. Ordinary noun phrases, negation, auxiliaries and
clauses should be split wherever their English constituents can be assigned
to individual source words. English spans may appear in English word order:
`[Holy|1] [Spirit|0]` does not require a combined span merely because the
Aramaic noun precedes its modifier. Prefixes and suffixes belong to their
source word; a phrase may therefore translate more than one English word.
One Aramaic word can own separate English spans when its prefixes, stem or
suffixes are separated in English: `[and by|7] [another|8] [way|7]` keeps
each English span attached to its actual source word. Repeated source links
are permitted for this purpose; every source word must still be covered.
Separate linked phrases with spaces where English words need separation;
markup does not insert spaces between spans. Review the running English as
well as each individual source-to-English mapping.

Both `db gen-runtime` and `db gen-translation` load this tracked source
from the supplied `--src-texts` directory on every build. The source file
is required: a missing file fails the build instead of dropping the links.
The runtime `haqor.db` is generated output.

The generator rejects missing or invalid source positions,
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
Read the construction as well as the numeric tense: nominal negation with
lā hwā, including inflected lā hwīt, can express present negation despite
its perfect-form morphology. It must not automatically become English past
tense; see [Joosten, section 2.1](https://www.academia.edu/34446491/The_Negation_of_the_Non_Verbal_Clause_in_Early_Syriac).
Perfect plus hwā likewise does not require English pluperfect: it can serve
as a narrative past construction. Choose English aspect from the clause and
context; see [Younansardaroud, composite tense structures](https://refubium.fu-berlin.de/bitstream/handle/fub188/24490/Classical_Syriac_Course_Book_3-2019.pdf?isAllowed=y&sequence=3).
Use the generator's established “ecclesia/ecclesias” terminology for
congregations so the reviewed source and runtime text match exactly.
Work in book and chapter order.

The review of shared spans is tracked separately in `alignment-review.json`.
Its baseline contains 26,851 shared spans across 260 chapters. The earlier
translation completion does not mean those spans meet the single-source
policy above. Each independently reviewed chapter records its final TSV
SHA-256 and a linguistic reason for every remaining shared span; changes
invalidate that chapter's checkpoint.

Run `python3 scripts/audit-peshitta-links.py --out /tmp/peshitta-links.json`
from the repository root to check source coverage, list every shared span
beside its exact source forms, and report reviewed and pending chapters.
Use `--require-reviewed` to require completion of the entire review. The
chapter exporter now includes the current linked translation alongside the
source morphology and all gloss alternatives. The inventory and coverage
checks are structural; they do not replace linguistic review.

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
- Matthew 17:24-25 has kesef risha, rendered "head tax" with the head
  modifier and tax noun linked separately. Compare
  [kesef in CAL](https://cal.huc.edu/oneentry.php?cits=all&lemma=ksp+N).
- Matthew 17:26 has bnay haire, "free individuals", before the copula;
  the final bnya supplies the subject "the children". The first bnay
  belongs to the status expression. Compare CAL's explicit
  [Matthew 17:26 citation](https://cal.huc.edu/oneentry.php?cits=all&lemma=br%40xyryn+N).
- Matthew 18:29 has no equivalent of Murdock's added "the whole".
- Matthew 19:5 says "one flesh"; 19:6 uses the distinct word "body".
- Matthew 20:30 links "by" and "the side" separately, and renders yahbu qala
  as "they gave a shout" with separate verb and noun links. Compare the
  spatial senses of [yad in CAL](https://cal.huc.edu/cal_entry_web.php?lemma=yd+N)
  and the vocalization construction under
  [yhb in CAL](https://cal.huc.edu/oneentry.php?cits=all&lemma=yhb+V).

Source checks for Matthew 21:

- The praise quotation divided across Murdock 21:16-17 is entirely in
  Peshitta 21:16 and is linked there.
- Matthew 21:41 has the reduplicated adverb bish bish, rendered "in the
  worst way", with its complement prefix linked separately. CAL explicitly
  cites this verse under the adverbial sense of
  [bish](https://cal.huc.edu/oneentry.php?cits=all&lemma=by%24+A).

Source checks for Matthew 22:

- Matthew 22:24 makes the brother the subject of taking the deceased man's
  wife; the wife's possessive suffix is retained. In 22:25, the number seven
  and the noun brothers have separate source owners.
- Matthew 22:30 renders hawayn l-gabre contextually as "become married to
  men", with the verb and its complement linked separately. Compare the
  marital example under "to become" in
  [CAL's hwy entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=hwy+V).
- Matthew 22:37 explicitly includes strength as well as heart, soul, and
  mind; all four are retained and linked to their source words.

Source checks for Matthew 23:

- The two-word face-taking expression is rendered as the single linked idiom
  "hypocrites", rather than adding a separate appearance phrase.
- Matthew 23:17 retains the separate anticipatory object pronoun and its
  explicit noun as "sanctifies it (the gold)". The apposition preserves each
  source owner.
- Matthew 23:26 has plural "blind Pharisees" and a plural imperative.
- Matthew 23:34 separates the independent subject, sending participle, and
  postposed personal copula as "I am sending".
- In 23:37, the participial descriptions of Jerusalem and the third-person
  "to her" are retained; Murdock changes the latter to direct address. Compare
  the [Peshitta interlinear, page 5](https://www.atour.com/media/files/religion/peshitta/interlinear-nt/Mattai/Mattich23.pdf).
- In 23:39, the negated future is a prediction that they will not see him,
  rather than a prohibition on seeing him.

Source checks for Matthew 24:

- Matthew 24:3 renders baynayhon w-leh as "privately". CAL records the
  different-suffix byn plus l construction as a private exchange; see
  [byn in CAL](https://cal.huc.edu/oneentry.php?cits=all&lemma=byn+p).
- Matthew 24:5 has Peal neṭ'un, "many will go astray", whereas 24:11 has
  the causative Aph'el form, "lead many astray". The vocalization, stem,
  and subject/object roles are retained; compare the G and C senses of
  [ṭʿy in CAL](https://cal.huc.edu/oneentry.php?cits=all&lemma=%2B%28y+V).
- Matthew 24:6 retains distinct wars and reports of wars, and the explicit
  obligation "must". Matthew 24:7 retains the distributive repeated-place
  construction as "in various places".
- Matthew 24:31 renders the repeated rishhon endpoints as "one end" and
  "the other end" with separate links. CAL records the spatial end sense
  and the rish-to-rish merism under
  [rish](https://cal.huc.edu/oneentry.php?cits=all&lemma=ry%24+N).
- The feminine form in 24:33 is rendered with neutral "it", rather than
  personal masculine "he".
- In 24:36 the source goes directly from the angels of heaven to the Father
  alone; no intervening person is supplied.
- The masculine and feminine pairs in 24:40-41 are expressed as two men and
  two women, respectively.

Source checks for Matthew 25:

- In 25:1 both the bridegroom and the bride are explicit source words and
  are retained.
- In 25:8 the third-person dative resumes the lamps, not their first-person
  possessors. "They—our lamps—have gone out" preserves both references.
- In 25:9 lma with an imperfect expresses apprehension, rendered "for fear",
  rather than a question asking why. See [CAL's lmh entry, sense 2](https://cal.huc.edu/oneentry.php?cits=all&lemma=lmh+c).
  The negative and the two recipient phrases remain separately linked.
- In 25:15 the repeated anash anash is distributive, "each person". The
  institution compound beth asire is retained as "in prison" in 25:36,
  25:39, 25:43 and 25:44; its two nouns do not separately mean "in" and
  "prison".
- The differently pointed past auxiliary is first person in 25:24 ("I knew")
  and second person in 25:26 ("you knew"). It is linked with the knowing
  predicate, rather than with the following description or reaping predicate.
- In 25:41 the king speaks to those on his left; the imperative with its
  reflexive dative is linked separately from "from me". The false-accuser
  title follows the established rendering used in earlier chapters.

Source checks for Matthew 26:

- In 26:3 the priests, scribes and elders are the subjects of "assembled".
  In 26:5 the past auxiliary belongs to "were saying", before the quoted
  prohibition. In 26:17 the preparing verb is first-person plural, "us to
  prepare", rather than a second-person act by Jesus.
- In 26:18 the time has arrived: the verb is perfect, and the following
  dative resumes the time rather than introducing another recipient.
- In 26:22 had had is distributive, "each one". Men bathar in 26:32 and
  26:73 is the compound temporal preposition "after".
- The feminine dative in 26:38 resumes "my soul", rather than its
  first-person possessor; "It—my soul—is grieved" retains both references.
- In 26:39 the cup petition is volitional, rather than a bare infinitive.
  The repeated first-person forms belong to the willing predicate; they do
  not require a detached extra English "I".
- In 26:44 the feminine pronoun anticipates the feminine "word" as the
  object of saying; "said that word" does not introduce another recipient.
  In 26:52 the relative clause retains the perfect "who have taken".
- In 26:65 the passive required predicate with "to us" is rendered
  "do we need witnesses", rather than an active act of seeking. The
  contracted first-person form in 26:74 supplies both "I" and "know",
  separated in English by the independently linked negative.
- In 26:69 the past auxiliary belongs to the assertion that Peter was with
  Jesus. It is not part of the following "Nazarene" epithet.
- In 26:75 the cock-crow clause is future-relative, not past: "before the
  cock crows" is followed by "you will deny me".

Source checks for Matthew 27:

- In 27:9 the source names only "the prophet"; no individual name is added.
  The quotation begins with first-person "I took". The agreeing verb has
  a plural subject, with "from among the sons of Israel" identifying them.
- The responsibility formulas in 27:4 and 27:24 are rendered contextually
  as "you are to see to it" and "you must see to it", following Murdock's
  sense. The former has a participle and copular pronoun; the latter has
  an imperfect. Neither is described as a morphological imperative.
- In 27:6 the putting verb is first-person plural, "for us to put it".
  The house of offering retains separate constituent links. The lexical
  beth qvura/qvure burial compounds in 27:7, 27:52 and 27:60 remain minimal
  shared spans; [CAL's house entry](https://cal.huc.edu/cal_entry_web.php?lemma=by+N)
  lists these compounds as burial place, tomb or cemetery.
- In 27:12 akhl qarso is the accusation idiom, with its auxiliary and
  object suffix separately linked. In 27:19 the negative paired-dative
  formula supplies "have nothing to do"; the following demonstrative and
  righteous one retain separate links.
- In 27:38 the two robbers are crucified: the verb is passive. In 27:52,
  by contrast, the bodies arise; the past auxiliary belongs to the sleeping
  description rather than making the rising verb passive.
- In 27:47 the past auxiliary belongs to "were saying". The explicit
  demonstrative then begins their claim: "This one has called for Elijah."
  The calling verb is perfect, not a participle.
- In 27:46 the two God forms have no possessive suffix; "my" is not
  imported from familiar versions of the quotation. In 27:51 appay can
  mean curtain independently of the following door noun (Muraoka,
  Classical Syriac, section 21); the singular English keeps separate links.
- In 27:55 the women had come after Jesus; the after word is a
  preposition, not a following verb. In 27:57 Joseph had become a disciple,
  retaining the derived verb and its past auxiliary separately.
- In 27:60 Joseph's laying verb is singular, but rolled, placed and
  departed are plural. In 27:63 the auxiliary belongs to "had said",
  before "while he was alive"; men bathar remains the compound "after".
  In 27:64 beth means "among" in "from among the dead", rather than
  adding a house. In 27:66 the feminine demonstrative modifies the stone,
  not the masculine tomb.

Source checks for Matthew 28:

- In 28:1 the evening wording and the first-day-of-the-week idiom are
  retained, without replacing the source timing with a different formulation.
  The first day and week constituents have separate links.
- In 28:3 the existential form and past auxiliary are separated with
  a resumptive English "it" for his appearance. In 28:4 the first past
  auxiliary belongs to "were guarding", before the separate became verb.
- In 28:6 the dative with arose is coreferential with its subject, rendered
  "he has risen". The laid participle, past auxiliary and locative relative
  retain separate owners. In 28:7 beth is the preposition "among" in
  "from among the dead".
- In 28:10 the possessive in "my brothers" is first person; the morphology
  dataset uses person code 3 for first person.
- In 28:11 the demonstrative modifies the guards, rather than adding
  a clause saying they were there. In 28:12 the litotes "no little money"
  has separate negative, adjective and noun links.
- In 28:14 the second assurance retains the negative anxiety predicate
  with make, "make you free from anxiety". In 28:15 the object of taught
  becomes the English subject of "they had been taught"; it does not own
  the next clause's conjunction. The story has spread, retaining its verb.
  The teaching verb is Pael: [CAL's ʾlp entry](https://cal.huc.edu/cal_entry_web.php?lemma=%29lp+V)
  distinguishes this teach stem from Peal learn. The English passive
  preserves the instructed guards and leaves the implicit teachers unnamed.
- In 28:16 the object pronoun means "them", not "had". The verb is
  "had summoned"; [CAL's wʿd entry](https://cal.huc.edu/cal_entry_web.php?lemma=w%28d+V)
  lists invite/summon in the Pael and directly cites this verse. In 28:17
  the perfect with the past auxiliary gives "had doubted", distinct from
  a participle with its auxiliary.
- The distinct verbs in 28:19 and 28:20 are rendered "instruct" and "teach",
  respectively. Holy and Spirit, I and am, and all and the days have
  separate links; the final Amen follows the closing speech.

Source checks for Mark 1:

- In 1:2 Isaiah is explicitly named and is retained.
- In 1:6 the hair and camels, strap and leather, and wild and honey
  constituents remain separately owned. The food's existential form and
  past auxiliary are rendered with a resumptive "it was".
- In 1:7 the sandal straps are plural. The relative prefix and negative
  in d-la can own separated English spans; the past auxiliary belongs
  to the proclaiming verb. In 1:11 the voice says "in you", second person,
  rather than substituting "in whom".
- In 1:14 the dative after came is coreferential with Jesus; it does not
  make John the recipient of his journey. The destination is Galilee.
  In 1:15 the time has reached fulfillment, and the kingdom "has arrived"
  renders the actual perfect arrival verb, rather than replacing it with
  a drawing-near verb from another text.
- In 1:16 nets is plural. In 1:17 bnay anasha is the human-being idiom,
  "of people", consistent with Matthew 4:19. In 1:19 the two brothers'
  resumptive object becomes the English subject of the mending clause,
  retaining the same actors. Sabbaths and synagogues in 1:21 are plural.
- In 1:24 the ma lan wa-lakh involvement formula retains a minimal shared
  "What have we to do", with its addressee separately linked. The later
  identification is made into fluent English with an appositive "you are".
  Shut and your mouth in 1:25, and one, with and another in 1:27, can be
  split without losing their meanings.
- In 1:31 the woman ministers to "them", preserving the plural source object
  rather than copying the singular object of Matthew 8:15.
- In 1:32 and 1:34 the badly-afflicted constituents and their auxiliaries
  are separately linked. In 1:34 the demons' stative knowing with its past
  auxiliary is rendered "were aware of him". In 1:35 the early-acting and
  rising verbs give "rose very early" with separate owners.
- In 1:38 d-af retains both the purpose connective and also, "so that
  I may preach there also". In 1:40 the two independent second-person
  pronouns supply "you", rather than making either an object "it".
- In 1:44 lma supplies the caution "that ... not" after see;
  [CAL's lmh entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=lmh+c)
  records the lest sense. The following pronoun, telling verb and anyone
  remain separate. In 1:45 the perfect beginning verb and past auxiliary
  give "had begun"; the solitary place is singular.
- In 1:21-22 the teaching predicates retain their subjects and auxiliaries;
  the postpositive explanatory particle stays with the second teaching clause.

Source checks for Mark 2:

- In 2:2 the gathering is intransitive, and the singular containing predicate
  refers to the available space, not to the assembled people.
- In 2:4 the ascent includes a reflexive dative. The lowering verb anticipates
  the following bed noun; no separate human object is added.
  The passive participle describes the man's position, "was lying";
  [CAL's rmy entry](https://cal.huc.edu/cal_entry_web.php?lemma=rmy+V)
  explicitly records this passive-participle sense and cites Mark 1:30.
- In 2:6 the location adverb remains "at that place"; the following
  partitive supplies "some". In 2:8 the first demonstrative can identify
  the reasoning men, while the later demonstrative is the object
  "these things" in Jesus' question.
- Ela en in 2:7 and 2:26 is the compound exception particle "except".
  Bar shateh in 2:12 is the immediate-time expression "at once".
  These three minimal shared spans retain reasons in the ledger.
- In 2:14 the house and plural tax collectors are rendered literally
  with separate links. [CAL's house entry](https://cal.huc.edu/cal_entry_web.php?lemma=by+N)
  also lists the custom-house compound; no singular tax-office noun is
  assigned to the tax collectors word alone.
- In 2:17 the illness expression is one idiom, not an additional action of
  laboring.
  Its repeated bish intensifies the state: "who are very badly afflicted"
  retains separate links for the relative prefix, intensifier and predicate,
  as with the illness state in Mark 1:32 and 1:34.
- In 2:18 the first Pharisees noun lacks the genitive prefix found in
  the later "of the Pharisees". In 2:19 the interrogative particle and
  ability predicate are separate; the explicit answer "No" is retained.
  In 2:20 the temporal clause anticipates the bridegroom being taken,
  followed by then and the future fasting verb.
- In 2:26 Abiathar is retained as named; no other priest is substituted.
  Chief and the plural priests retain separate links. The relative prefix
  and negative in d-la have separate English spans. In 2:27 the second
  nominal clause is elliptical, "man was not for the Sabbath", rather
  than claiming a second created stem in the source.

Source checks for Mark 3:

- In 3:4 the plural pronoun begins "But they were silent"; it is not
  an object of destroy in the preceding question. In 3:5 the experiencer
  dative supplies "he" with the grieved predicate.
- Bar shateh in 3:6 retains the immediate-time compound "at once".
  The household members and Herod have separate links.
- In 3:8 the crowd expression is plural: "great multitudes".
  The preceding following verb in 3:7 is perfect with a past auxiliary,
  "had followed". In 3:10 the healing verb is a participle with its past
  auxiliary, "was healing", distinct from that perfect construction.
- In 3:11 the afflicted people remain the actors of the falling and
  speaking. Their possessive/experiencer pronoun is resumed as the English
  subject "they". In 3:12 the negative and the modal belonging to the
  revealing verb have separate owners.
- The purpose clauses in 3:14-15 keep the plural selected disciples and
  singular sender distinct. The list continues through 3:19, with naming
  remarks in parentheses. In 3:17 "them" is plural and gives the name
  Boanerges to both James and John.
- In 3:21 the out-of-his-mind expression remains the kinsmen's reported
  claim, with the explanatory particle linked to "they were saying".
  It can be rendered separately as "had gone out / of / his mind".
  In 3:23 in parables describes the speaking, rather than the calling.
- In 3:26 the rising and dividing verbs are perfect in the conditional
  clause, and the following feminine copular pronoun resumes the end noun.
  Ela en in 3:27 remains the compound "unless". Bnay anasha in 3:28 is
  the human-being idiom "people", consistent with the fishers expression.
- In 3:29 the source specifies eternal judgment; no different object is
  substituted. The first-person family possessives in 3:35 are retained.
  The past auxiliary in the explanatory 3:30 belongs to "were saying";
  the unclean-spirit assertion stays inside their reported speech.

Source checks for Mark 4:

- All 136 original shared spans were checked against the source forms,
  morphology and connected English. The final five exceptions are the
  immediate-time idiom in 4:5 and four existential/dative possession
  constructions in 4:9, 4:23 and 4:25. The dative possessor is not a
  relative pronoun or part of the following hearing or giving clause.
- In 4:1 the beginning verb is perfect with a past auxiliary, distinct
  from the participial teaching and standing predicates. In 4:10 the
  twelve retain their possessive suffix, "his twelve".
- In 4:13 and 4:15 the anticipatory pronouns and their explicit noun
  objects have separate links. In 4:19 the pronoun resumes the word,
  which becomes fruitless; the desires remain the agents of choking.
- In 4:21 the connective and preposition in d-al have separate English
  spans owned by the same source word. In 4:28 the anticipatory dative
  is "it" before the explicit fruit noun, rather than a demonstrative.
- In 4:39 the first command is imperative, while the following zgir is
  a passive participle with an explicit second-person pronoun:
  "Be quiet; you are restrained." The two forms are not conflated.
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

- All 157 original shared spans were checked against the exact forms,
  morphology and connected English. Three shared expressions remain:
  the distributive temporal "whenever" in 5:4, the departure verb with
  its subject-coreferential ethical dative in 5:17, and the immediate-time
  compound in 5:42. Ordinary predicates, noun phrases and prohibitions
  have separate source owners.
- In 5:2 the unclean spirit is in the man; the locative pronoun is not
  reassigned to a possession construction. In 5:8 the saying predicate
  is participial with a past auxiliary, "was saying".
- In 5:9 the asking verb carries its own object suffix; the following
  saying verb and dative belong to the reply. In 5:15 the legion's past
  existence is separate from the man's clothed, composed and sitting
  descriptions. In 5:18 the past auxiliary belongs to asking.
- In 5:23 asking, its auxiliary, the person asked, and the degree adverb
  have separate links, as do the severe affliction and its modifier.
  The woman's description continues through 5:25-27; in 5:26 the negative
  includes "at all", and 5:27 distinguishes the press from the crowd.
- In 5:34 the causative healing verb retains its feminine object suffix,
  "has made you live". In 5:35 the arriving verb is perfect, "came",
  distinct from the present speaking participle.
- In 5:40-41 father, hand and girl have separate links. In 5:42 the
  age classifier supplies "old" in "twelve years old", the attested
  feminine age sense of bart; compare the
  [Syriac dictionary entry](https://assyrianlanguages.org/sureth/dosearch.php?language=id&searchkey=18462).
- In 5:9 the name suffix is plural: "our name". In 5:12 the speakers are
  explicitly identified as those demons, not left as an unnamed pronoun.
- In 5:30 the knowledge clause says that power had gone out from Jesus;
  the clause-introducing prefix is linked with the power noun.
- In 5:41 the command is simply "Girl, arise"; no explanatory wording from
  another textual tradition is supplied.
- In 5:42 the first past auxiliary belongs to the walking predicate. The
  following existential clause explains that she was twelve years old.

Source checks for Mark 6:

- All 236 original shared spans were checked against the inflected source
  forms, morphology and connected English. Thirty-two justified shared spans
  remain; ordinary noun phrases, auxiliaries, prohibitions and quantities
  have separate source owners.
- In 6:1 the disciples were following him. In 6:2 the anticipatory dative
  possessor and explicit demonstrative both own "this man" in the question
  about where his teaching comes from; the past auxiliary belongs to the
  saying predicate.
- In 6:3 "behold" is explicit in the sisters question. In 6:5 the second
  negative reinforces "not one"; the negative and numeral now have
  separate owners, and it does not negate the following healing.
  Ela en in 6:4, 6:5 and 6:8 remains the compound exception particle.
- In 6:12 the implicit plural repentance subject is clarified as "people",
  the hearers of the apostles' preaching. The finite-plus-hwa predicates
  in 6:12, 6:14 and 6:34 express narrative "went out", "said" and "began",
  rather than an obligatory earlier past. In 6:21 "had made" backgrounds
  the supper before the daughter enters. In 6:13 the three past auxiliaries
  belong separately to casting out, anointing and healing.
- In 6:17 the past auxiliary belongs to sending; the explicit John noun
  resumes the seizure verb's object suffix. The prison compounds in
  6:17 and 6:27 and the burial-place compound in 6:29 retain both owners.
  Their locative English has separate links to the nominal head; the
  prison compounds have bare beth, while 6:29 has an actual b-prefix.
- In 6:31 the ethical dative belongs to the plural command "Come";
  "let us go" is first-person plural, while "rest" is a plural command
  to the disciples. The negative existential eating clause retains both
  negations and its separate room noun and dative.
- In 6:34 the relative "that" has its own link to the d-prefix;
  "had" retains the existential and dative possession construction,
  while "no" belongs separately to the negative existential word.
  Neither source word alone is treated as a lexical "have" verb.
  The positive possession in 6:38 and negative possession in 6:36 likewise
  retain both existential and dative owners. In 6:36 the single English
  word "nothing" retains the negator and indefinite together, with
  "at all" separately linked to the emphatic indefinite. These ownership
  decisions follow the possession and negation constructions in
  [Younansardaroud, section 6.6](https://refubium.fu-berlin.de/bitstream/handle/fub188/24490/Classical_Syriac_Course_Book_3-2019.pdf?isAllowed=y&sequence=3).
- Named possessors retain their anticipatory suffix links: Mary and James
  in 6:3, Herodias in 6:22, the kingdom in 6:23, and John in 6:24-27.
  The first-person "my" in 6:23 belongs to the kingdom word, separately
  from the third-person suffix on "half." In 6:27 the cutting verb's
  object suffix resumes the head, while the head's possessive suffix
  identifies John; these have separate shared referent spans. Quantifier
  suffixes retain the cities in 6:33, the region in 6:55 and "those" in
  6:56. All 56 verses retain their English wording.
- In 6:39-40 the grouping and numerals are distributive repetitions,
  "group by group", "a hundred by a hundred" and "fifty by fifty".
  The source in 6:39 names grass without a separate green adjective.
- In 6:47 the existential and past copula jointly express the boat's
  location. In 6:49 the appearance is explicitly deceptive; the two
  first-person pronouns in 6:50 retain the emphatic identity assertion.
  Its "And" belongs to the waw prefix outside the shared "immediately".
  In 6:52 the understanding verb is perfect with its past auxiliary,
  "had not understood".
- In 6:55 the severely afflicted people, carrying, beds, and the hearing
  predicate have separate owners. Despite the passive-participle label,
  shqil has the attested active sense "carrying"; compare
  [CAL's shqil entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=%24qyl+A).
  In 6:56 the placing is active, "they were laying", and each past
  auxiliary remains with its own entering, placing, asking, touching or
  healing predicate.
- In 6:20 the adverb "gladly" belongs to hearing John, following the
  separate statement that Herod listened and acted in many matters.
- In 6:21 the linked compound means "birthday", rather than a literal
  house of birth. "On" and "his" have separate links to the actual
  prepositional prefix and possessive suffix; compare the
  [CAL entry for beth yalda](https://cal.huc.edu/oneentry.php?cits=all&lemma=byt%40yld%29+N).
- In 6:41 the plural verb says that they divided the two fish. In 6:43
  the twelve full baskets contain fragments and fish; fragments are not
  omitted or assigned to the fish noun.
- In 6:45 Bethsaida is explicitly named as the destination. In 6:52
  the demonstrative specifies "that bread"; in 6:56 the streets are
  explicitly retained as the place where the sick were being laid.

Source checks for Mark 7:

- All 122 original shared spans were checked against the exact forms,
  morphology and connected English. Twelve justified exceptions remain:
  the conditional marker in 7:3, the face-taking idiom in 7:6, six
  human-being expressions, and four existential/dative or past-state
  constructions. Ordinary predicates, negation and noun modifiers have
  separate source owners.
- In 7:1 the Jerusalem relative clause modifies the Pharisees and scribes.
  In 7:2 and 7:5 the hands are not washed; the passive descriptions do
  not become an active washing event. In 7:3 en hu supplies the conditional
  marker, with the washing negation retained separately.
- In 7:6 the collective people's lips have a singular possessor, while
  "their heart" retains its actual plural possessor. The face-taking
  expression is the hypocrisy idiom. In 7:8 the forsaking verb is perfect,
  separate from the following holding predicate and its explicit subject.
- In 7:10 the death noun intensifies the finite dying verb, rendered
  "shall surely die" with separate owners. In 7:11 both second-person
  pronouns remain before the conditional; the offering noun retains
  "my offering", and the following benefit is from the speaker.
- In 7:16 the existential and its dative express possession of ears;
  the dative is not moved into the following hear command. In 7:18
  subject and copular pronouns remain distinct, and the separate can/not
  owners form the English word "cannot" without a shared span.
- In 7:19 la hwa is rendered as a negative cleft, "it is not into his
  heart that it enters", rather than forcing a past-time assertion.
  Compare the analysis of negative clefts in
  [Pat-El's study of Syriac negation](https://doi.org/10.1093/jss/fgl006).
  The following feminine cleansing predicate describes physical elimination;
  it is not reassigned to the speaker. In 7:23 the cleft and anticipatory
  object pronoun retain their own source positions.
- In 7:25 the relative daughter clause follows the woman, while "about
  him" remains masculine. The existential and past auxiliary jointly
  express the daughter's possession of an unclean spirit. In 7:26 the
  woman's past identification is separate from her asking predicate.
- In 7:29 the ethical dative resumes the departing demon, rather than
  naming an additional beneficiary. In 7:30 "her demon" retains the
  genuine possessive suffix. In 7:31 Tyre and Sidon both belong to the
  departure region; the arrival is at Galilee's sea in Decapolis's region.
- In 7:32 the hand is singular; in 7:33 the fingers and ears are plural.
  In 7:35 "at that hour" has separate temporal owners. In 7:36 the
  negative warning and the two progressive predicates retain their
  respective pronouns and auxiliaries. In 7:37 the making verb governs
  both hearing and speaking, with the negative speaking description
  kept separate from the final positive speaking verb.
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

- All 123 original shared spans were checked against the exact forms,
  morphology and connected English. Seven exceptions remain: the en hu
  conditional, two circumstantial full-basket descriptions, two human-being
  idioms, the openly expression, and the go command with its ethical dative.
- In 8:1 the positive and negative existential clauses have separate
  auxiliaries. In 8:2 the negative existential and the following indefinite
  have separate owners within "nothing". In 8:3 the copular pronoun belongs
  to the conditional marker, not to an additional participant.
- In 8:6 the crowds are plural. In 8:7 the plural object of setting out
  refers to the fish; the verb's English constituents surround that object
  as "set them out". In 8:12 the feminine anticipatory dative identifies
  the generation receiving no sign, rather than making the sign a recipient.
- In 8:16 and 8:17 the bread clause keeps its own possessors and negation.
  In 8:17 the heart's possessor belongs to its existential predicate, not to
  the preceding understanding clause. In 8:18 the eyes and ears are distinct
  possessions, each followed by its own negative perception clause.
- In 8:19-20 kad mleyn describes the collected baskets as full; its
  circumstantial meaning is retained in the English attributive expression.
  In 8:23 the first hand belongs to the blind man, while the laying hand is
  the healer's. In 8:25 the restored subject is clarified as the man; the
  following seeing predicate retains its past auxiliary.
- In 8:27 and 8:29 both the "of me" and "that I am" references retain their
  own first-person forms. In 8:31 the prediction retains cardinal "three"
  and plural "days", and the rising predicate is active. In 8:32 ayn
  b-galya is the adverbial openly expression, not an independent physical
  eye; compare [CAL's bgly entry](https://cal.huc.edu/cal_entry_web.php?lemma=bgly+X).
- In 8:33 the command's dative is subject-coreferential; the actual direction
  is behind the speaker. In 8:34-37 self, life, save, lose and their genuine
  possessive/object suffixes remain with their own source words. In 8:36
  the impersonal English benefit construction introduces no additional
  person as an agent.
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

- All 157 original shared spans were checked against the exact forms,
  morphology and connected English. Eight justified exceptions remain:
  the human-being expression in 9:3, suddenness in 9:8, the exception
  particle in 9:9, three immediate-time expressions, spirit possession
  in 9:17, and the dying predicate with its ethical dative in 9:26.
- In 9:1 the kingdom's coming verb is perfect, "has come". In 9:3 the
  clothing has two state descriptions, radiant and very white; the people
  on earth are the actors of whitening, without an added fuller noun.
  In 9:4 Elijah and Moses are the speakers with Jesus.
- In 9:7 the Son retains "my" and "beloved", and hearing him is a plural
  command. In 9:9 the prohibition's exception is temporal. In 9:10 the
  first question word asks what the saying means; the later ma d-qam
  introduces "when he has risen", parallel to the preceding verse.
- In 9:11 wale supplies the right/proper/necessity predicate, "must".
  In 9:12 the citation reading "as it is written" follows Murdock; the
  particle also admits an interrogative reading, so this is a contextual
  choice rather than a different source form. In 9:15 "whole" modifies
  the crowd, separately from the immediate-time expression.
- In 9:17 the spirit is the non-speaking subject. In 9:18 it seizes and
  convulses the boy, while the boy foams, gnashes his teeth and withers.
  The disciples were asked to cast out the spirit, not the boy. In 9:20
  the spirit sees Jesus and convulses the boy; the boy then falls and
  writhes. In 9:21 English supplies the duration predicate implicit in
  the nominal time question, retaining for him, now and since separately.
- In 9:24 the father cries out, retains "my Lord", and asks help with
  the lack of his faith. In 9:25 the two first-person forms frame
  "I am commanding you". In 9:26 the observers' death claim is distinct
  from the narrator's comparison to a dead person.
- In 9:31 handing over and being killed are passive, the killers are
  plural, and rising is active. The singular day with its numeral gives
  the third day. In 9:32 the anticipatory pronoun resumes the saying;
  the following fear and asking clauses keep their separate owners.
- In 9:38 the following verb is perfect. In 9:39 the negative existential
  governs both doing mighty works and quickly speaking evil. In 9:40
  both pronouns are "you", rather than another textual tradition's "us".
  In 9:42 the millstone's donkey modifier and both counterfactual passive
  predicates remain explicit.
- In 9:43-48 each body part, possessive, causative stumble predicate,
  command, quantity and destination has its own owner. The three refrain
  clauses retain both possessives and both negatives. In 9:50 the final
  hwaw is the plural imperative "be at peace" in context; its isolated
  SEDRA perfect tag does not override the homographic command reading.
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

- The chapter's 185 original shared spans were reviewed against exact
  source forms, morphology, and context. Eight remain: the displeasure and
  sorrow predicates with their experiencer datives, the exceptive connective,
  past and relative possession, the human-being idiom, the hundredfold
  expression, and the spatial beside idiom. The ledger records each reason.
- The husband is the divorcing subject in 10:11; 10:12 switches to the woman
  divorcing her husband and belonging to another. Ordinary negation,
  auxiliaries, noun modifiers, and the separate commands retain their own links.
- In 10:13 Jesus is the singular subject of touching the children. In
  10:14 the dative belongs to his being displeased, rather than identifying
  someone he was offended at; the kingdom belongs to those like the children.
- In 10:19 the false testimony noun and modifier retain separate links.
  "Defraud" is a Syriac sense of [tlm in CAL](https://cal.huc.edu/oneentry.php?cits=all&lemma=%2Blm+V),
  beyond the export's shorter gloss list. In 10:21 the dative completes
  "one thing is lacking to you"; it does not belong to "Go". The cross noun
  has no possessive suffix.
- In 10:24 "my children" and "their possessions" retain their distinct
  possessors. In 10:27 both occurrences of "with God" and the explanatory
  "for" are retained, with the positive predicate supplied in the ellipsis.
- The lists in 10:29 and 10:30 remain distinct: the first includes a wife,
  father, and fields; the second includes mothers and fields, without a
  repeated wife or father. The first-person "my gospel" is retained.
- In 10:38 the repeated first-person pronouns complete their drinking and
  immersion predicates, rather than adding possessors to the immersion noun.
  In 10:42 the copular pronoun completes "are their lords" before the next
  clause. In 10:45 passive "to be served" contrasts with active "to serve";
  [CAL also attests ransom for purqana](https://cal.huc.edu/oneentry.php?cits=all&lemma=pwrqn+N).
- In 10:46 the source names Timaeus and then son of Timaeus; al yad is the
  spatial "beside" idiom. In 10:48 the silent subject is the blind man.
  In 10:52 the imperative is "See", and the ending describes going along
  the way; no following-after-him phrase is supplied from the reference.

Source checks for Mark 11:

- The chapter's 119 original shared spans were checked against source
  forms, morphology, and context. Seven remain: the near and immediate-time
  idioms, the compound exceptive, the observed withered state, an anaphoric
  possession predicate, and two occurrences of the human-being idiom.
  The ledger records a specific reason for each.
- In 11:1 the approach verb is singular. In 11:2 the final word includes
  the command to bring the colt. In 11:3 the title is "our Lord", and
  "immediately" belongs to sending it here. In 11:6 the plural pronoun is
  the object of commanding: Jesus commanded them. Jesus himself rides
  the colt in 11:7; the source does not say that others seated him.
- In 11:9-10 the copular pronouns retain their links, the kingdom belongs
  to "our father David", and the final phrase is "in the heights" without
  adding a separate heavens noun. In 11:11 the evening-time noun and
  its modifier remain separate.
- In 11:13 the tree's leaves and the season of figs remain distinct;
  the negative finding clause retains its exceptive, and both hwa forms
  contribute to "had not come" in the seasonal clause. In 11:16 the
  carried vessels are plural. In 11:15-17 buying and selling, the prayer-house
  nouns, and the den and robbers retain separate links. In 11:20 kad
  introduces the tree's observed withered state, not the observers' state.
- In 11:22 "Let faith in God be in you" retains the separate predicate,
  faith noun, and in-you phrase. In 11:23 both predicates are retained:
  believing that what is said will happen, and having whatever is said.
  The mountain command ends at the sea. In 11:24 the receiving predicate
  and the future possession consequence remain distinct.
- In 11:25 the grievance is something the hearers have against someone.
  The negative conditional and its consequence in 11:26 remain in full.
  In 11:30 the question is "From where is John's immersion?"; the suffix
  on immersion refers to John. In 11:32 the people hold John to be a
  prophet, while the questioners express fear of the people.

Source checks for Mark 12:

- The chapter's 151 original shared spans were checked against exact
  forms, morphology, and context. Ten remain: the servant's empty state,
  two past possession predicates, the compound from preposition, the
  partiality and human-being idioms, the sit command with its ethical
  dative, and three treasury compounds. Each has a reason in the ledger.
- In 12:1 the opening parables noun is plural. "Leased it" is supported
  by Aphel ahed, "to let out", in the vineyard context; see
  [Brock and Kiraz's entry](https://sedra.bethmardutho.org/lexeme/get/71).
  In 12:3 spiq means empty, rather than sufficient: [CAL cites this verse](https://cal.huc.edu/oneentry.php?cits=all&lemma=spyq+A).
  The servant is sent away empty-handed; the state does not describe
  the senders. In 12:6 the dative belongs to the owner's past possession.
- In 12:10 the stone and its becoming the head of the corner remain
  distinct. In 12:11 "this was from the Lord" is past, while "it is a
  wonder in our eyes" retains the present existential and our-eyes suffix.
- In 12:13 the source names scribes and the household of Herod; no
  different group is substituted from another textual tradition. In
  12:14 the negative care-taking and partiality predicates remain distinct,
  with both alternatives in the tax question. In 12:16 the plural pronoun
  belongs to the people replying, rather than to the inscription.
- In 12:19 the surviving brother is the subject of taking the wife.
  In 12:24 Jesus addresses them; in 12:29 he addresses the individual scribe.
  In 12:25 women being married to men is distinct from taking wives.
  The negative nominal clauses retain their present meaning in context.
- Both 12:30 and 12:33 retain heart, soul, mind, and strength, in their
  respective source orders. In 12:31 "like it" refers to the first
  commandment. In 12:36 "my right" belongs to the speaker, while enemies
  and feet belong to the addressee; the command's ethical dative adds no
  separate beneficiary. In 12:38-40 the desired seats and couches remain
  distinct, and those devouring widows' houses receive greater judgment.
- In 12:42 the two coins are explicitly explained as small coins. The
  rendering keeps that explanation without assigning a specific value;
  compare the broader weight/value gloss in
  [Brock and Kiraz's entry for manya](https://sedra.bethmardutho.org/lexeme/get/1803).
- In 12:41 the watching auxiliary and the rich contributors' casting
  auxiliary retain their separate owners. In 12:44 the past auxiliary
  belongs with what the widow had; the feminine demonstrative begins
  the contrast with the other contributors, and her entire property is
  retained.

Source checks for Mark 13:

- The chapter's 147 original shared spans were checked against exact
  forms, morphology, and context. Two remain: the compound exceptive in
  13:32 and the sudden-arrival idiom in 13:36. Ordinary noun phrases,
  negation, auxiliaries, commands, and coordinated clauses have separate
  source owners; the ledger gives the reasons for the two exceptions.
- In 13:2 both stone nouns and both negative predicates are retained.
  In 13:4 the question includes both when the events happen and the sign
  of their approaching completion. In 13:6 the two first-person forms
  express one claim, "I am he". In 13:8 the place repetition is rendered
  "in place after place", preserving its distributive meaning.
- In 13:9 the delivery verb is active with a plural subject and a
  second-person plural object. The explanatory "for" belongs to delivering
  the hearers to judges; the following synagogues retain their possessive.
  In 13:10 the first-person possessive is retained: "my good news".
  In 13:11 the delivery verb explicitly includes its "you" object, and
  the separate anxiety and premeditation prohibitions precede the contrast
  between the hearers speaking and the Holy Spirit.
- In 13:13 the one enduring to the end is the subject of "will live".
  In 13:14 Daniel is explicitly named; the sign is standing, the reader
  is to understand, and those in Judea are to flee. The roof and field
  commands remain singular, distinct from the plural flight and prayer
  instructions. In 13:19-20 both future and conditional negatives, both
  shortening predicates, and the elect whom he chose remain intact.
- In 13:24-27 the sun, moon, stars, and heavenly powers retain their
  separate predicates; the Son of man sends his angels and gathers his
  elect from the four winds. In 13:28 the branch and leaf possessors
  remain attached to the fig tree, and mata is rendered "has arrived".
  In 13:29 the occurrences are plural, while "it is near" remains
  impersonal. In 13:31 the separate negator belongs to the words not
  passing away.
- In 13:32 the Son is explicitly included before the exception for the
  Father. In 13:33 taking heed, watching, and praying remain three commands.
  In 13:34 the journey, leaving, giving, and commanding verbs are past;
  each person's work remains distinct from the servants' authority.
  In 13:35 the four time alternatives retain their individual owners.
  In 13:36 the sleeping state belongs to the hearers whom he may find.
  In 13:37 the two saying clauses remain distinct: the first addresses
  "you", and the second addresses "you all".

Source checks for Mark 14:

- The chapter's 257 original shared spans were checked against exact
  forms, morphology, and context. Eight remain: relative possession,
  displeasure, availability as capacity, the universal locative relative,
  sorrow with its experiencer, sleep with its ethical dative, the compound
  source preposition, and the immediate-time idiom. The ledger gives each
  expression's specific reason; ordinary negation, auxiliaries, modifiers,
  commands, and clauses have separate source owners.
- In 14:3 and 14:4 the relative "who" belongs separately to its
  d-prefix, outside the shared possession and displeasure predicates.
  In 14:3 "with her" belongs to the preposition and feminine suffix,
  while "had" retains both words of the possession construction.
  In 14:3 the woman opens and pours the vessel's contents, with the
  ointment, choice nard, and price kept distinct. In 14:4 the disciples'
  displeasure has an experiencer dative. In 14:5 the masculine suffix in
  vocalized bēh is rendered "at it", rather than the reference's "at her";
  compare the [vocalized Peshitta text](https://aramaicnewtestament.org/gospel/mark_14.htm).
  In 14:8 the availability clause is rendered "what she could"; the dative
  owns "she", and the following demonstrative identifies this woman.
- The first-person possessives in 14:9 ("gospel of mine") and 14:14
  ("our Rabbi") are retained. In 14:13 the man will meet the hearers;
  carrying the water jar is a separate description. In 14:21 the man
  through whose hand the Son of man is delivered remains distinct from
  the Son, with the counterfactual birth clause retained in full.
- In 14:23 "all" describes the people drinking, rather than how much
  was drunk. In 14:24 the new covenant modifies the blood, and the
  poured-out predicate retains "for many". In 14:30 and 14:72 the
  two crowings and three denials retain their counts.
- In 14:31 "my Lord" is explicit, and the past auxiliary belongs to
  the emphatic saying predicate. In 14:34 the soul is the experiencer
  of sorrow. In 14:36 the separate "my own" and "yours" retain their
  owners. In 14:37 the sleep predicate has its ethical dative, and the
  one-hour watch question has no supplied "with me" from the reference.
  "Have you" belongs to the finite sleep verb's perfect second-person
  morphology; "fallen asleep" retains that verb and its ethical dative.
- In 14:41 the end "has arrived", followed by the distinct
  arrival of the hour. In 14:42 the first-person object belongs to
  handing over, rather than adding a second "to me" after drawing near.
  In 14:44 the sign and the first-person kiss remain distinct from the
  hearers' commands to seize securely and lead him away. In 14:45
  "And" belongs to the waw prefix; "immediately" retains both lexical
  words of the bar-shateh idiom.
- In 14:49 the Scriptures' fulfillment clause retains its own predicate.
  The young man and linen cloth in 14:51-52 remain distinct from the
  fleeing disciples. In 14:53 Caiaphas is explicitly named. In 14:55
  the assembly retains its possessive. In 14:56 the testimonies are
  plural and do not agree; 14:59 instead has singular collective testimony.
- In 14:58 the quoted destruction and rebuilding remain the witnesses'
  reported claim. In 14:62 "I am", the Son of man, the right of power,
  and the clouds of heaven retain their individual owners. In 14:65
  spitting, face covering, maltreatment, the prophecy command, and the
  guards' cheek strikes remain separate actions. In 14:70 "also"
  qualifies the Galilean claim, not the preceding claim of membership.
  In 14:72 the recalled saying and the beginning of weeping remain distinct.

Source checks for Mark 15:

- The chapter's 150 original shared spans were checked against exact
  forms, morphology, and context. Five remain: the accusation and greeting
  idioms, the curtain compound, and two retrospective temporal expressions.
  The ledger gives each reason; ordinary modifiers, auxiliaries, negatives,
  commands, and clauses retain separate source owners.
- In 15:1 the three with-phrases distinguish elders, scribes, and the
  whole council. In 15:3 akhel qartse is the accusation idiom; [CAL's
  eat entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=%29kl+V)
  expressly lists this accusatory construction. In 15:11 the release verb
  has Pilate as its singular subject, not the crowds. In 15:15 mnaggad
  is the passive scourged state of Jesus, not an active claim that Pilate
  personally scourged him.
- Deyn can introduce a narrative topic without a strict contrast. The
  "Now" and "And" renderings retain that discourse function; [Brock and
  Kiraz and Costaz](https://sedra.bethmardutho.org/lexeme/get/690)
  attest these broader senses beyond the export's shorter but/yet glosses.
- In 15:18 the conventional greeting remains distinct from its quoted
  "Hail". In 15:21 Simon, his Cyrenian description, his two named sons,
  and carrying the cross remain distinct. In 15:23 the myrrh is mixed
  in the wine, which Jesus does not take. The third, sixth, and ninth
  hours remain explicit without conversion to modern clock times.
- In 15:32 the title is "king of Israel", rather than the local Murdock
  reference's "king of the Jews"; the descent clause is third-person
  volitional, rendered "Let the Messiah ... come down". The people
  crucified with him remain a separate group reviling him.
- In 15:34 the quoted cry and its explicit English interpretation remain
  distinct. In 15:35 the past auxiliary belongs to "they were saying";
  calling for Elijah is a separate past predicate. In 15:36 the runner
  is singular, while those saying "let us see" are plural.
- In 15:38 the curtain expression appay tara is rendered as an idiom;
  compare [Brock and Kiraz's dictionary entry](https://sedra.bethmardutho.org/lexeme/get/6121).
  Its tearing, two parts, and top-to-bottom extent have separate links.
  In 15:39 "this man" is explicit in the centurion's statement.
- In 15:40-41 the women, family relationships, following and service,
  and the many others coming to Jerusalem remain distinct. In 15:43
  taking courage and going in to Pilate remain separate actions. In
  15:44 the already and some-time-before expressions retain their
  retrospective sense without adding an exact elapsed duration.
- In 15:46 Joseph buys a linen sheet, without an added quality adjective;
  the lowering, wrapping, placing, and rolling predicates remain distinct.
  In 15:47 the understood mother relationship identifies the Mary already
  named as Joses' mother in 15:40; it introduces no new person or kinship.
  The women see where he was laid, preserving the source's locative.

Source checks for Mark 16:

- The chapter's 72 original shared spans were checked against exact
  forms, morphology, and context. Five remain: three tomb compounds,
  the rising predicate with its ethical dative, and the temporal compound
  "after". The ledger records their exact positions and reasons.
- In 16:1 Mary the mother of James and Salome remain separate members
  of the three-woman list. In 16:3 the rolling verb is morphologically
  perfect; "will roll away" renders the prospective question in context,
  rather than claiming a future inflection. In 16:4 the women see the
  stone already rolled away.
- In 16:8 astonishment and trembling seize the women, who remain the
  plural recipients of the singular predicate. The participle's active
  holding sense is supported by [CAL's aḥīd entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=%29xyd+A);
  "had seized them" preserves these roles in English.
  The negative and indefinite spans jointly spell "nothing".
- In 16:10 the first past auxiliary belongs to
  those who had been with Jesus, and the second to their mourning.
- In 16:11 the feminine plural saying, recipient, and object forms are
  retained as the reporting women; no individual women are identified
  beyond what the source states. In 16:13 the additive negative is
  rendered "did not believe them either".
- In 16:15 "my good news" is explicit. In 16:16 the living and condemnation
  predicates retain the source's present formulation.
- In 16:19 and 16:20 "our Lord" retains its first-person possessive.
  In 16:20 the Lord was helping and confirming their words; the signs
  were being performed by the plural actors.

Source checks for Luke 1:

- The chapter's 199 original shared spans were checked against exact
  forms, morphology, and continuous English. Twelve remain, with precise
  reasons in the ledger: compound prepositions, the conception and greeting
  predicates, the human-beings compound, narrative and existential copulas,
  Gabriel's relative-clause agreement, and the sign-making and mute-state
  constructions. Ordinary auxiliaries, negatives, names, and modifiers
  have separate source owners.
- In 1:1 both independent pronouns and the convinced predicate are
  first-person plural. In 1:4 the instruction concerns the preceding words.
- In 1:2 the ministers serve the word; the anticipatory feminine suffix
  refers forward to that noun. In 1:3 both "all of them" and "everything"
  are retained, as is the distinct ordering phrase "in its order".
- In 1:8 the source names God, rather than the local Murdock reference's
  Lord. In 1:15 the Holy Spirit is explicit, and the temporal phrase is
  "while he is in the womb of his mother".
- In 1:19 the first-person speaker is Gabriel; the English relative
  "who stands" still describes him. In 1:20 the causal preposition belongs
  to "because", not to the preceding happening predicate. The negation,
  belief verb, and "these words of mine" remain distinct.
- In 1:22 the cognate infinitive and participle describe one sign-making
  activity; the auxiliary and recipients are separate. The circumstantial
  mute state remains the complement of his remaining.
- In 1:28 "Our Lord" and in 1:43 "my Lord" retain their distinct possessives.
- In 1:31 the receive-plus-conception predicate means "you will conceive".
  The noun has no locative prefix or possessive suffix; [CAL's bṭn entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=b%2Bn+N)
  lists conception and pregnancy in Syriac. In 1:36 "and this" introduces
  the sixth month, with Elizabeth as the recipient of the time expression.
- In 1:39 the adverb is rendered "with diligence", following the careful
  and diligent senses in [SEDRA's lexical entry](https://sedra.bethmardutho.org/lexeme/get/368).
- In 1:40 the asking-peace construction is the greeting idiom, with
  Elizabeth separately linked. In 1:58 the source names God as the one
  magnifying his mercy, rather than the reference's Lord. Neighbors,
  family members, and their rejoicing remain distinct.
- In 1:66 the existential and past copulas jointly express the past
  location of the Lord's hand. In 1:80 the suffixed existential and past
  copula have separate English "he" and "was" spans.
- In 1:72 the holy covenants are plural, as are the oaths in 1:73.
- In 1:77 the forgiveness phrase is introduced by "through", rather than
  an added conjunction. In 1:78 the visitation is future and explicitly
  includes its first-person object: "will visit us". Both mercy nouns are
  retained, and the dayspring is the subject of that visitation.

Source checks for Luke 2:

- The chapter's 182 original shared spans were checked against exact
  forms, morphology, and continuous English. Eleven remain, with exact
  reasons in the ledger: compound adverbial and prepositional expressions,
  two existential past predicates, two anticipatory objects, and three
  predicates with ethical datives. Ordinary names, numbers, modifiers,
  auxiliaries, negatives, and clause constituents remain separately linked.
- In 2:4 the house and family-line suffixes anticipate David, rather than
  introducing separate possessions of Joseph. The suffixed existential
  and past copula have distinct English "he" and "was" spans.
- In 2:9 the angel is explicitly "of God", while the following glory is
  "of the Lord". In 2:10 the announced joy is for all the world.
- In 2:13 men shly is the suddenness idiom; the plural hosts praise God.
  In 2:15 the reciprocal "with one another" and extent "as far as Bethlehem"
  have separate source owners. In 2:17 the repeated about-him/about-the-boy
  expression introduces one referent, not an additional person.
- In 2:14 the source says "good hope", which is retained alongside peace.
- In 2:22 the purification has a plural possessor, "their". In 2:27 the
  parents are explicit; the anticipatory object and the name Jesus identify
  the same child. In 2:41 the source instead uses "his people".
- In 2:25 the upright predicate uses the just/honest sense documented in
  [CAL's kēn entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=k%29n+A),
  beyond the export's shorter homograph glosses. The consolation belongs
  to Israel, and the Holy Spirit is the subject of being upon Simeon.
- In 2:29 the release request is imperative, with "you, my Lord" as its
  explicit address and the servant as one resumptive object. In 2:30 the
  object is "your mercy"; in 2:35 the piercing weapon is a spear.
- In 2:36 the seven years with Anna's husband are explicit. In 2:37 the
  eighty-four-year wording is retained without resolving its age/duration
  ambiguity; both fasting and prayer are linked separately.
- In 2:43 and 2:45 the dative following "returned" is idiomatic, without
  an added external recipient. The dative with "remained" in 2:43 likewise
  belongs to Jesus' remaining in Jerusalem. In 2:44 the company members
  and those who knew the parents remain separate groups. In 2:47 the
  source has his wisdom and his words, rather than the reference's answers.
  In 2:49 the Father's house is explicit, and 2:52 retains both "his stature"
  and "his wisdom".

Source checks for Luke 3:

- The chapter's 160 original shared spans were checked against exact
  forms, morphology, and continuous English. Eight remain: three tetrarch
  titles, three existential-with-dative possession predicates, the prison
  compound, and the copular age predicate. The ledger records exact
  positions and reasons. All genealogy relationship words and names have
  separate source owners, as do ordinary modifiers, negatives, and auxiliaries.
- In 3:1 the repeated head/fourth expression is the tetrarch title.
  In 3:2 high and priesthood are separately linked, and the past copula
  belongs to the word of God. In 3:4 both the plain and the first-person
  plural "our God" are explicit. In 3:5 the future copula carries into
  the parallel elliptical clause about the difficult place becoming a plain;
  that place and its modifier retain separate links.
- In 3:8 "Our father is Abraham" renders the same existential/dative
  possession claim as "we have Abraham as our father". God remains the
  subject of ability and raising, with stones as source and sons as object.
- In 3:9 the tree must produce good fruits; the negated clause retains
  both the fruit noun and its adjective.
- In 3:11 the three possession constructions fuse as English "has";
  the relative prefix and negative indefinite remain separate spans.
- In 3:14 the second prohibition is rendered "do not oppress anyone"; compare
  [CAL's entry for the verb](https://cal.huc.edu/oneentry.php?cits=all&lemma=%28%24q+V),
  which cites this verse under the oppression sense.
- In 3:16 the source has no separate "after me" phrase. The straps and
  sandals are plural, as are the threshing floors in 3:17; the burning
  predicate in 3:17 is future. The active holding sense of aḥīd is supported
  by [CAL's participle entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=%29xyd+A).
- In 3:19 John is the source of the reproof, and Herod is the doer of
  the evils. In 3:20 beth asire is the prison compound, not an additional
  house and a separate group of prisoners.
- In 3:22 the final address is second person, "in you", rather than the
  reference's "in whom". The final first-person perfect has the stative
  English "I delight". In 3:23 the named subject is Jesus himself; the
  existential and past copula jointly supply the age predicate's "was".
  No beginning-of-ministry wording is added.
- The genealogy retains John in 3:27 and Aram in 3:33, rather than the
  local reference's Joanna and Ram. The final relation in 3:38 is
  "from God", not a further explicit "son of" phrase.

Source checks for Luke 4:

- The chapter's 161 original shared spans were checked against exact
  forms, morphology, and continuous English. Eight remain, with exact
  reasons in the ledger: two source prepositions, the exception marker,
  the paired-dative involvement question, the above-her relation, past
  possession, and two departure predicates with ethical datives. Ordinary
  auxiliaries, negatives, names, modifiers, and clause constituents have
  separate source owners.
- In 4:4 the source includes every word of God. In 4:5 Satan is explicit;
  no separate dismissal of Satan is supplied in 4:8. In 4:6 "all" qualifies
  the authority, followed by its glory. The final dative in 4:6 is the
  recipient "to him", resumed by "whom I choose"; the understood gift
  object belongs to "give it", not to that recipient pronoun.
- In 4:4 and 4:22 la hwa is nominal negation in the present statements,
  rather than an instruction to force English past tense. The written/said
  predicates in 4:4, 4:8, and 4:12 retain separate copular links.
- In 4:18 the final verb is rendered "strengthen", following the
  [dictionary entry for sharrar](https://sedra.bethmardutho.org/lexeme/get/3306).
  The brokenhearted, release for captives, sight for the blind, and the
  final forgiveness phrase are all retained; no sins noun is added.
  The adjacent broken/hearted spans jointly spell "brokenhearted".
  In 4:19 the year is "acceptable to the Lord", retaining the preposition.
- In 4:20 the eyes belong to all those in the synagogue, and are the
  subject of gazing. In 4:23 "we heard" and "you did" retain their
  distinct persons within the anticipated local criticism.
- In 4:27 the negative and exception mean none was cleansed except Naaman.
  In 4:29 "their city" and the relative clause ending with "on it" stay
  together, before the distinct purpose of casting him from the cliff.
- In 4:34 "Leave me alone" is singular, but the following question uses
  plural "we". "I know you" retains its explicit object before "who you
  are". In 4:35 the demon throws the man and leaves him without harm.
  In 4:36 the feminine command verb agrees with the word, which is its
  grammatical subject.
- In 4:38-39 the fever leaves the woman, who rises and serves them.
  In 4:40 the people who had sick people remain distinct from the patients
  they bring; the existential, past copula, and dative jointly express
  "had". His hand is singular, and each/one/of-them remain separately linked.
- In 4:41 "also" and both Messiah and Son of God are retained. The final
  clause gives the reason for silencing the demons: they knew he was
  Messiah; compare [Etheridge's translation](https://www.studylight.org/bible/eng/jet/luke/4.html).
- In 4:43 the sending verb is first person, "I was sent", and the preceding
  copula emphasizes the purpose. In 4:42 both departure verbs retain their
  ethical datives without another beneficiary, while "from them" remains
  a distinct source phrase. In 4:44 the source names Galilee.

Source checks for Luke 5:

- The chapter's 138 original shared spans were checked against exact
  forms, morphology, and continuous English. Nine remain, with exact
  reasons in the ledger: two predicates with ethical datives, the people
  and wedding-guests idioms, the boats' anticipatory object, the immediate
  timing expression, the exception and conditional markers, and the illness
  predicate. Ordinary modifiers, negatives, auxiliaries, names, and clause
  constituents have separate source owners.
- The narrative introduction in 5:1 continues into the seeing predicate
  in 5:2. The gathered crowd and Jesus standing beside the lake remain
  distinct from the fishermen washing their nets.
- In 5:3 the boat belongs to Simon Cephas; both the ownership and the
  second name are explicit. The net is singular in 5:4 and 5:6, while
  the fishermen's nets in 5:2 are plural.
- In 5:5 "at your word" introduces Simon's decision to cast the net,
  rather than qualifying the unsuccessful catch. In 5:6 the final
  quantity word intensifies "many" fish, without calling them good fish.
- In 5:7 the object pronoun is explicit before its apposition "both boats";
  the boats are the feminine plural subject of nearing the sinking state.
  In 5:8 the departure imperative has its subject-coreferential ethical
  dative, with "from me" kept separate. In 5:10 "for life" retains the
  phrase with ḥayyē; [CAL's noun entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=xyyn+N)
  records its life sense. In 5:11 the anticipatory object and explicit boats
  name one object, without an additional group.
- In 5:13 the two-word hour expression means "immediately", not an address
  to a son. The explicit final statement that he was cleansed is retained.
- In 5:14 the priests are plural. In 5:17 the healing object is "them";
  in 5:20 "Man" is a direct address to the paralytic. In 5:25 glorifying
  God accompanies the journey home, rather than following it.
- In 5:17 the existential and past copula have distinct English "present"
  and "was" spans. In 5:19 the ascent's ethical dative adds no recipient.
  Both questions in 5:21 remain distinct, and "except" is the fixed ela en
  construction. In 5:26 the report is first-person plural: "We have seen".
- In 5:24 the Son of man has authority to forgive sins; the suffix in
  the title anticipates the following genitive, without a separate "his".
  The source copula and authority predicate have separate English owners.
- In 5:30 the question addresses the disciples in the second-person plural.
  In 5:31 bish avidin is the illness predicate, while the first d-bish owns
  the relative and intensifier "who/very" and the stative participle also
  supplies "are".
- In 5:34 the wedding guests expression and the causative "make fast"
  remain under the negative; adjacent can/not spans spell "cannot".
  In 5:36 the feminine final predicate agrees with the patch: it does not
  fit the old garment. The masculine nominalized new/old expressions
  retain their understood garment head. In 5:37 the en-la conditional
  is "otherwise", with the contrast separately linked. In 5:38 the generic
  plural actively puts the wine into new wineskins; the passive preservation
  predicate is separate. In 5:39 the old wine is pleasant, without an added
  comparative.

Source checks for Luke 6:

- The chapter's 213 original shared spans were checked against exact
  source forms, morphology, and continuous English. Seven remain: the
  accusation, people, hypocrite, and immediate-timing idioms, the coming
  imperative with its ethical dative, and the rhetorical ability question.
  Their exact source positions and reasons are recorded in the ledger.
  Ordinary modifiers, names, negatives, auxiliaries, and clause constituents
  have separate source owners.
- In 6:1 the disciples rub the ears in their hands. In 6:4 the source
  names the bread of the Lord's table; its possessive suffix anticipates
  the Lord, without a separate "his".
- In 6:6 the past auxiliary belongs with teaching before the separate
  man/hand clause. The right hand is explicit. In 6:13 Jesus actively
  names the apostles; 6:14 names Cephas, and 6:16 says Jude son of James.
- In 6:7 eating accusations is the accusation idiom. In 6:8 the coming
  imperative's ethical dative adds no beneficiary. In 6:12 the prayer
  noun's suffix anticipates the following objective genitive: prayer
  to God, without a second standalone possessor.
- In 6:18 people came to hear his word and to be healed; the purpose
  clause is distinct from the following statement that healing was ongoing.
  In 6:26 the source says people, without an added "all".
- In 6:35 the lending command is followed by "do not cut off anyone's
  hope". The kindness clause addresses the evil and then the ungrateful.
- In 6:38 the measure is pressed down and overflowing, without a separate
  shaking action; the recipients' laps are plural. In 6:41 the beam is
  the subject of the passive seeing clause.
- In 6:42 the first casting-out form is first-person singular, following
  "allow": "me to cast out". The later imperative is second-person
  singular. The final seeing predicate retains the addressee as experiencer.
- In 6:45 both stores of treasures are plural, and the lips speak from
  the heart's abundance. In 6:46 both titles retain their first-person
  possessive: "My Lord, my Lord".
- In 6:48 foundations are initially plural, while the final foundation is
  singular. In 6:49 the river beats against the house, which immediately
  falls; the two-word hour expression is not part of the river noun.

Source checks for Luke 7:

- The chapter's 210 original shared spans were checked against exact
  source forms, morphology, and continuous English. Only the illness
  predicate in 7:2 remains shared, with its past auxiliary separate and
  its exact reason in the ledger. Ordinary negatives, modifiers, copulas,
  names, noun phrases, and clause constituents have separate source owners.
- In 7:1 all the words are completed in the people's hearing. In 7:6–8
  the centurion's first-person speech retains "My Lord", "my roof",
  "my servant", his own coming to Jesus, and his command over soldiers.
- In 7:12 the dead man is his mother's only son, and his mother is a
  widow. In 7:15 he sits up and begins speaking; Jesus gives him to his
  mother, without a change of actor.
- In 7:12 the perceived scene becomes an English object clause: he saw
  that they were escorting a dead man. In 7:18 John remains the recipient
  of his disciples' report in the English passive construction.
- In 7:18 John's disciples tell him what happened. In 7:26 prophets are
  plural. In 7:27 "my messenger" has a first-person suffix, while "the way"
  has no possessive suffix. The prophet and Baptizer titles in 7:28 and
  the tax collectors in 7:29 are explicit.
- In 7:25 the relative clause about clothing and luxury and the matrix
  location clause retain separate copulas. In 7:30 "for themselves"
  retains the rejecters' personal reference; [CAL's verb entry](https://cal.huc.edu/cal_entry_web.php?lemma=%2Blm+V)
  records the Syriac spurn/deny sense used for "rejected".
- In 7:36 the Pharisee invites Jesus; Jesus then enters and reclines.
  In 7:37 the feminine demonstrative describes "that city", and the
  masculine demonstrative describes "that Pharisee".
- In 7:41 one debtor owes five hundred denarii and the other fifty.
  In 7:42 both debts are forgiven.
- In 7:41 the creditor expression is split as "debt holder"; the
  existential and past copula also have separate owners. In 7:42 the
  lack of means to repay belongs to the debtors, and the creditor
  forgives both of them before asking which will love him more.
- In 7:44 Jesus says he entered the house; in 7:45 the entry verb is
  feminine, referring to the woman. In 7:46 oil for his head and perfumed
  oil for his feet remain distinct.
- In 7:47 the loving verb is perfect, "she has loved much", with the
  explicit causal connector retained. In 7:50 the causative life verb
  is rendered "has given you life".

Source checks for Luke 8:

- The chapter's 274 original shared spans were checked against exact
  source forms, morphology, and continuous English. Eighteen remain, with
  exact reasons in the ledger: temporal and possession constructions,
  ethical datives, the confrontation formula, and repeated references to
  one participant. Ordinary modifiers, negatives, auxiliaries, names,
  noun phrases, and clause constituents have separate source owners.
- In 8:1 the twelve are explicitly his; in 8:3 the women serve "them",
  plural. In 8:4 people come from all the cities, and parables are plural.
- In 8:10 the mystery is singular. In 8:12 the enemy takes the word from
  their heart so that they may not believe and live. In 8:13 their faith
  is temporary; in 8:15 holding the word and producing fruits are distinct.
- In 8:6 the two-word hour expression is "immediately". In 8:13 the
  anticipatory object pronoun and the explicitly named word identify one
  received object. In 8:16 the spatial relation is split as "on top of".
  In 8:17 adjacent no/thing spans spell "nothing"; in 8:18 the completed
  hearing and the distinction between having and thinking one has remain.
- In 8:19 the family cannot speak with him because of the crowd. In 8:24
  both calls retain "Our Rabbi". The winds, storms, and sea in 8:25 are
  distinct objects of the command.
- In 8:26 the place name is Gadarenes. In 8:28 the initial question uses
  "we", while the plea ends "do not torment me". In 8:29 the man is bound
  and breaks his bonds; the demon leads him into the wilderness.
- In 8:23 the sleep predicate's ethical dative adds no beneficiary. The
  explicit pronoun and Jesus name identify one subject here and in 8:37,
  8:42, and 8:48. In 8:29 the time clause leads into the man's captivity;
  the demon is the captor. The command's anticipatory addressee and the
  spirit noun identify one spirit, with its unclean modifier separate.
- In 8:35 the finding verb's object suffix anticipates the man. His
  demons have gone out; he is clothed, composed, and sitting at Jesus's
  feet. In 8:37 the departure's ethical dative adds no second destination.
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
- The woman introduced in 8:43 remains the subject of approaching in
  8:44. In 8:47 the negative stays with escaping his notice, and the
  reason for touching and the immediate healing have separate owners.
  In 8:49 and 8:53 the death predicates have subject-coreferential ethical
  datives. In 8:52 the finite death verb and the separate sleeping clause
  each supply their own English predicate. In 8:55 the giving verb,
  female recipient, and eating complement have separate source owners.

Source checks for Luke 9:

- The chapter's 250 original shared spans were checked against exact
  source forms, morphology, and continuous English. Twelve remain, with
  exact reasons in the ledger: the exception-condition marker, human
  expressions, the sudden-timing idiom, ethical datives, anticipatory
  objects, and possession constructions. Ordinary modifiers, negatives,
  auxiliaries, names, noun phrases, and clause constituents have separate
  source owners.
- In 9:1 the twelve are his, while the suffix on "all" anticipates the
  demons. In 9:11–12, 9:16, and 9:18 the crowds are plural. In 9:12 the
  villages are around "us", and the disciples say "we are" in the wilderness.
- In 9:3 adjacent no/thing spans spell "nothing". In 9:4 the house's
  resumptive dative supplies the English entry preposition, with the
  house head separate. In 9:12 the day's beginning is a narrative use
  of the homograph, despite its imperative morphology tag. In 9:13 the
  fixed ela en construction is "unless"; the quantities and food remain
  separately linked.
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
- In 9:32 the heaviness predicate's ethical dative adds no second
  experiencer, and its past auxiliary remains separate. In 9:33 hwa is
  a past auxiliary, distinct from the personal pronoun hu; it supplies
  English past do-support under the negative. In 9:36 the completed
  occurrence of the voice is "after the voice had sounded".
- In 9:39 the feminine spirit seizes the child; the masculine predicates
  describe the child crying out, gnashing his teeth, and foaming. The
  feminine subject returns for the spirit's departure and crushing him.
- In 9:39 min shelya is the fixed sudden-timing expression. The masculine
  object in 9:40 refers to the spirit being expelled. In 9:41 and 9:42
  the anticipatory object suffixes and the explicit son/boy noun identify
  one child in each predicate.
- In 9:48 the final demonstrative resumes the least person: "this one
  will be great". In 9:49 the address is "Our Rabbi", followed by "your
  name", "with us", and "after you". In 9:50 both final suffixes are "you".
- In 9:45 the understanding verb's object anticipates "this word";
  later "about it" anticipates "about this word". Each English clause
  names one word, with the verbs, demonstratives, and preposition separate.
  In 9:47 the standing verb's object is the child, while the following
  location suffix refers to Jesus.
- In 9:51 the days being fulfilled are distinct from Jesus setting his
  face toward Jerusalem. In 9:52 messengers are plural, without a numeral.
  In 9:54 the address is "Our Lord", and Elijah is explicitly named.
- In 9:56 souls are not to be destroyed, and the causative life verb is
  rendered "to give life". In 9:58 his head is explicit. In 9:61 the
  household idiom does not identify its members as biological sons.
- In 9:56 the going verb's ethical dative adds no beneficiary or second
  destination. In 9:58 the two possession predicates keep the lairs and
  resting-place complement separate. In 9:62 adjacent plough/share spans
  spell "ploughshare", while the hand, looking, fitness, and kingdom have
  their own source owners.

Source checks for Luke 10:

- The chapter's 173 original shared spans were checked against exact
  source forms, morphology, and continuous English. Ten remain: the
  greeting idiom, three predicates with datives, two repeated references
  to one participant, three exclusion/free-choice constructions, and
  past possession. Their exact positions and reasons are in the ledger.
  Ordinary modifiers, negatives, auxiliaries, noun phrases, names, and
  clause constituents have separate source owners.
- In 10:1 Jesus sets apart another seventy from his disciples and sends
  them two by two. In 10:3 the disciples are the object of his sending,
  rather than recipients of a separate message.
- In 10:6 their peace rests on the son of peace or returns to them.
  In 10:7 the worker is worthy of his wage; these nouns remain distinct.
- In 10:4 asking after peace is the greeting idiom, with the negative
  separate. In 10:10 the going-out imperative's dative adds no beneficiary;
  in 10:11 the nearness predicate's feminine dative refers to the kingdom,
  while the second-person destination remains separate.
- In 10:12 and 10:14 the judgment expressions are comparative: more
  tolerable for one city than another. In 10:13 the conditional context
  gives the time expression its "long ago" sense.
- In 10:13 the actual and counterfactual mighty deeds have separate
  occurrence predicates. In 10:15 the relative prefix and limit stem in
  d-adma separately own "who" and "up to"; the later limit is distinct.
- In 10:16 rejecting the disciples means rejecting Jesus, and rejecting
  him means rejecting the one who sent him. In 10:17 the address is
  "Our Lord". In 10:18 Satan is named; 10:19 instead says the enemy.
- In 10:18 the anticipatory object and Satan name identify one entity.
  In 10:22 both exclusions use ela en, while the conditional relative
  supplies "whomever"; the choosing, revealing, and recipient preposition
  remain separate. In 10:26 the pronoun and Jesus name identify one speaker.
- In 10:21 the Holy Spirit is explicit and both addresses say "my Father".
  In 10:23 the eyes have no "your" suffix. In 10:28 "this" is the object
  of "do", followed by "you will live".
- In 10:30 the robbers fall upon the man, and little life remains in him.
  In 10:34 wine precedes oil, and his donkey is explicit. In 10:35 "when"
  begins the return clause after spending anything more; in 10:36 the
  hands of the robbers are explicit.
- In 10:34 the care predicate's dative adds no new participant; its
  separate patient remains "of him". In 10:35 the giving verb expresses
  repayment in the context of further expenses, with the return clause
  kept separate from the spending clause.
- In 10:39 Mary listens to the words of "our Lord", while Martha says
  "My Lord" in 10:40. In 10:42 Mary's portion is good, without an added
  comparative, and it will not be taken from her.
- In 10:39 past possession is "she had", with the possessor and sister
  separate. English "seated herself" keeps the sitting verb and its
  coreferential dative separate through ordinary reflexive valency.

Source checks for Luke 11:

- The chapter's 238 original shared spans were checked against exact
  source forms, morphology, and continuous English. Eleven remain:
  possession, a resumed subject, two predicates with datives, three
  generic human designations, two compound prepositions, the hypocrite
  designation, and the accusation idiom. Their exact positions and
  linguistic reasons are in the ledger. Ordinary negatives, auxiliaries,
  modifiers, noun phrases, and clause constituents have separate owners.
- In 11:1 the address is "Our Lord". In 11:2 the prayer explicitly has
  "Our Father in heaven" and the petition for his will on earth as in
  heaven. In 11:3 the bread is "of our need"; in 11:4 sins are forgiven,
  while the following clause names everyone indebted to us.
- In 11:5 the visitor has a friend and goes to him; "midnight" keeps
  the half/middle and night words separately linked. In 11:6 "nothing"
  separates the negative existential from the something noun, and 11:7
  "cannot" separates ability from negation. In 11:8 the final clause
  expresses how much the visitor needs, rather than another asking verb.
- In 11:11-12 the rhetorical question particle owns English "would",
  while the offering verb and its objects remain separate. In 11:13 the
  pronouns before and after the qualifying clause identify one subject;
  the copula, evil quality, knowing, gifts, and giving stay separate.
- In 11:12 `bartha` means egg in this context, despite the incomplete
  supplied glosses. The [Comprehensive Aramaic Lexicon's egg sense](https://cal.huc.edu/cal_entry_web.php?lemma=brh+N)
  gives Syriac textual examples, including a stork egg and a bird from
  its egg. The source word is retained without substituting another form.
- In 11:15 the demonstrative means "this one", the subject casting out
  demons. In 11:19 the sons cast them out "by what means", and those sons
  become the judges. In 11:20 the finger of God is explicit.
- In 11:20 the kingdom's feminine ethical dative adds no participant,
  while the destination "to you" remains separate. In 11:23 the two
  copulas are distinct, and the scattering infinitive supplies emphasis
  separately from the scattering predicate.
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
- In 11:44 the relative prefix on the negative form owns English "that";
  the passive participle separately owns "are" and "known".
- In 11:52 keys are plural, and taking, entering, and hindering are past.
  In 11:53 Jesus is actively saying these things; Pael `tks` means
  contradict/rebut, as documented in the [CAL entry](https://cal.huc.edu/cal_entry_web.php?lemma=tks+V).
  In 11:54 the final two-word idiom means accusing him, rather than eating.
- In 11:38 the first/before adverb remains separate from the compound
  temporal preposition before the meal. In 11:53 the displeasure predicate
  and its experiencer dative refer to the already named scribes and
  Pharisees; growing angry and contradicting remain separate predicates.

Source checks for Luke 12:

- The chapter's 250 original shared spans were checked against exact
  source forms, morphology, and continuous English. Ten remain: four
  idioms or lexical designations, possession, a temporal preposition,
  a passive predicate with its patient, and three repeated references
  to one participant. Their exact positions and linguistic reasons are
  in the ledger. Ordinary negatives, auxiliaries, modifiers, noun phrases,
  and clause constituents have separate source owners.
- In 12:2 both covered and hidden things are explicit. In 12:3 speech
  in darkness is heard in the light, while whispering in chambers and
  ears is proclaimed on rooftops. In 12:4 the speaker says "I" and
  addresses "my friends".
- In 12:1 the taking-and-faces expression is hypocrisy. In 12:4 the
  negative existential and possessor dative express "have", with "no"
  and "thing" separate. In 12:5 "after" is the compound men batar;
  the relative prefix on men separately owns "who".
- In 12:7 the sparrows belong to the value comparison, not the fear
  prohibition. In 12:10 both forgiveness clauses retain their recipient.
  In 12:11 the defense expression is rendered as a coherent idiom.
- In 12:15 Jesus explicitly addresses his disciples. In 12:16 the land
  yields produce. In 12:19 the rich man's speech remains first person,
  "I will say to my soul"; the requirement of his soul in 12:20 does not
  identify the unspecified requiring agents.
- In 12:6 "not one" preserves the universal negative. In 12:9 the passive
  denial's pronominal patient refers to the already expressed subject.
  In 12:20 the fronted soul and resumed feminine object identify the
  same patient, expressed once after the active plural requirement verb.
- In 12:25 the measure is a cubit of stature. In 12:28 the grass falls
  into the oven. In 12:30 "you too need" preserves the source's emphasis
  on the disciples' need, rather than adding "also" to the Father's knowing.
- In 12:35 the loins are girded and the lamps shine. In 12:36 the source
  specifies a feast. In 12:37 the master girds himself, has the servants
  recline, and serves them. In 12:39 the final past auxiliary belongs to
  not allowing the break-in.
- In 12:18 and 12:24 "storehouses" keeps the store and house words
  separately linked. In 12:30 the two references to the disciples remain
  distinct as the topic "as for you" and the subject of "you too need".
  In 12:37 the relative and resumed object pronouns jointly own "whom";
  the master finds the servants awake, then serves them.
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
- In 12:41 the anticipatory object and parable noun identify one spoken
  object; "this" and both alternatives' recipients remain separate.
  In 12:49 "by now" keeps the temporal preposition and time word separate,
  within the wish. In 12:56 the hypocrite designation is the taking-and-faces
  idiom; in 12:58 house-of-bound-people is the fixed prison designation.

Source checks for Luke 13:

- The chapter's 156 original shared spans were checked against exact
  source forms, morphology, and continuous English. Seven remain: two
  past-possession constructions, the singular hypocrite designation,
  two repeated references to the callers, and two departure predicates
  with subject-coreferential datives. Their exact positions and linguistic
  reasons are in the ledger. Ordinary negatives, auxiliaries, modifiers,
  noun phrases, names, and clause constituents have separate source owners.
- In 13:4 the comparison uses "sinners". In 13:3 and 13:5 the warnings
  address the whole audience; 13:5 explicitly says
  "like them".
- In 13:6–9 the fig tree belongs to a man and is planted in his vineyard.
  The three years, cultivation, and manure remain explicit. The positive
  fruit condition in 13:9 is elliptical; "next year" belongs to the
  following cutting-down clause.
- In 13:6 the possessor is separately linked as "a man"; the existential
  and past copula jointly express "had". In 13:11 the corresponding
  construction also includes the possessor dative, since its relative
  subject already identifies the woman. The relative prefix stays separate.
- In 13:11 the infirmity has lasted eighteen years. In 13:13 his hand
  is singular. In 13:15 the address is the singular "Hypocrite", and the
  animal-release example is a negative rhetorical question. In 13:16
  the source names the Accuser and repeats the eighteen years.
- In 13:17 the opponents are ashamed, while all the people rejoice at
  the wonders. In 13:19 the bird is singular and the tree is large.
  In 13:21 all three measures of flour are leavened.
- In 13:14 the instruction to come and be healed is directive, while
  the negative Sabbath restriction remains separate. In 13:15 "each"
  and "one" separately render the distributive repetition. In 13:17
  the speech, opposition, shame, rejoicing, and occurrence predicates
  keep their own past auxiliaries and participants.
- In 13:23 the asking verb has an explicit "him" suffix. In 13:25 both
  calls say "Our Lord"; the master's answer is followed by his own
  first-person "I tell you". In 13:26 the visitors ate and drank, but
  the master is the subject of "you taught".
- In 13:25 and 13:27 the anticipatory object of knowing and the subject
  of the origin question identify the same callers. English expresses
  each pair once as "you"; knowing, origin, and copula remain separate.
  In 13:27 and 13:31 the departure datives introduce no beneficiary;
  "from me" and "from here" remain separately linked.
- In 13:27 the phrase is "workers of falsehood". In 13:29 the direction
  order is east, west, south, north. In 13:32 the fox is "this"; the
  completion predicate does not supply an additional work or death noun.
- In 13:33 today's and tomorrow's activity is distinct from going on
  another day. In 13:34 the sent ones go "to her", before the address
  shifts to "your children". In 13:35 the house is left "to you", and
  the final coming is in the name of the Lord.

Source checks for Luke 14:

- The chapter's 131 original shared spans were checked against exact
  source forms, morphology, and continuous English. One remains: the
  existential and possessor dative jointly express "has" in 14:35,
  with its exact positions and linguistic reason in the ledger.
  Ordinary negatives, auxiliaries, modifiers, noun phrases, names,
  and clause constituents have separate source owners.
- In 14:1 the leaders and Pharisees are plural. In 14:2 the gathered-water
  expression describes fluid retention. In 14:5 the rescue example says
  "son or ox", and both drawing out and bringing up remain explicit.
- In 14:2 retaining fluid describes the man's condition; the gathering
  participle, past copula, and fluid noun stay separately linked.
  In 14:3 the question is reported with "whether", retaining its source
  question marker separately from the lawful predicate and healing verb.
- In 14:6 the answer belongs to the give-him clause. In 14:7 the seeing
  clause retains its past auxiliary. In 14:8 the invitation is to a feast;
  no wedding is specified. In 14:9 the place has no possessive modifier.
- In 14:10 the address is "My friend", and moving higher is followed by
  reclining. In 14:12 the invitation warning includes "not even" the
  brothers, and the final demonstrative refers to the repayment.
- In 14:8 and 14:10 English "recline yourself" keeps the posture verb
  and its coreferential dative separate through ordinary reflexive
  valency. In 14:10 "you will have honour" separately links the possessor,
  future predicate, and honour noun; no further beneficiary is introduced.
- In 14:15 one of those reclining hears these things and speaks to him.
  In 14:17 everything is ready "for you". In 14:18 all begin "as one";
  the field's buyer is obliged to go, rather than the field compelling him.
- In 14:19 the purchase is five pairs of oxen. In 14:20 taking a wife is
  the marriage idiom. In 14:22 the command has been fulfilled. In 14:23
  the route goes between the hedges. In 14:24 the negative covers every
  previously invited man, rather than only one member of the group.
- In 14:18 "as" and "one" separately render the unity expression.
  In 14:18-19 the excuse requests retain the asking, first-person
  speakers, addressee, allowing, and excused state. In 14:20 the marriage
  idiom remains "I have taken a wife", with verb and wife separate.
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
- In 14:35 the generic subject already identifies the possessor of ears,
  so the dative introduces no other participant. Ears, the hearing purpose,
  and the final hearing command remain separate from "has".

Source checks for Luke 15:

- The chapter's 115 original shared spans were checked against exact
  source forms, morphology, and continuous English. Six remain: four
  possession constructions, the temporal preposition "after", and the
  citizen designation. Their exact positions and linguistic reasons
  are in the ledger. Ordinary negatives, auxiliaries, modifiers, noun
  phrases, names, and clause constituents have separate source owners.
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
- In 15:4 and 15:8 the existential and possessor dative jointly express
  "has"; the relative prefixes separately own "who". In 15:7 the single
  comparison particle owns the separated English "more" and "than";
  repentance, its negative requirement, and the righteous participants
  retain their own source words.
- In 15:11 Jesus speaks to them again. In 15:12 the portion comes
  "from your house", and the father divides his property among them.
  In 15:13 the younger son is the subject gathering what fell to him,
  not an object gathered by someone else.
- In 15:11 the past-possession construction includes the resumed dative,
  since the fronted man already supplies the possessor. In 15:14 the
  possessor is separately linked as "he". In 15:13 "after" is men batar,
  with the conjunction separate; "son of his" retains the son's suffix.
- In 15:15 the sons-of-the-city expression means citizens; the employer
  sends the son to tend pigs; "the latter" clarifies that subject change.
  In 15:16 the food is carob pods; the longing, pigs' eating, and
  absence of giving all have past auxiliaries.
- In 15:17 "now", "here", and the speaker's hunger remain explicit.
  In 15:19 he says "no longer worthy"; in 15:21 he says "not worthy"
  without repeating the petition to become a hired servant. In 15:20
  the father sees him, has compassion on him, runs, embraces, and kisses him.
- In 15:15 English "attached himself" keeps the attachment verb and
  its coreferential dative separate through ordinary reflexive valency.
  In 15:20 "fell upon his neck" keeps the embracing expression's
  verb, preposition, and possessed neck separate.
- In 15:22 the robe is the best; no quickly adverb is supplied. In 15:23,
  15:27, and 15:30 the animal is a fattened ox. In 15:25 the elder son
  hears many singing, with no dancing term in this source.
- In 15:29 both never clauses remain, together with the labor/service
  expression. In 15:31 the first clause concerns the son always being
  with his father; the second says everything the father owns is the son's.
  In 15:32 the vocalized suitability predicate means it was right for
  them to rejoice, rather than being a negation.
- In 15:29 the two never claims are "not ever" and "not at any time",
  keeping their negatives and temporal expressions separately linked.
  In 15:31 the repeated subject pronoun supplies the English copula,
  while the two ownership forms separately express "of mine" and "yours".

Source checks for Luke 16:

- The chapter's 137 original shared spans were checked against exact
  source forms, morphology, and continuous English. Six remain: past
  possession, the accusation idiom, three stewardship designations,
  and the generic human designation. Their exact positions and linguistic
  reasons are in the ledger. Ordinary negatives, auxiliaries, modifiers,
  noun phrases, names, and clause constituents have separate source owners.
- In 16:1 the accusations are brought to the rich man against the
  steward; the two pronominal references remain distinct. In 16:2 he
  can no longer be the master's steward. In 16:4 leaving the stewardship
  and being received into their houses remain separate clauses.
- In 16:1 akal qarsa is the accusation idiom, with its recipient and
  accused person separately linked. "The latter" identifies the steward,
  distinct from the rich man receiving the report.
- In 16:2-4 rabbat baytutha is the abstract stewardship designation;
  [García González's lexical index, entry 55](https://gredos.usal.es/bitstream/handle/10366/137459/DLE_GarciaGonzalezV.pdf?sequence=1#page=15)
  records the expression as administration. In 16:2 the genitive and
  possessive remain separate; in 16:3 the anticipatory feminine object
  identifies that same office, expressed once in English.
- In 16:6 the oil debt is a hundred measures, changed to fifty, with
  sitting down quickly explicit. In 16:7 the wheat debt is a hundred
  cors, changed to eighty; sitting is repeated without the quickly adverb.
- In 16:8 the subject is explicitly "our Lord". The generation phrase
  retains both "this" and "theirs". In 16:9 it is the mammon that comes
  to an end, and the eternal shelters belong to the receiving group.
- The quotation closes after 16:7 so that 16:8's "our Lord commended"
  remains the narrator's statement. Direct speech reopens at 16:9.
  In 16:9 the two first-person pronouns separately own "I" and "am";
  making, beneficiaries, friends, mammon, its ending, and reception stay
  separately linked.
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
- In 16:26 the relative prefix and from-preposition separately own
  "those" and "from" in the reverse-crossing clause. In 16:28-29 the
  possessors separately own "I" and "they", while the existential owns
  "have". In 16:31 both negatives remain separate from hearing and belief.

Source checks for Luke 17:

- The chapter's 138 original shared spans were checked against exact
  source forms, morphology, and continuous English. Six remain: two
  possession constructions, the thanks idiom and its anticipatory servant
  reference, the return verb's ethical dative, and the anticipatory Jesus
  object. Their exact positions and linguistic reasons are in the ledger.
  Ordinary negatives, auxiliaries, modifiers, and clause constituents
  have separate source owners.
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
  The thanks noun's suffix and the explicit servant identify the same
  recipient, expressed once; the demonstrative and who-did clause remain
  separately linked.
  In 17:10 the final clause states the obligation without adding "only".
- In 17:11 the ethnic term is plural Samaritans, followed by the directional
  prefix on Galilee; the rendering follows the among-Samaritans route
  toward Galilee. In 17:12 ten leprous men meet him and stand at a distance.
- In 17:15 the man turns back and "was praising God"; the dative after
  turning is rendered naturally without adding a receiver. In 17:16
  "this man" was a Samaritan. In 17:17 both ten and nine remain explicit.
  The two copular forms in 17:17 separately own "was it" and "were";
  the relative prefix belongs to the cleansing word.
- In 17:18 `prsh` has the departure sense, as in
  [CAL's Syriac examples under G 3](https://cal.huc.edu/cal_entry_web.php?lemma=pr%24+V).
  The question follows Murdock's contextual gone-away/without-coming
  construction, with this man from a foreign nation as the exception.
  The opening `lma` introduces the question; it does not require "why"
  here, as discussed for this [interrogative particle](https://eprints.glos.ac.uk/3339/1/292355.pdf),
  p. 147, citing Payne Smith and Nöldeke. In 17:19 the faith has given him life.
- In 17:20 the Pharisees ask Jesus, and the kingdom does not come with
  observation. The asking verb's anticipatory object suffix identifies
  the explicitly named Jesus. In 17:21 it is "within you". In 17:22 the
  requested sight is one of the days; no plural object is supplied to the
  final seeing verb. In 17:23 no following-them command is added. In 17:24 lightning
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

- The chapter's 155 original shared spans were checked against exact
  source forms, morphology, and continuous English. Thirteen remain:
  the weary and two sorrow predicates with their datives, the legal
  adversary and patience idioms, negative concord, three repeated
  references, exclusion, possession, the generic human designation,
  and the immediate-time idiom. Their exact positions and linguistic
  reasons are in the ledger. Ordinary negatives, auxiliaries, modifiers,
  and clause constituents have separate source owners.
- In 18:1 the parable is also told to them, without an added demonstrative;
  the final dative belongs to growing weary, not being weary of them.
  In 18:3 the widow was coming to him in that city. In 18:5 tiring the
  judge and subsequently hurting him use distinct verbs.
  In 18:4-5 the concession reads "though ... yet because", following
  Murdock's clause sequence; [CAL's apn entry](https://cal.huc.edu/cal_entry_web.php?lemma=%29pn+c)
  records both even-if and nonetheless usages.
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
  The repeated negatives before willing and after even have one scope,
  rendered "not even willing"; even, willing, eyes, raising, and the
  past auxiliary retain their separate owners.
- In 18:14 the comparative explicitly has more/than, with justified
  belonging to the tax collector's descent to his house. The comparison
  names that Pharisee. The two following maxims retain their reflexive
  subjects and opposite humbling/exalting results.
- In 18:15 they were also bringing infants for Jesus to touch them;
  the disciples are his. In 18:16 the kingdom is of heaven, while in
  18:17 it is of God. In 18:18 one of the chiefs asks him, then speaks
  to him. In 18:20 killing precedes adultery, stealing, false testimony,
  and honoring father and mother.
  In 18:16 the two copulas separately own "is" and "are", and the
  possessive construction owns "the possession of those". In 18:21 the
  fronted demonstrative and repeated object pronoun identify the same
  commandments, expressed once as "these".
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

- The chapter's 172 original shared spans were checked against exact
  source forms, morphology, and continuous English. Nine remain:
  five repeated pronominal references, three possession constructions,
  and the ascent verb's ethical dative. Their exact positions and
  linguistic reasons are in the ledger. Ordinary negatives, auxiliaries,
  modifiers, numeral ratios, and clause constituents have separate owners.
- In 19:2 Zacchaeus is rich and chief of the tax collectors. In 19:3
  the stature clause repeats his name. In 19:4 he runs ahead of Jesus
  and climbs the sycamore fig tree to see him. In 19:5 the suitability
  predicate means he must be in Zacchaeus's house today, not a negative.
- In 19:7 they were murmuring; the final expression concerns entering
  to lodge with a sinful man. In 19:8 half of his possessions go to
  the poor, and the repayment is fourfold. The source's "one" and "four"
  belong to that multiplier, not an additional defrauded person.
  In 19:9 life comes to this house, and this one also is Abraham's son.
- In 19:7 position 10's `על` is read as the entering verb, followed by
  position 11's lodging verb in the serial construction. The export tags
  the homograph as a preposition, unlike the same entering form in 19:1
  and 19:45. The clause context supports the verbal reading; the
  [Peshitta text](https://www.peshitta.eu/nt/luke/19.html)
  confirms the two-word sequence. In 19:8 the rate is "four for one",
  with both numerals and the rate preposition separately linked.
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
  The resumed recipient is expressed once as "to everyone"; the negative
  possession's relative, auxiliary, and negative remain separate from
  the shared possession predicate.
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
  In 19:42 the dative pronoun supplies English "they" for the hidden
  things, with the hiding predicate separately linked and no new receiver.
- In 19:45 both buying and selling occur in the temple. In 19:46 the
  house is a house of prayer, without a verb meaning call. In 19:47
  the chief priests, scribes, and elders seek to destroy him. In 19:48
  both doing something to him and hearing him retain their object suffixes.

Source checks for Luke 20:

- The chapter's 166 original shared spans were checked against exact
  source forms, morphology, and continuous English. Seven remain:
  two generic human designations, two repeated servant references,
  the architectural corner designation, the partiality idiom, and one
  possession construction. Their exact positions and linguistic reasons
  are in the ledger. Ordinary negatives, auxiliaries, modifiers, and
  clause constituents have separate source owners.
- In 20:1 the chief priests, scribes, and elders confront Jesus while
  he is teaching and preaching. The English has those leaders stand up
  against him, preserving their roles with separate source owners.
  In 20:4 John's baptism is contrasted as from heaven or from people.
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
  In 20:21 the negative, subject, and auxiliary remain outside the
  partiality idiom. In 20:24 the writing form is plural, rendered
  "inscriptions". In 20:28 the possessive clitic in "a man's brother"
  belongs to the brother word's suffix, with the man's stem separate.
- In 20:28 the brother who dies has the wife without children, and his brother
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
  The nested quotation marks distinguish David's "my Lord" from the
  Lord's "my right hand"; the seating imperative and its coreferential
  dative separately own "seat" and "yourself".
- In 20:44 David calls him my Lord. In 20:45 the disciples are his.
  In 20:46 robes have no added long adjective, while marketplaces,
  synagogues, and suppers are plural. In 20:47 widows' houses, the
  pretext of prolonging their prayers, and greater condemnation remain.

Source checks for Luke 21:

- The chapter's 141 original shared spans were checked against exact
  source forms, morphology, and continuous English. Ten remain:
  three repeated references, three nearness predicates with their ethical
  datives, the prison and defense compounds, the expression for wringing
  of hands, and the adverb "suddenly". Their exact positions and
  linguistic reasons are in the ledger. Ordinary negatives, auxiliaries,
  modifiers, and clause constituents have separate source owners.
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
  21:12 the delivery to councils and prison precedes being brought
  before kings and governors, because of my name.
- In 21:14 the answer/defense expression is a compound idiom; its
  component spirit word does not introduce a separate actor or means.
  [CAL's npq entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=npq+V)
  records the defensive-response sense with ruha. The learning/preparing
  word remains outside that compound. In 21:15 the relative and resumed
  feminine object of "against it" identify the same wisdom, expressed
  once as "which"; the preposition, ability, and negative stay separate.
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
  The expression for wringing of hands combines the abstract anxiety noun
  with hands; the anxiety noun is not independently assigned a physical
  wringing action. The conjunction remains separate from this expression.
- In 21:27 the clouds are plural. In 21:28 deliverance belongs to the
  hearers. In 21:29 Jesus told the parable. In 21:30 the sprouting
  trees supply the inference about summer. In 21:31 these things
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

- The chapter's 259 original shared spans were checked against exact
  source forms, morphology, and continuous English. Seven remain:
  the compound after, two repeated personal references, the prison
  designation, the denial construction with its negative knowledge
  complement, and two existential-dative possession predicates.
  Their exact positions and linguistic reasons are in the ledger.
  Ordinary negatives, auxiliaries, modifiers, and clause constituents
  have separate source owners.
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
  authority are called doers of good, with the two parts of the
  benefactor title separately linked. In 22:26 greatness is contrasted
  with being least and leadership with service. In 22:29 the kingdom
  is promised to you as my Father promised to me; in 22:30 eating and
  drinking, sitting on thrones, and judging twelve tribes all remain.
- In 22:31 Satan's sifting object is plural you, while the prayer,
  faith, turning, and strengthening command in 22:32 address Simon
  in the singular. The turning clause is imperative, not a subordinate
  when-you-have-turned clause. In 22:33 the address is my Lord; in
  22:34 Jesus names Simon, with three denials before today's crowing.
  The negative knowledge complement belongs to the denial construction;
  English expresses the repudiation once as denying that he knows Jesus.
  Repeating both deny and not know would reverse that repudiation.
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
  In 22:58 Cephas belongs to the following speech clause; his copular
  denial answers the present claim of membership, "I am not." In 22:61 Jesus,
  Cephas, Simon, and our Lord's word retain their distinct references.
  In 22:63-65 mocking, blindfolding, striking his face, and uttering
  blasphemies retain their ongoing past forms. In 22:68 answering and
  releasing him are distinct and both fall under the negative scope.
  In 22:69 the right hand is of God's power;
  in 22:71 they have heard from his mouth.

Source checks for Luke 23:

- The chapter's 199 original shared spans were checked against exact
  source forms, morphology, and continuous English. Eleven remain:
  two accusation idioms, the prohibition with its negative payment
  complement, two prison designations, the curtain expression, and five
  repeated references. Their exact positions and linguistic reasons are
  in the ledger. Ordinary negatives, auxiliaries, modifiers, and clause
  constituents have separate source owners.
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
  The separate auxiliary and speech verb have separate English owners
  in did say; no prior saying is introduced by a past-perfect rendering.
- In 23:35-37 the people watch while rulers and soldiers mock him;
  the saving verbs preserve their respective objects and reflexives.
  In 23:38 the inscription is above him in Greek, Latin, and Hebrew,
  and reads this is the king of the Jews. In 23:39 a single wrongdoer
  is reviling him among the plural men crucified with him.
- In 23:41 just punishment, deserved recompense, and the other man's
  innocence remain distinct. The passive participle with the l-dative
  permits the active perfect he has done; see [Younansardaroud, section 11.5](https://d-nb.info/1184881278/34#page=107).
  The demonstrative subject and agent dative identify the same man,
  rather than an act being done to him. In 23:40 the resumed in-it phrase
  and explicit in-judgment phrase identify the same sentence.
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
  were nearby and see how his body is placed; his is explicit.
  In 23:53 the ordinary burial-chamber nouns are separately linked,
  while the demonstrative, relative prefix, and resumed locative
  identify the same chamber in English which.
  In 23:56 spices and ointments remain
  distinct, followed by Sabbath rest as commanded.

Source checks for Luke 24:

- The chapter's 191 original shared spans were checked against exact
  source forms, morphology, and continuous English. Four remain:
  rising and day-inclining predicates with their ethical datives,
  the repeated Jesus subject, and negative existential-dative
  possession. Their exact positions and linguistic reasons are in the
  ledger. Ordinary negatives, auxiliaries, modifiers, and clause
  constituents have separate source owners. The burial-chamber nouns
  and the honey and comb components are separately linked.
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
  not believe them; their words appear as mad ravings in their eyes.
  In 24:10 the demonstrative identifies the women who had spoken to
  the apostles. In 24:12 the name is Simon, and the linen sheets
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
  has no added object. In 24:31 recognition and withdrawal are distinct;
  the reflexive English removed himself keeps the verb and dative
  separately linked without adding an intensive himself to the subject.
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
  The Peal clothing verb is rendered actively as put on power.
  In 24:50 Bethany and his lifted hands remain; blessing, separation,
  ascent, worship, return, continual praise, and Amen complete 24:51-53.

Source checks for John 1:

- English uses it/its for the Word, light, and Spirit in this chapter.
  References to human participants, including John and Jesus, retain
  personal pronouns. This includes its glory and its fullness in the
  Word passage and it remained on him for the Spirit resting on Jesus.
- The chapter's 191 original shared spans were checked against exact
  source forms, morphology, and continuous English. Thirteen remain:
  four predicates with ethical datives, three repeated subject
  references, and six relative/resumed references. Their exact positions
  and linguistic reasons are in the ledger. Ordinary negatives,
  auxiliaries, modifiers, and clause constituents have separate owners.
- In 1:1 the Word's being with God and being God retain their distinct
  predicates. In 1:3 both finite predicates and the final relative about
  coming into being remain. Through it renders the hand idiom of agency;
  the same idiom with John as agent is through him in 1:7. In
  1:4 life was in it and is humanity's light; in 1:5 the light shines,
  while the darkness does not comprehend it.
- In 1:6 John is sent from God. In 1:7 the source explicitly includes
  the purpose that everyone might believe through him, although this
  clause is absent from the supplied Murdock reference. In 1:8 John is
  not the light and was to testify about it. In 1:9 the true light
  enlightens everyone coming into the world, with for and the past copula.
- In 1:10 the world came through it and did not know it.
  In 1:11 its own do not receive it. In 1:12 those receiving it are
  given authority to become God's children. In 1:13 the plural birth
  is from God, contrasted with blood, flesh's desire, and man's desire;
  the birth predicate remains with its actual final source position.
- In 1:14 the Word becomes flesh and dwells among us; we see its glory.
  Only-begotten, from the Father, and full of grace and truth remain
  distinct. In 1:15 the coming-after, being-before, and prior-to-me
  expressions keep their respective first-person references. In 1:16
  we all receive from its fullness, and grace in place of grace.
- In 1:17 the source order is truth and grace, after the law given
  through Moses' hand; they come through Jesus the Messiah's hand.
  In 1:18 the recorded title is only-begotten God. He is in his Father's
  bosom and makes the unseen God known, with the understood object
  supplied naturally in English.
- In 1:19 the Jews from Jerusalem are the senders, priests and Levites
  the delegates, and John the recipient of their questions. The sending
  verb, senders, origin, delegates, recipient, and purpose are separately
  linked while preserving those roles in English.
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
  Jesus, Joseph, and Nazareth distinct. The b- prefix is retained as
  "and in the prophets". In 1:51 the future audience is
  plural, with God's angels ascending and descending toward the Son
  of Man; both introductory amens remain.

Source checks for John 2:

- John 2:1 renders משתותא as a wedding feast and preserves Cana as a city of Galilee. The day, ordinal, city, Galilee, mother, Jesus, and final was/there have separate source owners.
- John 2:2 retains the independent subject pronoun beside Jesus. The anticipatory dative and explicit feast complement jointly own “to the feast,” with a ledger exception.
- John 2:3 retains the source's explicit “they have no wine” negative and the mother's address to Jesus. The perfect plus past auxiliary is “had run out”; the negative existential and possessor dative jointly own “have,” separately from “they,” “no,” and “wine.”
- John 2:4 preserves the question “What is that to me and to you, woman?” and the not-yet-arrived hour.
- John 2:6 preserves six stone water jars, their Jewish purification use, and the two-or-three-measure capacity. Existential “there,” past “were,” locative “there,” stone, jars, and six have separate owners; the repeated two/two construction is distributed as “two measures each.”
- John 2:7 retains the anticipatory object as “them—the water jars,” with “with water” linked to its actual water noun. The master and feast nouns in 2:8-9 have separate owners.
- John 2:9 distinguishes the master’s ignorance of the wine's source from the attendants' knowledge and their having filled them with water, without adding a causal conjunction or an unlicensed “jars.”
- John 2:10 renders the source drunk predicate directly and retains the source “then” before the inferior wine. The object suffix of “have kept” is retained as “it—the good wine,” with each lexical word separately linked.
- John 2:15 preserves the reflexive dative in “he made himself a whip,” the singular cord, the expulsion of all named groups, poured-out money, and overturned tables.
- John 2:19-21 distinguishes the Jews' literal temple from Jesus' temple-as-body explanation; “I will raise it up” has no added “again.” The explanatory perfect plus past auxiliary is “he had spoken,” with separate verb and auxiliary owners.
- John 2:20 separates forty, and six, years, temple, this, the building verb, the second-person pronouns, three, days, raising, and its object. The postposed personal copulas in 2:19-20 supply the English auxiliary separately from the raising participle.
- John 2:13 separates the movement verb, Jerusalem, and Jesus at positions 4-6, repeating the verb's owner for its separated conjunction.
- John 2:22 retains plural “Scriptures” and the word spoken by Jesus after the disciples remember his saying. “From,” spatial “among,” and “the dead” have separate owners; the auxiliary belongs to the earlier “had said,” not to the final “Jesus spoke.”
- John 2:23 retains the personal existential's subject, its past copula, Jesus, the narrative particle, Passover, and feast with separate links.
- John 2:24-25 preserves Jesus' refusal to entrust himself, his knowledge of everyone, and the explicit claim that he needs no human testimony. The negation, participle, auxiliaries, pronouns, recipients, and human nouns have separate source owners.

Source checks for John 3:

- John 3:3,7 renders מנ דריש idiomatically as “anew,” rather than mechanically translating its parts as “from the beginning.” Younansardaroud's Classical Syriac Course Book, p. 97, lists /men d-rēš/ as “again” (https://d-nb.info/1184881278/34#page=108).
- John 3:1 separates the existential, past copula, narrative particle, locative there, numeral, man, name, and naming copula. In 3:2 the sent predicate is second-person passive perfect; the negative, person, ability, signs, and their demonstrative retain separate owners. The relative prefix and resumed with-him suffix jointly own “with whom.”
- John 3:4 retains both תוב (“again”) and דתרתינ זבנינ (“a second time”), and the rhetorical possibility of entering the mother's womb, with “enter” linked to its actual source position.
- John 3:5-8 retains birth of water and the Spirit; רוחא is rendered “wind” in the blowing/sound comparison and “Spirit” in its birth application. Both meanings occur in the supplied lexicon.
- John 3:11 preserves the plural knowing, speaking, seeing, testimony, and reception clauses; John 3:12 retains the final first-person object suffix as “believe me,” without duplicating the heavenly-things condition.
- John 3:13 retains the final source clause “who is in heaven”; John 3:15-18 keeps each universal believer clause, the non-perishing/eternal-life contrast, and the already-judged negative explicitly linked.
- John 3:14 retains the destined predicate and passive lifting infinitive separately. In 3:15-16 the plural future existential agrees with life and combines with the singular possessor dative as “may have”; the two possession constructions have ledger exceptions. In 3:16 the giving verb is imperfect, rendered “might give.” In 3:17 the Peal imperfect is “might live,” with the world as the English subject and “through him” separately linked, following Murdock's life wording. In 3:18 the fixed men kaddu expression jointly owns “already.”
- John 3:20 retains causal גיר (“for”) and the reproof of deeds. John 3:22-24 retains Judea, Aenon near Salim, abundant water, ongoing staying/baptizing and coming/being baptized, and John's not-yet-imprisoned chronology.
- John 3:25 identifies one of John's disciples and a certain Jew, rather than changing them into collective groups. John 3:26 retains “Our Rabbi,” testimony about the other baptizer, and “many” going to him.
- John 3:22 retains “Jesus and his disciples,” rather than assigning “with” to the source conjunction. In 3:23 the baptizing, coming, and being-baptized predicates retain their past auxiliaries; the existential and locative words also have separate owners. In 3:24 beth asire is the fixed prison designation, while the fall verb, past auxiliary, not, and yet remain separate.
- John 3:27 renders the own-will/self expression as “by his own will,” with separate preposition, reflexive, and will owners. [CAL's ṣbw entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=cbw+N) records both the will sense and the men ṣbut nafsh construction. The except-if particles jointly own “unless.” John 3:28-30 retains John's denial of being the Messiah, being sent ahead, the bridegroom/friend distinction, emphatically completed personal joy, and the increase/decrease contrast. In 3:28 the inflected negative nominal copula lā hwīt is “am not,” with separate “am” and “not” owners; the perfect-form morphology does not impose past time ([Joosten, section 2.1](https://www.academia.edu/34446491/The_Negation_of_the_Non_Verbal_Clause_in_Early_Syriac)); in 3:29 the existential and possessor jointly own “who has.”
- John 3:31 preserves all three earth expressions and both comparisons as “higher than all,” with each component separately linked. Both coming verbs are perfect, “has come,” and the speech clause retains “from the earth.” In 3:33 the receiving verb is also perfect, “has received.” John 3:34 keeps both causal links and “it was not by measure that God gave the Spirit”, without adding a recipient; the relative prefix and resumed object suffix jointly own “whom.” The existential possession in 3:36 jointly owns “has.”
- John 3:36 renders מתטפיס contextually as “heed”; CAL's Syriac Ct entry explicitly includes “obey, consent” (https://cal.huc.edu/cal_entry_web.php?lemma=pys+V). The final future predicate remains “God's wrath will remain upon him.”

Source checks for John 4:

- John 4:1-4 retains Jesus as the named subject, the comparative baptizing clause, the explicit statement that his disciples rather than Jesus baptized, and the necessity of passing through the Samaritans with the discourse transition.
- John 4:1 separates the making perfect from the baptizing participle. In 4:2 the negative focus and the baptizing predicate retain both HWA forms, with the independent he and Jesus in apposition. In 4:3 the leaving verb's object suffix and explicit Judea remain visible, and the ethical dative supplies the departure's coreferential he. In 4:4 the coming and passing verbs have separate owners, as do through, midst, and the Samaritans.
- John 4:5-8 retains Sychar, Jacob's gift to Joseph his son, the water well, Jesus' weariness from the journey, the sixth hour, and the causal explanation that his disciples had gone to buy food for themselves.
- John 4:5 separates the by/side/field construction and the gift's verb/past auxiliary. In 4:6 the sitting perfect and past auxiliary are “had sat,” with the ethical dative supplying he; both existential/past pairs also have separate owners. The plural source hour-count construction is idiomatically English “the sixth hour,” as the corresponding count in 4:52 is “the seventh hour.”
- John 4:9-12 preserves the Samaritan/Jewish contrast, God's gift, the hypothetical request and living-water answer, the missing bucket and deep well, and Jacob, his sons, and his flocks. The demonstrative at John 4:11,15 stays with “that woman,” distinct from the following address “my lord.”
- John 4:12,29,33 treats למא contextually as an interrogative, without mechanically importing causal “why.” CAL's general-interrogative entry directly cites near-identical Old Syriac John 4:33 as “has anyone brought him something to eat?” (https://cal.huc.edu/oneentry.php?cits=all&lemma=lmh+c).
- John 4:14-18 preserves both water/giver relative clauses, the spring welling up to eternal life, the woman's request, five husbands, and the separate present “not your husband” predicate.
- John 4:11,17-18 retains the existential/possessor constructions as minimal shared “have” spans, separately from the possessor, negative, bucket, or husband. In 4:14 the future “I will give” and later participial “I give” remain distinct; “not ever thirst” separates the negative and temporal expression. In 4:15 the inflected negative-copula carrier supplies I in “and I do not come”; 4:18 likewise retains present “is not your husband,” separately from past “five husbands were yours.” Numeric perfect morphology alone does not determine the tense of these negative nominal constructions.
- John 4:19-26 keeps the mountain/Jerusalem contrast, the plural worship/knowledge contrasts, salvation from the Jews, worship in spirit and truth, the Father's “also,” and the speaker's “I am he” self-identification.
- John 4:20 locates the worship place in Jerusalem, retaining the actual locative prefix. The repeated personal forms in 4:22 supply separately linked subjects and auxiliaries; in 4:23 the Father and resumed he remain in apposition. In 4:25 the coming perfect under when is “has come”; in 4:26 all three first-person forms remain visible as “I am he—I who speak with you.”
- John 4:27-38 retains the disciples' amazement and unasked questions, the woman's testimony, “Our Rabbi,” the food/will/work imagery, four months, fields already ready for harvest, future shared rejoicing, and the causal “for others have labored.”
- John 4:30 keeps coming separate from its past auxiliary. In 4:31 the temporal construction is “during these things,” with separate owners; in 4:34 my/own/food and the work's anticipatory object retain their actual words. In 4:35 the lift command remains imperative, the whitening and reaching predicates are perfect (“have become white” and “have reached harvest”), and men kaddu jointly owns “already.” In 4:38 the relative prefix and resumed on-it reference jointly own “which”; the labor noun's suffix and its resumed possessor jointly own “their.”
- John 4:39-44 preserves the woman's testimony and the Samaritans' own hearing as distinct grounds for belief. John 4:42 keeps the fronted “we” with the hearing clause and retains “the Messiah, the Life-Giver of the world.” John 4:41 says “many,” without adding a comparative “more”; both two-day references and the prophet's own-city statement remain explicit.
- John 4:39 retains that woman and her testifying participle with its past auxiliary. In 4:42 the opening speech is “were saying”; the quoted negative focus is present “it is not because of your word,” and the knowing perfect is “have come to know.” All three facts keep separate source owners.
- John 4:45-50 retains the Galileans' reception after seeing the Jerusalem feast signs, Cana and Capernaum, the royal official's sick son, and the subject “This man” when he hears of Jesus' arrival. The request, signs-and-wonders condition, living-son declaration, belief, and departure remain distinct.
- John 4:45 keeps the past auxiliary with “had come.” The king's servant in 4:46,49 is rendered with separate servant, of, and king owners, and one/that retains its actual position. In 4:47 the past auxiliary belongs to “was begging.” In 4:48 “if,” both negatives, seeing, and believing are separate; translating if as unless while retaining its negative would reverse the condition. In 4:49 the dying verb and subject-coreferential ethical dative jointly own “dies.”
- John 4:51-54 retains the servants' meeting, announcement and saying, the seventh hour yesterday, the fever leaving, the father's recognition and household belief, and the second-sign conclusion after the journey from Judea to Galilee.
- John 4:51 keeps the descending perfect and its past auxiliary as “had come down”; meeting him, bringing him good news, and saying to him retain separate verbs and object suffixes. The life predicates in 4:51,53 describe the son's resulting living state as “is alive.” In 4:53 the father noun retains his possessive suffix, and the hour and its locative relative remain separately linked.

Source checks for John 5:

- John 5:1-5 retains the unnamed Jewish festival, Jerusalem, Bethesda's Hebrew name, five porticoes, the waiting sick, the angel's periodic descent and stirring, the first descent and healing, and the man's thirty-eight years of illness. The source's angel clause is retained in full.
- John 5:2-5 separates existential, past auxiliary, locative there, numerals, and nouns. In 5:3 lying and waiting retain their separate past auxiliaries. In 5:4 the repeated temporal expression jointly owns “from time to time”; the anticipatory pool and water references, compound after, and finite past possession “had” have minimal ledger exceptions, while into, descent, first, stirring, healing, disease, and the possessor remain separate. In 5:5 the personal existential and past auxiliary are “been” and “had,” with thirty, and eight, and years individually owned.
- John 5:4,7 uses “pool” for מעמודיתא; CAL explicitly gives Syriac “basin, pool” alongside baptism/ablution (https://cal.huc.edu/getlex.php?coord=620441037&word=12). The auxiliary after the first descent stays with that predicate, followed by the explicit stirring “of the waters.”
- John 5:6-15 preserves the request to be healed, the lack of a helper and another person's prior descent, the bed/walk commands, the immediate healing, Sabbath objection, and later temple encounter. John 5:13 renders the withdrawal with “himself,” without treating the ethical/reflexive dative as a newly introduced recipient.
- John 5:6 retains the temporal existential “it had been a long time for him,” without assigning an invented illness predicate to the duration words. In 5:7 before and me are separate: only the compound men qdam jointly owns “before.” In 5:9 bar shateh is the fixed “at once” idiom, with its conjunction separately linked. In 5:10 the negative, permission predicate, recipient, carrying, and bed are individually owned. In 5:13 “who it was” ends at position 6: Jesus at 7 is the subject of the following withdrawal clause, not part of “who Jesus was.” The crowd phrase is locative “in the large crowd,” and both past auxiliaries and the relative existential retain their actual owners.
- John 5:16 retains the causal explanation and active-participle-plus-HWA aspect: the Jews “were persecuting” and “were seeking,” because Jesus “was doing these things” on the Sabbath. The initial ד can introduce a causal subordinate clause; see Younansardaroud §14.6 (https://d-nb.info/1184881278/34#page=140).
- John 5:17-19 distinguishes Jesus' short first answer, the narrative reaction, and the ensuing discourse. The reaction's יתיראית remains “all the more,” followed by the Sabbath and Father/equality claims.
- John 5:17 retains the independent he beside Jesus and the separate first-person subject and copular auxiliary. In 5:18 the Sabbath-breaking, saying, and making-equal participles retain their past auxiliaries; the personal existential in the Father claim supplies “he is.” In 5:19,30 the own-will construction has separate by, own, and will owners, and ability, negation, doing, and anything remain separate.
- John 5:19-27 retains the Father/Son working, loving, showing, raising, life-giving, judgment, and honor clauses; the repeated “also” particles and the hearing/believing/passing-from-death clauses remain explicit. Authority “also to execute judgment” is retained at John 5:27.
- John 5:20 retains “his Son,” including the possessive suffix. In 5:22 nominal negative HWA supplies present “does”; the giving verb's object suffix is retained as “it—all judgment.” In 5:24 the ethical dative supplies the coreferential he in “he has passed,” while existential possession jointly owns “has.” In 5:26 the Father's dative prefix and existential jointly own the first “has”; the later existential purpose clause is rendered as the controlled infinitive “the Son also to have life in himself,” keeping the Son as the reflexive's antecedent. In 5:27 the purpose auxiliary, executing, also, and judgment have separate owners.
- John 5:28 begins with the actual stored clause דברה הו דינ דאנשא, rendered “But as for his being the Son of Man,” followed by “do not marvel at this.” It is a clause about the subject, not a vocative addressed to the hearers. The clause stays linked to its actual John 5:28 positions despite the different boundary in the supplied reference; hearing and coming from the graves lead to both resurrection outcomes.
- John 5:28 separates the nominalizer/Son word, personal copula, discourse particle, and of Man; all, those, the relative prefix, are, and in the graves also retain individual owners. Both done predicates in 5:29 are perfect. The negative nominal copulas in 5:31,34 remain present “is not,” despite perfect-form morphology. In 5:32 the testimony clause is “his testimony, which he bears about me, is true,” with separate predicate, subject, copula, and relative owners.
- John 5:30-38 keeps the own-will/sender's-will contrast and the distinct testimonies of the speaker, John, the works, and the Father. John 5:35 renders דתשתבהרונ as “take pride,” consistent with CAL's reflexive “boast, be proud” sense (https://cal.huc.edu/oneentry.php?cits=all&lemma=%24bhr+V).
- John 5:36 retains both first-person datives as “As for me” and “I have.” The relative prefix on gave and the resumed object of complete jointly own “which”; the later “these very works,” doing, and testimony remain separate. In 5:37 not, ever, hearing, voice, seeing, and appearance have separate owners, and in 5:38 the one, whom, he, and sent remain distinct.
- John 5:39 remains the imperative “Search the Scriptures,” with the relative “in which you think you have eternal life” and their testimony about Jesus. John 5:40-44 retains unwillingness to come, eternal life, human glory, God's love, the Father's name, another's own name, and glory from “the only God.”
- John 5:39 records the existential possession as a minimal “have” exception. In 5:40 the future copula and possessor are separately “will be” and “yours.” In 5:41 bnay anasha jointly owns the generic “people,” rather than asserting literal sons. In 5:43 the name word owns both in and name, with my Father's and his own separately linked; 5:44 likewise keeps one, from, and another separate.
- John 5:45 preserves the source's למא as a contextual question, “Do you think,” followed by Moses as accuser and the hope placed in him; CAL allows the general-interrogative use (https://cal.huc.edu/oneentry.php?cits=all&lemma=lmh+c). John 5:46-47 retains both Moses' writing about the speaker and the final writings/words belief question.
- John 5:45 records the two akhel qartsa accusation idioms as minimal shared predicates; who and the object suffix you remain separately linked to their actual words. In 5:46 both B-prefixes remain “in Moses” and “in me,” with the conditional auxiliary separate from believing. In 5:47 the resumed possessor is “the writings of that one,” followed by separately linked my, own, and words.

Source checks for John 6:

- John 6:1-6 retains Galilee/Tiberias, the mountain, the near Jewish Passover feast, Philip, and Jesus' testing question and prior knowledge. The feeding account keeps the two hundred denarii, five barley loaves, two fish, five thousand, and twelve baskets explicit.
- John 6:2-6 separates seeing/sitting/knowing from their past auxiliaries. The independent he in 6:6 belongs to Jesus' knowing clause, distinct from the object him in the testing clause. John 6:7 renders the plural suffix מנהונ as “of them,” referring to each person, rather than “from it”; the repeated little/little and one/one remain “a little bit” and “each one,” with separate owners.
- John 6:9 keeps the existential possession as a minimal shared has, separately from who, the boy and the food. John 6:10 separately links the locative there, in, that and place, and preserves the actual men in the count. John 6:11 keeps from the fish, and 6:12 separates so that, not, a thing and is lost.
- John 6:13 preserves the attachment of the fragments left for those who had eaten “from the five barley loaves”; the from-phrase belongs to the loaves, not to the people. John 6:14-15 keeps the prophet claim distinct from the crowd's intended seizure and kingship.
- John 6:15,17 keeps the coreferential datives with withdrew and grew dark as he and it; the latter belongs to the darkening clause, not an added yet in Jesus' non-arrival clause. The dative's suffix refers back to the verbal subject, without requiring an invented destination or location; compare [Farina on coreferential datives](https://ricerca.sns.it/retrieve/e3aacdfd-ef1d-4c98-e053-3705fe0acb7e/Farina.pdf). The two past auxiliaries in 6:18 remain separately linked to the sea's heaving and the wind's blowing.
- John 6:19 retains the generic travel verb and “about twenty-five or thirty stadia,” rather than importing a rowing verb. John 6:20-21 renders the identity reply “It is I,” willingness to take him into the boat, and the time clause before the boat's arrival; willingness is distinct from gladness.
- John 6:22 retains the sole boat and disciples' boarding, with “except the one” as natural conditional-exception English. Jesus is explicitly the subject who had not entered the boat with his disciples. John 6:23-24 keeps the other boats from Tiberias, the bread/blessing location, and the crowd's movement to Capernaum.
- John 6:21-24 records minimal shared relative/resumed-pronoun references for the destination land, boarded boat and eating place. The temporal and at that/time in 6:21 has separate owners. John 6:22 retains with them—with his disciples as a resumed apposition; in 6:24 the anticipatory him and named Jesus jointly identify the person sought, separate from seeking and its past auxiliary.
- John 6:25,34 retains “Our Rabbi” and “Our Lord.” John 6:27 links “remains” to its food noun and the fronted demonstrative להנא to “For God the Father has sealed this one,” keeping the Son-of-Man/giver and sealing clauses distinct.
- John 6:32-40 preserves Moses' negative giving clause, the Father's true bread, hunger/thirst, the coming/believing promises, the sender's will and loss/raising contrasts, and the last-day raising. John 6:37 uses the plain negative “I will not cast out.”
- John 6:26,42,46,58 retains the present negative nominal copulas rather than forcing past tense from perfect-form hwā. John 6:32,38,70 instead uses did with the completed giving, descent and choosing contexts. John 6:35 separates not, ever and thirst; 6:37 retains the cast-out verb's him suffix. John 6:39 keeps all that was given and of it in the loss clause. John 6:40,47,53-54 records minimal possession predicates, with the modal, negative and possessor separately linked wherever separable.
- John 6:41,51 preserves the explicit first-person נחתת, “I came down,” rather than changing the descent into a third-person bread-relative clause. John 6:42's whose joins the relative prefix and the coordinated father/mother possessive suffixes; the kinship nouns, we and know remain separately linked. Its later this one remains explicit.
- John 6:45 retains the singular בנביא, vocalised בַּנבִֿיָא and marked singular in the source, as “in the prophet,” with causal “For” and “Therefore.” “From him” stays with learning, followed by coming to Jesus. The discourse introduced at John 6:43 continues through John 6:51.
- John 6:50-58 preserves the recorded descent tense, the adjective חיא as “living” in “living bread,” the final active “I give for the life of the world,” and the body/give/us roles in the Jews' question. The body/blood, abiding, life, and last-day clauses remain explicit; John 6:58 retains manna and the fathers' death rather than inserting a second bread noun.
- John 6:51 records al appay as the minimal compound preposition for, separately from which, I give, the life and of the world. The life suffix anticipates the world; it is not first-person my life. John 6:55 keeps both truly and both present copulas.
- John 6:59-66 distinguishes synagogue narration, disciples' reaction, Jesus' reply and the narrator's knowledge explanation. John 6:60 takes מנ תלמידוהי as the partitive with many, yielding “many of his disciples who heard,” rather than inventing outsiders hearing from them. John 6:62 retains the conditional question without supplying an apodosis, with then separately linked and men qdim recorded as the temporal before; the Spirit/body/words and nonbelief clauses remain explicit.
- John 6:64 keeps both the nonbelieving group and the singular person who would hand him over. John 6:65 preserves the passive gift, to him and from my Father; 6:66 separately links the past auxiliary of walking.
- John 6:67 retains the contextual yes/no question “Do you yourselves also want to go?” CAL records למא as a general interrogative in Syriac direct speech (https://cal.huc.edu/oneentry.php?cits=all&lemma=lmh+c). John 6:68 separates the words, of, eternal, life, are and yours. John 6:69-71 consistently uses Simon Cephas, retains “the Messiah, the Son of the living God,” the adversary predicate and Judas Iscariot, son of Simon. One of the twelve refers to Judas, placed beside he in the prospective handing-over clause.

Source checks for John 7:

- John 7:1-10 preserves Galilee/Judea, the festival of Booths, the brothers' request and nonbelief, and the time/readiness contrasts. John 7:8 retains “this festival” in the first command and “to this festival now” in Jesus' response; the later secret ascent remains distinct.
- John 7:1 separately links walking, willingness and seeking to their three past auxiliaries; in 7:2 near is adjectival with was. In 7:3 the anticipatory recipient him and named Jesus jointly identify the one addressed, with to separately linked; the ethical dative owns the command's you. John 7:5 retains in him—in Jesus as a resumed apposition. John 7:6 separates my/own/time, your/own/time, up to/now and at every/time; 7:9 keeps the ethical dative with he remained.
- John 7:11-15 keeps the Jews' search, crowd murmuring because of Jesus, the contrasting good/deceiver reports, fear, the festival's halfway point, temple teaching, and the book/learning question. The explanatory גיר in John 7:12 remains “For.”
- John 7:11-15 separately links both seeking/saying past auxiliaries, the existential there/was and there/were clauses, speaking, teaching and marveling. John 7:15 uses literature for sefra in the learning context: [CAL's spr entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=spr+N), sense 4 and 4.1, includes writing, literacy and Syriac learning; its book gloss does not require a particular physical book here.
- John 7:16-18 preserves teaching from the sender and the question of the speaker's own initiative. John 7:18's second relative clause has its own subject: “whoever seeks the glory of the one who sent him” is true, contrasting with the person seeking his own glory.
- John 7:16 retains present nominal is not my own despite perfect-form hwā. In 7:17 by/my own/will has separate owners, as does the repeated personal I/am framing. In 7:18 source position 8, the one, belongs to the second relative clause; it is not part of the preceding seeking glory for himself. The glory, sender, seeking, truth and negative unrighteousness predicates retain their own owners.
- John 7:19-24 retains the negative in “Did not Moses give you the law?”, the crowd's demon accusation, the one deed and amazement, circumcision's origin parenthesis, the whole man's Sabbath healing, and righteous judgment. John 7:22's מטל הנא is “For this reason,” distinct from the circumcision noun.
- John 7:23 retains the purpose clause “so that Moses' law is not set aside.” CAL explicitly lists meṭṭol with a preformative as “in order to, so that” (https://cal.huc.edu/oneentry.php?cits=all&lemma=m%2Bwl+c). The following עלי, “at me,” belongs to the grumbling clause, not to Moses' law.
- John 7:19 preserves did/not/give in the Moses question; 7:20 records existential/dative have as a minimal possession predicate. John 7:22 keeps the origin resumption from him, from Moses and the fathers contrast, with distinct negative copula and personal it. John 7:23 separates the passive setting aside, negative, grumbling at me, whole/person and made/well. In 7:24 taking faces is the partiality idiom, with with separately linked from the shared lexical predicate.
- John 7:25-31 keeps the Jerusalem speakers, “our elders” from the first-person plural suffix, the known/unknown origin contrast, the sender and hearers' knowledge, the still-unarrived hour, and the more-signs question. John 7:28's fronted ולי begins “You know me”; it is distinct from the preceding speech verb and is not a new recipient “to me.”
- John 7:28 retains his voice: קלה has a third-person suffix, distinct from the first-person forms inside the quotation. The initial and on ולי remains separately linked; the final whom minimally combines the relative prefix with the resumed object, while you, do, not and know remain separate. John 7:29 keeps I/do/know and the causal origin phrase; 7:30 keeps come and its past auxiliary separately. In 7:31 both doing verbs are active participles; the second comparison remains this one performs rather than has done.
- John 7:32-36 preserves the chief-priest/Pharisee officers, the speaker's departure and search promises, and the crowd's repeated question. The future/go predicate in John 7:35 stays in its own linked clause about going to the regions of the Gentiles, followed by teaching the pagans.
- John 7:32 retains the plural crowds and the explicit they with the chief priests. John 7:34-36 preserves both search/find object suffixes and the repeated where/I/am predicates. John 7:35 renders the final hanpe as the pagans, distinguishing it lexically from the preceding Gentiles; repeated personal forms can separately supply we/are and you/are in the ability clauses.
- John 7:37-43 distinguishes Jesus' invitation and living-water saying from the narrator's explicit Spirit explanation: the Spirit had not yet been given, because Jesus had not yet been glorified. The prophet/Messiah reports, David/Bethlehem origin question, and division because of Jesus remain explicit.
- John 7:37 separately links standing and its past auxiliary, crying out and saying. In 7:38 everyone/who and the plural Scriptures/have said remain explicit, as do rivers/of/living/waters and his belly. John 7:39's prospective were/going/to receive describes the believers, with separate auxiliaries for the Spirit's giving and Jesus' glorification. John 7:42 keeps David's/own/village separate; 7:43 retains both forms of the happening/past construction as had come about.
- John 7:44-49 preserves attempted seizure, untouched hands, the officers' return and reply, and the Pharisees' questions. The source's plural accursed predicate is rendered with natural English agreement: “these people ... do not know ... they are accursed.”
- John 7:44 separates existential there/were and wanting/past. John 7:46 keeps no, ever, spoken, thus and as separately linked. John 7:47's active Peal perfect is have gone astray, rather than a passive are deceived. In 7:49 ella en is the compound exception transition, contextually But as for the contrasted people, followed by a relative description and resumed they are accursed. It does not turn who do not know the law into a hypothetical condition. [Thackston's vocabulary](https://theswissbay.ch/pdf/Books/Linguistics/Mega%20linguistics%20pack/Afro-Asiatic/Semitic/Syriac%2C%20Introduction%20to%20%28Thackston%29%20%5Bwith%20Answer%20Key%5D.pdf) lists ella en as unless, except that; [CAL's conjunction index](https://cal.huc.edu/JLAentry.php?cits=all&lemma=l%29+X) likewise records the compound exception construction.
- John 7:50-53 retains Nicodemus' membership and previous nighttime visit, “our law,” hearing before judgment, the Galilee challenge, “Search and see,” the no-prophet claim, and each person's departure to his own house. Speaker changes and nested quotations are punctuated across the actual verse boundaries.
- John 7:50 separates the earlier coming from its past auxiliary. In 7:51 unless is a minimal ella en exception, with first, hearing/from him, knowing, what and he has done distinct. John 7:52 keeps both second-person forms in the origin question and both imperatives; 7:53 retains the singular distributive each one went to his own house.

Source checks for John 8:

- John 8:2,6,8-9 separately links coming, teaching, writing and going out to their past auxiliaries. In 8:4 in that/act/of adultery has separate owners. John 8:6's copular purpose and dative become so/that/they/might/accuse him, without inventing another recipient. In 8:7 the relative existential owns is, distinct from without/sin and first/cast. John 8:9 keeps the repeated one/one as one/by one, the elders as the starting group, and the woman alone in the midst. In 8:10 the anticipatory recipient and named woman share only the woman, with to separate; 8:11 keeps neither/do/I/condemn/you and from/now/not/sin/again distinct.
- John 8:12 keeps comes/after me separate and retains for himself with finding the light. Nominal negative copulas in 8:13,16 remain present is not and I am not, including inflected lā hwīt. In 8:16 the elliptical I and my Father is expressed as I am/with my Father, preserving both participants and the sending relative. John 8:18's three first-person forms become I/myself/am, separate from the relative witness predicate. John 8:19 preserves the counterfactual if you knew me/you would know, without automatically changing current knowledge into a past counterfactual. In 8:20 beth gaza is the minimal shared treasury compound, with in the separately linked.
- John 8:21 assigns says to its actual source word and again separately; both repeated first-person forms stay with going rather than seeking or the hearers' inability. John 8:23 keeps the postposed second-person copula with you are from below, distinct from I am from above; its inflected negative copula remains present I am not from this world. In 8:25 the repeated you is explicit in You—who are you? John 8:26 retains have as minimal existential possession, while the proleptic things/these/are/what/I am/speaking construction keeps each personal and resumptive form visible.
- John 8:28's lifting verb has an anticipatory him suffix, expressed once with the explicit the Son as a minimal shared object; of Man remains separate. By/my own/will and that/is/how/I/speak have separate owners. John 8:29 preserves both with me and my Father, the left-me suffix, and what pleases him; 8:30 keeps speaking/past distinct. John 8:31 retains present you are my disciples; 8:32 expresses the resumed truth subject as the truth itself.
- John 8:33 assigns the seed to the Abrahamic-descent clause, rather than the reporting speech. Its claim remains worked/in servitude/to anyone, retaining both the work predicate and slavery noun; the dative agent becomes English we, without reducing servitude to ordinary voluntary service. Men metum is the minimal temporal ever, with the negative separate. The free-person compounds in 8:33,36 have specific exception reasons. John 8:36's en hu conditional frame is jointly if, separately from therefore and the Son. John 8:37 preserves the hearers' lack of capacity to receive the word; [CAL spq](https://cal.huc.edu/oneentry.php?cits=all&lemma=spq+V) records the Syriac ability sense. In 8:38 the repeated first-person copula stays with I am speaking, not the you clause; 8:39 retains our/own/father and both conditional auxiliaries.
- John 8:41 retains the present negative origin claim and minimal existential have for one Father. John 8:42 keeps the two completed predicates came forth and have come, then separately did/not/come/by/my own/will and he sent me. John 8:43 records metol mana as the minimal reason question why, distinct from the following causal answer.
- John 8:44 separately renders a killer/of people/from/the beginning; the two personal copulas and the negative existential have their own owners. What is his own belongs to the lie-speaking clause. In 8:45 the d-prefix on truth supplies causal because and the noun stem separately supplies the truth; [CAL dy, sense 5](https://cal.huc.edu/oneentry.php?cits=all&lemma=dy+c) explicitly records causal because in Syriac. John 8:46 keeps and if distinct from why, and both hearer pronouns remain in the question. In 8:47 the negative inflected copula remains present you are not from God; for/this reason stays separate from the later because.
- John 8:48-49,52 records have as minimal existential possession, with the possessors, demon and negatives separately linked. John 8:51-52 separates will/not/ever from seeing or tasting death. John 8:54 keeps is/not/anything and the glorifying subject explicit. In 8:55 you have not known him is perfect with its object suffix; the next independent I begins the contrasting knowledge clause. The ethical li in the hypothetical liar clause supplies myself, and the closing his word/keep/I retains its own owners. John 8:56 separately links longing and its past auxiliary.
- John 8:1-11 retains the Mount of Olives, dawn return, accused woman, Moses-law challenge, writing on the ground, first-stone command, departures, and the condemnation/sin exchange. John 8:5 keeps “You therefore” with the question; the generic stoning command is rendered without adding an execution actor. The act/preposition and first/cast/stone/her links stay in their own source phrases.
- John 8:12-20 preserves light, darkness, life, self-testimony, origin/destination, the Father, two witnesses, the law, treasury, and the still-unarrived hour. John 8:15 renders the bodily criterion as “according to the body”; the repeated second-person pronoun stays with that clause. John 8:20's implicit subject is He, and the hour, past auxiliary, negative, yet and coming retain distinct owners.
- John 8:21-27 retains the going/seeking/death clauses, the Jews' self-killing question, below/above and world contrasts, the speaker's identity, the concessive beginning-to-speak clause, and the narrator's Father explanation. John 8:25 retains the explicit “Jews” subject.
- John 8:24 keeps “unless you believe.” CAL explicitly lists אלא as “except, unless,” including Syriac usage; an additional separate לא is not required for that conditional sense (https://cal.huc.edu/oneentry.php?cits=all&lemma=%29l%29+c).
- John 8:28-32 retains the Son-of-Man lifting, recognition, own-initiative and Father-taught speech, the sender's presence, pleasing actions, believers, remaining in the word, truth, and liberation.
- John 8:33-40 preserves Abrahamic descent, slavery, sin's slave, house/son permanence, freedom, intended killing, the speaker's word, and what each side has seen with its father. בני חארא is rendered idiomatically as “free people”; CAL defines the compound as “free individual” and records plural free-person usage (https://cal.huc.edu/oneentry.php?cits=all&lemma=br%40xyryn+N).
- John 8:41-43 retains the denial of prostitution origin, the one Father identified as God, coming out from God, being sent, and both occurrences of “my word.” The compound מטל מנא is the opening “Why”; the following על דלא introduces the causal explanation “Because you cannot hear.”
- John 8:44-47 keeps the accuser, the father's desire, the murderer-from-the-beginning predicate, standing “in the truth,” truth's absence, speaking a lie/from himself, the liar and its father, the truth/unbelief contrast, conviction concerning sin, and hearing God's words. The lie's possessive is retained in “its father.”
- John 8:48-56 preserves the Samaritan/demon accusation, the denial, honor and glory, the death/word promise and repeated challenge, Abraham and the prophets, the Father's glorifying, knowing and keeping his word, and Abraham's desire and rejoicing. John 8:54 retains the actual first-person plural “our God” in the reported claim.
- John 8:57-59 retains the fifty-year comparison, Abraham question, before Abraham came to be/I am, and the full final movements: stones taken up, Jesus hiding, leaving the temple, passing among them, and going away. In 8:57 the negative copula is present you are, and old/fifty/years have separate owners. John 8:58 contrasts came to be with the explicit present I/am. John 8:59 separately links the named Jesus, reflexive hiding, and all subsequent movements.

Source checks for John 9:

- The shared-span review checked all 41 verses and 575 source words, reducing 162 shared spans to 10 documented exceptions. Every final mapping and the continuous English were independently reviewed against the frozen chapter hash. Ordinary participants, predicates, negatives, auxiliaries, genitives and destinations retain separate owners.
- John 9:8 keeps the passive seeing, its auxiliary and resumed plural agent distinct; only whom shares the relative prefix and agent reference. Men qdim in 9:8,13 is the minimal temporal before. In 9:11,15 the passive ethhzi plus experiencer dative jointly expresses restored sight, with my/your separately linked to the dative. The anticipatory object suffixes in 9:13,24 share only the one/the man with their explicit complements.
- John 9:7's pool is directly listed for maʿmōdītā in [Thackston's English-Syriac vocabulary](https://theswissbay.ch/pdf/Books/Linguistics/Mega%20linguistics%20pack/Afro-Asiatic/Semitic/Syriac%2C%20Introduction%20to%20%28Thackston%29%20%5Bwith%20Answer%20Key%5D.pdf); the export's shorter gloss list is not exhaustive. In 9:9 the cognate infinitive and participle supply emphatic does/resemble, separate from the man's spoken I/am he.
- John 9:16 retains present is not in the nominal denial and the causal d-prefix in because/he does/not/keep/the Sabbath. In 9:17 the d-prefix on prophet owns that. John 9:20 retains the contrasting But and first-person plural we know; 9:21,23 distribute the age expression over come/of age without inventing another recipient. In 9:28,35,39 the noun suffix anticipates the explicit following genitive: disciple/of that one, Son/of God, and judgment/of this world.
- John 9:29 keeps that/with attached to the same source word and the unknown-origin claim separate from God's speech to Moses. John 9:30 retains my/own/eyes and the astonishment clause. In 9:31 the relative object and its resumed that one remain explicit. John 9:32 uses the minimal temporal ever for men alam under the separate negative; [CAL](https://cal.huc.edu/oneentry.php?cits=all&lemma=%28lm+N) lists the adverb, without requiring an added beginning of the world.
- John 9:40's lma introduces a direct yes/no question, Are we also blind ourselves?, rather than asserting blindness with why. [CAL lmh, sense 4](https://cal.huc.edu/oneentry.php?cits=all&lemma=lmh+c) records its general direct-question use. In 9:41 the first-person plural subject clitic owns We see, the negative existential and possessor dative share only have, and the final copula and predicate remain separately is/still present.

- John 9:1-7 retains blindness from the mother's womb, “Our Rabbi,” the parents/sin question, the works-of-God purpose clause, the speaker's singular obligation, day/night and world/light imagery, clay made from his spittle, and the Siloam washing. Jesus' answer continues through John 9:5. John 9:7 names the pool of Siloam without adding a supplied name etymology; John 9:11 explicitly says “the waters of Siloam.”
- John 9:8-14 preserves the neighbors' identification debate, ongoing begging, the man's identification and named-Jesus account, the nested washing command, ignorance of Jesus' location, the Pharisee referral, and the Sabbath setting. The subject and auxiliary in John 9:9 remain with the man's speaking, separate from his quoted answer.
- John 9:15-19 keeps the restored-sight question and reply, the Pharisees' Sabbath objection and disagreement, the man's prophet declaration, the Jews' disbelief, and the parents' questioning. The subject at John 9:15 belongs to “But he said,” and the auxiliary at John 9:16 belongs to “others were saying.” The object “them” in John 9:19 stays with “asked.”
- John 9:20-24 retains “our son,” blindness at birth, the parents' uncertainty about present sight, their separate assertion that he has come of age, his ability to speak for himself, fear of the Jews, the prior expulsion decision, the second summons, and the causal “for we know.” The age statement is not turned into another uncertainty clause.
- John 9:25-30 preserves the one-thing contrast, present sight, repeated questioning, the invitation to become disciples, the insult, the Moses/Jesus disciple contrast, known divine speech to Moses, unknown origin, and the man's astonishment and “my own eyes.” John 9:28's anticipatory possessive stays with “that man's disciple”; גיר is rendered emphatically as “indeed,” an attested Syriac sense in [CAL](https://cal.huc.edu/oneentry.php?cits=all&lemma=gyr+X).
- John 9:31-34 retains the voice of sinners, fear of God and doing his will, the hearing response, the unprecedented opening of a man born blind's eyes, the from-God counterfactual, the accusation of being born wholly in sins, and the expulsion. The quoted testimony runs through John 9:33.
- John 9:35-41 retains the actual “Son of God,” the healed man's explicit subject, both “My Lord” addresses, seeing/speaking identification, falling and worshiping, “the judgment of this world,” the seeing/blind reversal, and the final conditional and present sin. John 9:41's מטל הנא is the causal “for this reason,” without adding a demonstrative to “your sin.”

Source checks for John 10:

- The shared-span review checked all 42 verses and 564 source words, reducing 152 shared spans to 14 documented exceptions. All final mappings and the running English received independent review against the frozen chapter hash. Ordinary auxiliaries, negatives, possessives, relative prefixes and destinations retain individual owners.
- John 10:3 separates the recipient this one from keeper/of the door and the opened door; only the fronted recipient and its resumed dative share this one. In 10:4 his/own/sheep and go/after him remain separate. John 10:6 keeps the completed speech and the hearers' failure to know what he had spoken; 10:7 retains present says and the two first-person forms in I/am/the door. John 10:8 retains are/thieves and adds no before me.
- John 10:10 separates may and they from each minimal existential possession have. In 10:11 lays down/his life/for/his flock remain separate. John 10:12's the sheep are not his own becomes the equivalent relative and/does/not/own/the sheep, preserving the ownership relation and its participants. Only the resumed flock object remains shared. John 10:13 keeps the ethical dative with he/does/not/care, and 10:14 retains passive am known/by/my own.
- John 10:15,27 preserve repeated first-person forms as I/myself/know. John 10:16 keeps other sheep, those/which/are/not/from/this/fold, the fronted as for them, and the later bring/them and hear/my voice distinct. Its final elliptical copula is expressed as the whole flock will be one, and there will be one shepherd. John 10:17-18 retain the repeated laying down, taking up, again, own will, authority and received command.
- John 10:20 keeps the cognate emphasis as has/indeed/gone mad. In 10:21 the nominal negative remains present, and the direct-question particle supplies Is in Is a demon able to open the eyes of one blind? John 10:24 records only the temporal how long and the minimal suspense expression as shared, with keeping, us and the copula separate. John 10:25-26 retain present belief and present nominal you are not of my sheep. John 10:28 keeps not/ever/perish and plural my hands; 10:29 keeps the Father's singular hand and supplies the understood sheep object with the transitive gave and snatch verbs.
- John 10:32 records men lwath as the minimal source preposition from; [CAL lwt](https://cal.huc.edu/oneentry.php?cits=all&lemma=lwt+p) lists the compound from the presence of. Its question preserves singular which/work and the partitive among them referring to the preceding good works; [CAL mn, sense 2](https://cal.huc.edu/oneentry.php?cits=all&lemma=mn+p) records the partitive use. John 10:33 keeps the human-being compound minimal and the stoning, blaspheming and self-identification clauses separate. John 10:34-36 retain plural gods, the modal/negative split, and the single resumed relative object whom, separately from the Father, sanctifying and sending.
- John 10:37 retains the attested unless sense of ella and the separate negative prohibition. In 10:38 only the resumed works object remains shared; the two my Father clauses retain their own pronouns. John 10:39 keeps the narrative were/seeking and the ethical subject dative; men beth is the compound separation from, supported by [CAL byt](https://cal.huc.edu/oneentry.php?cits=all&lemma=byt+p). John 10:40 preserves the former-being and baptizing auxiliaries around the minimal before compound. John 10:41 retains not/even/one/sign and the reported present is/true; 10:42 has no added there.

- John 10:1-6 preserves the door/fold contrast, thief and robber, shepherd, doorkeeper, names and voice, leading out, following, stranger, and the hearers' failure to understand. John 10:3's first door noun forms “doorkeeper,” while its second is the door opened; the initial demonstrative is the recipient, not a modifier of the guard.
- John 10:7-14 keeps the renewed introduction, door-of-the-flock image, entry and pasture, theft/killing/destruction, life and “something more,” laying down life for his flock, the hired worker's lack of ownership and concern, the wolf's actions, and knowing/being known. John 10:8 has no source phrase “before me”; John 10:11 retains the flock's possessive suffix.
- John 10:15-18 retains “my Father,” the other sheep, this fold, bringing and hearing, the whole flock becoming one with one shepherd, the Father's love, voluntary laying down and taking up life, both authority clauses, and the Father's command. The object at John 10:16 position 17 belongs to bringing, followed by hearing at positions 18-19; the whole-flock quantifier remains explicit.
- John 10:19-24 preserves the division among the Jews, demon/madness accusation and reply, the Feast of Dedication in Jerusalem during winter, Solomon's portico, the surrounding Jews, the suspense question, and the request for a plain Messiah declaration. The spoken accusations remain distinct from the narrator's statements.
- John 10:25-30 retains the works in the Father's name as witnesses, unbelief and sheep membership, “my own sheep,” hearing/knowing/following, eternal life, never perishing, the Father's gift and greatness, and “I and my Father are one.” John 10:28's hand noun is plural in the actual source (“my hands”); John 10:29's Father's hand is singular.
- John 10:31-36 preserves the stones, good-works challenge, blasphemy accusation, human-being/God contrast, the law's plural “gods,” the word of God being with its recipients, Scripture's permanence, the Father's sanctifying and sending, and “I am the Son of God.” In John 10:36 the plural “you say” and singular “you blaspheme” have separate linked clauses; על ד introduces “because I said.”
- John 10:37 uses “Unless I do my Father's works,” followed by “do not believe me.” [CAL](https://cal.huc.edu/oneentry.php?cits=all&lemma=%29l%29+c) records אלא as “except, unless,” including Syriac usage. John 10:38 retains the works-directed pronoun with “believe the works,” the knowing/believing purpose, and both “my Father” phrases.
- John 10:39-42 retains the renewed seizure attempt, escape, journey across the Jordan to John's former baptizing place, staying there, the statement that John performed not even one sign, the truth of everything he said about Jesus, and the many believers. Quotation boundaries distinguish the two shepherd discourses, the later temple exchange, and the final crowd testimony.

Source checks for John 11:

- The shared-span review checked all 57 verses and 734 source words, reducing 220 shared spans to 11 documented exceptions. Every final mapping and the continuous English were independently checked against the frozen chapter hash. Ordinary participants, auxiliaries, negatives, genitives and temporal clauses retain separate owners.
- John 11:1 separates there/was/one/who was sick. In 11:2 this Mary/is/the one keeps the first identification present, while It was/the brother/of this woman and who/was/sick retain the two later past copulas. John 11:3 keeps both/his sisters, Our Lord, look and the one/whom/you/love distinct. John 11:4 retains present is/not/for death and the separate Son/of God and glory/of God genitives. The minimal stative loved in 11:5,36 carries the love participle and its past auxiliary, with subjects, objects and other clauses outside the exception.
- John 11:7 retains plural come and first-person plural let us go; 11:15 instead has the actual second-person plural imperative go. John 11:8 keeps the past were/seeking and the separate are you/going/there/again question. In 11:9 the demonstrative this belongs to world. John 11:12 keeps the participial recovering; 11:13 retains the death-versus-sleep misunderstanding and both speaker references. John 11:14's ethical dative stays with Lazarus himself/has died.
- John 11:17 separates the four-day duration and personal reference from the minimal tomb compound, with in separately owned. John 11:18 keeps the compound proximity near, the spatial situated/was and being/distant/from it, and the fifteen-stadia measurement. John 11:19 records the heart-filling comfort expression minimally, with purpose to and pronominal them separate. In 11:20 the heard arrival is perfect had come; Mary remains was/sitting/in the house. The counterfactuals in 11:21,32 keep would, not and have died distinct.
- John 11:24-27 retain resurrection, I/am, the believer, even if he dies/will live, not/ever/die, and I/myself/believe. John 11:28 keeps the named sister, secretly, Our Rabbi and the feminine you with calling. In 11:29 coming retains its past auxiliary; 11:30 keeps both locative references there/in that place. John 11:31's ethical plural dative stays with the following group, and 11:32 distinguishes Mary's arrival, seeing, falling and speaking. John 11:33 retains both the passive inner agitation and active troubled/himself; 11:35 separately links coming, were, the tears and of Jesus.
- John 11:37 places the second this one in the not-die clause, with the past ability question and eye-opening relative separate. John 11:38 retains the reflexive within himself and both minimal tomb compounds; the locative to, identification, cave, stone, passive placement and entrance remain separate. John 11:39's by now and elapsed-time predicate are minimal exceptions, while four/days and the two personal datives retain their distinct references. John 11:40-44 preserve the stone command, feminine belief/seeing address, raised eyes, thanksgiving, standing crowd, loud command and both binding predicates on hands/feet and face.
- John 11:47 separates the chief/priests and ends the action question before the causal For/this/man/performs/many/signs clause. John 11:49,51 express the singular office as chief/of the priests, preserving the plural genitive and the year separately. John 11:50 keeps for us. John 11:51 uses by/his own/initiative; [CAL ṣbw](https://cal.huc.edu/oneentry.php?cits=all&lemma=cbw+N) records the same own-accord construction. The about-to-die predication retains its auxiliary; 11:52 places together with gather before the scattered-children relative. John 11:53-57 keep the plot, ethical departure dative, from/there, Ephraim, staying/past, cleansing/themselves, resumed Jesus object, reciprocal one/to another, and the reported command's condition and purpose.

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

- The shared-span review checked all 50 verses and 680 source words, reducing 194 shared spans to five documented exceptions. Every final mapping and the continuous English received independent review against the frozen chapter hash. The remaining shared spans are four existential-possession predicates and the resumed Lazarus object in 12:10.
- John 12:1,9,17 separate from/among/the dead; [CAL bēṯ](https://cal.huc.edu/oneentry.php?cits=all&lemma=byt+p) records the prepositional among sense. In 12:3 a translucent jar follows the primary [CAL šṭyph noun entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=%24%2Byph+N), with ointment/of/choice/nard and of great/value separately linked. In 12:6 the negative cleft, concern/past/coreferential he, thief/past, money bag/was/with him and he/was/carrying/whatever/was/falling/into it retain separate owners. John 12:8 keeps present possession, both personal datives, and the explicit with you phrase.
- John 12:10 shares only Lazarus between the named object and its resumed suffix; to kill is separate. John 12:12 retains which/was/coming with the crowd and Jesus' separate coming clause. John 12:13 keeps the branches/of palm trees, crying/past/saying, blessed/is he/who comes and both title genitives distinct. John 12:16 keeps both disciples nouns and their different predicates; 12:17 separates testimony from the called/raised content. In 12:19 the repeated second-person forms and the world's ethical itself remain explicit. John 12:20 separates some/Gentile/men and among those/who had gone up; 12:21 keeps drew near/to/Philip, asked him and say/to him, and the singular My lord.
- John 12:23-28 separate the hour's arrival, Son/of Man, grain/of wheat, unless, fall/die and fruit predicates, life contrasts, future coming/following, and the two service clauses. John 12:27 retains My Father, the proposed deliver-me question, for/this reason and to this/hour. John 12:28 separates the name-glorifying command from the heard voice and its reply. John 12:30 renders the two hwā forms separately as did/not/arise, preserving the completed voice event and its contrasting purpose; [CAL hwy](https://cal.huc.edu/oneentry.php?cits=all&lemma=hwy+V) records come to pass/happen and come into existence. John 12:31 keeps both now tokens, this/world genitives, the passive casting and out; 12:32 keeps the temporal lifted-up clause distinct from drawing everyone.
- John 12:34 has plural crowds and separates both Son/of Man titles. In 12:35 the second light belongs to walk/while/you/have/the light, followed by lest/darkness/overtake you. John 12:36 separates the repeated possession, believe/in the light, children/of light purpose and Jesus' leaving/hiding actions. John 12:38 uses word/spoken/by Isaiah/the prophet to recast the source's active utterance and genitive without changing the speaker; the fulfillment predicate and the quoted My Lord, our report and Lord's arm remain separate. John 12:39 retains were/not/able/to believe and both cause expressions.
- John 12:40 keeps the actual third-person plural blinded/darkened predicates and first-person singular healer. Its Pael blinding and healing verbs differ from the Aphel darkening verb. John 12:42 keeps acknowledging/past and the single negative purpose so that/they would/not/be/excluded/from/the synagogue. John 12:44 retains present does/not/believe. John 12:45 distinguishes present sees from perfect has seen, and 12:47 keeps hearing/words/keeping/them apart from I/do/not/judge/him and the contrasting give-life purpose. John 12:48 preserves the first-person perfect that I have spoken and the word's resumed it with its judging predicate. John 12:49 separates on/my own, the Father/who sent me/himself/gave/me/a command and both speech contents; 12:50 preserves both speaking predicates and the Father's as/so correlative.

- John 12:1-6 retains the six days before Passover, Lazarus's Bethany setting and raising by Jesus, the supper, Martha's serving, Mary's ointment vessel of choice nard, Jesus' feet and her hair, the fragrance, Judas Iscariot's identity and betrayal, three hundred denarii, and the money-bag explanation. The source has an ointment vessel rather than an added weight measure. The speech verb stays in John 12:4; the next verse begins with “Why,” not another speech verb.
- John 12:7-17 preserves the burial-day explanation, the poor/Jesus contrast, the visitors' hearing and Lazarus purpose, the chief priests' plan, belief, palms and Hosanna, the donkey and Zion quotation, and the disciples' later understanding. John 12:14 says “donkey” without adding “young”; John 12:15 retains the female donkey. The crowd's hearing and testifying verbs remain with their actual source positions.
- John 12:18-22 keeps the sign as the reason for the crowds' meeting, the Pharisees' complaint, Gentile men among those who had gone up to worship, Philip's Bethsaida/Galilee origin, “My lord,” and the Philip/Andrew message sequence. The source names the nations/Gentiles without an explicit Greek-name word; the people and partitive clauses remain in their own linked phrases.
- John 12:23-28 retains the Son-of-Man hour, the grain of wheat's unless/fall/die contrast, much fruit, life/loss and eternal life, service and following, the Father's honor, the troubled soul, “My Father,” the hour's purpose, and the name-glorifying request and heavenly voice. John 12:26 preserves the source future “he will come after me”; Jesus' speech continues through the request in John 12:28 before the heavenly reply begins.
- John 12:29-36 preserves thunder/angel reports, the voice's purpose for the hearers, judgment and the ruler's casting out, lifting from the earth and drawing everyone, the death explanation, the crowd's Messiah/Son-of-Man question, and the light/darkness commands. The second light noun in John 12:35 belongs to “Walk while you have the light,” followed by the darkness clause.
- John 12:37-41 keeps the signs/unbelief contrast, fulfillment of Isaiah's word, “My Lord,” “our report,” the Lord's arm, the repeated Isaiah explanation, and his seeing glory and speaking about the subject. John 12:40's two actual predicates are third-person plural: “They have blinded their eyes and darkened their hearts.” The source is not normalized to a singular actor; the final healing predicate remains first person.
- John 12:42-50 distinguishes the leaders' belief from their non-acknowledgment because of the Pharisees, with the purpose of avoiding synagogue expulsion expressed without a double negative. Human/divine praise, believing/seeing the sender, light, keeping words, life-giving rather than judging, the rejecting hearer's judge, the last day, and the Father's command remain explicit. John 12:48's דמללת is a first-person perfect (“I have spoken”), not a present participle; its resumptive pronoun stays with the judging predicate. The Father's saying and the speaker's speaking in John 12:50 form one natural correlative clause.

Source checks for John 13:

- The shared-span review checked all 38 verses and 583 source words, reducing 169 shared spans to four documented exceptions: existential possession in 13:8, the temporal compound before in 13:19, stative past loved in 13:23 and at once in 13:30. Independent source and running-English reviews bind the final chapter hash.
- John 13:1,3,11 render the knowledge participle and past auxiliary separately as was/aware. In 13:2 the passive participle plus lamed agent becomes English Satan/himself/had/put it, preserving the same agent and the following betrayal intent. The pronoun anticipates Satan rather than supplying the object it; [Falla, printed page 101](https://gorgiaspress.com/images/uploaded/Gorgias%20Open%20Repository/978-1-60724-072-3_QAOK.pdf) explicitly discusses this verse's passive-to-active equivalence. The heart/of Judas genitive stays separate, as do son/of Simon/Iscariot.
- John 13:5 separates was/wiping and the wrapped cloth's loins phrase. John 13:6 retains the separate personal dative for me, while 13:8 restricts the HAVE exception to the negative existential and its possessor: subject, no, part and with me stay outside it. John 13:9 assigns both additive particles, and 13:10 separates bathing, need, exception, feet alone and complete cleanliness. John 13:11 retains present are/not/all/clean; 13:12 keeps the reclining action and first-person perfect I have done. John 13:13 keeps both our titles, the repeated you as English do, and the first-person identity affirmation.
- John 13:14 separates if, I, therefore, both titles, washing, feet and beneficiary; the reciprocal genitive can use separate of/one/another spans. John 13:15 preserves the purpose and as/so relation. John 13:16 keeps both comparisons, and 13:18 retains present nominal negation and prospective Scripture fulfillment. In 13:19 the belief predicate follows the source rather than Murdock's know. John 13:20 preserves each receive/send predicate with its own participants.
- John 13:21 retains Ethpaal agitation, Aphel testimony and the active future betrayer. John 13:22 separates the past auxiliary, negation, knowing and the earlier speech. John 13:23 keeps existence, one/of/his disciples, relative reclining, its past auxiliary, the named Jesus and the relative object outside the minimal loved predicate. John 13:24 uses an English relative gap after about for the resumed source object. John 13:25's chest suffix anticipates of Jesus. John 13:26 keeps the two first-person dipping/giving predicates and temporal context, followed by the separate completed actions.
- John 13:28 separates for/what reason and uses the demonstrative this as the speech object. In 13:29 expressly translates the intensifying infinitive alongside commanded for the finite verb; [Phillips, printed page 111](https://biblicalstudies.org.uk/pdf/e-books/phillips_george/syriac-grammar_phillips.pdf) describes the infinitive-plus-finite construction. John 13:30 retains both going-out clauses and the ethical coreferential he. John 13:31 separates Son/of Man; 13:32 keeps both future glorifying predicates and in himself. John 13:33 preserves both first-person forms and both second-person forms in their clauses. John 13:34 renders the auxiliary's command modality as should, with love separate. John 13:35 uses there is/love/among you/for/one/another, keeping the existential predicate and reciprocal relation clear. John 13:36-38 retain our/my addresses, after me/after you, the willing offer and its interrogative reply, and three/times separately.

- John 13:1-5 preserves the Passover setting, departure to his Father, love for his own to the end, supper taking place, Satan's influence on Judas, the Father's gift, coming from God and going to God, and the garments, linen cloth, basin, washing and wiping. John 13:3 says “from God” rather than the local Murdock's “from the Father”; its causal knowing clause continues into Jesus' rising in John 13:4.
- John 13:6-13 retains Simon Cephas, “my Lord,” not knowing now but knowing later, never washing, no part without washing, hands/head as well as feet, bathing and cleanness, and the betrayer explanation. John 13:10 explicitly retains both “all of you are clean” and “but not all of you.” John 13:13 keeps “Our Rabbi” and “Our Lord”; the repeated second-person pronoun belongs to “you speak well,” followed by “for that is what I am.”
- John 13:14-20 preserves “your Lord and your Rabbi,” the reciprocal washing obligation, the example, servant/master and messenger/sender contrasts, knowing/doing and blessedness, chosen recipients, the bread/heel quotation, foretelling before the event and belief when it happens, and the chain of receiving the sender. In John 13:18 the speaking, knowing and Scripture clauses remain with their actual source predicates. In John 13:20 the two consecutive receiving predicates have separate linked clauses.
- John 13:21-26 keeps Jesus' agitation and testimony, one betrayer, the disciples' uncertainty, the unnamed beloved disciple's reclining, Simon Cephas's signal, the disciple's question, and the dipped bread given to Judas son of Simon Iscariot. The unnamed disciple is not given an inferred name; the dipping/giving clause preserves the bread, speaker and recipient together.
- John 13:27-30 retains Satan's entry after the bread, the prompt-action command, the table companions' lack of understanding, their suppositions about Judas's money bag and festival/poor provisions, Judas's immediate departure, and the nighttime statement. John 13:28 renders על מנא as “for what reason,” preserving the reason question attested by [CAL](https://cal.huc.edu/oneentry.php?cits=all&lemma=%28l%40m%29+c). The ethical dative accompanying departure is not rendered as a new “from it” object or an added house.
- John 13:31-35 preserves the Son-of-Man/God glorification clauses, immediate future glorifying, “My children,” remaining briefly, seeking, the statement to the Jews and its repetition to the disciples, and the new commandment to love one another as the speaker loved them. The final love clause is grammatical English while retaining the reciprocal source expression.
- John 13:36-38 distinguishes “Our Lord” from “My Lord,” retains where/now/later following, the laying-down-life offer and reply, both “Truly” words, and the rooster's not crowing until three denials. The speaking predicates stay linked with Simon Cephas and Jesus in their respective introductions.

Source checks for John 14:

- The shared-span review checked all 31 verses and 444 source words, reducing 143 shared spans to four documented exceptions: the exclusive conjunction in 14:6, the resumed relative object in 14:17 and possession predicates in 14:21,30. Independent source and running-English reviews bind the final chapter hash. Ordinary subjects, auxiliaries, negatives, genitives and clauses are separately linked.
- John 14:1 retains singular heart with a plural possessor. The troubled predicate is a third-person singular Ethpaal imperfect used with negation as a prohibition; the two believe verbs are plural imperatives. John 14:2 separates many/dwellings/there are, the Father's house, the counterfactual auxiliary and speech, and the causal going/preparing clause. John 14:3 preserves both first-person movement predicates and the later coming/taking promise; 14:4 keeps both you/know clauses. John 14:5's enclitic knowledge subject is first-person plural we, and both Our Lord addresses remain in 14:5,8.
- John 14:6 restricts the exception to ella en, the exclusive conjunction recorded by [CAL](https://cal.huc.edu/oneentry.php?cits=all&lemma=%29lhn+c), with through me separate. John 14:7 distinguishes both counterfactual knowledge predicates from present know and perfect have seen him. John 14:9 retains the duration question and second-person perfect knowledge, then distinguishes present sees from perfect has seen. John 14:10 keeps both first-person pronouns with the speaking clause and the separate on/my own initiative phrase; the indwelling Father remains the subject of he/does/these/works.
- John 14:11's if not/at least construction expresses a concessive alternative, matching [CAL's ella ... apen usage](https://cal.huc.edu/oneentry.php?cits=all&lemma=%29pn+c). John 14:12 keeps the second first-person pronoun with I/am/doing, followed by the distinct he/too/will do clause, and retains unsuffixed the Father. John 14:13 separates the Father's glorification purpose. John 14:14's ask me has a genuine first-person object suffix; the later doing form instead has a first-person subject enclitic. John 14:15 keeps the plural keep command, while 14:16 separates another/Advocate and all participants and purpose clauses.
- John 14:17 shares only whom between the relative referent and the receive infinitive's resumed third-person object suffix. The world, ability, negation and to receive remain outside that exception. The actual perfect seeing and knowing forms become has not seen him/nor has it known him, contrasted with present you/do/know/him. Both with you and in you remain present. John 14:18 separates little/while at its actual source boundary; 14:19 preserves simple negation, both contrasting subjects and future life. John 14:20 separately links all three mutual-indwelling assertions and repeated pronouns.
- John 14:21 restricts has to existential ith plus with him; commandments, their my possessor, keeping and the separate them object stay outside. The loved predicate is Ethpeal imperfect with by/my Father separate, and the showing verb's suffix owns to him apart from myself. John 14:22 keeps Judas's present not-Iscariot identification, My Lord, prospective revealing to us and present nominal negation of the world recipient. John 14:23 keeps present keeps my word, future Fatherly love and both first-person plural we predicates, with make/a dwelling/with him separately linked. John 14:24 retains singular word and present is/not/mine/but/the Father's; 14:25 keeps both with you phrases and present I am.
- John 14:26 separates that/Advocate and reorders Holy/Spirit, preserving the relative identification, my Father's sending in my name, both he subjects and the distinct future teaching/reminding predicates. John 14:27 keeps peace/leaving and my/peace/giving distinct, then a present negative cleft and the repeated first-person forms. Its final two prohibitions retain singular heart with a plural possessor. John 14:28 separates the two counterfactual auxiliaries from loving and rejoicing, with the Father comparison independent. John 14:29's before is one source word; both occurrence clauses and the belief purpose remain separate. John 14:30's ruler suffix anticipates of the world; no extra his or this is added. The possession clause reads he/does/not/have/anything, with negation owned by the negative existential and anything by the indefinite noun. John 14:31 keeps the purpose conjunction with may know, the love statement, the Father's command and as/so correlation, followed by the plural rise command and first-person plural let us go.

- John 14:1-7 retains the untroubled-heart commands, belief in God and the speaker, the Father's house and dwellings, preparation and return, knowing the destination and way, Thomas's “Our Lord,” and the way/truth/life and Father-knowing statements. John 14:2 keeps the conditional and going/preparing clauses; the causal reading of the latter follows the supplied Murdock without imposing a remembered Greek question.
- John 14:8-12 preserves Philip's request, seeing the speaker and the Father, the mutual indwelling, words and works, the grounds for belief, and the believer's doing the works and greater things. The repeated first-person pronoun in John 14:12 belongs to “the works that I do,” followed by the separate “he too will do.” Its final Father noun is unsuffixed “the Father,” not the reference's supplied “my Father.”
- John 14:13-18 retains asking in the speaker's name, “ask me,” the Father's glorification in his Son, loving and keeping commandments, asking the Father, another Advocate, the Spirit of truth, and knowing, seeing, receiving and dwelling. John 14:17 distinguishes “with you” from “in you”; the final “in a little while” remains in John 14:18 at the actual source boundary.
- John 14:16,26 consistently uses “Advocate” for פרקלטא. [CAL](https://cal.huc.edu/oneentry.php?cits=all&lemma=prqly%2B+N) records this Syriac lexical sense and the Paraclete/Holy Spirit use. John 14:26 retains “my Father” from the actual possessive suffix and the present “all that I say to you.”
- John 14:19-24 retains the world's not seeing, the disciples' seeing and living, mutual indwelling, commandments and love, the self-revelation promise, Judas explicitly distinguished from Iscariot, and the Father's love, coming and dwelling. John 14:19 uses the source's simple “not,” without adding “no longer.” The speaking and love/coming predicates in John 14:22-23 remain with their own source words; the singular word heard in John 14:24 belongs to the Father who sent the speaker.
- John 14:25-29 preserves the present being-with-the-hearers clause, teaching and remembering, peace and its distinct giving, the heart's not being troubled or afraid, the repeated going/coming report, love and rejoicing, the Father's greatness, and the foretelling/belief sequence. John 14:29 keeps both “before it happens” and “when it happens” in their respective linked clauses.
- John 14:30-31 retains limited further speech, the coming ruler, “he has nothing in me,” the world's knowing the speaker's love for his Father, the Father's command and the speaker's doing, and the rise/depart commands. The ruler's possessive relation is retained rather than reduced to a statement that nothing is in the speaker. Quote boundaries keep Judas's question separate from Jesus' preceding and following discourses.

Source checks for John 15:

- The shared-span review checked all 27 verses and 368 source words, reducing 111 shared spans to five documented exceptions: already in 15:3, have and for in 15:22, and the two from compounds in 15:26. Every final mapping and the continuous English received independent review against the frozen chapter hash. Ordinary noun phrases, auxiliaries, negation, genitives and clauses are separately linked.
- John 15:1 separates I/am and true/vine. In 15:2 the feminine branch bears fruit, while the masculine cultivator takes/it/away and cleanses/it; the final bearing purpose retains its own fruit and much owners. John 15:3 renders men kaddu as already, retaining you/are/clean and because of/the word/that I have spoken/with you separately. [CAL](https://cal.huc.edu/oneentry.php?cits=all&lemma=kdw+X) records the temporal usage, and its [Syriac citation under yld](https://cal.huc.edu/oneentry.php?cits=all&lemma=yld+V) renders men kaddu as already.
- John 15:4 keeps the initial plural remain command, the branch's ability, negative and fruit-bearing predicate, both unless clauses and the second-person application. The source's d-tettel is an imperfect, although the English purpose/ability construction uses to bear. John 15:5 separates for/without/me and you/are/not/able/to do/anything. John 15:6 keeps the passive casting, branch/wither comparison, third-person plural gathering and throwing, and the feminine intransitive burning purpose. John 15:7 separately links both conditional remaining clauses, all/that/you wish/to ask for and will be/yours. John 15:8 retains the Father as the glorified subject. John 15:9 preserves both perfect love predicates and splits the emphatic in my/own/love.
- John 15:10 keeps my commandments distinct from the commandments/of my Father, whose anticipatory third-person suffix does not require an extra his. The final remaining predicate is first person. John 15:11 separates the two joy subjects and their purpose predicates; the completion verb is Eshtaphal imperfect. John 15:12,17 retain Aphel imperfect love clauses and separate one/another. John 15:13 keeps the existential negative and greater/than/this comparison outside the person/lay down/his life/for/his friends clause; its laying verb is third-person singular imperfect. John 15:14,15,17 preserve the first-person command/calling predicates and their separate pronouns.
- John 15:16 keeps the past choosing contrast, its explicit first-person focus, appointment, going, bearing and remaining purposes, and the Father's giving purpose. The second first-person perfect choosing and appointment have second-person plural objects. The final request/giving clause is reordered into natural English, with so that/he may give/to you/all/that you ask/of my Father/in my name independently owned. John 15:18 keeps the plural know imperative and distinguishes present hatred from the prior perfect hated. John 15:19 separates the counterfactual would/love from present you are/not/of the world, first-person choosing and the final because of/this causal clause.
- John 15:20 keeps both persecute and keep conditionals, their past predicates and future results, and the possessive yours. John 15:21 separately links all/these things, the first-person my/name expression and causal because/they do/not/know. John 15:22,24 use there/would have been/no/sin/for them for the counterfactual existence clauses. In 15:22 only the present existential and possessor share have; they, no, excuse and their sin remain outside it. The compound al appay is for/concerning, not a separate face participant; [CAL](https://cal.huc.edu/oneentry.php?cits=all&lemma=%28l%40%29py+p) records its relational and causal senses. Both sin nouns in 15:22 are singular, with the final noun carrying a plural possessor.
- John 15:23 keeps the two hating predicates separate. John 15:24 preserves works/in their sight, no/other/person/did, both plural perfect seeing/hating predicates and both objects. John 15:25 keeps the passive fulfillment, written predicate, their law and the source's first-person hated me object distinct from without cause. In 15:26 the relative whom belongs to the d-prefix on the first-person pronoun, while am belongs to the second pronoun and sending to its participle. Each men lwath is a minimal from compound, as [CAL](https://cal.huc.edu/oneentry.php?cits=all&lemma=mn%40lwt+p) records. Both my Father nouns, the appositional Spirit/of truth, present proceeds and future testimony remain independent. John 15:27 preserves all three second-person pronouns as subject, copula and duration clause, with because/from/the beginning separately linked.

- John 15:1-6 preserves the true vine and cultivator, unfruitful and fruitful branches, taking away and cleansing, cleanness through the word, mutual remaining, bearing fruit, and inability without the speaker. Both “unless” clauses in John 15:4 remain explicit. John 15:6 begins with אלא, rendered “Unless”; a separate לא is not needed for this attested Syriac conditional sense. The gathering and throwing predicates are both retained, followed by the fire and burning clause.
- John 15:7-12 retains remaining words, the request and its result, the Father's glory and much fruit, discipleship, the Father's and speaker's love, commandments and remaining in love, the speaker's and hearers' joy, and the reciprocal-love command. Repeated pronouns and possessives are rendered naturally; “my Father's commandments” does not become an extra owner or “his commandments of my Father.”
- John 15:13-18 keeps life laid down for friends, friendship and obedience, the servant/master distinction, disclosure of everything heard from the Father, the speaker's choice and appointment, going and bearing enduring fruit, asking the Father in the speaker's name, the love command, and the world's prior hatred of the speaker. The full chain of purpose clauses in John 15:16 remains explicit.
- John 15:19-24 preserves belonging to or being chosen from the world, hatred, the remembered servant saying, persecution and keeping words, the name and sender, the coming/speaking and unique-works counterfactuals, sin and excuse, and hatred of the speaker and his Father. John 15:22 retains singular “their sin”; John 15:24 keeps the works being done “in their sight.”
- John 15:25 uses “without cause” for מגנ. [CAL](https://cal.huc.edu/oneentry.php?cits=all&lemma=mgn+X) records the Syriac “for no reason” sense and a Peshitta Psalm 69:5 example, distinct from the word's gratis/free-of-charge sense. The quoted hatred remains part of the law-word fulfillment clause.
- John 15:26 retains “Advocate,” consistent with John 14, and both actual “my Father” phrases. נפק is vocalised as a present participle, supporting “proceeds.” John 15:27 keeps the hearers' predicative “you also are witnesses” and their being with the speaker from the beginning. The chapter remains one continuous speech, with nested quotations in John 15:20 and 15:25.

Source checks for John 16:

- The shared-span review checked all 33 verses and 455 source words, reducing 134 shared spans to eight documented exceptions: from the beginning in 16:4, resumed world objects in 16:8,33, possession in 16:15, narrative said in 16:17, plainly in 16:25 and the two from compounds in 16:27,28. Final source mappings and continuous English received independent review against the frozen chapter hash.
- John 16:6 retains both sorrow coming and filling the plural hearts. John 16:11 separates the passive judged predicate from its copula. In 16:14,15 the partitive from remains distinct from what is mine. John 16:17 uses the narrative perfect-plus-hwa said, while 16:18 separates were from the saying participle and assigns and to its actual prefix. The quoted he said perfects in 16:17,18 remain distinct from present he is saying in 16:18. John 16:19 preserves debating/with/one another/about/this as separate mappings.
- John 16:21 preserves the feminine experiencer and possessors, active bearing of a son and the separate passive human birth. The remembering clause simply says does not remember, without an added no longer. John 16:22 preserves the singular heart with plural possessors. In 16:23 the giving clause is reordered into English while retaining both the speaker as the first question's object and my Father as the later request's addressee. John 16:24 complete is a passive Shaphel participle, with may be separately linked. John 16:28 renders the first-person ethical dative as myself; 16:32 keeps present I am not alone despite nominal-negation perfect-form morphology. John 16:33 separates peace and affliction from their existence predicates and recipients, and gives the verb's resumed object and explicit world one minimal shared object span.
- John 16:1-4 retains the purpose of preventing stumbling, expulsion from their synagogues, the killing/offering supposition, ignorance of the speaker and his Father, and remembering the warning when its time arrives. The second “these things” belongs to the explanation about not speaking from the beginning, rather than being moved into the preceding warning.
- John 16:5-7 preserves going to the sender, the unasked destination question, sorrow filling the hearers' hearts, truthful speech, the benefit of departure, both conditional branches, and the sending of the Advocate. The repeated first-person pronoun after the going predicate remains with that predicate; it does not become the Advocate's subject.
- John 16:8-11 retains conviction of the world concerning sin, righteousness and judgment, with the explanatory clauses about unbelief, going to my Father and no longer being seen, and the judgment of this world's ruler. The d-prefixed explanations remain distinct from the discourse particle דינ.
- John 16:12-15 keeps the still-ungraspable teaching, the Spirit of truth's guidance, not speaking from his own mind, speaking what he hears, making future things known, glorifying the speaker, and receiving and showing what is his. John 16:15 explicitly retains “my Father.”
- John 16:16-19 preserves the two little-while sayings and the speakers' questions. The actual Father noun is unsuffixed in John 16:16, “the Father,” but possessive in John 16:17, “my Father.” John 16:19 ends with seeing again; the Father-going continuation in the supplied Murdock reference is absent from this source verse and is not added.
- John 16:20-24 distinguishes the hearers' weeping and sorrow from the world's rejoicing, sorrow becoming joy, the woman's day of childbirth, a son and the human born into the world, the speaker's seeing the hearers again, and joy no one takes away. Asking the speaker and asking his Father in his name remain distinct, followed by the ask/receive command and complete joy.
- John 16:25-28 keeps parables and future plain speech, asking in the speaker's name, the Father's own love, and departure and return. עינ בגלא is rendered together as “plainly,” an adverbial idiom recorded by [CAL](https://cal.huc.edu/cal_entry_web.php?lemma=%28yn%40bgl%29+N), rather than inventing a separate revealing predicate. John 16:27 says “God,” unlike the supplied Murdock's “Father”; John 16:28's first-person ethical לי is rendered naturally as “myself,” without introducing a third-person recipient.
- John 16:29-33 preserves plain speaking, not one parable, knowing everything, belief in coming from God, scattering each to his place, the Father with the speaker, peace, affliction, courage, and overcoming the world. John 16:31's actual הַימֵנו is tagged as a second-person plural imperative and is rendered “Believe!” rather than the reference's question. Quotation boundaries distinguish the disciples' speech from Jesus' preceding and final replies.

Source checks for John 17:

- The shared-span review checked all 26 verses and 376 source words, reducing 105 shared spans to twelve documented exceptions: the resumed recipient in 17:2, resumed work object in 17:4, possession and before in 17:5, people in 17:6, from compounds in 17:7,8, for compounds in 17:19,20, and the resumed people and before in 17:24. Final mappings and continuous English received independent review against the frozen chapter hash.
- John 17:2 separates giving authority from giving eternal life, with the later dative resuming the people given to the Son. In 17:3 the second-person relative who alone are the true God keeps the Father as the addressed object of knowing; Jesus Messiah remains the separate sent object of knowing. John 17:4 gives the work noun and its resumed object suffix one minimal object span. In 17:5 mine jointly expresses the existential and first-person possessor, while was separately retains the past auxiliary and both with you phrases remain present.
- John 17:6 renders the lexical plural human expression as people. [CAL](https://cal.huc.edu/oneentry.php?cits=all&lemma=br%40%29n%24+N) gives a Syriac plural BNY ANASHA example translated people. To, those, giving, me, from and the world stay separate. The later them is the object of you gave, followed by the separately owned keeping predicate. John 17:9 separates the present it is/not/for/the world/that/I/pray focus construction; 17:11 keeps the name as the given antecedent.
- John 17:12 separates was from keeping and retains the subsequent first-person perfect kept. John 17:13 complete is a passive Shaphel participle with may be separate. In 17:14 the world has hated them; the third-person pronoun is its object, followed by the present nominal they are not of the world clause. John 17:17 separately links for/your/own/word/is/truth. John 17:19,20 retain the three compound for expressions; [CAL](https://cal.huc.edu/oneentry.php?cits=all&lemma=%28l%40%29py+p) records AL APPAY as for the sake of. The active sanctifying myself and passive may be sanctified remain distinct.
- John 17:22 uses both first-person plural pronouns in we/are/one. John 17:23 keeps the passive perfected predicate, into one, and the distinct sending and loving clauses with the Father as actor. John 17:24 retains plural foundations, the later pronoun resuming those given to the speaker, and both the speaker's location and his desire for them to be with him. John 17:25 keeps all three perfect knowing predicates and my righteous Father; 17:26 distinguishes the perfect disclosure from present making known and retains both final in them clauses.
- John 17:1-3 keeps Jesus' raised eyes and prayer, “my Father,” the arrived hour, mutual glorification, authority over all flesh, the gift of eternal life, and knowing the only true God and the sent Jesus Messiah. The name and title remain with their actual source positions.
- John 17:4-5 preserves glorifying on earth, completing the assigned work, and the glory before the world's existence. John 17:5 retains both actual לותכ relations: glorifying the speaker with the Father and the glory he had with the Father. The compound מנ קדמ is rendered “before,” an attested Syriac sense in [CAL](https://cal.huc.edu/cal_entry_web.php?lemma=mn%40qwdm+p); it also occurs before the foundation phrase in John 17:24.
- John 17:6-8 retains disclosure of the Father's name to the people given from the world, their belonging to the Father, giving them to the speaker, keeping the word, receiving the words, certain knowledge, and belief in the sender. John 17:7's actual ידעת is first-person perfect, rendered resultatively as “Now I know,” with the same first-person subject as the supplied Murdock reference; it is not changed to a third-person plural reading.
- John 17:9-12 distinguishes prayer for those given from prayer for the world, the reciprocal possessions and glorification, no longer being in the world, the disciples' remaining, coming to the Father, and keeping them united. In John 17:11 the given antecedent is “your name”; John 17:12 separately speaks of keeping those given and the exception of the son of destruction, with Scripture's fulfillment.
- John 17:13-17 keeps the present coming and speaking, complete joy, the world's hatred, not belonging to the world, the request for protection rather than removal, and sanctification in the Father's truth. The causal גיר in John 17:16 and explanatory d-prefix in John 17:17 remain explicit; “your word” naturally renders the repeated possessive without inventing another owner.
- John 17:18-20 preserves the paired sending into the world, sanctifying himself on their behalf, their sanctification in truth, and prayer also for those believing through their word. The source's on-behalf expressions are rendered idiomatically rather than as literal faces; the believer clause remains attached to the recipients of prayer.
- John 17:21-23 retains all being one, “my Father,” mutual indwelling, unity in the speaker and Father, the world's believing and knowing the sender, the glory given and passed on, perfected unity, and the comparison of the Father's love for them and for the speaker. Each knowing, sending and loving predicate stays in its own linked clause.
- John 17:24-26 keeps the desire for those given to be with the speaker and see his glory, love before the world's foundation, “my righteous Father,” the world's lack of knowledge, the speaker's knowledge and the disciples' knowledge, disclosure of the name, and love and the speaker being in them. Bare “Father” in John 17:24 remains distinct from possessive “my Father” in John 17:25. The prayer forms one continuous quotation after its narrative introduction.

Source checks for John 18:

- The shared-span review checked all 40 verses and 633 source words, reducing 175 shared spans to 20 documented exceptions. These are the two from compounds, past possession in 18:10 and present possession in 18:29,39, openly in 18:20, the three polar-question copulas, the final past copula, and ten resumed references. Ordinary names, noun phrases, negatives, auxiliaries, genitives and clauses remain separately linked. Final source mappings and continuous English received independent review against the frozen chapter hash.
- John 18:2,4 separates the past auxiliary from aware, retaining the stative knowing sense without an episodic was knowing. John 18:2 separately links would/gather/many/times for the repeated meetings. In 18:3 both leading predicates remain, while from is the minimal MEN LWATH compound. John 18:5-8 keeps both first-person forms in each I/am assertion. John 18:9 retains the negative concord as not/lost/even/one/of them, with the given people and loss predicate separately owned.
- John 18:10 gives the struck servant and cut-off ear minimal resumed-object spans, leaving his and right separate. The past possession combines ITH, HWA and on him as had; the names, sword and later actions remain separate. John 18:12 preserves plural commanders and gives the seizure verb's suffix and explicit Jesus one shared object name. John 18:13 separates the one/who/was and high/priest/year constituents, keeping Caiaphas as the year-specific high priest. In 18:14 it/was/Caiaphas and the prior advice are independently linked.
- John 18:15-16 preserves the editorial known-to reading while refining its links: the source YADA is active, the fronted disciple is the object resumed by LEH, and the high priest is the knower. English passive perspective makes that disciple its subject. Only the repeated disciple reference shares a span; was, known to, high and priest remain separate. The English to belongs to the passive knowing predicate, and LEH is not recast as an agent preposition. This follows the [object and pendens constructions](https://d-nb.info/1184881278/34) discussed in sections 7.2, 14.3 and 15.5. John 18:16 also links the brought-in object suffix and Simon as one resumed name.
- John 18:17,25,35 combines only LMA and the relevant copular pronoun in Are/Are/Am; the other you/I, also and predicate stay separate. No why, perhaps or added not is introduced. John 18:18 keeps the fire-making auxiliary separate from its participle and retains the warming purpose. John 18:20 keeps openly separate from the actual spoken verb; in 18:21 the speech partner is rendered to them with English said. John 18:22 gives struck and the single Jesus object separate links while preserving his cheek. John 18:23's final verb is a second-person perfect with a first-person object, so it reads why have you struck me. John 18:26 separates the speaker, kinship, Simon as cutting actor, whose/ear and had/cut off.
- John 18:27 separately links and at/that/hour: the feminine anaphor supplies that while the hour noun owns its own position, parallel to in/that/garden in John 19:41. The English wording is unchanged. John 18:28 keeps the brought object and Jesus together but separates the movement, morning, non-entry and defilement-purpose clauses. John 18:30 keeps the counterfactual he were/not/doing/evil things and we would/not even/have handed/him/over/to you. John 18:34 assigns did to the saying verb rather than others. In 18:35 members/of your people renders the BNY people relation; [CAL, bar sense 2](https://cal.huc.edu/oneentry.php?cits=all&lemma=br+N) documents this membership sense, including Syriac member-of-their-people usage. John 18:36 keeps both present nominal kingdom negations distinct from the counterfactual fighting and its purpose. John 18:37 keeps the first-person form at the start of the birth clause distinct from the preceding king assertion; 18:38 links the ethical departure pronoun as he and the later pronoun with finding. John 18:40's personal existential plus past HWA jointly supplies was without an extra subject or an unwarranted had been.
- John 18:1-4 retains Jesus' departure with his disciples, the Kidron brook and garden, Judas's knowledge of that place and the repeated gatherings, the cohort and guards led from the chief priests and Pharisees, lanterns, lamps and weapons, and Jesus' foreknowledge and question. The source's two leading predicates in John 18:3 remain separate rather than being collapsed into one action.
- John 18:5-9 preserves the explicit Jesus names, Judas standing with the group, the repeated seeking question and identification, backward movement and falling, the command to let the others go, and the purpose of fulfilling the speaker's word. Repeated first-person forms are rendered naturally as “I am”; the fulfillment quotation retains “not even one of them” in its own linked phrase.
- John 18:10-14 keeps Simon Cephas, the sword, the servant's right ear, Malchus, the sheath and cup question, “my Father,” seizure and binding, Annas first, Caiaphas's father-in-law relation, and Caiaphas's year as high priest and prior counsel. The stored morphology marks John 18:12's commanders as plural; the translation preserves that number rather than adopting the supplied reference's singular commander. The English relative clause in John 18:13 attaches the year-specific priesthood to Caiaphas.
- John 18:15-18 retains the other disciple, Simon outside the door, the doorkeeper's question and denial, and the servants' and guards' fire, cold and warming. John 18:15-16 reads the l-marked disciple as a fronted object resumed by לה, giving “was known to the high priest”; the supplied Murdock's “knew the high priest” is an alternative syntactic reading. This is an editorial grammatical judgment, supported by the object-marking, word-order and pendens constructions described in Younansardaroud, [sections 7.2, 14.3 and 15.5](https://d-nb.info/1184881278/34), rather than a change to the stored source.
- John 18:19-24 keeps the questions about disciples and teaching, public speaking and teaching in synagogue and temple, the hearers' knowledge of everything said, the blow on Jesus' cheek, the conditional reply, and Annas sending Jesus while bound to Caiaphas. עינ בגלא in John 18:20 stays together as “openly”; the actual speaking predicate is in that same phrase. Jesus' answer continues across John 18:20-21 without an artificial quotation break.
- John 18:25-27 preserves Simon's standing and warming, the disciple question and denial, the high-priest servant's kinship to the man whose ear Simon cut off, his seeing question, the repeated denial and the rooster's immediate crowing. John 18:26 reorders the speech introduction and its subject/relative clause into natural English, with the speech verb, kinship, cutting actor and ear ownership separately linked.
- John 18:28-35 keeps the movement from Caiaphas to the praetorium, morning, avoiding defilement before eating Passover, Pilate's accusation question, the evildoer counterfactual, the law and killing restriction, the death-word fulfillment, and the king question and reply. למא is used as a general yes/no interrogative in John 18:17,25,35; [CAL's Syriac sense](https://cal.huc.edu/oneentry.php?cits=all&lemma=lmh+c) explicitly cites Peshitta John 18:35, “am I a Judean?” The repeated first-person form in that question does not introduce an extra “I say.”
- John 18:36-40 retains the kingdom's origin, the counterfactual servants fighting, the handing-over purpose, the king reply, birth and coming into the world, testimony and hearing the voice, the truth question, no charge, the release custom and “this king,” and the choice of Barabbas and his being a robber. The adjacent first-person forms in John 18:37 remain with their respective king and birth clauses. The ethical לה accompanying departure in John 18:38 is absorbed naturally, without an invented “away from him”; its later repeated first-person pronoun belongs to finding no charge. Pilate's speech continues from John 18:38 through 18:39.

Source checks for John 19:

- The shared-span review checked all 42 verses and 640 source words, reducing 177 shared spans to eighteen documented exceptions: eight resumed references, the wearing state and law possession, the authority minimizer, wanted and loved, except, the tunic's past copula, the cognate casting-lots construction, already and previously. Final source mappings and continuous English received independent review against the frozen chapter hash; ordinary noun phrases, auxiliaries, negatives, genitives and clauses remain separate.
- John 19:1 gives the scourging verb and explicit Jesus one resumed object name. In 19:3 and joins its actual saying word, with were separate; Peace/to you and King/of the Jews also split. John 19:4 keeps the independent him pronoun distinct from the bringing participle and renders the two negative-concord forms as not/find/even/one/charge. In 19:5 wearing expresses the nominal clothes-on-him state, while while stays separate. John 19:7's two first-person dative forms belong to one law-possession clause.
- John 19:10 translates the licensed/permitted state as I am/authorized in both clauses. John 19:11 separates there/would be/no/authority/for you/over me, with at all for the emphatic APH LA HAD minimizer; the passive gift and from/above remain separate. Its comparison reads the sin/of the one/who/handed me over/to you/is/greater/than/yours. John 19:12 wanted retains the stative past without weakening desire to mere willingness. John 19:13's two naming participles are Ethpeal; Stone/Pavement follows English modifier order. John 19:15 keeps the negative existential apart from the compound except; [CAL](https://cal.huc.edu/oneentry.php?cits=all&lemma=%29lhn+c) records ELLA EN under a negative clause.
- John 19:17 retains the active carrying sense of the formally passive SHQIL participle. [CAL](https://cal.huc.edu/oneentry.php?cits=all&lemma=%24ql+V) explicitly records passive participles with active carrying meaning, including a Syriac carrying-heavy-burdens example. John 19:20 links the read object's suffix and tablet as one referent, while place/in which/Jesus/was crucified, near/the city and the three languages remain separate. John 19:23 splits without/a seam and woven/throughout/from/the top, retaining the tunic's complete lack of a seam. In 19:24 the finite casting verb and cognate infinitive form one cast lots predicate; let us, over it and the whose clause remain separate.
- John 19:25 preserves every conjunction and gives Mary and her identifying feminine demonstrative one resumed name, with of Cleophas separate and no added wife. John 19:26 keeps stative past loved distinct from the standing disciple. John 19:29 retains prior passive placement without adding a standing or there predicate. John 19:31 keeps plural their crosses and reads the for/great-day explanation as a narrator's aside after the quoted Sabbath reason. The request uses passive English for the unnamed plural executioners, preserving both leg-breaking and taking down without making Pilate the singular breaker.
- John 19:33 reads had/already/died, with MEN KADDU as already and the later leg-breaking negation separate. John 19:35's saying verb is perfect, so the witness knows that he has spoken the truth; 19:36,37 retain the actual perfect said predicates with the Scripture citations. John 19:39 separates had from come and uses previously for the temporal compound. John 19:41 separately links in/that/garden and new/burial/chamber, retaining the empty tomb's prior non-use. John 19:42 separates was from beginning and retains both reasons for laying Jesus there.
- John 19:1-6 retains Pilate's scourging of Jesus, the soldiers' thorn crown and purple garments, the king greeting and blows, repeated bringing outside, the man declaration, repeated crucifixion demands, and Pilate's no-charge statements. The charge nouns remain with their actual predicates, and the explicit “not even one” in John 19:4 is retained rather than moved into a neighbouring span.
- John 19:7-11 preserves the law and death-liability claim, making himself the Son of God, Pilate's increased fear, the origin question, Jesus' silence, the two authority clauses, the counterfactual gift from above, and the greater-sin comparison. John 19:9's second Jesus name is the subject of giving no answer, not a vocative added to Pilate's question. John 19:11 explicitly ends “than yours.”
- John 19:12-15 keeps Pilate's desire to release Jesus, the Caesar-friend and rival-king claims, the tribunal and Stone Pavement, Gabbatha in Hebrew, Passover Preparation, about the sixth hour, the king declaration, the repeated take-away/crucify commands, and the chief priests' “no king except Caesar.” The negative existential belongs to the no-king clause, not to its speech introduction.
- John 19:16-22 retains handing Jesus over so that they might crucify him, taking him and bringing him out as distinct actions, his carrying the cross to Skull/Golgotha, two others and Jesus in the middle, Pilate's tablet and its wording, the languages, and the requested alteration and refusal. John 19:20 says “near the city,” without importing the supplied Murdock's explicit Jerusalem. The actual language order is Hebrew, Greek and Latin; the tablet includes “This is Jesus the Nazarene.”
- John 19:23-24 keeps four portions and a portion for each soldier, the seamless tunic woven throughout from the top, the decision against tearing it, casting lots, the Scripture quotation and the soldiers' doing these things. The tunic's lack of a seam is not reduced to a claim that it was merely not sewn from the top. Both the fulfilled Scripture and its saying predicate remain explicit.
- John 19:25-28 preserves the mother/sister/Mary list and its conjunctions, Mary of Cleophas without an added wife relation, Mary Magdalene, the loved disciple standing, the woman/son and mother declarations, taking her to himself from that hour, and the knowing/fulfillment/thirst sequence. No house or home is added to John 19:27. The quoted speech and nested quotations throughout the chapter have explicit speaker boundaries.
- John 19:29-37 keeps the vinegar, sponge, hyssop and mouth, the finished declaration, bowed head and spirit, Preparation and Sabbath reasons, the leg-breaking request and soldiers' actions, Jesus already dead, the side struck with a spear, blood and water, the witness's true testimony and knowledge, belief, the bone quotation and the pierced-one quotation. John 19:31 preserves plural “their crosses.” Its saying predicate remains in the introduction and its request keeps leg-breaking distinct from taking down. John 19:36 retains the actual saying predicate as “which said” before the quotation.
- John 19:38-42 retains Joseph from Arimathea, his request and disciple/fear explanation, Pilate's permission, both taking-body clauses, Nicodemus's earlier nighttime visit, myrrh and aloe, about a hundred Roman pounds, linen and spices, Jewish burial custom, the garden and unused new tomb, and the Sabbath and nearness reasons for laying Jesus there. The weight is the source's ancient Roman-pound unit, without a modern conversion. The causal explanation of Joseph's request is parenthetical so the request to take Jesus' body remains clear; the tomb's existential clause does not acquire an extra standing predicate.

Source checks for John 20:

- The shared-span review checked all 31 verses and 452 source words, reducing 139 shared spans to 18 documented exceptions. Every final mapping and the continuous English received independent review against the frozen chapter hash. John 20:14 keeps Mary's feminine past auxiliary with knowing; John 20:24 retains both copulas in its single negative locative predicate. John 20:19 describes where the disciples were present, and John 20:31 renders the future benefactive naturally as eternal life being theirs.
- John 20:1-4 retains the first day of the week, morning while still dark, Mary Magdalene, the removed stone, Simon Cephas and the other disciple whom Jesus loved, the taken-away Lord, Mary's singular “I do not know,” both disciples running and the other disciple arriving first. John 20:2 explicitly says “our Lord,” without replacing it with “my Lord.”
- John 20:5-10 keeps looking in without entering, Simon's entry, the lying linen cloths, the separate head cloth folded and set aside in one place, the other disciple's entry, seeing and believing, the Scripture explanation, resurrection from the dead, and returning to their place. The single going predicate in John 20:3 is not expanded into two independent going/coming actions. No original undisturbed position is added to the cloths, and no house is added to “their place.”
- John 20:11-14 preserves Mary's weeping and looking, two angels in white, the head/feet positions and Jesus' laid body, the question, “my Lord,” her turning and seeing Jesus standing without recognizing him. The headrest/feet expressions are rendered idiomatically “at his head” and “at his feet”; [CAL's rgl entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=rgl+N) quotes the exact Peshitta John 20:12 phrase with that rendering. The negative knowing clause keeps its own auxiliary before the separate Jesus identification.
- John 20:15-17 retains both questions, the gardener supposition, the respectful address, taking/placing/telling and fetching, Mary's name, the Hebrew reply and its explanation, not coming near, not yet ascending, and the message to the brothers. The actual title רבולי is kept as “Rabbuli,” with the source's “Teacher” explanation; [CAL](https://cal.huc.edu/cal_entry_web.php?lemma=rbwl%29+N) records this Syriac variant alongside the rbwn/rabbouni form. Its first-person ending is retained in the transliteration without an added second “My.” All four Father/God possessive expressions in John 20:17 remain explicit.
- John 20:18-23 keeps Mary Magdalene's announcement, “our Lord,” the evening of that first day, shut doors and fear, Jesus' coming and standing among them, peace, hands and side, the disciples' joy, the repeated peace and sending, breathing, receiving the Holy Spirit and the forgiveness/retention conditions. John 20:18 and John 20:20 retain “our Lord.” The initial “And when” in John 20:22 renders actual וכד; its “these things” and the next verse form one continuous saying.
- John 20:24-26 preserves Thomas as one of the twelve and called Twin, his absence when Jesus came, the disciples' report, the nail places, his fingers and hand, his refusal to believe, eight days, the disciples inside, Thomas with them, shut doors and renewed peace. John 20:25's הו begins “he, however, said”; it does not become “our Lord himself” in the preceding report. The source's plural “my fingers” remains plural, and John 20:26 does not add a possessive to “the disciples.”
- John 20:27-29 keeps singular “your finger,” Jesus' hands and side, the contrast between unbelieving and believing, Thomas's reply “My Lord and my God,” and the seen/believed and unseen/believed contrasts. John 20:29 retains “Now” and the perfect believing forms as a declaration, rather than importing a remembered question. Speech and nested-message quotation marks follow the actual changes of speaker.
- John 20:30-31 preserves many other signs before the disciples, their not being written in this book, the purpose of believing Jesus is the Messiah and Son of God, the temporal “when you believe,” and life in his name. “Eternal” is explicit in the final source phrase and is retained. The discourse particle דינ is rendered contextually, including “Then” in John 20:21, without forcing an adversative meaning into every occurrence.

Source checks for John 21:

- The shared-span review checked all 25 verses and 489 source words, reducing 149 shared spans to nine documented exceptions. Every final mapping and the continuous English received independent review against the frozen chapter hash. The net and Cephas resumptions retain narrow shared objects; ordinary auxiliaries, negatives, feeding benefactives and from/among/the dead are separately linked. John 21:9 has a singular fish, and John 21:23 links the relative prefix and negation to their actual source word before the separate dying predicate.
- John 21:1-6 retains the Sea of Tiberias setting, Simon Cephas, Thomas called the Twin, Nathanael from Cana of Galilee, Zebedee's sons and two others of the disciples, fishing without a catch that night, Jesus on the shore, the food question, the right side of the boat, and the net's heavy catch. The question in John 21:5 is a yes/no food question; למא does not force an added “why.”
- John 21:7-8 keeps the loved disciple's saying to Cephas, both “our Lord” declarations, Simon's tunic and girding, his nakedness, entering the sea to come to Jesus, the other disciples' boat, the net and fish, and “not very far,” about two hundred cubits. The speech verb stays in its own subject/relative-clause span; the distance explanation is parenthetical and the source unit is retained without a modern conversion.
- John 21:9-14 preserves coals laid out, fish laid on them, bread separately, bringing some newly caught fish, Simon Cephas going up and drawing the net ashore, 153 large fish, all that weight and the untorn net, the meal invitation, knowing “our Lord,” Jesus giving the food to them, and the third appearance after rising from the dead. No boat noun is added to Simon's going-up verb and no extra disciples noun is added to the giving clause.
- John 21:15-17 retains Simon son of Jonah, the first comparison “more than these,” all three love questions and replies, “my Lord,” and the feed-for-me requests. The actual lambs/sheep terms are kept distinct in their source links; the final נקותי is rendered “my sheep,” a sense recorded alongside “ewe” by [CAL](https://cal.huc.edu/oneentry.php?cits=all&lemma=nqy+N), without inferring a restricted audience from grammatical gender. John 21:17 has both understanding everything and knowing the speaker's love; repeated second-person pronouns remain with their respective predicates.
- John 21:18-19 preserves the doubled “Truly,” the contrast between young self-girding and future old age, stretched hands, another's girding and leading, the unwanted destination, the death/glorifying-God explanation, and “Come after me.” The perfect in the future temporal old-age clause is rendered naturally “when you grow old,” rather than treating the predicted age as already past.
- John 21:20-21 keeps Simon Cephas turning, the loved disciple coming after him, the recollection of supper and Jesus' breast, “my Lord,” the betrayal question, and the question about this other disciple. There is no added “called” predicate between Simon and Cephas, and the demonstrative is rendered “this one.”
- John 21:22-23 preserves “this one” remaining, coming, the personal question and the emphatic “You come after me,” then distinguishes the brothers' report from what Jesus actually said. הנא refers to the disciple and is not an added locative “here.” The first לכ is read with coming (“come to you”), while the second remains in the question “what is that to you?” Both forms are preserved in the repeated saying. The negative saying clause keeps the actual speech predicate before the contrasting conditional quotation.
- John 21:24-25 retains the disciple's testimony about all these things, his also writing them, plural “we know” and true testimony, many other things Jesus did, “one by one,” the world's insufficiency for the books, singular “I suppose,” and the further writing clause. The negative remains with “not even the world itself.” The final relative/anaphoric link is rendered through two natural English sentences, retaining its reference as “they” in the conditional.

Source checks for Acts 1:

- The shared-span review checked all 26 verses and 414 source words, reducing 136 shared spans to 23 documented exceptions. Every final mapping and the continuous English received independent review against the frozen chapter hash. Ordinary participants, predicates, negatives, auxiliaries, genitives and reordered modifiers retain separate owners. Acts 1:7 keeps the nominal negation in the present; 1:15 preserves both temporal prefixes and the resumed demonstrative; 1:19 retains Dama as part of the field's name despite its misleading exported lemma gloss. The controlled infinitive in 1:22 keeps the chosen man as the witness, and the narrator's aside in 1:18-19 stays outside Peter's speech.
- Acts 1:1-9 retains the first book, Theophilus, our Lord Jesus Messiah, the apostles and Holy Spirit, forty days, eating bread together, the Father's promise, water and Spirit baptism, the kingdom question, the Father's authority, witnesses among the Samaritans, and both the cloud receiving Jesus and his being hidden. Acts 1:7 says “the time or times,” without replacing the actual terms with a remembered Greek formulation.
- Acts 1:10-14 keeps the two men in white, the ascension and return, the mountain called House of Olives, about seven stadia from Jerusalem, the upper room, all eleven names, women, Mary the mother of Jesus, and his brothers. The distance retains the source unit; no Sabbath-day journey is substituted. Judas is explicitly “son of James,” rather than “brother of James.”
- Acts 1:15-19 retains Simon Cephas, about one hundred twenty, “our brothers,” Scripture, the Holy Spirit speaking through David, Judas's guide role and ministry share, and the field, wages of sin, fall, bursting and entrails. The narrator's parenthesis in 1:18-19 is separate from Peter's speech, which resumes in 1:20. The field's name is preserved as “Haqel Dama,” followed by its explicit “Field of Blood” explanation and the source's “language of the country.”
- Acts 1:20-22 preserves both Psalm sayings and the requirement for a witness of the resurrection from among those who were with the group during Jesus' activity. The initial co-verb in 1:22 is rendered “beginning from the baptism,” continuing the preceding description of Jesus' activity; [CAL's nqp entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=nqp%232+V), causative sense 3, cites this exact verse for “to begin to do something.”
- Acts 1:23-26 keeps Joseph, Barsabbas, Justus and Matthias, the prayer to the Lord who knows all hearts, present “you choose,” ministry and apostleship, Judas's departure to his own place, casting lots, the lot coming up for Matthias and his being counted with the eleven apostles. No different tense or downward lot motion is imported from the reference translation.

Source checks for Acts 2:

- The shared-span review checked all 47 verses and 633 source words, reducing 190 shared spans to 23 documented exceptions. All final mappings and the continuous English received independent review against the frozen chapter hash. Ordinary modifiers, predicates, auxiliaries, negatives and reordered constituents retain separate owners. In 2:23, “prior knowledge” has separate modifier and noun links. The feminine suffixes in 2:24 resume Sheol; they do not introduce an extra male possessor or agent. The plural resumptive in 2:39 belongs to “whom,” while “God himself” remains singular.
- Acts 2:1-4 retains the completed days of Pentecost, the gathered people, sudden sound from heaven like a mighty wind, the filled house, divided tongues like fire resting on each, the Holy Spirit and speaking in various languages. The co-verb in 2:4 is rendered “began”; [CAL's nqp entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=nqp%232+V) records this causative co-verb sense explicitly.
- Acts 2:5-11 preserves the listeners' own languages and every group and region in the list. The actual אלניא is retained as “Alans,” without asserting a modern geographic identity or substituting the supplied reference's “Elamites.” The source has plural “Jews” and “Cappadocians” in 2:9 and “Arabs” in 2:11. The question about native languages continues through the ethnic list; “Look, we hear” then begins a declaration within the same speech.
- Acts 2:12-21 retains the separate astonishment and mockery, drinking new wine and becoming drunk, Simon Cephas and the eleven, the raised voice and address, three elapsed hours, Joel, sons and daughters prophesying, young men's visions, elders' dreams, servants and handmaids, signs and wonders, the great and terrible day, and calling on the Lord's name to live. The cardinal “three hours” is preserved rather than replaced by a modern clock time.
- Acts 2:22-24 keeps Jesus the Nazarene, manifestation by God, deeds of power, signs and wonders, foreknowledge and will, appointment, delivery to the wicked, crucifixion, killing, God's raising and the pains of Sheol. The passive agent in 2:22 may be introduced by מנ; [CAL's mn entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=mn+p) explicitly records this Syriac use. The appointment in 2:23 is a passive participle with a past auxiliary and anticipatory dative, without an additional invented male subject.
- Acts 2:25-32 retains David's foreseeing and right-hand declaration, my glory, the body resting in hope, the soul and holy one, Sheol and corruption, singular “path of life,” open speech about the patriarch and his tomb, the oaths and fruit of the body, first-person “I will seat,” the Messiah's resurrection and all the witnesses. The impersonal permission expression in 2:29 is rendered naturally “I may speak.” The Psalm quotation continues through 2:28; the oath in 2:30 remains direct speech. The actual SEDRA form for “oaths” is feminine plural, despite the reference's singular rendering. The temporal co-verb in 2:25 is rendered “beforehand,” consistent with [CAL's qdm entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=qdm+V), rather than adding a movement predicate.
- Acts 2:33-40 preserves exaltation by God's right hand, the Father's promise concerning the Holy Spirit, the poured-out gift, seeing and hearing, David's non-ascension and Psalm quotation, Jesus as Lord and Messiah, the listeners' hearts, Simon and the apostles, repentance, baptism in the Lord Jesus' name, forgiveness, the Spirit's gift, the promise's recipients, God's call, testimony and the imperative to live free from the crooked generation. Both speech verbs in 2:34 retain their own source links, and the final pronoun in 2:39 is absorbed as a resumptive rather than duplicated as an extra object.
- Acts 2:41-47 retains readily receiving, believing and baptism, about three thousand souls, apostolic teaching, prayer and the explicit Eucharist, signs and wonders in Jerusalem, common property, selling and distributing according to need, temple and house, bread and food, gladness, simplicity of heart, praising God and our Lord's daily addition to the ecclesia. The stative passive participle in 2:45 is rendered “he was in need of,” preserving the recipient's state; compare [CAL's snq entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=snq+V). In 2:47 the passive יהיבינ is rendered “they were held in favor,” preserving the believers as recipients of favor rather than changing them into givers of mercy. The final living expression remains “those who were coming to life.”

Source checks for Acts 3:

- The shared-span review checked all 26 verses and 378 source words, reducing 126 shared spans to 13 documented exceptions. All final mappings and the continuous English received independent review against the frozen chapter hash. Ordinary names, participants, predicates, auxiliaries and reordered modifiers retain separate owners. The two possession predicates in 3:6 share only “have,” with the subjects and negation separately linked. The relative and resumed denial object in 3:13 share “whom,” without a redundant English “him.” The compound prepositions and resumed genitives have narrow exceptions rather than whole-clause links.
- The carrying construction in 3:2 preserves the plural carriers and singular man through English passive voice; [CAL's shql entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=%24ql+V) records the passive participle's active carrying sense. In 3:11, “he was holding on” likewise follows the active use of the passive participle recorded under [ahd](https://cal.huc.edu/oneentry.php?cits=all&lemma=%29xd+V). Acts 3:12 supplies the seeing verb's implicit object as “it,” while the explicit demonstratives keep their own source links. The impersonal opening in 3:23 is “it will be”; its ambiguous exported second-person tag does not turn the saying into a new address to “you.”
- Acts 3:1-3 retains Simon Cephas and John, going up together, the prayer time and ninth hour, the man lame from his mother's womb, people carrying him, their habitual bringing and placing him at the Beautiful gate, his request for alms and the temple entrants. Carrying, bringing and placing remain distinct source predicates. The opening “when” clause continues into the second verse.
- Acts 3:4-9 preserves both apostles' looking and speaking, the man's expectation, gold and silver, the gift, Jesus Messiah the Nazarene, rising and walking, the right hand, immediate strengthening of feet and heels, leaping, standing, entering and praising God. Acts 3:6 has no source words for the supplied reference's “our Lord,” so that title is not added.
- Acts 3:10-14 keeps recognition of the beggar, his daily sitting and asking, amazement and wonder, holding Simon and John, the people running to Solomon's portico, Simon's reply, the power/authority question, Abraham, Isaac, Jacob, our fathers, his Son Jesus, delivery and denial before Pilate, and Pilate's judgment concerning release. Both “Holy” and “Righteous” remain explicit in 3:14, and its final request retains “be given to you.”
- Acts 3:15-18 retains the Prince of life, killing, God's raising from among the dead, all the witnesses, faith in his name, the explicit strengthening and healing, faith's gift of soundness, “my brothers,” acting “in error,” the leaders, God's earlier proclamation through all the prophets and its fulfillment. Acts 3:18 explicitly says “his Messiah,” without dropping the possessive. The two occurrences of “this” in 3:12 distinguish the deed from the man who walks.
- Acts 3:19-21 keeps repentance and turning back, blotted-out sins, times of rest coming from the Lord's presence, sending the one prepared for the hearers, Jesus Messiah, heaven receiving him and the fulfillment of the times of all the things God spoke through his holy prophets from ancient times. The rest clause belongs to source 3:19, despite its placement in the supplied reference's 3:20. No different restoration formula is substituted for the actual fulfillment/times wording.
- In 3:19, men qdam is the compound “from,” with “the presence” and its resumed Lord genitive separately linked; [CAL](https://cal.huc.edu/cal_entry_web.php?lemma=mn%40qwdm+p) records both the source-presence construction and simple “from” with divine figures. In 3:15, the separate words for “from,” “among” and “the dead” each own their English constituent. In 3:24, the relative prefix and past auxiliary remain outside the compact “after him” prepositional span.
- Acts 3:22-25 preserves Moses' prophet “like me,” the Lord's raising from among the brothers, listening, the person perishing from their people, Samuel and those who were after him, the proclaimed days, children of the prophets and of the covenant, “our fathers,” Abraham, his seed and all the earth's families. The noun in 3:23 is read as “people” in context rather than the exported homograph's base gloss “with”; its feminine possessive resumes the source's feminine “soul/person.” The Moses quotation continues through 3:23.
- Acts 3:26 retains “to you first,” God's raising and sending his Son, the blessing clause and the explicit condition “if you turn back and repent from your evil deeds.” Both turning and repenting remain present; the sentence is not changed into an unconditional statement that he turns each hearer from evil.

Source checks for Acts 4:

- The shared-span review checked all 37 verses and 544 source words, reducing 174 shared spans to 22 documented exceptions. Every final mapping and the continuous English received independent review against the frozen chapter hash. Ordinary names, predicates, auxiliaries, negatives and reordered constituents retain separate owners. The remaining exceptions cover narrow resumptions, the “openly” and punishment idioms, and past possession predicates. Relative prefixes can own English separately from their verb stems, including “whom” and “raised” in 4:10 and the complement “that” and “standing” in 4:14.
- Acts 4:1-7 retains speaking to the people, the priests and Sadducees, plural temple rulers, anger, teaching and preaching in the Messiah about the resurrection from among the dead, arrest, evening, about five thousand men, the assembly, Annas, Caiaphas, John and Alexander, and the high-priestly family. The resurrection noun and its “about” preposition retain separate links; the phrase is not reduced to “the Messiah's resurrection.” The belief auxiliary in 4:4 stays with believing, before the separate count predicate. The ethical dative in 4:3 resumes evening itself. Annas' title in 4:6 is split as “the chief / of the priests.”
- Acts 4:8-12 preserves Simon Cephas, the Holy Spirit, the rulers and elders of the house of Israel, examination for the good deed and healing, Jesus Messiah the Nazarene, crucifixion and God's raising, the man's standing in sound health, the rejected stone and head of the corner, and salvation in no one else. The stone quotation is nested within Peter's speech. The modal in 4:12 is rendered “by which we are to live,” retaining its fitting/proper sense rather than treating it as bare ability; compare [CAL's wālē entry](https://cal.huc.edu/cal_entry_web.php?lemma=wly+X).
- Acts 4:13-18 keeps hearing Simon and John's openly spoken message, their being unlearned in books and ordinary men, recognition that they had gone about with Jesus, the healed lame man, inability to answer, removal from the council, the manifest sign known throughout Jerusalem, the inability to deny it, threats and the prohibition on speaking or teaching in Jesus' name. The speech's “eye/open” idiom is translated naturally rather than made into an anatomical claim.
- Acts 4:19-24 retains Simon Cephas and John, the choice of whom to obey before God, the single “cannot keep from speaking” assertion about what they saw and heard, renewed threats, release, grounds for punishment, the people praising God, the man's age of more than forty, the report to their brothers, and the united prayer to God who made heaven, earth, plural seas and everything in them. In 4:20, “not” and “keep from” preserve the two negative constructions separately; only the relative/object resumption in “of what” is shared. The punishment idiom in 4:21 is documented by [CAL's swm entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=swm+V), which cites this verse explicitly. In 4:22, the personal copula, age constituents and healing clause are split, with a narrow relative/resumptive exception for “whom.”
- Acts 4:25-30 preserves the Holy Spirit, David as “your servant,” the Psalm's past verbs, the kings and rulers, the Lord and his Anointed One, the holy Son Jesus, Herod and Pilate with the nations and Israel's assembly, God's hand and will, looking and seeing the threats, servants' open proclamation, healings, wonders and signs. Herod and Pilate are the people who gathered, not additional objects of anointing. The Psalm quotation closes at 4:26; the community's prayer continues through 4:30.
- Acts 4:31-35 retains praying and supplication, the shaken gathering place, filling with the Holy Spirit and openly speaking God's word, one soul and one mind, possessions and common ownership, the apostles' resurrection testimony, great grace “with them all,” no one in need, selling fields and houses, bringing the proceeds, laying them at the apostles' feet and distribution according to need. Acts 4:32 says “mind,” without substituting “heart”; 4:33 does not add “Lord” to Jesus Messiah; 4:34 retains “and” between fields and houses.
- The possessive-state participles with past auxiliaries in 4:32 and 4:34 form narrow “owned” exceptions; [CAL's qny entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=qny+V) records the passive participle's active possession meaning. The subjects, relative prefixes and properties stay outside those spans. The existential possession constructions in 4:32 and 4:37 share only “had.” Acts 4:35 retains the singular passive giving predicate and the individual recipient's state, “he was in need of.” In 4:30, the name's anticipatory suffix and Son's genitive prefix share only “of,” while “your,” “holy,” “Son” and “Jesus” retain separate source owners.
- Acts 4:36-37 names Joseph, the apostles' naming him Barnabas, the source explanation “Son of Encouragement,” his being a Levite from Cyprus, his field, sale, price and laying it before the apostles' feet. No different personal name or additional birthplace claim is supplied.

Source checks for Acts 5:

- The shared-span review checked all 42 verses and 646 source words, reducing 192 shared spans to 27 documented exceptions. Every final mapping and the continuous English received independent review against the frozen chapter hash. Ordinary participants, predicates, auxiliaries, negatives, titles and reordered constituents retain separate owners. Exceptions cover narrow resumptions, lexical compounds, temporal expressions, the narrative “began” and “rose up” predicates, and the conditional formula in 5:38. The partitive “some / of / the money” in 5:2 keeps separate owners without introducing another male actor.
- Acts 5:1-4 keeps Ananias and his wife Sapphira, the field sale, taking and hiding part of the price, the wife's knowledge, bringing money to the apostles' feet, Satan filling the heart, lying to the Holy Spirit and God, and the owner's continuing authority over the sale money. The questions and statements continue as one speech through 5:4. The text's knowing predicate is not expanded into a separate claim that the wife expressly consented.
- Acts 5:5-11 preserves falling and dying, the young men's gathering and carrying the body and burial, three hours, the wife's lack of knowledge, Simon's price question and her answer, agreement to test the Lord's Spirit, the buriers at the door, her falling before plural “their feet,” and burial beside her husband. Acts 5:9 begins with the causal “Because you agreed,” rather than a substituted “Why?” question. The feet/buriers/husband chain retains both possessive resumptions. The same-hour idiom in 5:10 is rendered “at once,” and the circumstantial death predicate as “found her to be dead.” Acts 5:11 uses the established “ecclesia” terminology.
- Acts 5:12-16 retains many signs and wonders, gathering in Solomon's portico, the others' reluctance to approach, the people's magnifying them, added believers in the Lord, a multitude of men and women, the sick in the streets on beds, Simon's coming, his shadow, the surrounding cities of Jerusalem, unclean spirits and all being healed. The multitude noun in 5:14 is singular. Acts 5:15 renders the focus before “his shadow” as “at least”; [CAL's ˀpn entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=%29pn+c) records the Syriac “if only” use before nouns. The Lord in 5:14 belongs with believing, rather than being reassigned as the target of adding. The possession construction in 5:16 shares only “had,” with “who,” “unclean” and “spirits” separately linked.
- Acts 5:17-23 keeps the jealous high priest and those of the Sadducees' teaching, laying hands on the apostles, seizing and binding them, the prison, the Lord's angel at night, bringing them out, going and standing in the temple, all the words of life, departure at daybreak, entry and teaching. The opened door in 5:19 is singular; the guards' doors in 5:23 are plural. The prison report retains secure locking, guards at the doors, opening and finding nobody inside; its introductory אמרינ is a third-person plural participle, “saying.” The beth asire prison compound has narrow shared spans, with the prepositions separately linked.
- Acts 5:24-28 preserves the chief priests and temple rulers hearing and wondering, the messenger's report, the confined men now standing and teaching in the temple, bringing them without force for fear of the people, the whole council and the high priest beginning to speak. His opening is a strict-command question, followed by Jerusalem filled with the apostles' teaching and the desire to bring this man's blood upon the authorities. The beginning co-verb and the following saying infinitive keep their own source links.
- Acts 5:29-34 retains Simon with the apostles, obedience to God more than human beings, the God of our fathers, Jesus raised and killed by hanging on a tree, God's establishing him as leader and giver of life and also exalting him, repentance and forgiveness for Israel, the witnesses and Holy Spirit, anger and considering killing, and Gamaliel's honored standing and order to remove the apostles briefly. Acts 5:32 says God “gave” the Spirit to those who “believe in him,” without substituting a different tense or the familiar “obey him” reading.
- Acts 5:35-39 keeps caution, consideration, Theudas' claim, about four hundred men, his killing and the followers' scattering and coming to nothing, Judas the Galilean, registration for the head tax, leading many astray, his death and all his followers' scattering. Both “this plan” and “this work,” their dissolving and passing away, human versus divine origin, inability to overthrow and the danger of standing against God remain explicit. No external dates or chronological harmonization are supplied.
- In 5:38, en hu d is the compound conditional “if,” documented by [CAL's en entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=%29n+c). Only the conjunction is shared; “for,” “from” and the body's “are” retain separate owners. The control metaphor in 5:39 reads “it does not lie in your hands to overthrow it,” with the negative, predicate, hands and infinitive separately linked. In 5:37, “head” and “tax” retain separate owners; the preposition and noun also stay separately linked in “from before them” in 5:41.
- Acts 5:40-42 retains agreeing with Gamaliel, calling the apostles, scourging, the speaking prohibition in Jesus' name, release, rejoicing in worthiness to be dishonored for the name and continuing daily teaching in the temple and at home. The assent clause belongs to source 5:40, despite the supplied reference's placing it at the end of 5:39. The final preaching clause explicitly includes “our Lord Jesus Messiah.”
- Acts 5:41 renders the worthiness construction as “they were worthy,” without forcing a pluperfect. The final persistence clause is “every day they did not cease to teach,” preserving the daily adverb and the negative past auxiliary separately from “cease,” “teach” and “preach.”

Source checks for Acts 6:

- The shared-span review checked all 15 verses and 213 source words, reducing 66 shared spans to eight documented exceptions. Every final mapping and the continuous English received independent review against the frozen chapter hash. Ordinary names, predicates, negatives, auxiliaries and reordered constituents retain separate owners. The opening temporal phrase has narrow case and demonstrative resumptions; the testimony relative, Lord/God genitives, apostles after “before,” and destroy/place object likewise retain their actual resumed referents. The narrative past “arose” in 6:9 is a minimal finite-predicate exception.
- Acts 6:1-4 retains the increasing disciples, Hellenist disciples' complaint against the Hebrews, neglected widows and daily service, the twelve apostles, the whole assembly, the word of God and serving tables. The opening resumptive pronoun belongs to “those days,” without adding a separate “among them” group. The speech preserves searching and choosing seven, “my brothers,” attestation, fullness of the Spirit of the Lord and wisdom, appointment over the matter, and continued prayer and ministry of the word.
- In 6:3, “about whom there is testimony” separates the relative, preposition, existential and testimony noun; the source has no separate adjective “good.” “And we will be constant” in 6:4 keeps the pronoun, future copula and adjective separate. The Spirit's anticipatory Lord suffix in 6:3 and the word's God suffix in 6:7 have narrow genitive exceptions, unlike the unsuffixed word/God phrase in 6:2.
- Acts 6:5-6 keeps Stephen, Philip, Prochorus, Nicanor, Timon, Parmenas and Nicolas, the latter described as an Antiochian proselyte. Stephen is full of faith and the Holy Spirit. The selected men “stood” before the apostles: the source's intransitive verb is retained rather than replaced by a different actor presenting them. Prayer and laying a hand on them remain distinct predicates.
- Acts 6:7-10 distinguishes God's word growing from the number of disciples increasing greatly in Jerusalem; each past auxiliary stays with its own predicate. The final group in 6:7 is many people “from among the Jews,” without substituting “priests.” Acts 6:8 has “grace and power,” and the synagogue/group list retains the Freedmen, Cyrenians, Alexandrians, Cilicia and Asia. In 6:10 the Spirit “was speaking through him,” preserving the source speaker rather than reversing the roles into Stephen speaking by the Spirit.
- Acts 6:11-14 retains sending men and instructing them to make their allegation, rather than supplying a different inducement verb; stirring up the people, elders and scribes; coming, rising against, seizing and bringing Stephen before the council; and false witnesses speaking against the law and this holy place. Their second statement continues into 6:14, including hearing him say that this Jesus the Nazarene will destroy the place and change the customs Moses delivered “to you,” not “to us.” These remain the witnesses' quoted allegations.
- In 6:14, the independent subject pronoun resumes Jesus as “himself,” while the destroy verb's object suffix resumes “place.” “This” remains separately linked to its demonstrative; no additional “it” or second place is introduced. The relative prefix and delivered verb in the Moses clause share their source word through separate English spans.
- Acts 6:15 preserves all those sitting in the council looking at him and seeing his face “like the face of an angel.” The simile is retained without adding a claim that he became an angel.

Source checks for Acts 7:

- The shared-span review checked all 60 verses and 862 source words, reducing 277 shared spans to 49 documented exceptions. All final mappings and continuous English received independent whole-chapter review against the frozen chapter hash. Ordinary participants, predicates, auxiliaries, negation, modifiers, numbers and reordered constituents retain separate owners; the remaining exceptions cover narrow resumptions, genuine compound expressions and narrative predicates that cannot naturally be split.
- Acts 7:2 separates the pronominal subject in `ithawhy` from the perfect past auxiliary `hwa`; the latter is a verb, not the similarly spelled independent pronoun. The speech in 7:3 uses narrative “he said,” without imposing an earlier speech through a pluperfect. In 7:5 the promise is active; the inheritance infinitive retains both its nominal and possess-it force through repeated links to its actual owner. In 7:11 the negative possession predicate and the explicit fathers' resumption have separate narrow exceptions, with the satisfaction infinitive retained as “to satisfy their hunger.”
- In 7:5 and 29, “he” belongs to the singular possessor suffix on leh, while “had” retains the past existential and dative jointly. In 7:29 the plural hwaw agrees with the two sons and cannot independently own singular “he.” Compare the possessive l-construction under [CAL’s ith, sense 2(a)](https://cal.huc.edu/getlex.php?coord=620060624&word=4). The narrative predicates in 7:3, 5, 10, 12, 14, 23 and 54 likewise keep their joint past or copular heads while separately assigning their pronouns; the object “him” in 7:10 belongs to the rescue verb’s suffix.
- Genitive prefixes and personal possessives remain outside the minimal resumed noun heads. In 7:11, “our” comes from the first-person suffix on “fathers,” while the third-person dative resumes those fathers; in 7:14, “his” refers to Joseph through the father noun’s suffix, while the brought object is Jacob. In 7:58, the suffix on “feet” resumes “young man,” with “of a” separately linked to the following word. The kneeling idiom in 7:60 retains both lexical words under “knelt,” with “he” on the finite verb. In 7:55, kad owns “as,” while the masculine singular participle owns “he was” and “standing” in the narrative context. These refinements change links in 22 verses and preserve all 60 verses’ English wording.
- In 7:21, the passive casting-out verb takes men with “his mother” as its agent: [CAL’s men, sense 3](https://cal.huc.edu/oneentry.php?cits=all&lemma=mn+p) documents this Syriac use, supporting “by his mother.” The preposition’s basic gloss “from” does not require changing that passive-agent relation.
- Acts 7:1-7 preserves the high priest's question, Stephen's address to “our brothers and our fathers,” the God of glory, Abraham, Mesopotamia and Haran, departure from land and kindred, the promise without an existing son or even a footstep of inheritance, four hundred years of foreign sojourn and mistreatment, judging the enslaving nation and serving God in this place. The God quotation continues through 7:7 inside Stephen's continuing speech.
- Acts 7:8-16 retains circumcision on the eighth day, Isaac, Jacob and “our twelve fathers,” jealousy of Joseph, Egypt, Pharaoh's appointment, the famine and Canaan, the first and second journeys, Joseph actively making himself known to his brothers, and seventy-five persons. In 7:16 both transport and deposition are singular, and the text names Shechem, Abraham as buyer and Hamor's sons. The wording is retained without harmonizing the burial account with another passage.
- Acts 7:17-22 keeps the sworn promise, the people's increase and strength, the king who did not know Joseph, mistreated fathers, the infant order, Moses beloved by God, three months in his father's house, his casting out “by his mother,” Pharaoh's daughter and his Egyptian education. The actual adjective in 7:22 is rendered “prepared” in words and deeds; [CAL's ʿtyd entry](https://cal.huc.edu/cal_entry_web.php?lemma=%28tyd+A) records the prepared/made-ready sense rather than requiring a different remembered adjective.
- Acts 7:23-30 retains forty years of age, the thought entering Moses' heart, visiting his brothers, the oppressed kinsman, avenging and doing justice, killing the Egyptian, his expectation that the brothers understood God's deliverance through his hand, the peace appeal and its rejection, yesterday's killing, flight to Midian and two sons. The next forty-year period is explicitly spent “there”; the angel of the Lord appears in the wilderness of Mount Sinai in fire in a bush.
- Acts 7:31-38 preserves seeing and approaching the sight, the Lord speaking in a voice, the God of Abraham, Isaac and Jacob, Moses' trembling, sandals and holy ground, seeing oppression and hearing groaning, descending to deliver and sending Moses to Egypt. God sends the rejected Moses as ruler and deliverer through the angel; signs, wonders and mighty deeds, Egypt, the Sea of Reeds and forty wilderness years remain explicit. The prophet saying includes “Lord God,” “from among your brothers,” “like me” and listening to him. The final living words are received “to give to us.”
- Acts 7:39-43 retains the fathers' refusal to submit, turning back to Egypt in their hearts, Aaron's requested gods, uncertainty about Moses, making the calf, sacrifices to idols and rejoicing in their own handiwork. The prophetic question keeps forty years, a slaughtered animal or a sacrifice, and Israel. The actual names are retained as Malchum and Rephan, with the images explicitly made so that they might worship them and the removal beyond Babylon. In 7:42 `lma` has its general Syriac interrogative use, rather than requiring a question asking why; see [CAL's lmh entry, sense 4](https://cal.huc.edu/oneentry.php?cits=all&lemma=lmh+c). In 7:43 `lhal men` is the compound “beyond,” retaining the far-side relation; see [CAL's lhl entry, sense 3.1](https://cal.huc.edu/cal_entry_web.php?lemma=lhl+X).
- Acts 7:44-50 keeps our fathers' tabernacle of testimony, the command and pattern shown to Moses, the fathers bringing it in with Joshua, the inherited land, expelled nations and the tabernacle being carried until David's days. David's request is for the God of Jacob, and Solomon builds the house. The Most High's refusal to dwell in what is made by hands, heaven as throne, earth as footstool, the house/rest-place questions and God's own hand remain in the quoted prophecy.
- Acts 7:51-53 retains the hard-of-neck and uncircumcised-heart/hearing descriptions, the third-person possessives describing the addressed people, then the direct “you” standing against the Spirit of holiness. The causal question retains both persecuting and killing, those giving notice beforehand about the Righteous One's coming, the hearers' handing him over and killing him, the law through angels' command and failure to keep it. Stephen's outer speech closes only after 7:53; the prophetic quotations remain nested within it.
- In 7:56 `kad` with each participle marks the accompanying state in perception syntax. The narrow exceptions “open” and “standing” retain those predicates naturally, without adding an independent “lies” or an opening event; the Son/Man possessive resumption is separately documented. In 7:60 the knee-placement idiom is “knelt,” while the following cry, loud voice, sin and prayer remain separately owned.
- Acts 7:54-58 preserves anger in the hearers' souls and gnashing, Stephen explicitly full of faith and the Holy Spirit, his looking and seeing God's glory, Jesus standing at God's right hand, the Son of Man, the loud cry and stopped ears, rushing, seizure, removal outside the city and stoning. Those who testified against him lay their garments at the feet of the young man Saul; no additional action by Saul is inserted here.
- Acts 7:59-60 retains stoning Stephen while he prays and speaks, “Our Lord Jesus, receive my spirit,” kneeling and the loud cry, “Our Lord” and the request not to hold this sin against them, and falling asleep after speaking. Both first-person plural Lord possessives remain explicit.

Source checks for Acts 8:

- The shared-span review checked all 40 verses and 590 source words, reducing 169 shared spans to 31 documented exceptions. Every final mapping and the continuous English received independent whole-chapter review against the frozen chapter hash. Ordinary subjects, predicates, auxiliaries, negation, modifiers, names and reordered constituents remain separately linked. The exceptions cover narrow resumptions, possession, lexical compounds, narrative predicates and subject-coreferential datives.
- In 8:5, 8:9 and 8:25, the dative pronouns refer back to the subjects of going down, dwelling and returning. The English renders each as one natural predicate with a documented narrow exception, rather than adding emphatic “himself” or a different recipient. See Farina's discussion of the subject-coreferential dative in [An Outline of Middle Voice in Syriac, chapter 5](https://ricerca.sns.it/retrieve/handle/11384/86010/37632/Farina.pdf). The ordinary past auxiliary and relative prefix remain separately owned in 8:9.
- Acts 8:1-4 retains Saul's approval and participation in Stephen's killing, that day's persecution of the ecclesia in Jerusalem, scattering through Judea's villages and among the Samaritans except for the apostles, Stephen's gathering and burial by believing men, and Saul's persecution of God's ecclesia. His entry into houses, dragging away men and women and delivery to prison remain distinct from the scattered believers' proclamation.
- Acts 8:5-8 keeps Philip's descent to a Samaritan city, proclamation concerning the Messiah, the people's listening and conviction because of the signs, and the city's great joy. In 8:7 the masculine human criers are distinguished from the feminine-plural spirits that come out of them; the translation does not silently substitute a different subject throughout the sentence.
- Acts 8:7 renders the feminine holding predicate with the unclean spirits as its subject, while the masculine crying predicate belongs to the people. [CAL's ahd entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=%29xd+V) explicitly records the Peal passive participle in the active sense “holding”; the tense label alone must not reverse the source roles. The relative prefix and object dative share only “whom,” with the holding, crying and exit predicates separately linked.
- Acts 8:9-13 preserves Simon's long residence, sorceries, misleading the Samaritan people, self-exaltation, the people's description of him as God's great power, and their subsequent belief and baptism. Philip proclaims God's kingdom in the name of “our Lord Jesus Messiah”; Simon's belief, baptism, attachment to Philip and astonishment remain explicit.
- Acts 8:14-19 names Simon Cephas and John as the apostles sent from Jerusalem, distinguishes receiving God's word from receiving the Holy Spirit, and retains baptism in the name of “our Lord Jesus.” The laying on of a hand, reception of the Spirit and Simon's offer of money and request for authority remain separate actions.
- In 8:17, “those people were receiving” clarifies the implicit receiving subject as the people prayed over in 8:15, rather than the apostles laying the hand on them. In 8:21 the nominal negation is present “your heart is not upright,” with copula, negation and adjective separately owned. In 8:25 the pronoun after the testimony verb is rendered “to them,” preserving the recipients within the testimony and teaching sequence.
- Acts 8:20-24 retains Simon Cephas' condemnation, God's gift and worldly wealth, no portion in “this faith,” the heart not upright before God, repentance, prayer, possible forgiveness of the heart's deceit, bitter gall and bonds of iniquity. Simon's answer asks the apostles to pray to God for him; it does not add an unexpressed resolution of his story.
- Acts 8:25-31 preserves Simon and John's testimony, teaching, return and preaching in Samaritan villages, the Lord's angel, the southward desert road from Jerusalem to Gaza, and the eunuch from Cush serving Candace, queen of the Cushites, in charge of her treasury. Verse 8:29 says “the Spirit,” without adding “Holy.” The eunuch's question, need for instruction and invitation into the chariot remain explicit.
- Acts 8:32-35 keeps the lamb, slaughter, sheep, shearer, silence, unopened mouth, humiliation, imprisonment and judgment, generation question and life taken from the earth. The humiliation phrase belongs to source 8:33, although the supplied reference places it in 8:32. The Scripture quotation continues across both verses; Philip begins from this Scripture to proclaim “our Lord Jesus.”
- Acts 8:36-40 translates verse 37's actual dataset words, including wholehearted belief and the confession that Jesus Messiah is God's Son. Both men descend to the water, Philip baptizes the eunuch, the Lord's Spirit snatches Philip away, and the eunuch continues rejoicing. Azotus, preaching in all the cities and arrival at Caesarea are retained without adding a route or place identification to the source.
- Reading “in Isaiah” retains the source preposition in 8:28 and 8:30. The life-taking predicate in 8:33 remains passive and participial, “his life is being taken away.” In 8:37, “all the heart” has an ordinary quantifier and noun; the suffix on “all” does not supply a separate personal possessor of the heart. The Son/God and Spirit/Lord anticipatory genitives in 8:37,39 have narrow exceptions without an extra independent “his.” In 8:38 the Aphel verb has the lexical transitive sense “baptized,” attested in [CAL's amd entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=%28md+V), rather than an added third-party commissioning action. The anaphoric eunuch references retain separate owners for the demonstrative and noun.

Source checks for Acts 9:

- The shared-span review checked all 43 verses and 670 source words, reducing 200 shared spans to 33 documented exceptions. All 702 final mappings and the continuous English received independent whole-chapter review against the frozen chapter hash. Ordinary clauses, participants, auxiliaries, negation and reordered modifiers retain separate owners; each remaining shared expression has a specific linguistic reason in the ledger.
- In 9:2 the first `leh` belongs to Saul's requesting, while the later `leh` is the recipient of the priest's active giving; the letters are plural. The `en hu` formula expresses the conditional “if,” rather than an additional actor; [CAL's en entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=%29n+c) documents this construction. In 9:8 the companions are holding Saul's plural hands, and 9:9 retains his lack of sight without inventing a new agent. In 9:19 the relative “who” remains separate from the past presence predicate. In 9:36 Tabitha was practicing almsgiving; the distinct giving verb in 9:39 describes the garments she had given the widows.
- Acts 9:1-6 retains Saul's threats and murderous fury against the disciples of “our Lord,” plural letters requested from the high priest for the synagogues at Damascus, men or women following this way, binding and bringing them to Jerusalem, and the sudden heavenly light. The goads saying occurs in source 9:4, and Jesus identifies himself as “the Nazarene” in 9:5. The instruction to enter the city includes what Saul ought to do, rather than a statement of mere ability. The singular chief-of-priests title in 9:2 is kept distinct from the plural chief priests in 9:14; compare [CAL’s compound entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=rb%40khnyn+N).
- Acts 9:7-12 keeps the companions' hearing only the voice while seeing no one, Saul's open eyes without sight, leading him by his hands, three days without sight, food or drink, and Ananias' vision in Damascus. The Lord's instructions retain the street called Straight, Judas' house and Saul from the city of Tarsus. The temporal clause “while he was praying” begins in source 9:11 and leads into his seeing Ananias in 9:12; the vision includes laying a hand on him so that his eyes open.
- Acts 9:13-16 preserves Ananias' report about harm to the Lord's saints in Jerusalem, authority from the chief priests and binding all who call on the Lord's name. The chosen vessel, carrying the name among the nations, kings and Israel's sons, and suffering on account of the name remain in the Lord's reply, which continues through 9:16.
- Acts 9:17-22 keeps “Saul, my brother,” “our Lord Jesus,” his appearance on Saul's journey, Ananias' sending, opened eyes and filling with the Holy Spirit. The falling material is compared with plural scales; baptism, food, renewed strength, time with the Damascus disciples, proclamation in the Jews' synagogues and Jesus as God's Son remain explicit. The hearers recall both Saul's earlier persecution and his being sent to bind and bring believers to the chief priests.
- Acts 9:23-30 retains the plot, guarding the city gates by day and night, lowering Saul from the wall in a basket at night, the Jerusalem disciples' fear, and Barnabas bringing him to the apostles. In 9:27 Saul “saw the Lord,” rather than substituting the supplied reference's “the Lord appeared to him.” His open proclamation, going in and out, disputes with Jews who knew Greek, the death threat and the brothers' nighttime journey to Caesarea and sending to Tarsus are retained.
- Acts 9:31-35 preserves singular “ecclesia,” all Judea, Galilee and Samaria, peace, being built up, walking in the fear of God and increase in the Holy Spirit's encouragement. Simon visits the saints at Lydda; Aeneas' paralysis and eight years in bed, Jesus Messiah's healing, the bed instruction and immediate rising remain distinct. The residents of Lydda and Saron turn to God; “Lord” is not substituted here.
- Acts 9:36-39 names Tabitha as a female disciple in Joppa and retains her good deeds and almsgiving without adding an absent Dorcas/gazelle explanation. Her illness, death, washing, upper room and the two men sent for Simon remain explicit. The request in 9:38 is that he not hesitate to come; [CAL's mʾn entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=m%29n+V) documents this impersonal feminine verb with a dative and following action. In 9:39 the widows gather around “him,” not the supplied reference's “her,” and show garments Tabitha had given them, without substituting a making verb.
- Acts 9:40-43 retains putting all the people outside, kneeling, praying, turning to the corpse and the command “Tabitha, rise.” Her opening her eyes, seeing Simon and sitting up, his outstretched hand and raising her, and presenting her alive to the saints and widows remain separate actions. Many believe in “our Lord,” and Simon's stay in Joppa at the house of Simon the tanner is retained without adding a duration or address absent from the source.

Source checks for Acts 10:

- The shared-span review checked all 48 verses and 712 source words, reducing 218 shared spans to 26 documented exceptions. All 762 final mappings and the continuous English received independent whole-chapter review against the frozen chapter hash. Ordinary participants, predicates, auxiliaries, negation and reordered modifiers retain separate owners; the remaining exceptions cover narrow resumptions, subject-coreferential datives and genuine compound expressions.
- In 10:14 `has` retains the refusal “Far be it,” separately from the later negated eating clause. The personal `hu` in 10:20 and 10:32 is distinguished from the past auxiliary `hwa`; the acceptance predicate in 10:35 keeps “is,” “acceptable” and “to him” separate. In 10:39 the repeated object pronouns identify the one person hanged, and in 10:44 the pronoun and name identify one speaking Simon. In 10:46 the repeated language noun supplies the distributive “various languages,” while the hearing, speaking and magnifying predicates keep their individual owners.
- Acts 10:1-8 names Cornelius, Caesarea and the Italian cohort, retains his being a centurion, righteousness, fear of God with his whole household, almsgiving and continual prayer. The vision is about the ninth hour of the day; the temporal preposition is not rendered as an additional person “before him.” His prayers and alms, the memorial before God, sending to Joppa for Simon called Cephas and lodging with Simon the tanner by the sea remain in the angel's continuous speech. Two household members and one soldier who feared God and obeyed Cornelius remain distinct.
- Acts 10:9-16 preserves the next day's journey, the roof and sixth-hour prayer, hunger and food preparation, the trance, open heavens, a vessel fastened at four corners and resembling a great linen sheet, and its descent to earth. Four-footed creatures, creeping things and birds, the slaughter/eat command, “my Lord,” Simon's refusal and never having eaten defiled and unclean food remain explicit. The second utterance and the event occurring three times are retained without converting the source's hours to a modern clock.
- Acts 10:17-23 keeps Simon's pondering, the men sent by Cornelius, the house inquiry, the courtyard gate, Simon called Cephas, and the Spirit's explicit three men. Its command to go without doubting continues through 10:20. The messengers describe Cornelius as upright and God-fearing, retain the Jewish people's testimony and the holy angel's instructions, and ask Simon to come so Cornelius may hear a message. The following day's rising and departure and some Joppa brothers accompanying “him” are retained.
- Acts 10:24-29 has singular “he entered” Caesarea, rather than the supplied reference's plural arrival, and retains Cornelius' relatives and close friends, meeting Simon, falling at his feet, Simon's raising him and “I too am a human being.” The statement about association with a foreigner is kept within Simon's speech, including the stranger not being of his lineage, God's showing him not to call anyone unclean or defiled, his coming readily and his question about the summons.
- Acts 10:30-33 preserves four days up to the present, the fasting reference with “since,” ninth-hour prayer in Cornelius' own house, the man in white, heard prayer and remembered alms. It does not add a continuous four-day fast or convert the hours to modern times. Cornelius is the speaker of 10:30; the angel addresses him by name in 10:31. The angel's nested quotation ends in 10:32; Cornelius continues through 10:33, including “you did well to come,” “we are all before you” and wanting to hear everything God commanded.
- Acts 10:34-38 retains God's impartiality, all nations, fearing him and practicing righteousness, the message sent to Israel's sons, and both peace and tranquility. The message begins from Galilee after John's baptism; [CAL's nqp entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=nqp%232+V) attests the beginning use, including a closely parallel beginning-from-baptism clause. Jesus of Nazareth, God's anointing with the Holy Spirit and power, healing and God being with him remain explicit. “The Evil One” in 10:38 follows [CAL's byš entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=by%24+A), which cites this exact verse under that substantival sense. No absent “doing good” clause is supplied from another text.
- Acts 10:39-43 keeps the witnesses, Judea and Jerusalem, hanging on a tree and killing, God's raising on the third day, and being seen openly. The open-eye expression is rendered idiomatically, as documented in [CAL's ˁyn bglˀ entry](https://cal.huc.edu/cal_entry_web.php?lemma=%28yn%40bgl%29+N). God chose the witnesses, who actually ate and drank with Jesus after his resurrection; the meal is not changed into a future purpose of their selection. The command to proclaim and testify, appointment as judge of living and dead, prophetic testimony and forgiveness for everyone who believes in his name remain in Simon's speech through 10:43.
- Acts 10:44-48 preserves the Holy Spirit descending on those hearing, the circumcised brothers' amazement and astonishment, the gift poured out on the Gentiles also, speaking in various languages and magnifying God. Simon's speech introduction belongs to source 10:46, although the supplied reference places it in 10:47. His water/baptism question retains reception of the Holy Spirit “just as we have”; the subsequent command uses the full name “our Lord Jesus Messiah.” The request to stay for days adds no numerical duration.

Source checks for Acts 11:

- The shared-span review checked all 30 verses and 416 source words, reducing 119 shared spans to 28 documented exceptions. All 444 final mappings and the continuous English received independent whole-chapter review against the frozen chapter hash. Ordinary participants, clauses, auxiliaries, negation, numbers and reordered modifiers retain separate owners; each remaining shared expression has a specific linguistic reason in the ledger.
- In 11:5 the vessel's descent is distinguished from its later progressive lowering, and its four corners retain the actual possessor without adding a partitive. In 11:6 the feet's plural suffix refers to the animals; the number and noun remain separately linked. In 11:8 the refusal is “Far be it,” and the negation and “ever” have separate owners. The gift's object resumption in 11:17, the named possessors in 11:21-23 and the distributive possession in 11:29 have narrow shared links; ordinary noun phrases and the whole sending decision are split.
- Acts 11:1-4 keeps the apostles and brothers in Judea hearing that the Gentiles too received God's word, Simon's ascent to Jerusalem, the circumcision group's dispute and the allegation about entering among uncircumcised men and eating with them. The allegation's actual verbs are third person, so it remains indirect “he,” without importing a second-person accusation. Simon begins his account step by step; [CAL's nqp entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=nqp%232+V) explicitly cites 11:4 under the beginning/resuming construction.
- Acts 11:5-10 preserves prayer in Joppa, a descending vessel like a linen sheet, fastening at its four corners, its lowering from heaven and coming right to Simon, four-footed animals, creeping things and birds. The voice addresses Simon, the slaughter/eat command and “my Lord” remain explicit, and the heavenly reply retains what God cleansed and the prohibition on defiling it. Three repetitions and “everything” being taken up are retained. These inner utterances remain inside Simon's outer account, which continues beyond 11:10.
- Acts 11:11-14 names Cornelius and Caesarea, the three men, the courtyard gate and Simon's lodging; passive-agent “by Cornelius” is distinct from origin “from Caesarea.” The Spirit's instruction, the six accompanying brothers and entering the man's house remain explicit. The angel's command names Joppa, Simon and Cephas; the promised words are those by which “you and all your household will live,” retaining the source's active life verb rather than mechanically substituting a different passive formulation.
- Acts 11:15-18 keeps Simon beginning to speak “there,” the Holy Spirit descending on them as on the original witnesses, remembering “our Lord,” John's active baptizing with water and the promised baptism with the Holy Spirit. The equal gift is expressly given to the Gentiles who believed in “our Lord Jesus Messiah,” as to the witnesses; the believing clause is not reassigned to a different group. Their response retains quieting, glorifying God and repentance leading to life. The ambiguous kbar is read contextually as “already,” with the past giving verb; [CAL records both already and perhaps in Syriac](https://cal.huc.edu/cal_entry_web.php?lemma=kbr+X). No exclusive lexical claim is made for the alternative.
- Acts 11:19-21 retains scattering because of the oppression concerning Stephen, Phoenicia, the region of Cyprus and Antioch, the initial restriction to Jews alone and the subsequent people from Cyprus and Cyrene speaking with Greeks and preaching about “our Lord Jesus.” No unexpressed ethnic qualification is added to “Greeks.” The Lord's hand, belief and turning to the Lord remain distinct actions.
- Acts 11:22-24 keeps the news reaching the Jerusalem ecclesia and sending Barnabas to Antioch, his arrival, seeing God's grace, rejoicing and urging wholehearted devotion to “our Lord.” The sons-of-the-ecclesia community idiom is rendered naturally as its members. Barnabas remains a good man filled with the Holy Spirit and faith, and the many people are added to “our Lord”; neither first-person plural Lord possessive is dropped.
- Acts 11:25-26 retains the journey to Tarsus to seek Saul, finding him and bringing him along to Antioch, a whole year assembled together in the ecclesia, teaching many people and the source's “from then on” and “first” naming of the disciples as Christians at Antioch. No additional duration, journey membership or reason for the name is supplied.
- Acts 11:27-30 preserves prophets arriving from Jerusalem, Agabus standing, making the famine known by the Spirit, the great famine throughout the land and its occurrence in the days of Claudius Caesar. No modern date or wider historical extent is inferred. The disciples' contribution follows what each had, with aid sent for the brothers in Judea and delivery through Barnabas and Saul to the elders there.

Source checks for Acts 12:

- The shared-span review checked all 25 verses and 412 source words, reducing 137 shared spans to 32 documented exceptions. All 418 final mappings and the continuous English received independent whole-chapter review against the frozen chapter hash. Ordinary participants, predicates, auxiliaries, negation, numbers and reordered modifiers retain separate owners; the remaining exceptions cover narrow resumptions, subject-coreferential datives, lexical prison compounds and other genuine compound expressions.
- In 12:6 the handover verb is active: he was ready to hand Simon over in the morning. Simon is the separate subject of the sleeping clause. In 12:9 each clause retains its own auxiliary and gender agreement, and in 12:22 the two copulas remain in their respective clauses; the nominal negative clause is present, “are not of human beings.” In 12:15 the cognate infinitive supplies the intensifier in “quite mad,” while the subject-coreferential dative stays with the mental-state predicate; [CAL cites this exact construction](https://cal.huc.edu/oneentry.php?cits=all&lemma=zw%28+V). In 12:10 “of its own accord” retains separate owners for its preposition, reflexive possessor and noun; [CAL records the will/desire sense of that noun](https://cal.huc.edu/oneentry.php?cits=all&lemma=cbw+N).
- Acts 12:1-4 keeps Herod the king's explicit name Agrippa, seizure of people of the ecclesia, intended mistreatment, killing James the brother of John with the sword, and proceeding to seize Simon Cephas when this pleased the Jews. The days of Unleavened Bread, prison, sixteen soldiers and guarding remain explicit. “After Passover” governs the planned handing over to the Jewish people, rather than postponing the guarding.
- Acts 12:5-9 preserves constant prayer to God by the ecclesia on Simon's behalf, the night before the planned morning handover, two soldiers, two chains and other guards at plural prison doors. The angel stands over Simon, light fills the building, and the angel strikes his side, wakes him and speaks. The commands to gird his loins, put on his sandals, wrap himself in his cloak and follow remain distinct, along with the chains falling from plural hands and his uncertainty about reality versus a vision. The causal “for” in 12:9 is retained.
- Acts 12:10-11 retains the first and second watches, iron gate opening of its own accord, passage along one street and the angel's departure. Simon's realization and statement name “the Lord” as sender of his angel, rather than the supplied reference's “God,” and retain release from Herod the king and the Jews' plotting. Agrippa is not added again where the actual verse says only Herod.
- Acts 12:12-14 names Mary as the mother of John called Mark, the many brothers gathered and praying, the courtyard gate and the maid Rhoda coming out to answer. Her recognition of Simon's voice, joy, failure to open for him, running back and announcement that he is standing at the courtyard gate remain explicit. Her actions are not reassigned to Simon.
- Acts 12:15-17 preserves the group's disbelief, Rhoda's insistence, and their speculation “perhaps it is his angel,” without replacing the actual angel noun with the supplied reference's “ghost.” Simon continues knocking; the group comes out and sees him. His hand signal, silence, entering and reporting the Lord's release, telling James and “our brothers,” and departing to another place remain distinct. No destination for his departure is supplied.
- Acts 12:18-19 retains morning, the soldiers' great uproar about what happened to Simon, Herod's search, failure to find him, judgment of the guards and order that they die. The departure is from Judea and the stay is in Caesarea; an ethical dative is not expanded into an additional “from him” destination or relationship.
- Acts 12:20-23 keeps the causal anger toward the Tyrians and Sidonians, their assembled approach, persuading Blastus the king's chamberlain, the peace request and dependence on supplies from Herod's kingdom. The certain day, royal clothing, tribunal and address to the assembly remain explicit. The crowd's plural utterances and contrast with human beings are preserved; [CAL's compound entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=brt%40ql+N) records the voice/utterance expression as a lexical unit. The striking angel is “of the Lord,” not the supplied reference's “of God”; lack of glory to God, the same hour, worm infestation and death remain explicit.
- Acts 12:24-25 calls the proclaimed and growing message God's “good news,” retaining its actual noun rather than substituting “word.” Barnabas and Saul return explicitly from Jerusalem to Antioch after completing their service, and take John called Mark with them. No different travel direction, additional relative or historical date is supplied.

Source checks for Acts 13:

- The shared-span review checked all 52 verses and 744 source words, reducing 231 original shared spans to 40 documented exceptions. Every final mapping, exception reason and the continuous English received three independent whole-chapter reviews against the frozen chapter hash. Ordinary names, numerals, noun phrases, negatives, auxiliaries, prepositions and clauses retain individual source owners.
- Acts 13:8 keeps the sorcerer as the active subject standing against them; 13:42 likewise keeps the active request rather than changing the source roles into a passive. In 13:14 the stored Sabbath word begins with a conjunction, represented by explanatory apposition after the day. In 13:26 the masculine focus copula becomes “it is to you,” without making it a feminine word's demonstrative. The construct heads own the genitive relations in “son of” (13:1) and “forgiveness of” (13:38); resumed possessors and objects, ethical datives, compound prepositions and idioms have specific minimal exceptions in the ledger.
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

- The shared-span review checked all 28 verses and 420 source words, reducing 112 original shared spans to 27 documented exceptions. Every final mapping, exception reason and the continuous English received three independent whole-chapter reviews against the frozen chapter hash. Ordinary participants, noun phrases, numbers, negatives, auxiliaries and clauses retain individual source owners.
- Acts 14:8 separates “not” and “ever” and keeps the description of never having walked attached to the man, with “womb” owning its construct genitive relation. In 14:12 naming is a perfect with narrative past auxiliary, while beginning to speak has its own participle and separate past auxiliary. The teaching form in 14:21 is Taphel. In 14:18 men mahsen is the minimal “with difficulty” compound; [CAL's entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=lmxsn+X) explicitly records that expression. “No” and “one” remain separate, as do ordinary temporal modifiers and the pronouns and auxiliaries around the active entrusting in 14:23 and passive earlier entrusting in 14:26.
- Acts 14:1-5 keeps entry into the Jewish synagogue, many Jews and Greeks believing, the unpersuaded Jews' incitement, mistreatment of the brothers, and the divided city crowd. The Lord testifies to the word of his grace through signs and wonders performed through their hands. The eye/revelation expression is rendered idiomatically as “openly,” consistently with earlier occurrences. The Gentiles, Jews and their chiefs remain coordinated in the assault and intended insult and stoning.
- Acts 14:6-9 retains both departure and taking refuge, the Lycaonian cities Lystra and Derbe and surrounding villages, and continued preaching. The man is sitting in Lystra, afflicted in his feet, lame from his mother's womb and never having walked. His faith is rendered in its healing context as “faith to be healed”; [CAL's ḥyy entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=xyy+V) attests healing within the G-stem's live/revive senses. The clause continues into Paul's command in 14:10.
- Acts 14:10-13 retains the loud voice, “I say to you,” the full name “our Lord Jesus Messiah,” rising on his feet and all three ensuing actions: leaping, standing and walking. The crowd speaks in the region's language; no specific language name is inserted. Barnabas is called “the lord of the gods,” without replacing this actual title with Zeus; Paul is Hermes because he began speaking. The priest, location outside the city, oxen, wreaths, courtyard gate, lodging and intended sacrifice remain explicit.
- Acts 14:14-17 preserves Barnabas and Paul's hearing, torn garments, leaping and going out toward the crowd, and their cry. Their speech continues through 14:17 and retains their shared human susceptibility to suffering, the command to turn from useless things to the living God, creation of heaven, earth and plural seas, and everything in them. Former generations and all nations following their own ways remain distinct from God's testimony through good gifts from heaven, rain, fruits growing in their seasons, and hearts filled with food and gladness. The beneficiaries in 14:17 are third-person plural, not an imported “you” or “us.”
- Acts 14:18-20 retains the difficulty of restraining the people so that no one would sacrifice to them. Jews arrive from Iconium and Antioch, stir up the people against them, stone Paul and drag him outside the city because they think he has died. The disciples gather around him; he rises, enters the city and departs the next day with Barnabas for Derbe. No assertion about his actual death is added.
- Acts 14:21-23 preserves preaching to the city's inhabitants and teaching many, the return to Lystra, Iconium and Antioch, strengthening the disciples' souls, remaining in the faith and necessary entry into God's kingdom through much tribulation. Appointment of elders is in every ecclesia, with fasting together and prayer. Entrusting is to “our Lord,” in whom they believed. Ethical datives are absorbed into natural English without inventing an additional recipient on the return journey.
- Acts 14:24-26 retains the region of Pisidia, Pamphylia, proclamation of the Lord's word in the city of Perga, descent to Attalia and travel by sea to Antioch. The reason for the return is their prior entrusting from there to the Lord's grace for the work now completed; it is not changed into a new commission.
- Acts 14:27-28 keeps gathering the whole ecclesia and recounting everything God had done with them. God remains the singular subject who opened a door of faith to the Gentiles. Their long stay is with the disciples; no numerical duration is added.
- In 14:19 “that” and “he had” belong separately to the finite death verb's relative prefix and perfect third-person morphology; “died” retains the verb and its ethical dative. The dative does not supply an English auxiliary or add a beneficiary.
- In 14:1, 14:12-14, 14:21 and 14:24-26 the conjunctions and finite-person material have separate links outside the shared verbal heads. The second English “and” in 14:21 expresses paratactic coordination with the arrival head; that source verb is bare ethau, with no waw prefix.
- Genitive prefixes in 14:10-11, 14:22 and 14:26, the comparative prefix in 14:11, and “in” in 14:23 have separate links from the shared referent or compound. In 14:10 “of our” belongs to d-maran; the third-person suffix on “name” remains linked with the named Lord rather than being assigned the first-person “our.”

Source checks for Acts 15:

- The shared-span review checked all 41 verses and 565 source words, reducing 153 original shared spans to 31 documented exceptions. Every final mapping, exception reason and the continuous English received three independent whole-chapter reviews against the frozen chapter hash. Ordinary participants, negatives, auxiliaries, prepositions and clauses retain individual source owners.
- In 15:21, “has had” retains the complete ith-hwa-leh possession predicate. The English present perfect reflects possession continuing from former generations alongside habitual Sabbath reading; “has” is not assigned to ith alone merely because its basic gloss is “is.” This reads the construction and context together, consistently with the distinction between tense and aspect discussed in [Kiraz, section 4.7, page 70](https://ceulearning.ceu.edu/pluginfile.php/623783/course/overviewfiles/Kiraz_Syriac%20Primer.pdf?forcedownload=1#page=94).
- Genitive prefixes in 15:10-11, 15:15-18, 15:26 and 15:34-35 have separate links from their resumed noun heads; the preposition in 15:17 is likewise separate from “whom.” In 15:11 and 26, “our” belongs to d-maran, while the different-person suffix on “grace” or “name” remains linked with “Lord.” In 15:22, “among” retains the repeated b-prepositions and “the brothers” retains the pronominal and explicit referent as separate shared heads. In 15:24, “have” retains the experiential predicate and “heard” links separately to the lexical hearing stem. In 15:39, the conjunction has its own link outside the motion verb and ethical dative. These refinements change links in 12 verses while preserving all 41 verses’ English wording.
- Acts 15:1 and 15:24 retain the contextual intransitive circumcision sense; [CAL's gzr entry](https://cal.huc.edu/cal_entry_web.php?lemma=gzr+V) records “to be circumcised” under Peal, alongside the active sense. The actual plural possessive suffix in 15:13 remains “our,” while 15:7 has no possessive. In 15:29 the welfare formula retains the verb be, and 15:34 retains the noun will with Silas as its resumed possessor. The circumcision conditions, active life verb, third-person troubling predicate, hearing state, past possession, relative resumptions and journey datives were checked separately against the source forms and morphology.
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

- The shared-span review separates ordinary noun phrases, names, negation and participial auxiliaries. It retains only documented minimal joins for narrative perfect plus hwā, subject-coreferential ethical datives, resumed referents, existential possession, fixed expressions and institutional compounds. In 16:24, “inner room” keeps the room noun separate from its adjective and the prison compound; in 16:35 and 16:38, “rod” and “bearers” each retain their own source word.
- Acts 16:16 assigns the English complementizer “that” and temporal “while” to the separate components of d-kad, rather than assigning “that” to the young-woman noun. In 16:33, “washed their wounds” renders the affected-person object and ablative wound expression as natural English body care; the minimal shared possessive records that change of construction without adding blood or claiming healing. The two repeated locative expressions for the hour and night remain distinct.
- Acts 16:19 and 16:27 link “had” separately to the finite perfect departure and flight verbs. Their anterior reading follows the clauses' context; the ethical datives remain with “gone” and “fled” and do not supply a past auxiliary. The hwā in 16:27 belongs to “he was thinking,” and the prisoners' supposed flight remains the jailer's assumption. This contextual reading follows the perfect's completed aspect described in [Kiraz, section 4.7](https://ceulearning.ceu.edu/pluginfile.php/623783/course/overviewfiles/Kiraz_Syriac%20Primer.pdf?forcedownload=1#page=94).
- The ownership refinement keeps genitive and directional prefixes outside shared noun heads, conjunctions outside shared predicates, and the plural subject outside “went down.” Resumed possessors remain linked to both their suffix and named referent; “our” in 16:20 belongs to the actual first-person plural suffix on “city.” In 16:12, 16:18 and 16:33, repeated locative prefixes and shared referents have separate spans, with independently identifiable pointers outside the hour and night heads. The “of” connecting hour and night is an English temporal relation, without claiming an Aramaic genitive prefix. The invitation's “and” in 16:15 coordinates a bare imperative with the preceding “come,” without claiming an unexpressed waw. All 40 verses retain their English wording.

Source checks for Acts 17:

- Acts 17:1-3 retains the cities Amphipolis and Apollonia, arrival at Thessalonica and the Jewish synagogue. Paul enters according to his custom and speaks from the Scriptures for three Sabbaths. His explaining and showing concern the Messiah's suffering and rising from among the dead; the min-beth expression does not require a literal house-building in English. The following claim is Paul's first-person “this Jesus is the Messiah, whom I proclaim to you,” not an unmarked continuation about what someone else proclaimed.
- Acts 17:4-11 preserves believing and joining Paul and Silas, many God-fearing Greeks and not a few notable women. Jealousy, evil men from the city's marketplaces, the crowd, disturbance, attack on Jason's house, bringing the men out and handing them to the crowd remain distinct. The accusation continues through 17:7, including Jason as host, Caesar's decrees and another king, Jesus. The additive “here also” does not invent an earlier visit. Security is taken from Jason and the brothers before release. Sending Paul and Silas to Berea is immediate and on the same night. The Jews there are contrasted with Thessalonica's Jews; they gladly hear the word from the speakers daily and discern from the Scriptures whether these things are so.
- Acts 17:12-17 retains many believers, Greek men and notable women, Thessalonica Jews hearing of God's word proclaimed by Paul in the city of Berea, their arrival and unceasing agitation. The brothers send Paul toward the sea while Silas and Timothy remain. His escorts reach Athens and receive an actual letter from him to Silas and Timothy for their urgent coming; the letter is not replaced by oral instructions. Paul's remaining in Athens has no added “for them” object. His spirit is grieved at the whole city's idols. Synagogue speech is with Jews and God-fearers; the marketplace audience is those present every day.
- Acts 17:18-22 names philosophers from Epicurus's teaching and others called Stoics, their dispute with Paul, and the question about what the collector of words wants. The foreign-gods remark ends before the narrator's reason: Paul proclaimed Jesus and his resurrection to them. The court is called Areopagus, and the connected question and statement retain the new teaching, strange words sown into “our hearing” and the wish to know their meaning. Athenians and visiting foreigners care for saying and hearing something new. Paul's address retains the explicit reverence/fear of demons rather than replacing it with a generic statement of religiosity.
- Acts 17:23-25 keeps Paul's walking, viewing the place of worship and finding a certain altar with the nested inscription “To the unknown God.” The stored בית is singular; “your place of worship” does not import plural houses from the supplied reference. The knowing/worshipping clauses and emphatic identification of the one proclaimed remain correctly related. God made the world and everything in it, is Lord of heaven and earth and does not dwell in hand-made temples. He is not served by human hands and needs nothing; his gifts here are life and breath to everyone, without an added “everything.”
- Acts 17:26-29 retains one blood, all humankind, dwelling on the whole earth, seasons appointed by God's command and bounds of human habitation. Seeking and investigating lead to finding God through his plural “creations”; no absent groping clause is supplied. He is not far from each of us. Living, moving and existing in him are followed by plural wise men among the hearers who “have said,” not one poet or sage. Their nested quotation says “Our descent is from him.” The next comparison has the Godhead as its grammatical subject and gold, silver and carved stone as its comparison objects. English word order preserves that relation while the subject, comparison verb, materials and modifiers each retain their own source links.
- Acts 17:30-31 retains causal “For” and times of “error.” The causative אעבר is read with its temporal object as God making those times pass away, consistent with the make-pass sense recorded in [CAL's ˁbr entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=%28br+V), rather than importing a different ignorance/overlooking formula. The present command names all people, each person and every place. The appointed day, judgment of the whole earth in righteousness and the appointed man remain explicit. The source says God turned everyone to faith in him through raising him from among the dead; the actual turning and faith words are not replaced by an assurance formula.
- Acts 17:32-34 preserves hearing of resurrection from among the dead, some mocking and others saying “At another time we will hear you about this.” The time phrase stays with its own words, without an added “again.” Paul departs from among them. Some join him and believe; Dionysius is from among the judges of the Areopagus, and the woman Damaris and other adherents remain in the concluding account. Paul's speech closes at 17:31 before this narration.

- The shared-span review separates ordinary names, noun phrases, negation, participial auxiliaries and complete clauses. In 17:3, 17:31 and 17:32, “from” and “among” have separate source owners: [CAL records bēṯ as an independent preposition meaning “among”](https://cal.huc.edu/cal_entry_web.php?lemma=byt+p), alongside the homophonous house noun. Remaining joins are minimal documented narrative predicates, ethical datives, resumed referents, a time idiom, a court compound and experiencer/copular constructions.
- Acts 17:23 distinguishes the object of revering from the repeated object introducing “this one I am proclaiming”; its relative pronouns, temporal connective and knowing clause keep their own links. In 17:34, the English restores the coordinated “adhered to him and believed” predicate for the woman and others with repeated links to the same source verbs. Dionysius remains the singular “one of them,” without making that one person refer to several adherents.

Source checks for Acts 18:

- Acts 18:1-4 retains Paul's departure from Athens and arrival at Corinth, Aquila as a Jewish man from the region of Pontus, his arrival from Italy at that time with his wife Priscilla, and Claudius Caesar's command for all Jews to leave Rome. Paul's shared trade, lodging and work with them and their tentmaking remain distinct; the craft noun is not lost in a generic reference to trade. Sabbath synagogue speech and persuasion concern both Jews and Gentiles.
- Acts 18:5-8 keeps Silas and Timothy arriving from Macedonia, Paul's constraint in speech and the explicit cause: the Jews stood against him and blasphemed while he testified that Jesus is the Messiah. Shaking his garments is followed by “From now on I am clean; I am going to the Gentiles.” The subsequent householder is Titus, whose house adjoins the synagogue. Crispus is the synagogue leader and believes in “our Lord,” he and all his household. Many Corinthians hear, believe in God and are baptized.
- Acts 18:9-11 keeps the Lord's vision and speech to Paul, the prohibition on fear, the command to speak and not be silent, the Lord being with him, inability to harm him and the many people in this city. The quotation continues through 18:10. Paul's stay is explicitly one year and six months in Corinth, with teaching them God's word. No night-time detail is added to the vision.
- Acts 18:12-17 names Gallio as proconsul of Achaia, the Jews' assembly against Paul and his appearance at the tribunal. The accusation concerns persuading people to fear God contrary to the law. Paul's attempt to open his mouth and speak precedes Gallio's reply. His counterfactual names wrong, deceit and a hateful act, while the following questions concern speech, names and the hearers' own law. The internal knowledge clause and refusal to judge those matters remain. He drives them from his tribunal. The assailants are explicitly all the Gentiles, and Sosthenes is an elder of the synagogue; Gallio disregards the beating before the tribunal.
- Acts 18:18 preserves Paul's many days there, farewell to the brothers, sea journey toward Syria, and Priscilla followed by Aquila accompanying him. The shaving at Cenchrea and the vow use an unnamed masculine subject, rendered “he”; the source does not explicitly name the man in these clauses, so the English does not force a choice between possible antecedents. The passive participle with a dative in the vow clause is rendered naturally as his having made a vow, without inventing a recipient to whom a vow was owed.
- Acts 18:19-23 keeps arrival at Ephesus, Paul's synagogue speech, the request to stay and his refusal to be persuaded. His stated obligation concerns the approaching feast in Jerusalem, followed by a God-willing return. Leaving Aquila and Priscilla in Ephesus belongs to the actual final clause of 18:21, although the supplied reference places it at the start of 18:22. The following sea journey reaches Caesarea, goes up and greets the members of the ecclesia, then goes to Antioch; no unexpressed city is named for that ascent. The next route names Galatia before Phrygia, preserving the stored order, with strengthening of all disciples.
- Acts 18:24-26 describes Apollos as a Jew with origins in Alexandria, trained in speech and well versed in the Scriptures. The way of the Lord is singular. His fervent spirit, speaking and teaching fully about Jesus, and knowledge limited to John's baptism remain explicit. The except construction is not turned into a new hypothetical condition. The eye/revelation expression is rendered idiomatically as “openly.” Aquila and then Priscilla hear him, bring him to their house and fully explain the Lord's way to him.
- Acts 18:27-28 keeps the intended journey to Achaia, the brothers encouraging him and writing to the disciples to receive him. Grace modifies the helping: through grace he greatly assists all the believers. The final causal “For” introduces his powerful argument against the Jews before plural assemblies and his demonstration from the Scriptures concerning Jesus that he is the Messiah.

- The shared-span review separates ordinary names, noun phrases, numerals, negation, conditional clauses and participial auxiliaries. Retained joins are minimal narrative or copular predicates, ethical datives, repeated referents, possession and fixed expressions. In 18:8, Crispus and the resumed “he” identify one believer, while the house members and their quantifier remain separately represented. The counterfactual in 18:14 and the knowledge clause in 18:15 are not merged into whole-clause spans.
- Acts 18:2 links “had” to the past auxiliary and “arrived” or “commanded” to the lexical verb separately; 18:5 does the same for “had come.” These splits preserve the English wording and its pluperfect aspect, consistently with the separate auxiliary in 20:13. They do not change one-word narrative past predicates into pluperfects merely to split their links.
- Acts 18:3 gives “member of” its own source word, [bar in its cohort or acquired-skill sense](https://cal.huc.edu/cal_entry_web.php?lemma=br+N), while retaining “their trade.” In 18:18, the vow construction gives “he,” “had,” “made” and “a vow” separate owners, without naming the unnamed man or adding a recipient. The final clause of 18:21 keeps Aquila, Priscilla, the leaving verb and its resumed object separate through the English topic construction “as for.”

Source checks for Acts 19:

- The shared-span review separates ordinary modifiers, auxiliaries, speech and movement predicates, and named actors throughout all 41 verses. In 19:8 `על` is the entering verb despite its preposition homograph tag: Paul is the subject and the synagogue the destination. In 19:32 the masculine “different people” and feminine “different things” retain separate ownership; in 19:40 the first-person inability and the account idiom remain distinct.
- Resumed possessors retain minimal shared links on the named referent, including “God” after the actual faith noun in 19:20; “of” belongs separately to the following d-prefix. In 19:5, 13 and 17, “our” belongs to d-maran, rather than the different-person suffix on “name.” The number in 19:19 is five myriads: [CAL's numeral rbw](https://cal.huc.edu/cal_entry_web.php?lemma=rbw+b) confirms the ten-thousand unit, so the conversion to fifty thousand is shared without falsely assigning “fifty” to five alone. In 19:26 nominal `לא הוא` supplies present “not,” with “only” and the active persuasion and turning-away predicates separate.
- Acts 19:36 links “no” and “one” separately; 19:27 likewise uses adjacent “no” and “thing” spans to preserve “nothing.” In 19:14, “man” retains the suffix resumed from “sons,” while “of” belongs to d-gabra. In 19:17, “became” retains the knowledge-event verb and narrative auxiliary, with “known” on the lexical stem; 19:22 separates the auxiliary’s subject “he” from the composite past “sent.” In 19:27, “is” belongs to the participial predication and “reckoned” retains the ethical dative. The conjunction in 19:39 and complementizers in 19:33 and 40 have separate links outside their conditional or speech idioms. These refinements preserve all 41 verses’ English wording.
- Acts 19:1-7 keeps Apollos at Corinth while Paul travels through the upper regions to Ephesus. The inquiry at the end of 19:1 introduces the question in 19:2. The disciples have not even heard whether there is a Holy Spirit; John's baptism and his instruction to believe in the one coming after him remain. The baptismal name is “our Lord Jesus Messiah.” Paul lays his singular hand on them, the Holy Spirit comes upon them, and their speaking in various tongues and prophesying remain separate. The total is exactly twelve persons, without an added approximation.
- Acts 19:8-12 preserves three months of open synagogue speech, persuasion about God's kingdom, hardening and opposition, reviling God's way before the assembly, Paul's withdrawal and separation of the disciples, and daily speech in Tyrannus's school. No hours are added. The subsequent duration is two years, and all who dwell in Asia, Jews and Gentiles, hear the Lord's word. God's great works are through Paul's singular hand. Cloths or patches from his clothing are brought and laid on the sick; diseases leave them and demons go out. The alternative “or” is retained.
- Acts 19:13-17 distinguishes the Jews' existing itinerant adjuration of demons from their desire to pronounce an adjuration over people with unclean spirits. Both adjuration predicates and both occurrences of “in the name” remain; the narrative has “our Lord Jesus,” and the quotation names Jesus whom Paul proclaims. Sceva is a Jewish chief of the priests with seven sons. The demon's recognition of Jesus and knowledge of Paul precede its question to them. The man with the evil spirit overpowers and throws them down; naked and wounded describes those fleeing from that house. The episode becomes known to all Jews and Gentiles in Ephesus, fear falls on all, and the full name “our Lord Jesus Messiah” is exalted.
- Acts 19:18-23 retains believers recounting their sins and confessing their deeds, sorcerers gathering and bringing their books for public burning, and the computed value of five myriads, rendered fifty thousand in silver without naming a coin. The source says the “faith of God” grows strong and increases, rather than replacing it with a word/Lord formula. Paul resolves in his mind to travel through all Macedonia and Achaia to Jerusalem and then see Rome. Two assistants, Timothy and Erastus, are sent to Macedonia while he remains in Asia. The disturbance concerns the way of God.
- Acts 19:24-27 names Demetrius, his silver shrines for Artemis and the craftsmen's great profit. He gathers all the craftsmen of his trade and those working with them; “all” is not transferred from craftsmen to the second group. His speech says all their business comes from this work. The hear/see, persuasion, turning away and saying predicates remain distinct. Gods made by human hands are denied. The business's disgrace and cessation, the temple being reckoned as nothing and the goddess of all Asia being despised remain explicit. The public-disgrace sense of מתפרסיא follows [CAL's prsy, Qt sense 2](https://cal.huc.edu/oneentry.php?cits=all&lemma=prsy+V), which cites this verse directly. The name remains Artemis throughout the chapter.
- Acts 19:28-34 retains the city's uproar, the crowd's running together and entering the theatre, and Gaius and Aristarchus as Macedonian members of Paul's company. בני לויתה דפולוס is read as a company-membership construction, without adding a first-person owner from an ambiguous morphological tag. The Classical Syriac construct plural bnay is illustrated in chapter 3, page 52 of [Attributive constructions in North-Eastern Neo-Aramaic](https://langsci-press.org/catalog/view/123/302/1034-1); the construction is applied here in context. The disciples restrain Paul; the chiefs of Asia, his friends, send and plead with him. The confused crowds shout different things because many do not know why they assembled. The Jewish people bring Alexander forward, and he wants to make a defense. The repeated cry lasts about two hours, preserving the approximation and the single voice.
- Acts 19:35-37 calls the official the city's chief. Ephesus is the “priestess” of great Artemis and her image, not a temple substituted for כומרתא. Descending from heaven modifies the image; no additional divine name is introduced. The instruction to remain quiet and do nothing rash is followed by the explicit grounds that the brought men have neither plundered temples nor reviled “our goddess.” The chief's speech continues through 19:40.
- Acts 19:38-41 preserves the singular proconsul in the city and the separate plural statement that they are craftsmen, followed by approaching and litigating against one another. Other matters are settled in the place appointed by law for assembly, keeping the passive appointment. The closing danger and inability to account for this day's assembly use first-person plural forms, with both purposeless gathering and disturbance without cause. נפוק רוחא is idiomatic in the defense at 19:33 and giving an account at 19:40, consistent with [CAL's npq, G sense 5 with rūḥā](https://cal.huc.edu/oneentry.php?cits=all&lemma=npq+V); no additional spirit actor is created. After the speech, the chief dismisses the assembly.

Source checks for Acts 20:

- The shared-span review separates ordinary travel, speech, time and place expressions throughout all 38 verses. In 20:7 the n-prefixed departure form is resolved as third-person singular by Paul, singular “prepared/was” and the singular dative; in 20:9 `אגר` is the narrative verb with `הוא` and Paul as subject, rather than the imperative homograph selected by the morphological tag. In 20:10 the compound spatial preposition is shared minimally, with “fell” and “him” separately owned.
- In 20:8 the past existential “were” and explicit location “there” remain distinct without repeating English “there.” In 20:13 “he had” and “instructed” have separate owners; in 20:16 the repeated dative names one Paul, with “had” and “decided” also split. In 20:18 the formerly shared how/time/from-first-day clause is split throughout. In 20:31 the perfect form of “be” has imperative force, a Syriac use discussed by [Cox, page 107, note 48](https://www.andrews.edu/library/car/cardigital/Periodicals/AUSS/1977-2/1977-2-01.pdf); “be” and “awake” retain separate links.
- Acts 20:1-6 retains the subsiding riot, Paul's calling, encouragement and kissing of the disciples, departure to Macedonia, regional travel and encouragement, Greece, three months there, the Jews' plot, intended travel to Syria and contemplated return to Macedonia. The companions include Sopater from Berea, Aristarchus and Secundus from Thessalonica, Gaius from Derbe, Timothy explicitly from Lystra, and Tychicus and Trophimus from Asia. They go ahead and await the speakers at Troas. The departure from Philippi follows the days of unleavened bread; the sea journey takes five days and the stay at Troas seven. The “Philip” lexical tag at 20:6 is not treated as a different person or an instruction to rename the city: its city context and the same vocalised name in [Dukhrana's analysis of Acts 16:12](https://www.dukhrana.com/peshitta/analyze_verse.php?font=Estrangelo+Edessa&size=125%25&source=&verse=Acts+16%3A12) support the conventional English Philippi.
- Acts 20:7-12 keeps the first day of the week and the explicit אוכרסטיא, “Eucharist.” The gathering is first-person plural, while Paul's speaking is “with them,” preserving the third-person plural suffix. His intended next-day departure and speech until midnight remain. Many burning lamps are in the upper room. Eutychus is sitting in the window and listening before sinking into deep sleep during Paul's prolonged speech. His fall from the third floor and being picked up dead are retained without an added apparent-death qualification. Paul goes down, falls upon and embraces him, and says his soul is in him. Going up, breaking bread, tasting it, speaking until daybreak and departure by land belong to the subsequent narrative. The young man is taken away alive and they rejoice greatly over him.
- Acts 20:13-17 preserves first-person travel to the ship, the appointed meeting place at Thasos, the plan to pick up Paul from there and his instructions while he travels overland. The source's Thasos name is retained without replacing it with a different route label. Receiving him from there and taking him aboard remain separate actions. The route includes Chios, Samos, the explicit stay at Trogyllium and Miletus, with the successive day expressions. Paul's decision to pass Ephesus retains causal “For,” the concern about delay, his haste and the conditional possibility of celebrating Pentecost in Jerusalem. The decision's passive participle plus dative is expressed naturally as his having decided. From Miletus he sends for and brings the Ephesian ecclesia's elders.
- Acts 20:18-21 opens the speech to the elders and preserves their knowledge of how Paul was with them throughout the time since his first day in Asia. That temporal/how clause is reordered within one linked grammatical unit. Service to God, great humility, tears and trials through Jewish plots remain distinct. The source speaks of what benefited “your souls” and teaching in streets and houses. Testimony concerns Jews and Gentiles, repentance toward God and faith in the full “our Lord Jesus Messiah.” The Gentile sense of ארמיא is retained, rather than restricting the audience to an inferred ethnic nationality.
- Acts 20:22-25 keeps being bound “in spirit” distinct from the explicitly Holy Spirit in the next verse, without resolving the first phrase's referent by an added name. Paul goes to Jerusalem without knowing what will happen to him there. The Spirit's city-by-city testimony includes a nested quotation addressing Paul as “you,” not an unmarked switch to “me.” His life counts for nothing to him in relation to finishing his course and the ministry received from “our Lord Jesus,” with testimony to the gospel of God's grace. The repeated first-person pronoun in 20:25 belongs to “I know”; the following seeing clause is second-person plural, “you will not see my face again.” The final source noun is only “the kingdom,” without the supplied reference's added “of God.”
- Acts 20:26-30 retains the causal “For this reason,” testimony today and purity from the blood of all the hearers. The source's אשתאלת at 20:27 is read in its refuse/refrain sense, supported by [CAL's šʾl, Gt sense 1](https://cal.huc.edu/oneentry.php?cits=all&lemma=%24%29l+V), rather than the exported base gloss “ask.” Paul has made known God's whole will. The warning concerns themselves and the entire flock, the Holy Spirit's appointment of overseers and tending God's ecclesia, acquired with his blood. The blood clause is not emended to a different possessor or divine title. Fierce wolves enter after Paul's departure and do not spare the flock; men from among the hearers speak perverse things to turn the disciples into following them. The disciples noun stays with its own turning predicate in the links.
- Acts 20:31-35 preserves the call to stay awake and remember three years of unceasing admonition, night and day, with tears for each hearer. Paul entrusts them to God and the word of his grace; the feminine relative and building/giving predicates are retained with the word clause. The inheritance is among all the holy ones. Silver, gold and clothing are not coveted. “These hands of mine” retains the explicit first-person hand suffix and service for Paul's needs and those with him. The instruction to labor and care for the weak includes remembering the words of “our Lord Jesus.” In the nested quotation the comparative word belongs with “More blessed is the one who gives,” followed by “than the one who receives.”
- Acts 20:36-38 resumes narration after the speech and nested quotation close in 20:35. Kneeling on his knees and prayer with all those present are followed by everyone's great weeping, embracing and continued kissing. Their particular distress concerns his statement that they would not see his face again. They accompany him as far as the ship. No neck-embrace or other unexpressed detail is imported into these actions.

Source checks for Acts 21:

- The shared-span review checked all 40 verses and 651 source words, reducing 193 original shared spans to 64 documented exceptions. Every final mapping, exact exception reason and the continuous English received three independent whole-chapter reviews against the frozen files. Ordinary participants, modifiers, negatives, genitives and reordered clauses retain separate owners. Only two English words change: “fulfill” to “follow” in 21:24 and “Were” to “Are” in 21:38.
- Past possession in 21:9 retains the existential, feminine plural auxiliary agreeing with the daughters, and masculine singular possessor in “had”; “he” belongs to the possessor suffix. In 21:13 the positive copular pronoun supplies “am,” while the separate impersonal nominal negation supplies “not” in the not-only construction. In 21:18 the singular “with him” is distinct from the plural suffix resuming the elders. In 21:22 the passive hearing predicate, copular pronoun and dative experiencer jointly supply “have”; in 21:23 the vow's dative is not reassigned to the purification infinitive.
- The active taking sense of the passive participle in 21:16 is supported by [CAL's dbr entry, passive-participle note](https://cal.huc.edu/oneentry.php?cits=all&lemma=dbr+V). In 21:24 Peal שלם with the law complement is “follow,” separately linked from “observe”; [CAL's šlm entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=%24lm+V) explicitly cites this verse with “follow and keep the law.” In 21:38 the nominal negative perfect form does not impose past tense on the current identity question, “Are you not that Egyptian”; the earlier uprising belongs to the relative clause. See [Joosten, section 2.1](https://www.academia.edu/34446491/The_Negation_of_the_Non_Verbal_Clause_in_Early_Syriac).
- Narrow exceptions retain anticipatory and resumed references, existential and past predicates, ethical datives, lexical greeting and immediate-time expressions, composite prepositions, and reduced circumstantial or complement clauses. In 21:28 “our” belongs to the separate דילן. In 21:32 the beating gerund retains its complement marker, participle and past auxiliary, while Paul retains both explicit object references. In 21:33-34 the asking, capacity and ordering auxiliaries stay with their respective predicates, rather than the following object or infinitive.
- Acts 21:1-7 preserves the straight course to Cos, the following day's Rhodes arrival, Patara, the ship for Phoenicia, approach to Cyprus and leaving it on the left, Syria and Tyre with the ship's cargo unloading. The stay with the disciples is seven days; their Spirit-mediated warning to Paul is explicitly every day, retaining כליומ omitted from the supplied reference. The escort includes the disciples, their wives and children. They, not the first-person travellers, kneel and pray by the shore. The subsequent farewell is an actual mutual kiss, followed by boarding and the others' return home. The next city is the source's Acco, with greeting and a one-day stay; no different place-name is substituted.
- Acts 21:8-11 keeps Caesarea, lodging with Philip the evangelist, his membership among the seven and four virgin daughters who prophesy. Agabus comes down from Judea after many days. The strap taken from Paul is explicitly associated with his waist, and Agabus binds his own feet and hands. The prediction retains the owner of this strap and third-person plural נֵאסרֻונֵה, “they will bind him,” rendered naturally as the man being bound by the Jews in Jerusalem. This is not a second-person address to the Jews; the object suffix is retained through the passive patient's role. Delivery into the hands of the Gentiles follows. The same source form appears in [Dukhrana's Peshitta text of Acts 21:11](https://dukhrana.com/peshitta/analyze_verse.php?font=XenoType+SY+Darmasuq&lang=en&size=125%25&source=ubs&verse=Acts+21%3A11).
- Acts 21:12-16 preserves the travellers and local people pleading with Paul, his question about weeping and breaking his heart, and readiness not only for bonds but also death in Jerusalem for the name of “our Lord Jesus.” No Messiah title is added where the stored source lacks it. The travellers fall silent when he is not persuaded and invoke our Lord's will. After preparation and ascent to Jerusalem, Caesarean disciples accompany them with Mnason, a brother from among the earlier disciples and from Cyprus. His receiving them at his house is a purpose or anticipated hospitality, not an already completed reception asserted during the journey.
- Acts 21:17-21 keeps the brothers' joyful reception, the next day's visit with Paul to James and all the elders, first-person “we” greeting them and Paul recounting God's work among the Gentiles through his ministry. The praise is to God. “Our brother” retains its own first-person plural suffix, and the believing myriads are in Judea, without an unexpressed exact number. The elders' speech continues through 21:25. The reported accusation concerns all Jews among the Gentiles departing from Moses, non-circumcision of their children and not following the law's customs. The Moses/name and teaching/departure predicates remain in their own linked grammatical clause.
- Acts 21:22-27 retains the causal report of Paul's arrival, the instruction continuing into 21:23, four men and their purification vow, taking them, purifying with them, paying their expenses and shaving their heads. The expected public knowledge concerns the false report and Paul's following and observing the law. For believing Gentiles, “we wrote” and the four prohibitions remain: sacrificed meat, fornication, what is strangled and blood. No additional idol noun is inserted. Paul's taking these men on the next day, purification, temple entry, notification of the purification days' completion and offering for each person remain distinct. The actual seventh day arrives in 21:27; it is not replaced by an imported near-completion formula. The Asian Jews' sighting, incitement and seizure lead into the following cry.
- Acts 21:28-33 preserves the help request to Israel's sons, teaching everywhere against “our people,” the law and this place, and the accusation of bringing Gentiles and defiling the holy place. The law and people nouns are not moved into each other's spans. The narrated basis is having seen Trophimus the Ephesian with Paul in the city and supposing that Trophimus entered the temple with Paul; this is not rewritten as a causative claim that Paul brought him. The whole city's agitation, assembled people, seizure, dragging outside the temple and immediate gate closure remain. The cohort commander's intervention names one centurion and many soldiers, followed by plural “they ran.” The crowd stops beating Paul. The binding command specifies two chains, with questions about his identity and deeds.
- Acts 21:34-38 keeps contradictory shouting, inability to learn what was true because of that shouting, and the order to take Paul to the barracks. The soldiers carry him on reaching the stairs because of the crowd's violence; the following crowd cries “Away with him!” Paul's approach to the barracks includes the complete conditional “If you permit me, I will speak with you,” preserving אנ and the first-person future speaking verb; the if sense is recorded in [CAL's ˀn, sense 1](https://cal.huc.edu/oneentry.php?cits=all&lemma=%29n+c). The commander's Greek-language question continues into the negative question about the Egyptian, rather than becoming a declarative denial. Four thousand men “doing evil” remain explicit, without replacing the broad source wording with an unexpressed group name.
- Acts 21:39-40 retains Paul's Jewish identity, Tarsus in Cilicia, a notable city, his birth there and request to speak to the people. The topical subject and enclitic copula have separate source owners in “I am a Jewish man,” without forcing duplicate English declarations. Permission, standing on the stairs, the singular hand gesture and the people's quiet precede his address. The source label עבראית is rendered “in Hebrew”; the supplied lexical alternatives also permit Aramaic, so the English label is not offered as an independent historical resolution of which language variety was spoken. The chapter ends with the introduction to his words; the actual quotation begins with the following chapter's speech.

Source checks for Acts 22:

- The shared-span review checked all 30 verses and 471 source words, reducing 142 original shared spans to 48 documented exceptions. Every final mapping, exact exception reason and the continuous English received three independent whole-chapter reviews against the frozen files. English is unchanged throughout all 30 verses. Ordinary names, modifiers, negatives, auxiliaries and reordered clauses retain separate owners.
- In 22:3 the compound relation “at” is distinct from the feet and Gamaliel's anticipated name; the comparison retains both conjunction words, the explicit hearers, the quantifier's suffix and the final copula. In 22:6 the first-person copular pronoun belongs to “drawing near,” rather than Damascus. In 22:8 “whom” and the explicit “you” share their actual prefixed source word, while “are” belongs to the separate copular pronoun. In 22:10 the passive telling clause retains the hearer as “you”; it does not add a second “with you.”
- In 22:11 “could” retains the passive seeing predicate, past auxiliary and first-person dative experiencer, with “I,” “not” and “see” separately linked. The companions' past copula remains in their relative clause. In 22:13 the single English “to me” retains both the arrival destination and the following speech addressee, while coming and speaking remain separately linked. In 22:20 the plural suffix on the will resumes the killers; the separate singular suffix on killers supplies “his.” The English does not confuse these different possessors.
- Other narrow exceptions retain lexical defense, prison, punishment, midday and suddenness expressions, existential and reduced predicates, ethical datives and resumed references. In 22:22 “such” retains the reduced characterization and copula, while “a man,” removal, earth and negation are separate; “ought” retains the impersonal propriety predicate with its dative experiencer. In 22:25 the single name Paul covers the explicit subject pronoun and appositional name. In 22:26 transitive “approached” retains the independent direction marker; in 22:30 “wanted” retains its narrative auxiliary separately from “to know.”

- Acts 22:1-3 opens Paul's defense after the introduction in 21:40, closes for the narrative interruption in 22:2 and resumes the speech in 22:3. מפק ברוח is a defense idiom, consistent with [CAL's npq entry, derivative mappaq brūḥā](https://cal.huc.edu/oneentry.php?cits=all&lemma=npq+V); no separate spirit actor is introduced. The listeners hear him speaking with them in Hebrew and become quieter. The Hebrew label follows 21:40 without claiming an independent historical resolution of the language variety. Paul's Jewish identity, birth in Tarsus of Cilicia, upbringing in this city at Gamaliel's feet, full instruction in the law of our fathers, past zeal for God and comparison with all the hearers remain. No unexpressed “today” is added.
- Acts 22:4-5 preserves persecution of this way to death, binding and handing men and women over to prison. The high priest and all the elders testify about Paul; the witness predicate remains with its subjects rather than being moved into the letters clause. The elders are not given an unexpressed first-person owner. Letters, travel to the brothers in Damascus, bringing those there to Jerusalem in bonds and their receiving punishment remain distinct. מסם ברשא is the lexical compound “punishment,” supported by [CAL's mɘsām brēšā entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=msm%40bry%24%29+N); the English does not make execution explicit merely from its literal head/placing components or the supplied reference's capital-punishment wording. The testimony clause continues the preceding persecution sentence.
- Acts 22:6-10 keeps travel, beginning to approach Damascus and midday. מן תחית שליא is rendered idiomatically “suddenly,” consistent with the compound usage in [CAL's šly entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=%24ly%232+N), whose citations include the same construction at Acts 9:3. “From heaven” belongs to the light's origin and is not made the possessor of quietness. The light shines upon Paul, who falls and hears the repeated “Saul, Saul.” Jesus identifies himself as the Nazarene. The men see the light but do not hear the voice of the one speaking with Paul; this is not changed to an understanding formula to harmonize another account. “My Lord” in Paul's question and “our Lord” in the narrator's next clause remain distinct, as do the rise/go commands and subsequent instructions at Damascus.
- Acts 22:11-13 retains inability to see because of the light's glory, the companions taking Paul by the plural hands and Paul's own entry into Damascus. The copular word at 22:11:13 belongs to the companions, not the next entry predicate. Ananias is upright according to the law, as all the local Jews testify. The noun phrase in 22:12 carries into the actual coming predicate at the start of 22:13; no coming verb is imported into the previous verse. His command to Saul is “open your eyes.” That nested quotation ends before Paul narrates the opening of his own eyes at that time and looking at Ananias.
- Acts 22:14-18 resumes Ananias's words with the God of our fathers, appointment to know his will, seeing the Righteous One and hearing a voice from his mouth. Witness to all people concerns everything seen and heard. The commands to rise, be baptized and be cleansed from sins, while invoking his name, remain through 22:16. The next narration keeps returning, coming here to Jerusalem and praying in the temple. The subsequent sight is explicitly a vision; no separate trance is added. The command to hurry and leave Jerusalem retains the stated cause: they will not receive his testimony about the speaker.
- Acts 22:19-21 preserves “my Lord,” the hearers' knowledge, delivery to prison and beating believers in all the synagogues. Stephen is “your witness”; his blood being shed, Paul's standing with the killers, consent to their will and guarding the garments of those stoning him remain separate. The final command sends Paul far away to preach to the Gentiles. The nested words and Paul's outer speech both close before the crowd narrative begins.
- Acts 22:22-27 explicitly names Paul in the hearing clause, the crowd's raised voice and cry for such a man to be removed from the earth because he ought not to live. Shouting, throwing garments and tossing dust into the sky continue into the commander's response. Questioning is by scourging, intended to discover why they shout against Paul. The binding apparatus is plural leather straps, not an unexamined rope substitution. Paul's question includes the actual flogging predicate with the Roman and uncondemned man. The centurion asks “What are you doing?” and identifies the man as Roman. The commander's “Tell me, are you a Roman?” is one question; a repeated subject/copular pronoun is not made into another utterance. Paul's answer is yes.
- Acts 22:28-30 keeps the large monetary acquisition of Roman citizenship and Paul's contrasting birth into it, with the even/also particle retained. Those intending to scourge him withdraw; the departure verb is not forced into a flight claim. The commander fears because he has bound a Roman, not because the source suddenly substitutes a stretching verb. The next day brings inquiry into the actual accusation, unbinding, an order for the chief priests and the whole assembly of their leaders to come, then taking Paul, bringing him down and standing him among them. Each of these final motion and placement predicates stays with its own source positions.

Source checks for Acts 23:

- The shared-span review checked all 35 verses and 541 source words, reducing 170 original shared spans to 41 documented exceptions. Every final mapping, exact exception reason and the continuous English received three independent whole-chapter reviews against the frozen files. English is unchanged throughout all 35 verses. Ordinary names, modifiers, negatives, auxiliaries and reordered clauses retain separate owners.
- In 23:3 “God,” the copula, future expression and striking predicate are separate; the reduced judging and coordinated ordering predicates retain their explicit copular pronouns. In 23:5 “I / did / not / know” has separate owners, and the scriptural writing clause keeps “for” separate from the copula and participle. In 23:9 the genuine “what of that?” idiom has separate question and pronominal-relation exceptions, rather than a whole-clause link. In 23:10 “was” and “afraid” are separately owned.
- Resumed references retain narrow spans for Paul, the young man and the people. In 23:14 transitive “approached” retains both coordinated direction markers; in 23:15 “ask” retains the requested-from relation while the commander remains separate. In 23:16 “of” in “son of” belongs to the construct relation on son; Paul's anticipated name belongs with the suffix on sister. The other “of,” after heard, belongs to the English hearing relation, separately from the plot noun.
- The possession predicates in 23:17-19 retain the existential and dative together in “has/have,” with their persons and objects separately linked. In 23:21 “do” retains the addressed topic and prohibitive predicate despite the English imperative's omitted subject; “not” and “be persuaded” remain separate. In 23:25 “read” is the English document-content idiom for what was in the letter, with no additional reading event. In 23:29 “there” and “no” belong to the negative existential and “was” to its separate past auxiliary. The final farewell retains the two-word be-well formula.

- Acts 23:1-5 retains Paul's look at their assembly, “Men, my brothers,” conduct in all good conscience before God and the explicit “until today.” Ananias is called the priest; no additional high-priest title is inserted into the stored wording. His command names Paul and the mouth. Paul's reply retains God's impending blow, the whitewashed-wall address, sitting in judgment according to law while transgressing it, and ordering others to strike him. The bystanders ask about insulting God's priest. Paul's lack of knowledge, “my brothers,” priest identification and the written prohibition on cursing the ruler of your people remain. The writing predicate and copula belong to positions 9-10, with “for” separately owned at 11, rather than the priest clause, and the scriptural quotation is nested within Paul's reply.
- Acts 23:6-10 preserves the Sadducee and Pharisee groups, Paul's Pharisee identity and “a son of Pharisees,” with the actual plural at 23:6:18. Hope concerns resurrection of the dead. Their falling upon one another and the divided people precede the explanation: no resurrection, plural angels or a singular spirit, while the Pharisees confess all these things rather than an imported “both.” Scribes from the Pharisees' party contend with the others, find nothing evil in this man and retain the final “what of that?” question after the spirit-or-angel possibility. No extra fighting-against-God clause is supplied. The commander fears Paul's being torn apart and sends for the explicitly named Romans to come, snatch him from their midst and bring him into the barracks; no unexpressed descent is added here.
- Acts 23:11-15 keeps night, the appearance of “our Lord” to Paul, “Be strong,” the causal comparison with Jerusalem testimony and the prospective testimony in Rome. Readiness alone does not replace that future assertion. The gathered men put themselves under a curse not to eat or drink until killing Paul. The pact with oaths involves more than forty men. They approach priests and elders, not an added set of chief priests, and their words continue through 23:15. The imperative “ask” stays with the requesters, the hearers and council chiefs; the request concerns bringing Paul to them under the pretext of examining his affair. Their readiness to kill precedes his arrival.
- Acts 23:15:19 נמטא is rendered “he reaches,” referring to Paul. Although the exported tag selects first-person plural, the form can also be third-person masculine singular. [Younansardaroud's Classical Syriac Course Book, section 8.1, printed pages 55-56](https://d-nb.info/1184881278/34#page=64) gives the shared third-singular and first-plural imperfect forms and discusses contextual interpretation. The preceding “him” and destination “you” support Paul's arrival here; no letters are changed to force the reading. The next account retains the son of Paul's sister, hearing the plot, entering the barracks and reporting to Paul. Paul sends for and calls one of the centurions, who brings the young man to the commander. The prisoner title, requested transport and message recipients remain distinct.
- Acts 23:19-22 explicitly retains the young man, the commander's taking him by the singular hand, leading him aside and asking what he has to tell. The reply identifies the Jewish planners, tomorrow, bringing Paul down to their assembly and the pretense of learning something more. The warning not to be persuaded retains more than forty men from among them, the ambush, their self-imprecatory curse, both food and drink prohibitions, the killing condition, readiness, waiting and “your promise.” The commander dismisses the young man with a command that no one know of his disclosure.
- Acts 23:23-26 keeps two centurions, two hundred Romans, seventy horsemen and two hundred described as throwing with the right hand. The wording does not specify a weapon for that last group. Departure is three hours into the night, retaining the cardinal hours expression without converting it to a modern clock time. The following command continues the same quotation and calls for a singular beast of burden to carry Paul safely to Felix the governor. The letter is written and given to them; the narrator's introduction precedes a continuous quoted letter from 23:26 through 23:30. Its address names Claudius Lysias, Felix and the excellent governor's title and greeting.
- Acts 23:27-30 remains the commander's first-person account, including his assertion of rescue when he learned of Roman identity; it is not rewritten to reconcile his account with the preceding narrator's chronology. The Jews' seizure has the stated purpose of killing. The reason for accusation, descent to their assembly, questions of their own law and the absence of a charge worthy of bonds or death remain. Negation remains in its own clause; “there” and “no” belong to the negative existential, while “was” belongs to the separate past auxiliary. The letter names a treacherous ambush prepared against Paul, immediate sending to Felix, orders to the accusers to come and speak with him before the governor, and the final farewell.
- Acts 23:31-35 resumes narration with the commanded night journey to the city of Antipatris. The source has the horsemen dismissing their companions, the foot soldiers, so that they may return to the barracks; that direction of dismissal is not reversed. Arrival at Caesarea, delivery of the letter and presenting Paul precede reading, the province question and learning of Cilicia. The final speech promises a hearing when the accusers arrive. The separate guard command belongs to positions 8-9; the following location is Herod's praetorium. No unexpressed Herodian personal identification is inferred from a lexical name tag.

Source checks for Acts 24:

- The shared-span review checked all 27 verses and 421 source words, reducing 119 original shared spans to 56 documented exceptions. Every final mapping, exact exception reason and the continuous English received three independent whole-chapter reviews against the frozen files. English is unchanged throughout all 27 verses. Ordinary names, modifiers, negatives, auxiliaries and reordered clauses retain separate owners.
- In 24:2 “proceeded” retains its co-verb and narrative auxiliary, while Tertullus and the accusation remain separately linked. [CAL's nqp entry, causative co-verb sense 3](https://cal.huc.edu/oneentry.php?cits=all&lemma=nqp%232+V) supports the begin/proceed construction. In 24:10 the genuine נפק ... רוחא defense idiom retains its copular pronoun; “on,” the compound behalf relation and “my own” have distinct owners. In 24:11 the duration predicate retains the first-person dative, while “no,” “more” and “than” remain separate. In 24:13 “can” retains the hand-power idiom, “they” belongs to its actual plural suffix, “prove” to its own verb and “before you” to its own preposition.
- In 24:14 the repeated locative preposition, anticipated teaching and resumed relative reference have narrow exceptions. In 24:15-17 the possession predicates, accompanying hope and human-group expressions retain their actual components, while pure conscience and the separate from/among/dead words remain individually owned. In 24:19 “ought,” the past modal support rendered “to have” and “stood” have separate links. In 24:23 the English “from” belongs to the hindering relation; the source has no separate from-word there. “Serving” retains the dependent being verb and participle.
- In 24:25 the single English “about” retains all three repeated prepositions governing righteousness, holiness and judgment; each “and” stays with its actual prefix. In 24:26 one past auxiliary supports the coordinated sending, causative bringing and speaking predicates. The English “had him brought” expresses causation, without inventing another source auxiliary or forcing a pluperfect. In 24:27 completion retains the masculine singular active verb and incumbent's dative while English naturally presents the years as the completed period; the years are the patient, not a claim about the Syriac active subject. The coming, naming, resumed Paul and reduced bound state have their own narrow exceptions.

- Acts 24:1-4 explicitly calls Ananias the chief priest and retains five days, the elders, Tertullus the orator and their report concerning Paul. Tertullus's speech starts in the actual words of 24:2, with abundant peace because of Felix and many good things through his care. All of us, every place and your favor remain in 24:3. In 24:4, נלאיכ is first-person plural with a second-person object: “we do not weary you,” followed by the singular “I ask” and first-plural “our humble plea.” It is not changed to Felix's gentleness or a singular subject in the wearying clause.
- Acts 24:5-8 keeps the accusation of being a corrupter and stirring an uproar among all the Jews throughout the whole land. The final noun is teaching, specifically the teaching of the Nazarenes, without importing a sect noun. Our temple, our law and our hands remain the accusers' first-plural claims. Lysias comes as commander, takes Paul with great force and sends him to the governor. The accusers' ordered arrival is followed by the governor's ability to learn when questioning him. Both occurrences of “him” remain pronouns; no disputed antecedent is resolved by inserting a name. Tertullus's quotation closes after 24:8.
- Acts 24:9-11 preserves the Jews' outcry against Paul rather than reducing it to agreement with Tertullus. The governor beckons to Paul, who answers and begins a continuous quotation. The many-years, knowledge and judge clauses are reordered in English with separate source owners. נפק ... רוחא is rendered as making a defense, using the defense idiom recorded in [CAL's npq entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=npq+V), rather than adding a separate spirit participant. על אפי נפשי is naturally “on my own behalf.” The next verse retains the ability to learn, no more than twelve days, the ascent to Jerusalem and worship.
- Acts 24:12-15 keeps speaking with anyone in the temple, gathering a crowd, the repeated negatives, their singular synagogue and the city. The inability to prove charges before the governor is rendered idiomatically without physical hands. Paul's acknowledgment names the teaching about which they speak and the God of my fathers; no “to you,” sect noun or calling predicate is supplied. All that is written in the law and prophets, hope in God, the other men's expectation and resurrection of the righteous and unrighteous remain. מנ בית מיתא is “from among the dead,” following the Syriac preposition documented in [CAL's bēṯ entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=byt+p), rather than requiring a physical house from the homophonous noun's base gloss. The dead noun stays with its own prepositional phrase.
- Acts 24:16-19 retains the effort to have a pure conscience continually before God and people, the many-year interval, coming to his own people, alms and presenting an offering. The temple discovery occurs when Paul is purified; מדַֿכַּי is rendered as a resultant state without forcing an unexpressed cleansing agent. No crowd and not even an uproar remain distinct. The Jewish men from Asia belong to 24:19 in the actual source, with the conditional “if,” their disturbance, their obligation to stand with Paul before the governor and their own accusation. These clauses are not moved across the verse boundary to imitate the supplied reference.
- Acts 24:20-23 keeps the alternative invitation to these men to identify wrongdoing before their assembly and the exception for the one cry about resurrection. That earlier cry is nested within Paul's speech, which closes at 24:21. Felix's knowledge of this way is explicitly causal, followed by his deferral and promise to hear when the commander comes. The title does not assert a separate headcount. The centurion's guard of Paul at rest and the prohibition on hindering acquaintances from serving him remain; the rest expression is not made a release from custody.
- Acts 24:24-26 preserves both sending and calling for Paul, Felix and Drusilla, her description as his Jewish wife, their plural hearing and faith in the Messiah. The discourse concerns righteousness, holiness and the coming judgment; holiness is not replaced with a Greek-derived self-control formula. Felix is filled with fear and promises to send when he has opportunity. The next verse explicitly names a bribe from Paul, its causal “For,” the consequent repeated sending, bringing and conversation. No release clause or bargain absent from the actual source is added.
- Acts 24:27 retains completion of two years, another governor replacing him and the full name Porcius Festus. The two-year phrase does not assert Felix's entire tenure. Felix's purpose of doing the Jews a favor and leaving Paul bound remain separate from the replacement governor's arrival. Every source position remains covered; separate English constituents may repeat a source owner, and documented shared spans retain the constructions described above. Both the continuous speeches and nested resurrection cry have balanced quotation marks.

Source checks for Acts 25:

- The shared-span review checked all 27 verses and 450 source words, reducing 133 original shared spans to 48 documented exceptions. Every final mapping, exact exception reason and the continuous English received three independent whole-chapter reviews against the frozen files. English is unchanged throughout all 27 verses. Ordinary names, modifiers, temporal phrases, negatives, auxiliaries and reordered clauses have separate source owners.
- In 25:4 the answer expression retains the returning verb and its word noun, while Festus, the connective and the quoted custody predicate remain separate. In 25:5 the reduced existential designation “those” and the hand-power ability idiom have narrow exceptions; “the man” and “in him” remain distinct English realizations. In 25:6 the descent retains its subject-referring ethical dative, without absorbing Caesarea. This use of a motion verb with a coreferential dative follows [Younansardaroud, section 8.4, printed page 57](https://d-nb.info/1184881278/34#page=66).
- In 25:8 “was” is separate from the genuine defense idiom, which [CAL's npq entry, sense 5(1)](https://cal.huc.edu/oneentry.php?cits=all&lemma=npq+V) documents with both the verbal and nominal defense expressions. “No” retains the negative and indefinite components, while the offense predicate has its own owner. The same distinction keeps single-word negative indefinites narrow in 25:10-11 and 25:25; ordinary separate English negatives remain separate.
- Resumed recipients and patients in 25:9, 25:11, 25:13, 25:17 and 25:22 retain their actual suffixes and explicit names or nouns. The possessive references in Paul's case and Caesar's judgment likewise retain both owners while “case,” “for” and “judgment” keep their own links. Counselors, the singular accuser, the greeting and the courthouse retain only their lexical compounds. In 25:19 “had” is the past possession of disputes, with its existential, auxiliary and dative possessor; it is not a pluperfect auxiliary. In 25:25 the done-state predicate retains its agentive dative, without inventing a source past auxiliary.

- Acts 25:1-3 retains Festus arriving at Caesarea, three days and the ascent to Jerusalem. The chief priests and Jewish leaders inform him about Paul and petition him; the request continues into the next verse. Both sending and having Paul brought to Jerusalem remain, followed by the plural ambushes on the road and the purpose of killing him. The report predicate stays with its own subject clause rather than being assigned to the later Paul phrase.
- Acts 25:4-7 keeps Festus's answer as one quotation through 25:5: Paul is kept at Caesarea, Festus is hurrying to travel, and those able among the hearers should go down with us and accuse the man. The ability expression is idiomatic rather than a statement about physical hands. The return follows eight or ten days, without importing an absent “not more than.” The tribunal session is on the next day. Jews who came down from Jerusalem surround Paul and bring many serious charges they cannot prove. רשא is rendered as legal heads of accusation in context, rather than an extra group of leaders being brought; [CAL's rēš entry, sense 7](https://cal.huc.edu/oneentry.php?cits=all&lemma=ry%24+N) records the broader main-point sense supporting this contextual use.
- Acts 25:8-10 preserves Paul's defense as an indirect statement that he committed no offense against the Jews' law, the temple or Caesar. נפק ... רוחא is the established defense idiom, not an extra spirit participant. Festus's desire to grant the Jews a favor explicitly causes his question about going to Jerusalem and being judged before him. Paul's answer retains Caesar's tribunal, standing before it, here as the proper place for judgment, no wrong to the Jews and the governor's own knowledge; no unexpressed “well” is added. Paul's continuous quotation runs through 25:11.
- Acts 25:11-12 keeps the conditional wrongdoing and death-worthy act, willingness not to refuse death, the conditional absence of the accusers' charges and the prohibition on handing Paul over to them as a gift. The passive participle with first-person dative in the first condition means wrongdoing committed by Paul, not wrongdoing done to him. “I do not refuse to die” follows the refuse/abstain-with-min usage in [CAL's šʾl entry, Gt sense 1](https://cal.huc.edu/oneentry.php?cits=all&lemma=%24%29l+V). The appeal to Caesar ends Paul's quotation. Festus speaks with his counselors and answers with two distinct clauses: the appeal to Caesar and the journey to Caesar. The second occurrence of Caesar stays with the going clause, not with the preceding appeal.
- Acts 25:13-16 preserves King Agrippa and Bernice coming down to Caesarea to greet Festus, their days with him and Festus relating Paul's case to the king. Felix leaves a certain man as a prisoner; the passive agent is naturally “by Felix,” without an obligatory physical-hands description. The Jerusalem report names chief priests and elders and explicitly asks for judgment against Paul for them. Festus's reply quotes the Roman custom: handing a person over as a gift to be killed is forbidden until the singular accuser comes, charges him to his face and he receives opportunity to make a defense. The killing clause is present in the actual source and is retained. Opportunity does not require a physical place, and the defense compound does not add a spiritual actor. The embedded quotation closes before 25:17 while Festus's outer report continues.
- Acts 25:17-21 keeps arrival here, no delay, the next-day tribunal session and the order to bring the man to Festus. The accusers stand with him but cannot substantiate the expected charge of wrongdoing. Their disputes are with Paul about their religion and about Jesus, a man who died whom Paul said was alive. The religion sense of דחלתהונ follows [CAL's deḥləṯā entry, sense 2](https://cal.huc.edu/cal_entry_web.php?lemma=dxlh+N), rather than requiring ordinary fear from the base gloss. Festus's lack of clarity concerns the inquiry into these matters, followed by his quoted question about Jerusalem and judgment there. קאם with על has the ascertain/understand sense recorded in [CAL's qwm entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=qwm+V), without requiring a physical-standing or literal-establishment phrase. Paul's request for Caesar's judgment, the custody command and sending to Caesar close Festus's report at 25:21.
- Acts 25:22-25 distinguishes Agrippa's wish to hear this man from Festus's tomorrow reply. The next-day arrival names Agrippa and Bernice, great pomp, the courthouse, plural commanders and city leaders. Festus gives the order and Paul comes. The new speech addresses King Agrippa and all the men with us. כלה in 25:24 belongs to all the Jewish people petitioning Festus, not to the preceding “you see.” Their crying retains its third-person plural subject. Festus's finding is that Paul has done nothing deserving death; the passive-participle/dative construction is not reversed into a claim that nothing was done to Paul. The requested Caesar hearing leads to the sending order.
- Acts 25:26-27 keeps both occurrences of what to write, Caesar as recipient, all the hearers versus the singular King Agrippa and the purpose of examining the case. אשתאל is a passive interrogation form, with the case as its subject; it is not turned into a new first-person act of asking. אשכח stays with its own linked “I may find,” before the final what-to-write span. Its contextual first-person imperfect reading follows the first-person speaker and purpose clause, despite the exported homograph tag; [Younansardaroud's Classical Syriac Course Book, section 8.1, printed page 55](https://d-nb.info/1184881278/34#page=64) gives the first-singular prefix, and section 14.2, printed page 128, describes the irregular e-a forms of this verb. No letters are changed to force a new subject. The final causal “For,” explicit impropriety and first-plural sending and writing remain, with the prisoner's offense. Festus's speech closes at 25:27; the chapter's speeches and embedded quotations are balanced.

Source checks for Acts 26:

- The shared-span review checked all 32 verses and 522 source words, reducing 158 original shared spans to 59 documented exceptions. Every final mapping, exact exception reason and the continuous English received three independent whole-chapter reviews against the frozen files. English is unchanged throughout all 32 verses. Names, modifiers, negatives, auxiliaries, temporal phrases and ordinary reordered clauses have separate source owners.
- In 26:2 “because” belongs to the subordinating ד prefix on דקדמיכ, while “before you” realizes that word's stem and suffix. The following clause gives the ground for Paul's being fortunate; no separate causal source word is required. [Younansardaroud, section 14.6, printed page 129](https://d-nb.info/1184881278/34#page=140) describes the causal use of such relative clauses. Both English spans retain the actual prefixed source word.
- In 26:1-3 the defense idioms retain their verbal and nominal components, while the actual past auxiliary or present copular pronoun remains separate where English explicitly renders it. The behalf relation, reflexive self-assessment, fortunate predicate and patience expression have narrow exceptions. The plural reference in “all the questions and laws” retains both named members of that coordinated group without combining the whole noun phrase. In 26:6 “that came” describes the promise's occurrence to the fathers from God; it does not add physical travel or reverse the source relation. [CAL's hwy entry, sense 3](https://cal.huc.edu/oneentry.php?cits=all&lemma=hwy+V) records the come-to-pass use.
- The long light account in 26:13 is split throughout: midday, the road, seeing, the king address, light, heaven as origin, brightness, the sun comparison and all the illuminated companions retain their actual owners. The goad comparison in 26:14 likewise keeps its copula, adjective, hearer, kicking and goads separate. In 26:16 the prospective seeing predicate keeps its future orientation and copular pronoun, while Paul remains the seer and Jesus the object of both seeing verbs.
- In 26:22 the accompanying “saying” retains its subordinating marker and personal predicate; “nothing,” “beyond,” Moses and the prophets have separate narrow links. The resurrection phrase in 26:23 keeps “from,” “among” and “the dead” distinct. In 26:24 the defense, actual past auxiliary, ethical dative in the madness accusation, degree of learning and causative madness statement remain separate. “Openly” in 26:26 retains its genuine lexical compound without absorbing the speech predicate.
- In 26:29 “ask” retains the request verb and its governing “of” relation, naturally “ask God” in English; God and “I would” have their own owners. [CAL's bʿy entry, sense 3](https://cal.huc.edu/oneentry.php?cits=all&lemma=b%28y+V) documents the request construction with מן. The little/much alternatives preserve their separate source words. “Not” retains the nominal-negation copula without forcing a past reading, while “only,” “you,” “but” and “also” remain separate. In 26:30-32 the king's rising retains its ethical dative, reciprocal endpoints remain separate from “with,” and the past frame for speaking also supports “saying.” The final modal possibility, passive release and counterfactual appeal clause have distinct owners.

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

- The shared-span review checked all 44 verses and 676 source words, reducing 205 original shared spans to 69 documented exceptions. Every final mapping, exact exception reason and the continuous English received three independent whole-chapter reviews against the frozen files. English is unchanged throughout all 44 verses. Names, modifiers, negatives, destinations, ordinary auxiliaries and reordered clauses have separate source owners.
- Resumed patients, possessors and recipients retain both their actual suffixes and their explicit referents. In 27:10 the cargo's suffix anticipates our ship; the ship is not a second threatened object. In 27:39 the suffix on driving is specified by the following ship noun, rather than describing a journey toward another ship. In 27:43 the command's plural recipient resumes fronted “Those”; a second English “them” is unnecessary. These resumptive constructions follow [Younansardaroud, section 14.4, printed page 128](https://d-nb.info/1184881278/34#page=139).
- In 27:3 the conduct verb and its toward relation are rendered transitively as “treated Paul,” while kindness and Paul keep their separate owners. In 27:13 “and” belongs to the earlier conjoined hoping verb, not the unprefixed sailing participle. In 27:15 “gave way” stays with the yielding verb; the following relational and pronominal words supply “to it.” Subject-referring ethical datives in 27:9, 27:18, 27:20 and 27:42 retain narrow predicate exceptions, following [Younansardaroud, section 8.4, printed page 57](https://d-nb.info/1184881278/34#page=66), without introducing extra travelers or reversing the actors.
- The owner in 27:11 follows [CAL's mārē entry, sense 2](https://cal.huc.edu/oneentry.php?cits=all&lemma=mry+N). In 27:26 “must” retains the existential and first-plural dative necessity construction, separately from “be cast”; [CAL's ʾīṯ entry, sense 2(1)(4)](https://cal.huc.edu/oneentry.php?cits=all&lemma=%29yt+V) documents this necessary-outcome use. The counterfactual auxiliaries in 27:21 and the agentive tasting construction in 27:33 retain their actual owners. In 27:44 “The rest” retains both the fronted group and its resumed object, while the active plural causative is naturally rendered “were carried across.” The single English “on” retains both repeated prepositions before planks and timbers; the coordinated material nouns remain separate.

- Acts 27:1-3 explicitly names Festus ordering the sending to Caesar in Italy and handing Paul and other prisoners to Julius, a centurion of the Augustan cohort. The boarding predicate remains at its own positions in 27:2, with Adramyttium as the ship's origin and Asia as its destination. Aristarchus boards with us and is a Macedonian from the city of Thessalonica. Sidon, the next day, the centurion's kindness, permission to visit Paul's friends and refreshment all remain.
- Acts 27:4-8 preserves Cyprus, Cilicia, Pamphylia, Myra and Lycia in route order. The ship's difficult progress precedes the many days and arrival opposite Cnidus. The stored source explicitly calls Cnidus an island and Salmone a city; these labels are not silently changed to match another geographical account. The wind's prohibition on a straight course and our going around Crete remain separate predicates in their own spans. Fair Havens and the nearby city Lasea retain the actual source names.
- Acts 27:9-14 keeps the long stay through the passing of the Jewish fast day and the danger of sea travel, without assigning a modern calendar date. Paul's warning concerns our voyage, hardship and great loss, the cargo of our ship and our own lives; ship is not made a separate threatened object in the cargo clause. The centurion listens to the shipmaster and owner more than Paul's words. Many of us want departure, not an unexpressed majority, with the conditional prospect of reaching Phenice in Crete for winter. The harbor faces south in the stored wording, without additional directions. The south wind, their hope of reaching their desired destination, their sailing around Crete and the tempestuous Euroclydon remain. The wind name follows the actual אורקלידונ rather than replacing it with another familiar name.
- Acts 27:15-19 keeps the ship as the feminine singular subject that cannot face the wind, followed by our yielding. Cauda is the actual island name; the reference's Cyra is not imported. After lifting the boat, we strengthen and secure the ship, rather than securing the boat to the ship. The first-plural verbs remain throughout 27:17-19. The sea's descent expression is rendered as a precipice in the sea, following the supplied reference and the topographic descent/incline sense in [CAL's maḥḥatṯā entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=mxth+N). The exact nautical hazard remains uncertain; no unexpressed Syrtis proper name or definite sandbank identification is inserted. Lowering the foresail and drifting remain. Equipment is thrown into the sea on the next day; the third day names the ship's own equipment and our hands.
- Acts 27:20-24 explicitly includes the moon with the sun and stars during the continuing storm. The hope of our lives is completely cut off. No one having eaten precedes Paul standing among them. His conditional addresses you, while both consequences are we: we would not have sailed from Crete and would have been spared the loss and distress. His counsel against distress distinguishes the loss of the ship from the survival of every person. The angel's appearance is this night; the God to whom Paul belongs and whom he serves remains. The angel addresses Paul, his future standing before Caesar and God's gift of all those sailing with him. The gift noun is retained rather than replaced only by an inferred promise of life. The angel's words are nested within Paul's speech.
- Acts 27:25-29 keeps courage, trust in God, the promised outcome and the necessary casting upon a certain island. The duration is fourteen days, followed by midnight in the Adriatic Sea; it is not replaced by a fourteen-night formula. The sounding operation and both twenty and fifteen fathoms are retained, using the contextually attested wording in [CAL's qaw/ōməṯā entry](https://cal.huc.edu/cal_entry_web.php?lemma=qwmh+N), which cites 27:28 directly as casting the sounding line and finding twenty fathoms. No modern distance conversion is added. Our fear of rocky places is followed by their casting four anchors from the stern and praying for daybreak; the first- and third-plural subjects are not flattened into one actor throughout.
- Acts 27:30-34 preserves the sailors' attempted flight, lowering the boat from the ship into the sea and the pretense of going in it to fasten the ship to land. An absent bow-anchor explanation is not substituted. Paul sees this before speaking to the centurion and soldiers; his conditional requires these men to remain aboard for the hearers' survival. The soldiers cut plural boat ropes and let it drift. Paul urges everyone to take food until daybreak. His statement explicitly links fourteen days without tasting anything to the danger; no waiting predicate is supplied from another reading. The passive-participle/dative construction concerns what you have not tasted. Food for sustenance of your lives and the assurance that no hair from any hearer's head will perish close this speech before the meal narration.
- Acts 27:35-40 retains taking bread, glorifying God before everyone, breaking it and beginning to eat. Encouragement and nourishment precede the explicit total of two hundred and seventy-six souls aboard. After eating, they lighten the ship, take the wheat and throw it into the sea; both taking and throwing remain. At daybreak the sailors do not recognize the land but see a bay along its edge and plan, if possible, to drive the ship there. They cut away the anchors, leave them in the sea, loosen the plural rudders' fastenings and hoist a small foresail to the blowing wind before heading toward land.
- Acts 27:41-44 keeps the raised place between two depths of the sea, the ship's sticking there, its immovable forepart and its rear breaking under the waves' force. The soldiers' intended killing of the prisoners has the purpose of preventing swimming escape; the centurion restrains them because he wants to save Paul. Able swimmers are ordered to swim first and cross to land. The final causative plural verb says the rest are carried across on planks and other ship timbers, without assigning a new singular agent. Everyone's safe arrival on land remains explicit. Every source position is covered, and the speeches in 27:10, 27:21-26 with its angel quotation, 27:31 and 27:33-34 close before the following narration.

Source checks for Acts 28:

- The shared-span review checked all 31 verses and 518 source words, reducing 163 original shared spans to 49 documented exceptions. Every final mapping, exact exception reason and the continuous English received three independent whole-chapter reviews against the frozen files. English is unchanged in 30 verses; 28:11 adds “Now” for the actual discourse particle דין. Names, modifiers, negatives, ordinary auxiliaries and reordered clauses have separate source owners.
- In 28:1 the subordinating “that” belongs to the prefix on Melita, while “the” realizes the demonstrative before island. The same source word can own a separated prefix and stem in English. In 28:4 “whom” retains the relative marker and resumed patient suffix; the actual perfect verb supports “has not allowed,” without requiring a future permission statement.
- The “immediately” of 28:6 retains only בר שעתה; the swelling predicate remains separate. [CAL's bar entry, compound list](https://cal.huc.edu/oneentry.php?cits=all&lemma=br+N) lists this temporal idiom as “at once.” The accompanying dead state has its own narrow exception, distinct from falling. The repeated locative relations and their resumed island or place references in 28:7, 28:9 and 28:11 retain both owners without grouping whole location phrases.
- In 28:11 the relative prefix, actual past auxiliary and perfect wintering verb are separately linked as “which,” “had” and “wintered.” The later “had” expresses the sign's past presence on the ship with the existential and past copula; it is possession, not a pluperfect auxiliary. The possessive “had” in 28:19 likewise retains the existential and first-person dative in the retrospective as-if clause, without inventing a past source auxiliary.
- Request and opposition predicates in 28:14 and 28:19-20 retain their governing relations in narrow exceptions, while their participants remain separately linked. In 28:19 “people” preserves the membership expression בני עמי; [CAL's bar entry, sense 2](https://cal.huc.edu/oneentry.php?cits=all&lemma=br+N) records the use for a member of a group. In 28:20 the causal cleft's “it is because of” and “that” both realize מטל. The source bound participle has no relative prefix; the explicit first-person copular pronoun owns “I” and “am.”
- In 28:26 the hearing noun supplies “indeed,” separately from “You will” and “hear” on the finite verb. The following negatives and imperfect predicates remain separate. The hardening predicate in 28:27 retains its subject-referring ethical dative; its heart and resumed people reference have their own owners. In 28:30 renting keeps its beneficiary, while “at his own expense” retains the funding expression מן דילה and leaves house separate. In 28:31 “openly” is the established עין בגלא adverb documented by [CAL's compound entry](https://cal.huc.edu/cal_entry_web.php?lemma=%28yn%40bgl%29+N), separate from teaching and its actual past auxiliary.

The chapter retains the island's name, the foreigners' kindness, the viper and the change from expecting Paul's death to calling him a god. Publius, his father's fever and bowel pain, the healing, the island's other sick people, honors and provisions remain explicit. The route preserves the three months, Alexandrian ship, sign of the Twins, Syracuse, Rhegium, the south wind, Puteoli, the seven-day stay and Rome, followed by the Appian Forum, Three Taverns, centurion, and soldier guarding Paul.

Paul's account to the Jewish leaders keeps the absence of an accusation worthy of death, his appeal to Caesar, the hope of Israel, the lack of letters or evil reports from Judea and Jerusalem, and the invitation to explain the teaching. The day-long explanation preserves the kingdom of God, Jesus, Moses, the prophets, and the responses of persuasion and refusal. The Isaiah quotation retains the continuous negative purpose concerning seeing, hearing, understanding, returning and forgiveness. The final statement keeps God's redemption being sent to the Gentiles: the source form is plural and its people/nation/Gentiles sense is resolved by the immediate contrast and Murdock context. Paul's two-year rented-house ministry preserves his own expense, reception of visitors, proclamation of God's kingdom, teaching about our Lord Jesus Messiah, openness and lack of hindrance. Quotation boundaries run continuously from Paul's speech in 28:17 through 28:20 and through the nested Isaiah quotation in 28:25-28.

Source checks for Romans 1:

- The shared-span review checked all 32 verses and 424 source words, reducing 48 original shared spans to 36 documented exceptions. Every final mapping, exact exception reason and the continuous English received three independent whole-chapter reviews against the frozen files. The review repairs source ownership and continuous English as well as splitting shared spans. All 32 verse rows change, including punctuation joining 1:1-7 into one epistolary opening. Ordinary names, modifiers, negatives, auxiliaries and reordered clauses retain separate source owners.
- In 1:8 the faith noun is the subject of being heard, with its causal prefix separately linked as “because.” In 1:9 God is Paul's witness; the relative “whom,” service, explicit copula, first-person dative and causal “For” retain separate owners. The source says “in spirit,” without a first-person possessive suffix. The following relative and negative prefixes supply “that” and “without,” while ceasing has its own noun. In 1:10 the conditional prayer construction preserves possible opening, rather than asserting that Paul has already traveled.
- In 1:11 the source has an adverb of degree and a desirous participle with an explicit first-person copular pronoun: “I greatly desire,” rather than a good-desire noun and possession verb. In 1:12 “together” belongs to the opening adverb, and faith to the following noun. In 1:13 the actual dative supplies the first-person possessor of prospective fruit; fruit and “to me” are not interchanged. In 1:14 “for” realizes the causal ד prefix on דלכלנש, while “to everyone” retains that word's preposition and stem. [Younansardaroud, section 14.6, printed page 129](https://d-nb.info/1184881278/34#page=140) describes this causal use of relative clauses.
- The causal particles in 1:16-18 remain separate from the shame, righteousness and revelation predicates. Resumed Gospel, God, world, nations and human possessors retain both suffixes and explicit referents in narrow exceptions. The truth-holding clause in 1:18 keeps its demonstrative, relative prefix, predicate, truth and iniquity individually linked. In 1:19 God and “for” keep their actual positions, and the manifested object's suffix supplies “it.”
- In 1:20 the hidden things and causal “For” have distinct owners. The actual ל on the plural created beings gives “to his creatures,” retaining the possessive suffix. “Are visible” follows the seeing verb's reflexive/passive use, documented by [CAL's ḥzy entry, Gt sense 1](https://cal.huc.edu/oneentry.php?cits=all&lemma=xzy+V). “His” retains both coordinated possessive suffixes on power and divinity; eternal remains separate. מפק ברוח is the legal defense expression listed under [CAL's npq entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=npq+V). The negative therefore gives “without defense,” without reversing it into “not without excuse” or adding a separate spirit phrase.
- In 1:21 “as God” belongs to its actual comparative particle and God noun. The glorifying verb's suffix supplies “him.” The heart, darkening predicate, negative and understanding remain separate. In 1:24 and 1:26 English passive delivery preserves God as agent and the people as patients; the source verb itself is active causative. The lust noun and adjective in 1:24 are plural. The recompense in 1:27 retains its resumed object suffix on receiving, while the fitting predicate and its past auxiliary remain separate.
- In 1:30 סניאינ is retained as an adjective in “hateful to God”; [CAL's sny adjective entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=sny+A) distinguishes its hated/wicked senses from the active hating verb. “Disobedient” retains the negative and obedience predicate, with the parents separately linked; [CAL's pys entry, Ct sense 2](https://cal.huc.edu/oneentry.php?cits=all&lemma=pys+V) documents obeying and consenting. In 1:31 the negative existential and dative possessors give “having no” before the separately linked covenant; the negatives for love, peace and mercy remain distinct.
- In 1:32 the active condemning predicate has God as subject, those doing such things as patients and death as the sentence. Parentheses mark the content of God's judgment within the concessive clause. The following nominal negation has present meaning: its actual copula owns “are,” while the negative, only and doers have separate owners. The final contrast retains both doing the acts and being companions of those who do them.

The opening preserves Paul's servant and apostolic calling, the Gospel promised through the prophets, Jesus Messiah as God's Son, grace and apostleship among the nations, the Roman greeting, Paul's thanksgiving and prayer, and his desire to visit Rome. The chapter retains the predicates in the witness clause (God is Paul's witness), the adversative in 1:13, Paul's indebtedness to everyone in 1:14, the negation and Gospel power in 1:16, and the manifestation of God's knowledge in 1:19. The righteousness of God is revealed from faith to faith, and the repeated darkness and lack of understanding in 1:21 remain distinct.

Romans 1:23-32 retains the source's non-corruptible God, human and animal images, delivery over by God, unclean lust and shame, feared and served creation, their Creator, pain and shame, their nature, the male and female clauses, fitting recompense received in their persons, the delivered-over mind and improper acts, the full vice lists, the covenant and three separate negative clauses, and the final distinction between doing such things and being their companions. Source suffixes, repeated negatives, “these” demonstratives, judgment, death, condemnation, and the final “those who do these” clause remain explicit.

Source checks for Romans 2:

- The shared-span review checked all 29 verses and 407 source words, reducing 113 original shared spans to 56 documented exceptions. Every final mapping, exact exception reason and the continuous English received three independent whole-chapter reviews against the frozen files. All 29 verse maps change; English changes in 20 verses. Ordinary participants, negatives, auxiliaries, modifiers and reordered clauses retain separate source owners.
- Romans 2:1 retains the no-excuse construction, the neighbor being judged,
  the emphatic self-condemnation and the judge's own conduct. The related
  breath/escape expressions in 2:1 and 2:15 are rendered idiomatically as
  excuse, rather than adding an independent spiritual actor; compare
  [CAL's npq entry, G 5(1)](https://cal.huc.edu/oneentry.php?cits=all&lemma=npq+V).
  In 2:2 the judgment clause owns its copula; in 2:3 the final explicit
  pronoun is “you,” rather than a reflexive “yourself.”
- Romans 2:4 keeps the repeated preposition with the riches of goodness,
  patience and opportunity, and renders the insolent/daring predicate as
  “do you presume.” The repentance predicate retains its recipient.
  [CAL's mrḥ entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=mrx+V)
  records the daring sense. Romans 2:5 links the negative with repentance,
  and keeps both laying up and a store of wrath, the day and the revelation.
- Romans 2:5-8 joins the relative repayment clauses to their governing God
  clause. In 2:7 “seek” precedes its objects in English, retaining glory,
  honor and incorruptibility and the explicit recipient in “he gives them
  life forever.” In 2:8 the singular repayment verb has God as its implicit
  subject; the separate plural pronoun is its patient, “them.” The supplied
  English “with” expresses verbal government, not an extra source preposition.
  Romans 2:9-10 retains the Jew/Gentile parallels. Romans 2:12
  keeps both without-law phrases in its first half, followed by the separate
  under-law and by-law judgment clauses.
- Romans 2:13 renders כאנינ as “who are righteous,” distinct from the next
  words, “before God.” The exported root's gloss alternatives do not exhaust
  the actual form's adjectival sense. [CAL's kʾn adjective entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=k%29n+A)
  defines just/honest and illustrates the righteous sense in Romans 3:10.
- The relative prefixes in 2:12 and 2:14 introduce whole clauses even when
  attached to a law noun or negative construction. In 2:14 “by their nature”
  modifies doing the law's things. The negative existential and possessive
  datives retain their distinct contributions. In 2:16 the relative prefix
  and judging stem share one source word; God remains separately linked.
- Romans 2:17-20 continues the conditional introduction into the rhetorical
  questions of 2:21-23. In 2:19 “and you are confident” retains the conjunction.
  The questions preserve their explicit participants, negatives and predicates.
- Romans 2:23 keeps boasting, transgression against the law and dishonoring
  God himself distinct. Romans 2:25 retains present “is profitable,” the two
  conditional clauses, “however,” and the circumcision/uncircumcision
  contrast. Its second-person ethical dative belongs with “you depart,”
  without inventing an object “it”; the final dative is absorbed into
  “has become.” Romans 2:26 retains the question and renders the presentative
  particle as “then”; [CAL records its now-then use and cites this question](https://cal.huc.edu/cal_entry_web.php?lemma=h%29+I).
  In 2:27 the conjunction and judging stem belong to the first source word;
  the separate dative pronoun supplies “you.” The relative prefix on Scripture
  supplies “who,” alongside the preposition and noun. Romans 2:28-29 keeps
  the outward/hidden Jew and circumcision contrasts. In 2:29 circumcision has
  its explicit identifying copula, and the feminine “whose” resumes circumcision.

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

Source checks for 2 Corinthians 7:

- All 16 verses and 275 source positions were compared with the exported
  Syriac forms, the complete SEDRA gloss alternatives and morphology, and
  Murdock in context, followed by peer repair and an independent final
  chapter review. The seven causal `gir`, two adversative `din`, and one
  inferential `hakil` were checked in their clauses. Verse 1 retains both
  "therefore" and "because," and renders the source's "holiness" rather
  than Murdock's "righteousness."
- Verse 2 has "our brothers" and three separate first-person plural
  denials: wronging, corrupting, and defrauding no one. "Bear with us"
  supplies the contextually understood object of the opening imperative.
  Verse 3 keeps "not to condemn you" attached to "I speak," followed by
  the causal reminder and the shared dying and living together.
- Verse 4 preserves both first-person singular possessive constructions,
  joy as the subject of "abounds," and "all my afflictions." Verse 5
  follows the vocalised plural gentilic and SEDRA's "Macedonian" lemma
  with "to the Macedonians," where Murdock has "to Macedonia." It also
  keeps "not even one" respite, "our body," conflict outside, and fear
  within. No geographic data or route is inferred from this wording.
- Verses 6-7 distinguish God's continuing comfort of the lowly from his
  past comfort of us through Titus's arrival. Titus is the person refreshed
  by the recipients and the person reporting their love, grief, and zeal;
  "on our behalf" renders the face idiom. The final "I had great joy"
  renders the joy/was/to-me construction without adding another owner.
  Verse 8 retains both grief predicates, both mentions of the letter,
  the present absence of regret, the prior regret, and "for I see."
- Verse 9 keeps the two grief clauses distinct from the sorrow that
  brought the recipients to repentance. "So that you might suffer no
  loss" preserves the purpose and imperfect verb rather than asserting
  a completed absence of loss. Verse 10 retains the negative "does not
  turn back" and the separate positive causative "leads to life";
  the latter must not be swallowed into a negated turning to life.
- Verse 11 treats the construct noun plus prepositional phrase
  `mappaq b-ruha` as the defense/apology idiom, linking its two source
  positions together. It is not a separate "spirit" entry in the list.
  See the [Comprehensive Aramaic Lexicon compound entry](https://cal.huc.edu/cal_entry_web.php?lemma=mpq%40brwx%29+N)
  and [Gutman, section 3.4, example 8, page 51](https://refubium.fu-berlin.de/bitstream/fub188/21964/1/123-3-1034-1-10-20180423.pdf).
  All seven responses remain, including the source's "love." The closing
  reflexive "yourselves" and proleptic "in that matter" each express one
  referent, without adding souls or another object.
- Verse 12 uses "let this be clear" for the introductory future/jussive
  be-construction; "clear" is supplied to make the introduction readable,
  as Murdock supplies "seen." It adds no separate showing action.
  The offender and the person wronged each retain their own negation;
  the analytic expression for the wronged person does not add an
  independent "in him" circumstance. The purpose concerns the recipients'
  care for us becoming known before God, not our care for them.
- Verses 13-16 retain Titus's joy, his refreshed spirit with all the
  recipients, both truth expressions, first-person singular boasting
  versus "our boasting," and Titus remembering their obedience and their
  receiving him. "All" in verse 15 belongs to their obedience; verse 16
  has "in everything" and then simply "in you."
- Three five-position groups preserve local English dependencies: the
  delayed boasting noun in verse 4, the joy/made/to-me construction in
  verse 9, and the passive predicate with its delayed care-for-us subject
  and before-God phrase in verse 12. All other groups have at most four
  source positions. Each source position occurs exactly once in order.

Source checks for 2 Corinthians 8:

- All 24 verses and 316 source words received source-to-English and running
  English review, peer repair, and an independent final chapter audit. The
  six causal `gir`, eight discourse/contrast `din`, and the inferential
  markers in verses 23-24 were checked in their actual clauses. In verse 1
  `din` is "But," not an extra causal "For."
- Verse 1 preserves "our brothers," God's single ownership of grace,
  plural ecclesias, and the Macedonians. Verse 2 keeps their affliction,
  abounding joy, depth of poverty, and rich generosity. "Generosity"
  expresses the giving context of `peshitutha`, also glossed simplicity
  and directness in SEDRA; compare Murdock's liberality and the
  [Classical Syriac noun entry](https://www.assyrianlanguages.org/sureth/dosearch.php?language=id&searchkey=11506).
- Verses 3-4 retain both ability expressions, the donors' own willingness,
  the finite "they asked us," and the third-person plural "that they
  might share." Verse 5 has their giving themselves first to our Lord
  and also to us, by God's will. Verse 6 keeps the first-person plural
  imperfect request as a result/purpose, followed by Titus's beginning
  and completion of the same beneficence.
- Verse 7 has "our love toward you," not the reverse direction. Verse 8
  treats the negative formula plus infinitive absolute and finite command
  as one emphatic negative-command predicate, followed by the distinct
  testing of the sincerity of their love through their companions'
  diligence. Verse 9 retains our Lord Jesus the Messiah, his becoming
  poor for their sake while rich, and their becoming rich by his poverty.
- Verse 10 likewise has one emphatic counsel predicate, rather than two
  separate recommendations. "From last year," "not only to desire," and
  "but also to do" remain distinct. Verse 11 retains both completion
  predicates, both deed expressions, prior readiness and willing intent,
  and the actual "from what you have." Verse 12 keeps the condition of
  willingness, acceptance according to what one has, and the separate
  negated comparison with what one does not have. Its implicit donor is
  expressed as "he," as in Murdock's supplied person, rather than making
  willingness itself the possessor of material resources.
- Verses 13-14 distinguish others' relief from the recipients' affliction
  and preserve both directions of the intended equality: your own
  abundance supplies these people's need, and those people's abundance
  supplies your own need. The perfect form `hww` functions contextually
  as the exhortation "be in equality," not an assertion that they were
  already equal. For the Syriac perfect's imperative use, see
  [Cox, page 107, note 48](https://www.andrews.edu/library/car/cardigital/Periodicals/AUSS/1977-2/1977-2-01.pdf).
  This is a contextual reading of the recorded form, not a morphology
  change. Verse 15 keeps each gathering verb with its much/little clause,
  followed by its own absence of surplus/lack; it does not say that the
  first person gathered no surplus or the second gathered no deficiency.
- Verses 16-17 keep God as the giver of diligence in Titus's heart and
  Titus as the one accepting our request and going out to the recipients
  of his own accord. The analytic dative in his concern adds no separate
  beneficiary. The unnamed brothers in verses 18 and 22 are not identified
  with a conjectured historical person.
- Verse 19 has "with us," where Murdock has "with me." The infinitive and
  passive participle express emphatic choosing by the ecclesias; the
  beneficence is ministered by us, for God's own glory and our own
  encouragement. Each explicit "own" survives with a single owner.
  Verses 20-21 retain our care against blame and concern for good things
  before both God and people, with the causal connector intact.
- Verse 22 keeps the brother's having been proved to us, "always," "in
  many things," and his current greater diligence with confidence in
  the recipients. Verse 23 supplies "if it concerns" for its elliptical
  conditional topics and preserves "my own partner," our other brothers,
  and the compact genitive chain "ecclesias of the Messiah's glory."
  Verse 24 retains the demonstration of your love and our boasting in
  you; the English resumptive "it" refers to that one fronted object.
- Only verses 8 and 10 use five-position groups, each to render a single
  emphatic predicate with its intervening particles or pronoun. All other
  groups contain at most four positions. All 316 source positions occur
  exactly once in ascending order, with no unlinked lexical text.

Source checks for 2 Corinthians 9:

- All 15 verses and 223 source words received source-to-English and running
  English review, peer review and repair, and an independent final chapter
  audit. All three causal `gir` and six discourse/contrast `din` were checked
  in their clauses. Verse 13 retains both the causal "For" and the distinct
  "because of the proof" construction. Every linked group contains at most
  four source positions, and all positions occur exactly once in order.
- Verse 1 retains the genitive "service of the saints," the first-person
  predicate "I do something unnecessary," and the explicit condition
  "if I write to you." Verse 2 has the readiness of your singular mind,
  one proleptic owner, and first-person singular boasting in you to the
  Macedonians. Achaia is the place ready since last year; your own zeal
  stirred up many. The explicit "own" and "many" are preserved.
- Verse 3 distinguishes "I sent" from our boasting and "we boasted in
  you." "Lest" carries the source negation: another "not" would reverse
  the intended prevention of empty boasting. The final purpose has the
  recipients prepared, as I said. Verse 4 retains the Macedonians coming
  with me and finding you unprepared, followed by both shame predicates:
  we might be ashamed, not to say that you might be ashamed.
- Verse 5 renders the impersonal concern plus "to me" as "I was concerned"
  and keeps these my brothers coming before me to you. Both mentions of
  blessing remain, together with advance preparation and the contrast
  with greed. The second-person plural Ethpeal hearing expression is
  rendered "which you were informed beforehand should be ready," following
  the source and Murdock's notice/advice reading. See the
  [source-form analysis](https://dukhrana.com/peshitta/analyze_verse.php?font=Estrangelo+Edessa&size=125%25&source=ubs&verse=2Corinthians+9%3A5).
- Verse 6 keeps each sowing/reaping modifier: sparingly twice and
  bountifully twice. The blessing language conveys generous giving here.
  The first reaping participle is gnomic present, and the second reaping
  verb is future. Verse 7 supplies "should give" for the opening elliptical
  exhortation to each person. It retains his mind, sadness and compulsion,
  and the causal cleft about a cheerful giver. The named subject is God,
  where the local Murdock reference says Lord.
- Verse 8 renders the reach/hands expression idiomatically as "within the
  power of God." It keeps all grace, always, everything, what is sufficient
  for you becoming yours, and your abounding in every good work. Verse 9
  preserves the separate scattering and giving predicates, the poor,
  and his righteousness abiding forever.
- Verses 10-11 distinguish the continuing provision of seed and bread
  from the prayer that he give, multiply your seed, and increase the plural
  fruits of your righteousness. Everything and all generosity survive.
  Generosity accomplishes thanksgiving through us; the hands expression
  is "through us," and the thanksgiving noun has no possessive suffix.
  "Generosity" in verses 11 and 13 uses the contextual sense of
  `peshitutha`, as in chapter 8; see the
  [Classical Syriac noun entry](https://www.assyrianlanguages.org/sureth/dosearch.php?language=id&searchkey=11506).
- Verse 12 keeps the work of this service supplying the saints' need
  and also abounding in many thanksgivings to God. Verse 13 has the actual
  third-person plural "they glorify God," where Murdock has "we." The
  recipients' submission and sharing are perfect predicates. "Confession"
  expresses the Gospel profession here; Messiah is the single owner of
  the gospel. Both sharing with them and with everyone remain explicit.
- Verse 14 has plural people offering singular prayer for you, with much
  love because of the abundance of God's grace upon you. Verse 15 retains
  the thanksgiving idiom, the discourse connector, his gift, and its
  negative passive description as something that cannot be expressed.

Source checks for 2 Corinthians 10:

- All 18 verses and 273 source words received source-to-English and running
  English review, peer repair, and an independent final chapter audit. The
  seven causal `gir` and six discourse/contrast `din` were checked in their
  clauses, along with the separate causal `metul` constructions. Every
  source position occurs exactly once in ascending order.
- Verse 1 retains I, Paul, the Messiah's single ownership of gentleness
  and humility, the concessive "although," humility in person, and
  confidence when distant. Verse 2 keeps the prospective coming clause,
  the negative constraint, the confidence I have, my thinking, and the
  other people's thinking about us. The accusation about walking according
  to the flesh remains within their reported judgment.
- Verse 3 keeps both first-person plural predicates, walking and serving.
  Its negative cleft is explicit: "it is not according to the flesh that
  we serve." The recorded third-singular `hwa` belongs to this construction;
  it does not introduce another person or an independent past event. See
  [Pat-El, Syntactical Aspects of Negation in Syriac](https://doi.org/10.1093/jss/fgl006)
  on Syriac negation and negative clefts. The verb `plah` is rendered
  "serve," with the military setting made explicit by verse 4's weapons
  and warfare. Verse 4 retains God's power, our subduing, and both the
  plural strongholds and their rebellious character.
- Verse 5 has two active first-person plural predicates, demolishing
  thoughts and taking all thoughts captive. Every lofty thing is raised
  against the knowledge of God; obedience is to the Messiah. Each explicit
  genitive has a single owner. Verse 6 retains prepared readiness, executing
  vengeance on those who disobey, and the prospective completion of your
  obedience. The source has no separate "all" before the disobedient group.
- Verse 7 renders the faces expression as outward appearances and retains
  the question, confidence in oneself, "this one," knowing for himself,
  and both statements of belonging to the Messiah, including "so also do
  we." Verse 8 preserves "even if," the somewhat greater boasting, and the
  distinct recipients in our Lord's giving authority to me and giving it
  to us for your building up. Causal "because" remains separate from that
  purpose and from the negated destruction purpose.
- Verse 9 retains forbearance, the passive "lest I be thought," and one
  emphatic terrifying predicate from the infinitive absolute plus finite
  verb. My letters are plural. Verse 10 likewise has plural letters and
  plural weighty/strong predicates, followed by his weak bodily presence
  and contemptible speech. Verse 11 separately has singular "our letter,"
  the plural speaker, and the correspondence between absent written speech
  and present action. "When we are present" is not a direction toward us.
- Verse 12 keeps both reckoning and comparing ourselves with others,
  their boasting of themselves, their comparing themselves with themselves,
  and their not understanding. Verse 13 retains measure, boundary, God's
  allotment to us, and reaching even as far as you. The third-singular
  reaching predicate is read here with the boundary as its antecedent;
  compare the recorded morphology in the
  [source analysis](https://www.dukhrana.com/peshitta/analyze_verse.php?font=Estrangelo+Edessa&size=125%25&source=&verse=2Corinthians+10%3A13).
- Verse 14 preserves both negatives in the opening comparison, both causal
  particles, stretching ourselves, and the distinct past "we reached"
  in the gospel of the Messiah. Verses 15-16 retain both mentions of our
  measure, plural others, growing faith, our magnification by you, the
  separate abounding predicate, preaching beyond you, and the negative
  boasting clause about others' measure and prepared things.
- Verses 17-18 distinguish boasting in the Lord from praising oneself.
  The final approval cleft preserves causal "For," the single reflexive
  actor, and the Lord's future praise of the object him. The emphatic and
  copular pronouns create no additional person.
- Two longer groups preserve local grammatical dependencies: five positions
  for the delayed giving predicate and purpose in verse 8, and six for the
  imperative, its relative subject, and the fronted object in verse 11.
  All other linked groups contain at most four source positions.

Source checks for 2 Corinthians 11:

- All 33 verses and 466 source words received source-to-English and running
  English review, peer repair, and an independent final chapter audit.
  The nine causal `gir` and three discourse/contrast `din` were checked in
  their clauses. The two `gir` in verse 2 both remain causal; neither is
  swallowed into the prepositional phrase about jealousy over you.
- Verse 1 retains the wish to be borne with a little, the purpose of
  speaking foolishly, and the separate present "you do bear with me."
  Verse 2 preserves one husband, a pure virgin, and the first-person purpose
  of presenting the recipients to the Messiah. Verse 3 has plural minds
  becoming corrupted and one noun for simplicity toward the Messiah; the
  preceding source word is "from," and no second purity noun is added.
- Verse 4 keeps the visiting speaker as the active proclaimer, the separate
  alternatives of another Jesus, spirit, and gospel, the earlier negative
  proclamation/reception clauses, and the conditional persuasion. "Readily"
  renders the closing adverb without endorsing the rival message. Verse 5
  retains "in nothing" and the greatly excelling apostles. Verse 6 has my
  speech and my knowledge, followed by plural "we have been made manifest."
- Verse 7 treats the opening as a rhetorical question with one emphatic
  wrongdoing predicate, followed by humbling myself, your being exalted,
  and the finite past proclamation of God's gospel freely. Verse 8 retains
  other ecclesias, plundering, plural payments, and serving you.
- Verse 9 has the brothers supplying my need. English uses the equivalent
  passive "my need was supplied by the brothers" so that the following
  relative clause clearly describes the brothers; the Syriac supplying verb
  is active, with those same brothers as agent. The plural Macedonian
  gentilic is retained. "In everything" belongs to guarding myself, not
  to the brothers' supplying clause. Past "I guarded" and present "I guard"
  remain distinct, with the purpose of not burdening the recipients.
- Verses 10-11 preserve the Messiah's truth in me, the boasting not being
  annulled in me in Achaia's regions, and both questions about why and love,
  followed by God knowing. Verse 12 keeps both occasion nouns and the
  single ownership by those seeking an occasion; they may be found like us.
  Verses 13-15 retain these false apostles, deceitful workers, their
  disguising themselves, Satan's disguising himself as an angel of light,
  and his servants' actual disguising predicate. Their future end is
  according to their deeds; no extra actor is introduced by a copula.
- Verse 16 renders the rebuttal about being thought foolish as the negative
  appeal "let no one," following Murdock's contextual reading. It retains
  the imperative "receive me," the hypothetical comparison with a fool,
  and the purpose of my boasting a little. The understood copula in that
  comparison is supplied in English. Verse 17 retains both speaking
  predicates and the negative cleft concerning speaking in our Lord.
- Verses 18-20 distinguish the many boasting, my future boasting, the
  recipients listening to those lacking understanding, and their own
  wisdom. All five independent recipient/relative branches remain:
  enslaving you, devouring you, taking from you, exalting himself over you,
  and striking you on your plural faces. Verse 21 has both first-person
  speaking predicates, plural weakness, and the anyone/I comparison.
- Verse 22 preserves all three conditional identities and all three "so
  am I also" responses. Verse 23 has the initial "I am more than they"
  before labor, followed by three further comparative expressions for
  labor, blows, and chains. The plural deaths expression is rendered
  figuratively as dangers of death, with "many times" retained.
- Verse 24 renders the repeated forty distributively: five occasions,
  forty blows less one each time. Verse 25 retains three rod-beatings,
  one stoning, three shipwrecks, day before night, and the separate
  shipless stay in the sea. Both "I was" predicates remain. Verse 26 keeps
  all eight peril categories and both first-person past predicates for
  cities and wilderness. Verse 27 retains both "much" qualifiers and
  each condition, including hunger and thirst as separate nouns.
- Verse 28 distinguishes the singular daily gathering/thronging upon me
  from care on behalf of all the ecclesias. The faces expression is an
  idiom for that concern. Verse 29 has two questions and both first-person
  negative responses. Verse 30's vocalised `wale` means fitting/proper;
  its surface resemblance to "and not" does not license a negative.
- Verse 31 has one knowing God, the Father of our Lord Jesus the Messiah,
  blessed forever and ever, and the explicit "I do not lie." Verse 32 keeps
  the army chief of Aretas the king, his guarding the Damascenes' city, and
  the purpose of seizing me. Verse 33 keeps the window, basket, wall,
  their lowering me, my escape, and the singular possessor's plural hands.
- Only verse 24 uses a five-position group, to keep the distributive
  numeral, subtraction, and receiving predicate readable together. All
  other linked groups have at most four positions. Every source position
  appears exactly once in ascending order, with no unlinked lexical text.

Source checks for 2 Corinthians 12:

- All 21 verses and 369 source words received source-to-English and running
  English review, peer repair, and an independent final chapter audit.
  The nine causal `gir`, five discourse/contrast `din`, and the inferential
  `hakil` were checked in their clauses, alongside the causal and purpose
  uses of `metul`. Every source position occurs exactly once in order.
- Verse 1 preserves positive `wale`, "is fitting," and the separate
  "but not profitable." The first-person dative is absorbed in "I proceed"
  without adding a beneficiary; `gir` is causal. Visions and revelations
  remain plural, with one owner, our Lord.
- Verses 2-4 retain present "I know," the fourteen-year date of the
  catching-up event, both repeated body/without-body alternatives, both
  God-knows formulas, and the same man across the parenthesis. Punctuation
  places the temporal phrase with the catching-up assertion. The plural-form
  `shamaya` uses conventional English "third heaven." Verse 4's "who"
  continues the man in verse 3. Both inexpressible words and the prohibition
  on a person's uttering them remain; the English relative expresses the
  one object resumed by the final Syriac pronoun.
- Verse 5 has plural infirmities without an explicit "my" suffix, unlike
  verse 9. Verse 6 retains both causal particles, the conditional wish to
  boast, not being a fool, the truth cleft, and "I refrain." The last
  clauses concern another person's seeing in me and hearing from me;
  the first-person enclitic on refraining adds no object "him."
- Verse 7 preserves both prevention clauses, read with the speaker as
  their understood subject in the context of "to me," "my flesh," and
  oppression of me. The giving remains passive; thorn, flesh, messenger,
  Satan, and oppression remain explicit. Verse 8 has three requests to
  my Lord, and the requested departure is of it from me, not of the Lord.
- Verse 9 marks the quotation boundary: my grace and my power belong to
  the replying Lord, followed by Paul's glad boasting in my infirmities.
  "For you" and "therefore" remain explicit. The Messiah is the single
  owner of the power that may rest upon me. Verse 10 retains all five
  hardship categories, including imprisonment, the Messiah-sake face idiom,
  and causal "For when I am weak, then I am strong."
- Verse 11 has you compelling me, your past obligation to bear witness
  for me, inferiority in nothing to the greatly excelling apostles, and
  "even if I was nothing." Verse 12 retains plural apostles and all three
  with-phrases for prodigies, wonders, and mighty works, with all patience.
  Verse 13 preserves causal "For in what," the exception construction,
  other ecclesias, my not burdening you, and the direct forgiveness request.
- Verse 14 states third-time readiness to come, preserves the contrast
  between what is yours and you, and retains the children/parents obligation
  with plural treasures. Verse 15 keeps plural expenses, giving myself
  for your plural souls, both "even if" and "when," and the asymmetrical
  more/less love comparison. Verse 16 retains "perhaps" and the rhetorical
  craft/deceit accusation, followed by the agency questions in verses 17-18.
- Verse 17 renders the hand expression as "through" the person sent.
  Verse 18 has plural brothers, Titus's question including "in anything,"
  one spirit, and the same plural footsteps. Verse 19 renders `npq ruha`
  as the defense/apology idiom, following the supplied defend gloss and
  Murdock's contextual reading. The noun is not an added "in spirit"
  adjunct. Before God, in the Messiah, we speak; all these things, my
  beloved, are for your building up.
- Verses 20-21 retain both finding directions and their distinct persons,
  the actual negative predicates as well as the lest clauses, all eight
  contention/vice categories, my God humbling me, my mourning over many,
  and their prior sin and non-repentance. The final impurity, fornication,
  and lasciviousness have three distinct from-phrases and a shared committed
  predicate. Verse 20 has no inserted "when" before its coming clause;
  verse 21 has its own explicit when-clause.
- Only verse 18 uses a five-position group, for Titus's question with its
  delayed subject and indefinite quantity. All other linked groups contain
  at most four positions; no lexical text is left unlinked.

Source checks for 2 Corinthians 13:

- Verse 1 retains the source's third-time wording and renders the mouth expression as "on the testimony of two or three witnesses."
- Verse 2 renders the two temporal statements as Paul's prior warning and renewed advance warning; the source's absent/far-away writing and sinful/rest groups are retained.
- Verse 3 keeps the complementizer introducing the Messiah's speech through Paul and the repeated claim that he is not weak but strong among the Corinthians.
- Verse 4 groups the postpositive causal particle with its opening clause and preserves the contrast between crucifixion in weakness and life by God's power: Paul and his co-workers share weakness and life with the Messiah, by God's power among the Corinthians.
- Verse 5 keeps both examination commands, the explicit copula in "Jesus the Messiah is in you," and the conditional "if not" clause.
- Verse 7 preserves the causal request that no evil thing be among them, the purpose connector leading to the proof appearing, the proof belonging to Paul and his co-workers, and the contrast with their doing good.
- Verse 8 retains both the inability to act against truth and the positive action for truth.
- Verse 9 preserves the contrast between Paul's weakness and their strength, followed by the prayer for their completion/perfection.
- Verses 10–11 retain the absent-writing purpose, authority for edification rather than destruction, and the closing imperatives and God-of-love/peace blessing.
- Verses 12–14 retain the holy kiss, the holy ones as the subject who salute, and the three-part final benediction with Amen.

Source checks for Galatians 1:

- Verse 1 retains both negations concerning human origin and agency, and
  the explicit clause about raising Jesus from among the dead. Verses 2–5
  preserve the ecclesias in Galatia, the grace and peace greeting, the
  rescue purpose, and the concluding glory and Amen.
- Verse 6 explicitly names the Messiah as the one from whom the hearers
  are turning and retains the grace-call clause. Verse 7 preserves the
  nonexistent-other-gospel contrast and the separate troubling and
  changing predicates.
- Verses 8–9 retain the conditional proclamation, the angel from heaven,
  the received message, and both curse declarations.
- Verse 10 retains both questions and the conditional contrast between
  pleasing people and being the Messiah's servant. Verses 11–12 retain
  the negative human origin, the not-even emphasis on receiving,
  both receiving and learning with their object, and revelation of Jesus the Messiah.
- Verses 13–14 preserve God's ecclesia and the separate wasting action,
  the comparative progress in Judaism, contemporaries among his own
  people, and intensified zeal for the teaching of Paul's fathers.
- Verses 15–17 retain the setting apart from the mother's womb, calling
  by grace, revelation of the Son through Paul, the Gentile proclamation,
  and the connected denial of consulting flesh and blood or going up to
  the earlier apostles. The Jerusalem, Arabia, and Damascus destinations
  remain distinct.
- Verse 18 retains three years, Cephas, staying with him, and fifteen
  days. Verse 19 preserves the exception for James and the first-person
  plural possessive in "our Lord." Verse 20 retains the before-God
  affirmation and explicit denial of lying.
- Verse 22 renders the ecclesias of Judea as the subject of not knowing
  Paul personally. Verse 23 retains both former-time descriptions and
  the now/behold transition; verse 24 preserves the source's "in me."

Source checks for Galatians 2:

- Verses 1–2 retain fourteen years, Barnabas and Titus, revelation, the
  gospel among the Gentiles, and both running predicates in the negative
  purpose clause. Verse 3 preserves the absence of compulsion to circumcise.
- Verses 4–5 retain the false brothers, the group's freedom in Jesus
  Messiah, the enslaving purpose, and the refusal to yield even for an hour.
  Verse 6 preserves both the earlier status qualification and the face-taking
  idiom about God's impartiality, followed by adding nothing to Paul.
- Verses 7–8 preserve the distinct circumcision and uncircumcision
  commissions and the parallel predicates concerning Cephas and Paul.
  Verse 8 renders the actual D-stem verb as "encouraged," following
  [CAL's direct Syriac sense](https://cal.huc.edu/getlex.php?coord=620480208&word=2),
  rather than borrowing a different stem's sense.
- Verses 9–10 retain all three named leaders, Barnabas, the right hand of
  fellowship, the two mission domains, remembering the poor, and Paul's
  own concern to do so.
- Verses 11–12 preserve Cephas's arrival, the face-to-face opposition,
  the timing relative to the people from James, withdrawal and separation,
  and the causal fear of those of the circumcision.
- Verse 13 renders the face-taking expression as showing partiality to
  them, retaining its plural recipient. Compare the Syriac examples under
  [CAL's nsb entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=nsb+V).
  Verse 14 retains the full manner-of-living question and begins the speech
  that continues through verse 21.
- Verses 15–16 preserve the causal wording, Jews by nature and Gentile
  sinners, all law/faith contrasts, the repeated name phrase in "we have
  believed in Jesus Messiah," and the emphatic final "no flesh."
- Verses 17–19 retain the conditional sinner question and rejection,
  rebuilding what Paul destroyed, with its feminine plural object,
  the commandment-transgression
  description, and both law phrases with the purpose of living to God.
- Verse 20 retains "loved me" but ends with the distinct first-person
  plural "gave himself for us." These suffixes are not harmonized with
  one another or with the supplied Murdock wording. Verse 21 retains
  the negated rejection of grace, the conditional righteousness-through-law
  claim, and the final "for nothing."

Source checks for Galatians 3:

- Verse 1 renders the actual Peal verb as "has envied," retaining the
  source reading rather than substituting the familiar "bewitched." Compare
  [CAL's Peal sense](https://cal.huc.edu/oneentry.php?cits=all&lemma=xsm+V).
  The portrayal clause preserves initial "how" and later "when" separately.
- Verses 2 and 5 preserve the distinct source/receiving clauses and both
  alternatives, works of the law and hearing of faith. Verse 3 retains
  the completed beginning, present completion, explicit now, and flesh.
- Verse 4 preserves the source's optative as "Would that it were in vain!"
  without adding a first-person speaker or "only." Verse 8 retains both
  the foreknowing and advance-proclamation predicates and causal links;
  verse 10 retains its repeated causal wording and final "this law."
- Verse 12 preserves the feminine plural suffix in "by them." Verses
  13–14 retain the first-person plural participants, including "for us"
  and "we might receive."
- Verse 16 distinguishes third-person singular "his seed" from both
  second-person singular "your seeds" and "your seed." Verse 17 explicitly
  retains four hundred thirty years, with the covenant/promise argument.
- Verses 19–20 retain the mediator continuation and the statement that
  God is one. The opening "Now" in verse 20 renders a discourse transition.
- Verse 23 distinguishes the law guarding us from our being confined;
  the passive confinement does not become a second active act by the law.
  Verse 24 retains "our tutor" and "we might be justified."
- Verses 26–27 preserve the second-person plural references and the
  distinct baptism and putting-on predicates. Verse 28 retains the
  three negative contrasts and renders the social status as "freeborn."
- Verse 29 renders the anticipatory possessive and explicit genitive
  naturally as "Abraham's seed," without creating a second owner.

Source checks for Galatians 4:

- Verses 2–5 retain the appointed-time and fullness wording, the Son's
  sending, and the redemption and adoption clauses, with their distinct
  source predicates and participants.
- Verse 3 explicitly retains the enslavement predicate.
  Verse 8 retains the causal connector and a finite serving predicate.
  Verse 9 preserves both the turning and serving clauses, with the later
  `min dresh` rendered idiomatically as "anew." Verse 10 retains all four
  time categories: days, months, seasons, and years.
- Verse 12 keeps the appeal and the reassurance that the hearers have not
  wronged Paul in anything. Verses 13–14 retain the weakness of his flesh
  and both comparisons, an angel of God and Jesus the Messiah.
- Verse 16 preserves the rhetorical enemy question without adding
  "therefore." Verse 17 distinguishes the third-person plural "they want"
  from the later second-person purpose clause concerning the hearers' zeal.
  Verse 18 explicitly retains the addressed second-person subject.
- Verses 19–20 retain Paul's labouring and the Messiah's formation in
  the hearers, followed by the wish to be present, change of voice, and
  astonishment at them.
- Verse 21 retains the repeated law reference but renders the resumptive
  object naturally as "hear the law." Verses 22–23 preserve the two sons,
  the two one-phrases, and the flesh/promise contrast.
- Verse 24 preserves plural "allegories," the two covenants, Sinai,
  bondage, and Hagar. Verses 25–26 retain the present Jerusalem relation,
  her children, the upper Jerusalem's freedom, and "our mother."
- Verse 27 retains both joy commands, both negative descriptions, the
  causal clause, and the more-than comparison and its multiplication predicate. Verse 28 renders the
  repeated first-person pronouns as one natural grammatical subject and
  copula, with Isaac and the promise retained.
- Verses 29–31 preserve the flesh/Spirit contrast, the persecution object,
  "so also now," the quoted expulsion and inheritance exclusion, and the
  concluding bondwoman/free-woman contrast.

Source checks for Galatians 5:

- Verse 3 retains both the renewed testimony and discourse connector.
  Verse 2 groups the repeated first-person pronouns with their speech
  clause without duplicating the English subject. Verses 4–5 retain severance from the Messiah, the law/grace contrast,
  and the explicit first-person plural waiting predicate.
- Verse 6 retains both negative circumcision/uncircumcision clauses and
  faith perfected through love. Verse 7 preserves the passive persuasion
  construction; verse 8 renders the hearers' persuasion without adding
  an unsupported demonstrative.
- Verse 10 retains emphatic "I myself," "our Lord," the judgment suffix,
  and the "whoever he may be" qualification.
- Verse 11 renders the final clause as a rhetorical "Could it be that…?"
  rather than a purpose clause introduced by "lest." The offense belongs
  to the cross; the cross is not an appositional explanation of the offense.
- Verses 12–13 retain the wish and emphasis, the contrastive transitions,
  the freedom calling, and service to one another through love. Verses
  14–18 preserve the neighbor/self quotation, active reciprocal consumption
  warning, Spirit
  and flesh contrasts, negation, and final guidance conditional.
  Verse 14's "in this" retains the preposition on its source demonstrative.
- Verses 19–21 preserve every source vice category and the separate
  resembling and doing predicates. Verse 21 renders the actual music/
  singing term as "singing," distinct from the preceding drunkenness.
- Verse 22 renders `magrat ruha` as one construct-linked "long-suffering"
  idiom; its second word is not an additional fruit called "spirit."
  The verse ends with faithfulness; verse 23 begins with meekness and
  endurance before the law statement.
- Verse 24 preserves the flesh, its passions, and its desires with their
  possessive relationships; English neuter "its" does not add different
  personal actors for differing Syriac grammatical suffix genders.
- Verses 25–26 preserve both Spirit phrases, the first-person plural
  life/following predicates, the negative becoming clause, vain glory,
  and both reciprocal "one another" expressions.

Source checks for Galatians 6:

- Verse 1 retains the plural spiritual addressees, restoring the person,
  their caution, and their own possible temptation. Verse 2 keeps the
  causal/result link and identifies the law as the Messiah's.
- Verse 3 retains both being something and being nothing, followed by
  self-deception. Verses 4–5 preserve the self/other contrast in boasting
  and each person's own burden. Verse 4 retains plural "others."
- Verse 6 preserves the sharing relationship between the person receiving
  instruction and the person instructing, without duplicating the linked
  demonstrative and pronoun into separate recipients.
- Verse 7 retains "in anything" in the mocking statement and the sower/
  harvest relation, with the human idiom rendered as generic "a person." Verse 8 preserves both "from" complements and the
  corruption/eternal-life contrast with their respective participants.
- Verse 9 renders the final impersonal clause naturally as "and it will
  not weary us," retaining the first-person plural dative without changing
  the final clause to an additional "we" subject. The earlier exhortation
  "let us not grow weary" renders the ethical-dative construction naturally
  with the same affected participants.
- Verses 10–11 preserve the time available to us, the universal and
  faith-household recipients, both writing predicates, and "with my hand."
- Verses 12–14 preserve the circumcision/purpose contrasts, "your flesh,"
  both crucifixion relationships, and "our Lord Jesus the Messiah" with
  the cross as one genitive relationship.
- Verses 15–16 retain both negatives, new creation, the following rule,
  peace and mercy, and the Israel of God. Verse 17 explicitly retains
  "our Lord" in the marks clause, the body suffix, and the bearing predicate.
- Verse 18 preserves the grace blessing, "our Lord," the hearers' spirit,
  the "my brothers" address, and Amen.

Source checks for Ephesians 1:

- Verses 1–3 retain the repeated Messiah names and the possessives in
  "our Father" and "our Lord," together with the blessing and heavenly
  setting. The divine-will genitive is rendered without adding a second
  owner.
- Verses 4–6 preserve choosing beforehand, foreordaining, the purpose
  that we should be holy, and the passive praise predicate. The relative after grace
  introduces grace being poured upon us through the Beloved.
- Verse 7 retains redemption, the conjunction before the blood phrase,
  and forgiveness of sins. Verse 8 renders the relative as "which abounded
  in us," retaining the in-all-wisdom and understanding complements.
- Verses 9–12 keep making the mystery known, the previously determined
  action, renewal anew in the Messiah, heaven and earth, election, and first
  hope. Verse 10 explicitly retains the d-linked content/purpose clause,
  "that everything may be renewed anew." Compare
  [Younansardaroud, §§4.3, 5.5, and 14.6](https://d-nb.info/1184881278/34).
  The sentence joins across verses 9–10 and 11–12 remain continuous.
- Verse 13 preserves "your life," hearing, belief, passive sealing, and
  the Spirit that was promised. Verse 14 renders the redemption phrase
  as a contextual temporal goal, "until the redemption," with the living
  participants and the distinct praise/honor wording retained.
- Verses 15–17 preserve the faith/love report and thanksgiving as a
  continuing prayer, with "our Lord" and the knowledge-of-him complement.
  Verse 18 makes the eyes of the hearers' hearts the subject of the
  enlightenment clause rather than adding a second divine action verb.
- Verses 18–19 retain both what-is clauses, the calling/inheritance
  possessives, the holy ones, and the distinct power/strength nouns with
  the believing group, with the power "in us." Verse 20 preserves both causative predicates and
  renders the resurrection idiom as "from among the dead."
- Verses 21–23 retain the repeated above relation, the clause about the
  name that is named, the present/coming contrast, ecclesia, and the body/fullness/filling
  relations, with their actual source pronouns and participants.

Source checks for Ephesians 2:

- Verses 1–3 preserve the fronted second-person topic and subsequent
  first-person shift. Relative and resumptive locatives are grouped with
  their own clauses, with "this spirit," the flesh/thought possessives,
  and the emphatic comparison with the rest retained.
- Verses 4–7 retain God's mercy and love, making alive, rescue, raising,
  seating, and the purpose of showing his grace and kindness. The joins
  across verses 3–7 keep the main predicate sequence intelligible.
- Verse 8 explicitly maps "by his grace" and first-person plural
  "we have been rescued through faith," followed by second-person
  "not from you."
  Verses 8–10 preserve the negative source contrast, God's gift, works,
  the no-boasting purpose, creation, and the prepared-works walking clause.
- Verses 11–12 retain the former flesh/circumcision descriptions,
  exclusion from Israel's customs and the covenant of promise, and the distinct
  hope/God statements. Verse 13 renders the blood suffix and explicit
  genitive naturally as the single owner in "the Messiah's blood."
- Verses 14–16 preserve the two/one contrasts, the intervening barrier,
  the flesh/enmity and commandment/decree phrases, one new person,
  reconciliation with God in one body, and the cross. The legal
  possessive is rendered "its decrees," with the law as its referent. The killing of
  enmity remains a completed predicate.
- Verses 17–18 retain the separate far/near recipients, access through
  him, both groups in one Spirit, and the Father. The source's "both"
  belongs to its own grouped access/Spirit phrase.
- Verses 19–20 preserve both negatives, fellow-citizens with the holy
  ones, God's household, apostles and prophets, and Jesus Messiah as
  cornerstone. "Cornerstone" renders the idiom "head of the corner."
- Verses 21–22 retain the building's joining and growth, the holy temple,
  the Lord, and the final second-person participation in God's dwelling
  by the Spirit, with the subordinate clause joined to its antecedent.

Source checks for Ephesians 3:

- Verses 1–4 preserve Paul's prisoner statement, the second-person
  recipients, the conditional hearing of the grace given to him, revelation,
  writing briefly, reading, and understanding his knowledge of the mystery.
  Verse 4 groups the explicit ability predicate with reading and understanding.
- Verses 5–7 retain the negative other-generations statement, the present
  revelation to the holy apostles and prophets, the Spirit, and all three
  Gentile relationships: inheritance, body, and promise through the gospel.
- Verses 8–10 preserve the least-among-all comparison, unsearchable riches,
  with the appositive "who am the least," bringing the mystery to light, its concealment from the ages, God as
  creator, and the ecclesia's role in making his varied wisdom known.
- Verse 11 groups the relative clause with both prepared/accomplished
  predicates, so its shared English object also owns the Syriac resumptive
  suffix without a duplicated "it." "Our Lord" and the full Jesus Messiah
  name remain explicit. Verses 12–13 retain confidence, "his faith,"
  afflictions, the recipients' sake, and their glory.
- Verses 14–16 preserve the kneeling prayer, the Father relationship,
  "our Lord," every family's naming, both heaven and earth, his glory,
  the strengthening purpose, his Spirit, and the hearers' inner person.
- Verse 17 separately owns faith, the hearers' hearts, love, and the
  temporal clause about their root and foundation. Verse 18 retains
  comprehension with all the holy ones and all four dimensions.
- Verse 19 renders the anticipatory possessive and explicit Messiah
  genitive as one owner in "Messiah's love," retaining both knowledge
  and greatness and the filling with all God's fullness.
- Verses 20–21 preserve separate comparisons, "more than everything"
  and "even more than what we ask and think," with the first-person plural
  asking/thinking and "in us," his ecclesia, Jesus Messiah, all generations,
  both plural age terms, and Amen.

Source checks for Ephesians 4:

- Verses 1–3 retain the speaker's prisoner status, "our Lord," the
  recipients' calling, humility of mind, gentleness, patience, mutual
  endurance, love, and preserving the Spirit's unity in peace.
- Verses 4–6 preserve "be in one body and in one Spirit," the calling's
  one hope, the repeated one-Lord/faith/baptism/God sequence, and the
  final first-person plural "in us all."
- Verses 7–10 retain the grace/gift measure, ascent quotation, captivity,
  gifts to human beings, the descent question and exception, beforehand,
  lower regions, above all heavens, and completing all things.
- Verses 11–13 retain apostles, prophets, evangelists, shepherds, and
  teachers; all three purpose phrases; and faith, knowledge of God's Son,
  maturity, stature, and the Messiah's fullness. The dependent clauses
  continue across the verse boundaries.
- Verses 14–16 retain the two motion predicates, every wind, human
  teachings, their craftiness, plotting and leading astray. Verse 15 keeps
  the first-person plural growth predicate, with "all that is ours" as
  its complement. The body/joints/gift/member/growth/building sequence
  remains intact.
- Verses 17–21 preserve the former Gentile conduct, ignorance and heart
  blindness, lost hope, surrender to impurity and greed, the Messiah
  object, the conditional hearing/learning clause, and truth in Jesus.
- Verses 22–27 retain the old/new person contrast, ongoing corruption,
  renewal, creation, truth/neighbor reciprocity, anger without sin, the
  sunset prohibition, and the accuser. Appositional and conditional
  clauses are punctuated as complete running English.
- Verses 28–31 preserve work with one's hands, giving to the needy,
  hateful speech, edification and grace to hearers, the Holy Spirit of God,
  sealing for redemption, and every bitterness/anger/wrath/uproar/
  blasphemy/wickedness category.
- Verse 32 keeps the distinct kind/compassionate/forgiving predicates
  and the actual first-person plural in "God in Messiah forgave us."

  The Syriac adjective is rendered "kind" using the
  [CAL adjective entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=bsym+A),
  which explicitly lists the Syriac sense "kind, lenient."

Source checks for Ephesians 5:

- Verses 1–4 retain beloved children, the Messiah's love for us and
  self-delivery for us, offering, sacrifice, fragrance, the prohibited
  conduct and speech, and thanksgiving in place of these things.
- Verses 5–7 preserve the idol-worship clause, inheritance in the kingdom
  of Messiah and God, empty words, the causal explanation of God's wrath,
  disobedience, and the prohibition on partnership. The anticipatory
  possessive on kingdom is absorbed into the following explicit genitives.
- Verses 8–10 preserve the causal particle, former darkness and present
  light in our Lord, the instruction to walk as children of light,
  plural fruits, goodness, righteousness, truth, and discerning what
  pleases our Lord. The closing instruction in verse 8 is linked to its
  own source positions.
- Verses 11–14 retain fruitless works, reproof, secret actions, impersonal
  loathsomeness even to mention them, exposure and revelation by light,
  and the wake/rise/illumination quotation. Verse 12 introduces no
  first-person speaker; verse 13 preserves the finite exposure and
  revelation predicates and the final identification with light.
- Verses 15–17 retain walking worthily, foolish/wise people, the wise
  buying their opportunity, evil days, understanding, and God's will.
- Verses 18–24 preserve wine and excess, filling with the Spirit, psalms,
  praises and songs, singing in the hearts, thanksgiving for everyone,
  our Lord Jesus Messiah, and God the Father. Verse 21 says the Messiah's
  love. Verse 22 retains our Lord, and verse 23 the body's life-giver.
- Verses 25–27 keep the Messiah's love and self-delivery for his ecclesia,
  sanctifying and cleansing, water and word, establishing it for himself,
  glory, stain, wrinkle, resemblance, holiness, and freedom from blemish.
  The purpose clauses continue as a complete English sentence.
- Verses 28–30 retain loving one's wife as one's body and oneself,
  nourishing and caring, the Messiah's ecclesia, and membership in his
  body, flesh, and bones. The comparison in verse 29 starts at the actual
  comparison word, and verse 30 continues the causal sentence.
- Verses 31–33 preserve father and mother, clinging to the wife, the
  explicit two becoming one flesh, the great mystery, the Messiah and
  his ecclesia, each husband's love, and the wife's fear of her husband.

Source checks for Ephesians 6:

- Verses 1–4 retain obedience to fathers in our Lord, the first
  commandment with a promise, father and mother, well-being and long life,
  the prohibition on provoking children, and raising them in our Lord's
  instruction and teaching.
- Verses 5–8 preserve masters according to the flesh, fear and trembling,
  sincerity, eye-service and pleasing people, Messiah's servants, God's
  will, whole-self service in love, recompense, and servant/free status.
- Verse 9 retains the actual command to forgive the servants' wrongdoing,
  the masters' own master in heaven, and the face-acceptance idiom as
  partiality. This is not replaced with a familiar Greek-based reading.
- Verses 10–12 retain our Lord's strength and power, God's armor, the
  accuser's singular trick, and the source's second-person plural in
  "your struggle." The flesh/blood, principalities, rulers, dark-world
  possessors, and evil-spirit contrasts all remain, ending under heaven.
- Verses 13–17 preserve preparation and standing, the complete armor,
  truth, righteousness, gospel readiness, peace, faith, flaming darts,
  deliverance, and the Spirit's sword as God's word. "Evil one" is a
  substantive reading of the actual adjective; verse 13 adds no day noun.
- Verses 18–20 keep plural prayers and petitions, prayer in the Spirit,
  watchfulness, constant prayer, intercession, all the holy ones, and the
  speaker's request. Verse 19 retains "to me," the opening of his mouth,
  and boldly proclaiming the gospel's mystery. The eye/open expression
  belongs to the boldness idiom rather than the preceding mouth phrase;
  the [Syriac dictionary](https://www.syriacdictionary.net/index.cgi?langsel=en&word=%DC%A5%DC%9D%DC%A2%DC%90)
  lists the exact expression as "openly, boldly." Verse 20 preserves
  ambassadorship, chains, bold speech, and speaking as is right.
- Verses 21–22 retain Tychicus as the subject who informs the recipients,
  his beloved-brother and faithful-minister descriptions, our Lord,
  sending him for this purpose, knowledge of the speaker's circumstances,
  and encouragement of their hearts. The apposition is rendered as a
  complete English copular sentence, continuing into the relative clause.
- Verses 23–24 preserve "our brothers," peace, love with faith, God the
  Father, our Lord Jesus Messiah, all who love him, incorruption, and Amen.

Source checks for Philippians 1:

- Verses 1–2 retain Paul and Timothy, Jesus Messiah's servants, all the
  holy ones at Philippi, elders and deacons, grace and peace, God our
  Father, and our Lord Jesus Messiah.
- Verses 3–6 preserve thanksgiving, remembrance, all the speaker's
  prayers, joy, gospel fellowship from the first day until now, confidence,
  the good work begun in the recipients, and its completion through the
  day of our Lord Jesus Messiah. The English prayer and cause clauses
  form complete sentences.
- Verse 7 renders the compound with its first-person suffix as
  "my defense." Verse 16 uses the same compound as "defense," without
  adding "spiritual." The [CAL compound entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=mpq%40brwx%29+N)
  records its apology/defense sense. Verse 7 also preserves heart, bonds,
  the gospel's truth, and participation in grace.
- Verses 8–11 retain God's witness, the Messiah's mercies, increasing
  love, knowledge and spiritual understanding, discernment, purity,
  freedom from offense, righteous fruit, and praise and glory to God.
- Verses 12–18 retain the gospel's advance, bonds, the whole praetorium,
  everyone else, the brothers' confidence, and "all the more" boldness.
  Verse 15 makes Messiah the object of proclamation, as in verse 17;
  goodwill and love describe the contrasting motivation. The insincere
  proclamation, added affliction, pretense/truth contrast, and rejoicing
  all remain.
- Verses 19–23 keep the causal "For," the speaker's life, recipients'
  prayers, the Spirit's gift, hope and expectation, freedom from shame,
  open proclamation, magnification in the body, and life/death contrast.
  This life of the flesh, fruitful labor, and uncertainty about choosing
  for oneself remain;
  verse 23 has the explicit cardinal "two," departure, being with the
  Messiah, and benefit to the speaker.
- Verses 24–26 retain remaining in "my body," the matter/business on
  the recipients' behalf urging the speaker, abiding, their joy and
  faith's progress, another visit, and rejoicing in Jesus Messiah alone.
  Verse 24's postposed subject and predicate are grouped for natural
  English order; no comparative necessity is supplied.
- Verses 27–30 retain worthy conduct, coming/seeing and absence/hearing,
  the repeated one-spirit/one-soul/united-striving expressions, gospel
  faith, being in no way shaken, and those who oppose "us." The sign is linked to
  its own source word, followed by their destruction and the recipients'
  life. God's grant includes both belief and suffering for the Messiah,
  continuing into the conflict seen and now heard in the speaker.

Source checks for Philippians 2:

- Verses 1–4 retain all four conditional appeals, the explicit heart
  noun, encouragement, love, Spirit fellowship, affections and mercies,
  the speaker's joy, one mind/love/soul/thought, humility, the companion,
  and the contrast between one's own concerns and the other's.
- Verses 5–8 preserve the mind in the addressees, Jesus Messiah, God's
  likeness, the distinct equality assertion, robbery, self-emptying,
  servant and human likeness, being found human, self-humbling, and
  obedience. The two death nouns and the cross qualifier are linked to
  their own source positions.
- Verses 9–11 retain God's exaltation, giving him the name, every name,
  Jesus' name, every knee, heaven/earth/under-earth, every tongue, and
  glory to God his Father. "Gave him" belongs to its own verb and pronoun;
  the nominal confession reads "Jesus the Messiah is Lord."
- Verses 12–13 retain the beloved addressees, their obedience, the
  near/far contrast and its actual "only" particle, all the more, fear
  and trembling, and both the work command and work noun. God urges
  them to will and work; the final clause explicitly says what they
  desire, preserving its second-person plural subject.
- Verses 14–16 retain the prohibitions on murmuring and dispute,
  harmlessness, freedom from blemish, pure children of God, dwelling
  in a perverse and crooked generation, and appearing as lights.
  Verse 16 preserves the indicative "you are" relation and renders the
  in-place-of idiom as "take the place of life for them," followed by
  the speaker's boasting, the Messiah's day, and running/laboring not
  in vain. [CAL](https://cal.huc.edu/oneentry.php?cits=all&lemma=dwkh+N)
  explicitly records the Syriac in-place-of construction under section
  3(3); it is not treated as a physical location or replaced with a
  different source noun.
- Verses 17–18 preserve the conditional outpouring, sacrifice and
  service of the addressees' faith, both rejoicing predicates, all the
  recipients, and reciprocal joy with the speaker.
- Verses 19–24 retain our Lord Jesus, Timothy's sending, the speaker's
  relief, knowledge of the recipients, sincere care, the contrast with
  self-seeking people, proven character, son/father service, contingent
  travel, confidence, and the explicit "also" in coming to them.
- Verses 25–30 keep Epaphroditus, brother/helper/fellow-worker and the
  recipients' sent-one/minister roles, longing and distress, illness,
  both recipients of God's mercy, sorrow upon sorrow, eager sending,
  renewed joy and relief, reception and honor, the Messiah's work,
  approaching death, risking his life, and completing their service
  deficiency toward the speaker. The causal illness and work clauses
  remain connected as complete English sentences.

Source checks for Philippians 3:

- Verses 1–3 retain my brothers, our Lord, the repeated same-things
  construction, writing without weariness, the warning for the recipients,
  all three beware commands, mutilation of flesh, circumcision, service
  of God in spirit, glorying in Jesus Messiah, and no confidence in flesh.
- Verses 4–6 preserve the speaker's own flesh-confidence, the causal
  "for" and conditional comparison, eighth-day circumcision, Israel,
  Benjamin, Hebrew ancestry, law and Pharisee identity, zeal, persecution
  of the ecclesia, and blamelessness. The ancestry list continues the
  comparison sentence. Concessive bare kad is an attested construction;
  [Francois section 11.3(c)](https://markfrancois.wordpress.com/wp-content/uploads/2023/02/classical-syriac-grammar-week-11.pdf)
  illustrates it with John 4:2.
- Verses 7–9 retain the gain/loss reckoning, Jesus Messiah my Lord,
  surpassing knowledge, loss of everything, refuse, gaining the Messiah,
  being found in him, and the contrasted law/faith righteousness sources
  with righteousness from God.
- Verses 10–11 keep the explicit Jesus name, resurrection power,
  participation in sufferings, conformity to his death, and the
  conditional possibility of attaining resurrection. "From among the
  dead" uses the attested prepositional sense of beth, recorded by
  [CAL](https://cal.huc.edu/oneentry.php?cits=all&lemma=byt+p), rather
  than imposing a physical house on the expression.
- Verses 12–14 preserve not-yet attainment and perfection, running,
  taking hold, Jesus Messiah's taking hold of the speaker, what is behind
  and ahead, the mark, victory, God's calling from above, and Jesus
  Messiah. The fronted "one thing" remains with "I know" after the
  postpositive particle, and the forget/stretch/run sequence forms
  coherent English.
- Verses 15–17 retain maturity, the conditional different way of thinking,
  God's revelation to the recipients, what has been attained, both one
  path and one accord, imitation, observing those who walk thus, and
  the pattern seen in us.
- Verses 18–19 preserve the many walkers, repeated prior telling,
  present weeping, enemies of the Messiah's cross, destruction, belly,
  the contrasted glory/shame, and earthly thoughts with the actual
  possessive relations.
- Verses 20–21 render the source noun as "our own service," preserving
  heaven, expectation from there, the life-giver and our Lord Jesus
  Messiah. Transformation retains our humiliation-body, his glory-body,
  likeness, great power, and subjection of all things to himself.

Source checks for Philippians 4:

- Verse 1 retains the beloved brothers, the speaker's joy and crown,
  the explicit "thus" with standing firm, our Lord, and the closing
  beloved address, punctuated as a vocative.
- Verses 2–3 preserve Euodia and Syntyche, one mind in our Lord, the
  feminine-plural recipients, the true yokefellow's first-person
  possessive, help for the women who labored with the speaker, Clement,
  the other helpers, and names written in the book of life. Requesting
  from someone is rendered with natural English direct objects without
  changing the people or their roles.
- Verses 4–7 retain both rejoice commands, always and again, humility
  known to everyone, our Lord's nearness, the negative anxiety command,
  prayer/petition/thanksgiving, requests before God, and God's peace
  guarding hearts and minds in Jesus Messiah.
- Verses 8–9 retain every quality and the distinct praise/commendation
  deeds, thinking on these things, learning/receiving/hearing/seeing in
  the speaker, and doing them. Verse 7's peace of God remains distinct
  from verse 9's God of peace.
- Verses 10–12 preserve beginning to care, prior care and lack of
  means, the denial that need motivates the statement, learning, and
  sufficiency. Verse 11 maps its single speech predicate separately from
  "for I have learned," and retains whatever the speaker has as the
  subject of sufficiency. Verse 10 uses the verb's causative co-verb sense
  of beginning an action, recorded by [CAL](https://cal.huc.edu/oneentry.php?cits=all&lemma=nqp%232+V)
  under C(3). Verse 12 retains training, with humbled/abounding,
  fullness/hunger, and abundance/need.
- Verses 13–17 retain ability through the strengthening Messiah,
  partnership in affliction, Philippians, Macedonia, the gospel's
  beginning, the ecclesia contrast, the account of receiving and giving,
  Thessalonica, the explicit "once and twice," needed support, and the
  gift-versus-increasing-fruit contrast.
- Verses 18–20 preserve receipt, surplus, explicit fullness, the repeated
  reception clause, Epaphroditus, pleasant odor, acceptable sacrifice,
  and pleasing God. My God fills the recipients' need according to his
  riches, in Jesus Messiah's glory. The doxology retains our Father,
  glory and honor, eternity, and Amen.
- Verses 21–23 retain greetings to all the holy ones in Jesus Messiah,
  the brothers with the speaker greeting the recipients, all the holy
  ones, especially Caesar's household, and our Lord Jesus Messiah's
  grace with all of you, retaining the actual closing recipients.

Source checks for Colossians 1:

- Verses 1–4 retain Paul, Jesus Messiah, God's will, Timothy, the
  Colossae address, holy brothers and believers, peace and grace, God
  our Father, thanksgiving, prayer, faith, and love for the holy ones.
- Verses 5–8 retain the reserved heavenly hope, the word of truth and
  gospel, proclamation throughout the world, fruit and growth, knowledge
  of God's grace, Epaphras as our beloved companion, his faithful ministry
  for the recipients, and the report of their love in the Spirit.
- Verses 9–14 preserve the continuing prayer, knowledge of God's will,
  wisdom and spiritual understanding, worthy walking, pleasing God,
  fruitful work and growing knowledge, strengthening, and God's glory.
  Long-suffering is one compound, followed separately by "and with joy";
  [CAL](https://cal.huc.edu/oneentry.php?cits=all&lemma=rwx+N) records
  the patience sense of spirit with words of length/endurance. Giving
  thanks belongs to its own source verb, God the Father to its own name
  phrase, and the qualification object is "us." Darkness, deliverance,
  the beloved Son's kingdom, redemption, and forgiveness all remain.
- Verses 15–18 retain the invisible God's image, firstborn of all
  creation, both creation statements, heaven/earth, visible/invisible,
  all four alternatives of throne/dominion/principality/authority,
  his hand and "in him," being before all things, and all things standing
  in him. The head/body/ecclesia statement is distinct from the following
  copula, beginning, and firstborn from among the dead, with being first
  in all things.
- Verses 19–20 preserve volition, fullness and dwelling, reconciliation
  of all things to him, the distinct peace-making verb, the blood of his
  cross, repeated instrumentality, and earth/heaven alternatives. The
  peace-making predicate and each hand/instrument phrase stay with
  their own source positions.
- Verses 21–23 retain the recipients' former estrangement and enmity,
  evil deeds, present reconciliation, flesh/body/death, and presentation
  holy, without blemish and without accusation. The condition keeps
  remaining in faith, their steadfast foundation, not being moved away
  from gospel hope, what was heard and proclaimed under heaven, and
  Paul as its minister.
- Verses 24–27 preserve suffering for the recipients, the speaker's
  flesh, what is lacking in Messiah's afflictions for his body/ecclesia,
  ministry and God's administration, the word and mystery, hidden ages
  and generations, present revelation, the Gentiles, Messiah in the
  recipients, and the source's "our glory."
- Verses 28–29 retain plural proclamation/teaching/presentation and
  every-person objects, wisdom and maturity in Jesus Messiah, followed
  by the speaker's labor and striving. The two first-person forms stay
  with their respective predicates, and the given power's assistance
  remains explicit. Relative and conditional clauses are joined into
  complete English sentences.

Source checks for Colossians 2:

- Verses 1–3 retain the speaker's desire and struggle for the recipients,
  Laodicea, and the rest who have not seen his face in the flesh. The
  first-person dative belongs with having the struggle; face and flesh
  stay with their actual source positions. The following purpose keeps
  encouraged hearts, approach in love, riches of assurance, understanding
  of knowledge, the mystery of God the Father and Messiah, and the hidden
  treasures of wisdom and knowledge.
- Verses 4–5 retain the discourse connective, the warning against
  persuasive words, the causal/concessive opening, absence in the flesh,
  presence in spirit, rejoicing, observed order, and steadfast faith.
  The flesh phrase remains explicit even though the opening span groups
  the postpositive causal particle with its English clause.
- Verses 6–8 retain receiving Jesus Messiah our Lord, walking in him,
  strengthening the recipients' roots, being built and established,
  learned faith and thanksgiving. The root-strengthening verb is Pael
  active, with roots as its object, followed by the passive building
  verb. The warning preserves
  plundering through philosophy and empty deceit, human teaching, worldly
  elements, and the negative contrast with Messiah.
- Verses 9–12 preserve the relative link to Messiah, bodily fullness of
  divinity, completeness in him, all principalities and authorities,
  circumcision without hands, putting off the flesh of sins, Messiah's
  circumcision, burial in baptism, rising in him and with him, belief in
  God's power,
  and his raising him from among the dead. The conjunctions at the start
  of the circumcision and burial statements remain explicit. The masculine
  suffix in the rising clause refers to him, rather than the feminine
  baptism noun.
- Verses 13–15 preserve the recipients' death in sins and uncircumcision,
  being made alive with him, and forgiveness of all our sins. The
  first-person plural suffixes stay first person. The record of our debt
  is blotted out by his decrees, removed and fastened to his cross. The
  source's stripping off his body remains the means of exposing the
  principalities and authorities, followed by their shame openly in
  his own person; neither possessive is dropped.
- Verses 16–17 retain the prohibition on troubling the recipients about
  food, drink, distinctions of festivals, new moons and Sabbaths, followed
  by shadows of coming things and the body/Messiah contrast.
- Verses 18–19 retain wishing, humility of mind, condemnation and the
  separate second-person subjection-to-angel-worship predicate. The
  negative caution is supported by
  [CAL's lmh entry, sense 2](https://cal.huc.edu/oneentry.php?cits=all&lemma=lmh+c), which records
  its lest-with-imperfect use in Syriac. Prying into unseen things, vain
  inflation in a fleshly mind, failure to hold the head, the whole body,
  joints and members, construction and establishment, and God's growth
  all remain. The while-clause keeps the same third-person actor through
  the following failure to hold the head.
- Verses 20–23 retain the causal conditional, dying with Messiah from
  worldly elements, and the question about being judged as though living
  in the world. One seven-word source span allows the question's predicate
  and comparison to appear in natural English order without moving a
  predicate into a neighboring span. The three prohibitions are quoted
  together; the quotation closes before the explanation of perishing use
  and human commandments and teachings. The appearance of wisdom,
  displayed humility, fear of God, not sparing the body, and the final
  honorable/use-of-flesh contrast remain explicit.
- Verse 21 retains the three prohibitions under shared English negative
  scope. The last verb has a physical-contact use documented in
  [J. Payne Smith, pp. 351–352](https://tmcdaniel.palmerseminary.edu/Payne_Smith_1-626%2BTP.pdf#page=353);
  the contextual rendering "handle" follows this physical-contact use.

Source checks for Colossians 3:

- Verses 1–4 retain the conditional rising with Messiah, seeking and
  thinking about what is above, Messiah at God's right hand, the negative
  contrast with earth, death and hidden life, and the revealing of both
  Messiah and the recipients in glory. The postpositive causal particle
  belongs to "For you have died." Verse 4 retains the source's "our life."
- Verses 5–7 preserve the command to put earthly members to death and the
  full list of fornication, impurity, passions, evil desire and
  covetousness. The rendering "passions" is a contextual interpretation
  of kēʾbē within this vice list; its primary lexical senses are pain,
  grief and suffering, as recorded in
  [CAL](https://cal.huc.edu/oneentry.php?cits=all&lemma=k%29b+N).
  Worship of idols is the attested religious sense of the fear compound;
  [J. Payne Smith, p. 89](https://tmcdaniel.palmerseminary.edu/Payne_Smith_1-626%2BTP.pdf#page=90)
  records this compound as idolatry. The idol noun is plural. The causal
  statement preserves God's wrath and the children-of-disobedience
  idiom, followed by the recipients' former conduct and life in these
  things. The anticipatory suffix in children of disobedience does not
  introduce another possessor.
- Verses 8–11 retain the complete anger/wrath/malice/blasphemy/speech list,
  the reciprocal prohibition on lying, stripping off the old human with
  his practices, putting on the new one, renewal in knowledge and the
  Creator's image. The source's Jew/Aramaean, circumcision/uncircumcision,
  Greek/barbarian and slave/free contrasts all remain, followed by
  Messiah as all and in everyone. The relative clauses and list
  punctuation form complete running English.
- Verses 12–14 retain God's chosen, holy and beloved people, compassion,
  tenderness, kindness, humility of mind, gentleness and long-suffering.
  Long-suffering is one compound with spirit. The reciprocal bearing and
  forgiving clauses remain, followed by the complaint conditional and
  Messiah's forgiveness of the recipients as the model for their own
  forgiveness. Put on love supplies the garment imperative understood
  with verse 14's nominal love phrase; the bond of perfection remains.
- Verses 15–17 preserve Messiah's peace guiding their hearts, their
  calling in one body, thanksgiving to Messiah, his word dwelling richly,
  wisdom, reciprocal teaching and instruction, psalms, hymns and songs of
  the Spirit, grace, singing in their hearts and God as recipient. Singing
  is attached to its actual auxiliary and participle, followed by hearts
  in their own span. Word and deed, our Lord Jesus Messiah's name, doing
  and thanksgiving through him to God the Father all remain. The name
  phrase and its doing predicate are grouped to permit natural English
  order within their actual six source positions.
- Verses 18–21 retain wives, husbands, children and parents as the actual
  addressed groups, the rightness/pleasing clauses, the prohibitions on
  bitterness and provoking children, and the discouraged outcome avoided
  by the latter prohibition. Provoke, children and the lest-clause each
  remain attached to their own source positions.
- Verses 22–25 preserve servants, obedience in everything, bodily human
  mastery rendered as earthly masters, the eyeservice contrast, pleasing
  people, sincerity and fear of the Lord. Doing with the whole soul is
  directed to our Lord, with the explicit negative contrast with people.
  Receipt of recompense as an inheritance, service to Lord Messiah, the
  wrongdoer's corresponding recompense, and the negative face-taking
  idiom rendered as no partiality all remain.

Source checks for Colossians 4:

- Verses 1–4 retain masters, equality and justice toward their servants,
  knowledge of their own master in heaven, continuing prayer, watchfulness
  and thanksgiving. The request for prayer retains God opening a door for
  speech, speaking Messiah's mystery, the speaker's being bound because
  of it, and revealing and speaking it as he ought. The speaking infinitive
  stays with source positions 8–10 in verse 3.
- Verses 5–6 preserve wise conduct toward outsiders, redeeming the
  opportunity, speech always with grace and the salt-seasoning comparison.
  The following knowledge clause retains the each-person object and the
  give-an-answer expression in a single span where their English order
  differs from the source order. Its opening remains a coordinated
  exhortation to know.
- Verses 7–9 preserve Tychicus, the speaker's circumstances, beloved
  brother, faithful minister and our companion in the Lord. The discourse
  particle is explicit as Now. The apposition is made a natural nominal
  sentence with the same Tychicus subject. The sending purpose retains
  his learning the recipients' circumstances and comforting their hearts,
  followed by Onesimus as a faithful and beloved brother from among them.
  The final plural subject refers to both messengers making known what
  concerns the speakers.
- Verses 10–11 retain Aristarchus as a fellow captive, Mark as Barnabas's
  cousin, instructions about welcoming Mark if he comes, and Jesus called
  Justus. The son-of-an-uncle kinship compound stays together in its own
  span; the possessive anticipates the explicit Barnabas name. The Mark
  instructions are parenthetical so that the greeting list continues to
  Jesus. The circumcision group, their unique help in God's kingdom and
  their comfort to the speaker remain.
- Verses 12–14 preserve Epaphras from among the recipients, servant of
  Messiah, continual labor in prayer, their standing mature and complete
  in God's will, the speaker's testimony to his zeal, Laodicea and
  Hierapolis. Luke remains physician and our beloved, followed by Demas.
  The greetings put each subject and greeting predicate in one coherent
  span where natural English reverses the source's verb-first order.
- Verses 15–17 retain the Laodicean brothers, Nymphas and the ecclesia
  in his house; the source's house suffix is masculine singular. Both
  directions of letter-reading remain, including the Laodicean letter
  and the explicit recipients' own reading. Archippus is addressed about
  the ministry received in our Lord and its fulfillment.
- Verse 18 retains this greeting, the speaker's own hand, Paul as its
  named author, the remembrance of his bonds, grace with the recipients
  and Amen. The signature is rendered as a complete English sentence.

Source checks for 1 Thessalonians 1:

- Verse 1 preserves Paul, Silvanus and Timothy, the Thessalonian ecclesia,
  the relative phrase placing it in God the Father and our Lord Jesus
  Messiah, and grace with the recipients and peace. The named senders
  are punctuated as separate people.
- Verses 2–4 retain thanksgiving to God always for all the recipients,
  continual remembrance in prayer, remembrance before God the Father,
  the works of their faith, the toil of their love and the endurance of
  their hope in our Lord Jesus Messiah. Works is plural in the source.
  The causal knowledge statement retains their election and the speaker's
  "my brothers," beloved by God; the beloved suffix anticipates God as
  the one possessor.
- Verse 5 retains the preaching contrast: not in words alone, but to the
  recipients also in power, the Holy Spirit and true assurance. Words is
  plural. The two second-person subject forms are grouped as "you
  yourselves know," with both subject forms belonging to the knowing
  predicate. The speakers' conduct among the recipients for their sake
  remains explicit.
- Verses 6–8 preserve imitation of both the speakers and our Lord,
  reception of the word in great affliction and the Holy Spirit's joy,
  being a pattern for believers in Macedonia and Achaia, and the word of
  our Lord being heard from the recipients. The negative only/but contrast,
  every place, their faith in God, and the speakers' lack of need to say
  anything about them all remain. The first-person plural in the imitation
  clause stays with its actual source positions.
- Verses 9–10 preserve what others report about the speakers' coming to
  the recipients, their turning to God from worship of idols, service of
  the living and true God, and awaiting his Son from heaven. The phrase
  about coming groups the actual entry noun, copula, first-person dative
  and destination into natural English. The fear-of-idols compound uses
  its attested religious worship sense. Jesus remains the named Son whom
  God raised from among the dead and who delivers us from coming wrath.

Source checks for 1 Thessalonians 2:

- Verses 1–4 retain the recipients' knowledge of the speakers' entrance,
  suffering and mistreatment at Philippi, great conflict, boldness and
  our God, and speaking Messiah's gospel. The suffix on gospel anticipates
  the explicit Messiah owner; it does not make Messiah another person's
  possession. Error, impurity and deceit remain negated. God's approval,
  entrusting of the gospel, pleasing God and his examination of hearts
  all remain.
- Verses 5–7 preserve the denial of flattering speech, a pretext for
  greed and seeking human glory, God as witness, the recipients/others
  alternatives, the apostolic claim and possible burden, and gentleness
  illustrated by a nursing mother cherishing her children.
- Verses 8–10 retain love and longing as distinct predicates, giving the
  gospel and the speakers' own lives, the recipients' dearness to them,
  remembered labor and toil, manual work night and day, avoidance of a
  burden, gospel proclamation, purity, righteousness and blamelessness.
  The longing verb stays with its own source span, followed by the
  separate giving infinitive. The actual hnan forms are first-person
  plural, despite the base pronoun's singular dictionary gloss.
- Verses 11–12 retain entreating each recipient as a father his children,
  comforting their hearts and bearing testimony, joined to the purpose
  of walking worthily of God who calls them into his kingdom and glory.
  The fronted objects and father comparison are grouped with their
  entreating predicate to allow natural English order. The comfort idiom
  is documented in [CAL, mly, Peal sense 7](https://cal.huc.edu/oneentry.php?cits=all&lemma=mly+V).
- Verses 13–14 preserve continual thanksgiving, receiving God's word
  from the speakers, the human/divine contrast, its actual divine status,
  and its work in believers. The imitation comparison retains God's
  Judean ecclesias in Messiah Jesus, the recipients' suffering from their
  own countrymen, and the parallel suffering from Jews.
- Verses 15–16 retain killing our Lord Jesus Messiah and the prophets
  from among them, persecution of the speakers, not pleasing God,
  opposition to all people, hindrance of speech to Gentiles so that they
  may live, continuing sins and arriving wrath. The killing verb and
  our-Lord possessive stay in their actual source group; the prophets'
  from-among-them relative stays with the prophets. The ongoing negative
  pleasing and opposed predicates remain present in English.
- Verses 17–20 preserve bereavement for a short time, presence contrasted
  with heart, eagerness to see their faces with affection, the desired
  visit, Paul, once and twice, Satan's hindering me, hope, joy, the crown
  of boasting, our Lord Jesus and his coming, and the recipients as the
  speakers' glory and joy. [CAL's hpt Ethpaal entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=xp%2B+V)
  supports the eagerness sense of the actual derived stem. Verse 19 is one
  coherent rhetorical question with the if-not-you construction.
- The vocative suffixes are retained: my brothers in verses 1 and 14,
  our brothers in verses 9 and 17. The singular me in verse 18 remains
  distinct from the speakers' plural we.

Source checks for 1 Thessalonians 3:

- Verses 1–3 retain the inability to endure, willingness to remain alone
  in Athens and to send Timothy, our brother, God's servant and our
  helper in Messiah's gospel. The gospel suffix anticipates its Messiah
  owner. Strengthening the recipients and asking them about their faith
  remain separate predicates; the explicit you object of asking is kept.
  Discouragement by afflictions and the knowledge of being appointed to
  them remain, with the clauses joined into complete English.
- Verses 4–5 retain the former warning of suffering, its occurrence and
  the recipients' knowledge, followed by the speaker's first-person
  singular sending to learn their faith. Until stays with its actual
  source word. The tempter's possible tempting and the labor's possibly
  proving vain remain hypothetical. The labor, its copular verb and vain
  quality form one coherent span.
- Verses 6–7 preserve Timothy's return from the recipients, his good news
  about faith and love, their good remembrance of the speakers at all
  times, and reciprocal longing to see one another. The return/report
  clause leads to comfort by the recipients amid the speakers' distress
  and affliction because of their faith. The first-person plural our
  brothers is retained; the English does not imply physical co-presence.
- Verses 8–10 retain And now, living conditionally on the recipients'
  standing firm in our Lord, the causal thanksgiving question, repayment
  to God for them, all the joy because of them, and earnest prayer night
  and day to see their faces and complete what is lacking in their faith.
  Faces is plural in the source. The ellā plus d construction is rendered
  as except that, joining the prayer to the rhetorical question.
- Verses 11–13 preserve God our Father, our Lord Jesus Messiah, the
  speakers' way to the recipients, increasing and abounding love toward
  one another and everyone, the speakers' own love for them, establishing
  hearts blameless in holiness, and our Lord Jesus Messiah's coming with
  all his holy ones. God our Father is an apposition, with the our suffix
  on Father. Love remains its own predicate, rather than a general do.
  The increase/abound verbs retain their actual causative stems and the
  continuing divine subject; [CAL's sgy causative entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=sgy+V)
  supports making something increase. The coordinating establish clause
  continues the prayer instead of becoming an isolated fragment.

Source checks for 1 Thessalonians 4:

- Verses 1–2 retain the speaker's my brothers, both requesting and
  pleading, our Lord Jesus, what the recipients received from the speakers,
  walking and pleasing God, increasing further and the commandments given
  in our Lord Jesus. The postpositive causal particle starts the English
  For you know clause rather than becoming a preposition before its object.
- Verses 3–5 preserve God's will, sanctification, separation from sexual
  immorality, each person's knowing how to possess his vessel in holiness
  and honor, and the negative contrast with lustful passion and Gentiles
  who do not know God. The source's vessel metaphor and its possessive
  remain explicit. Verse 3's causal particle is rendered For.
- Verse 6 preserves the prohibition on anyone daring to overstep bounds
  or defraud his brother in this matter, our Lord as avenger concerning
  all these things, and the speakers' prior warning and testimony. The
  indefinite subject and governing verbs form one span for natural English
  order. As belongs to position 16; also beforehand and the saying verb
  stay with their own following words. The our-Lord suffix is retained.
- Verses 7–9 preserve God's call to holiness with impurity negated,
  rejection of a human contrasted with rejection of God, and his giving
  his holy Spirit in the recipients. The writing-need construction is
  rendered naturally with the recipients as the people being written to.
  The following you begins the causal God-taught clause, not the preceding
  to-you phrase. Teaching by God to love one another remains explicit.
- Verses 10–12 retain love for all the Macedonian brothers, the speaker's
  renewed appeal to my brothers to abound further, quiet conduct, one's
  own affairs, work with one's own hands, the previous command, proper
  conduct toward outsiders and independence from others' support.
- Verses 13–15 preserve the first-person singular desire that my brothers
  know about those who sleep, the instruction against sorrow like those
  without hope, belief in Jesus' death and resurrection, and God's bringing
  those sleeping in Jesus with him. The message is given by our Lord's
  word; the living remaining at his coming do not precede the sleepers.
  The coming suffix anticipates the explicit our-Lord owner.
- Verses 16–18 retain the causal descent statement, our Lord, command,
  the voice of the chief of angels, God's trumpet, heaven, the dead in
  Messiah rising first, the living remaining, being caught up together
  with them in clouds, meeting our Lord in the air, being always with him,
  and reciprocal comfort with these words. Descend is rendered at its
  actual position 9; from heaven is at positions 10–11. The source order
  is preserved across these distinct instrumental and predicate spans.

Source checks for 1 Thessalonians 5:

- Verses 1–3 retain times and seasons, my brothers, the impersonal lack
  of need for writing, emphatic knowledge, our Lord's day and the thief
  at night comparison. The late coming predicate is grouped with its
  actual comparison words for natural English order. Peace and tranquility
  are coordinated in the reported speech. Sudden destruction, labor pains
  on a pregnant woman and the negative escape clause remain. The destruction
  noun at position 9 stays with its verb and target, while the comparison
  begins at position 10. The min-shely suddenly expression is also attested
  in [CAL's qpl examples](https://cal.huc.edu/oneentry.php?cits=all&lemma=qpl+V).
- Verses 4–7 preserve the darkness contrast, that day overtaking like a
  thief, the repeated sons-of-light/day/night/darkness statements, and
  both negatives. The postposed you belongs to the sons-of-light clause.
  Sleeping contrasted with wakefulness and sobriety remains, followed by
  the separate nighttime sleeping and drunkenness clauses. Night at
  position 8 belongs to the second drunkenness predicate, not its subject.
- Verses 8–11 preserve the daytime identity, mental wakefulness, the
  breastplate of faith and love, the helmet of life's hope, God's not
  appointing the speakers for wrath, acquiring life through our Lord
  Jesus Messiah, his death for them, the waking/sleeping alternatives,
  living together with him and reciprocal encouragement and building.
  The causal appointment clause and the following death-relative are
  joined to the preceding exhortation in complete running English.
- Verses 12–15 retain my brothers, recognizing those laboring among the
  recipients, standing before them in our Lord and teaching them, esteem
  in love because of their work and peace with them. The renewed appeal
  preserves the plural offenders, encouragement of the faint-hearted,
  bearing the weak people's burden and patience toward everyone. The
  prohibition on repaying evil for evil retains no one among you, followed
  by pursuing good toward one another and everyone.
- Verses 16–22 preserve continual rejoicing and prayer, thanksgiving in
  everything as God's will for them in Jesus Messiah, the prohibitions
  on quenching the Spirit and despising prophecies, examination of
  everything, holding what is good and fleeing every evil thing.
- Verses 23–24 retain the God of peace, sanctifying all the recipients
  completely, their whole spirit, their soul and their body, keeping
  without blame, and our Lord Jesus Messiah's coming. The keeping verb
  at position 11 stays with the entities it governs in positions 7–10.
  The first-person possessives belong to our Lord. The past calling and
  future doing remain distinct: he has called them and he will do it.
- Verses 25–28 preserve my brothers, prayer for the speakers, greeting
  all our brothers with a holy kiss, the first-person adjuration by our
  Lord, reading this letter to all the holy brothers, the grace of our
  Lord Jesus Messiah with the recipients and Amen.

Source checks for 2 Thessalonians 1:

- Verse 3 retains both giving thanks and being obliged, the speaker's
  “my brothers,” and love as the subject of the second growth predicate.
  Verse 4 has one boasting predicate, with faith and endurance retained.
- Verses 6–8 retain the conditional “if,” followed by the promise to the
  afflicted. The Aphel life verb is “make alive,” rather than substituting
  relief from another textual tradition; compare the
  [Peshitta morphology for 1:7](https://dukhrana.com/peshitta/analyze_verse.php?font=Estrangelo+Edessa&size=125%25&source=&verse=2Thessalonians+1%3A7).
  The vengeance clause is temporal, with no extra coming predicate.
  Both negative groups remain, including those who do not acknowledge
  the good news of our Lord Jesus Messiah.
- Verses 9–10 retain eternal destruction, our Lord's presence, his power's
  glory, his holy ones and believers, and the testimony concerning the
  recipients being believed on that day. The testimony predicate is not
  assigned to the believers as its subject. The temporal clauses join
  across verse boundaries into complete running English.
- Verses 11–12 retain the reason for prayer, calling, good purpose, works
  of faith and power, our Lord's name being glorified in the recipients,
  their being glorified in him, and the grace of our God and our Lord
  Jesus Messiah. The name and its genitive owner form one natural span.

Source checks for 2 Thessalonians 2:

- Verses 1–2 retain the speaker's “my brothers,” the assembly toward
  Jesus, both being shaken and being troubled, and the day-of-our-Lord
  claim framed as reported speech, rather than the narrator's assertion.
- Verse 3 retains the general interrogative, followed by the conditional
  about rebellion coming first and the man of sin being revealed. The
  revelation verb belongs to the man, not rebellion. Compare the
  [CAL entry for lma](https://cal.huc.edu/oneentry.php?cits=all&lemma=lmh+c).
- Verses 4–8 retain the opponent's self-exaltation, sitting in God's
  temple and displaying himself as God, the former teaching, the one
  restraining, removal from the midst, and our Lord Jesus consuming and
  abolishing the lawless one. The start-plus-derived verb in verse 7 is
  rendered “beginning to be stirred up,” preserving its inciting sense;
  compare the [CAL entry for hpt](https://cal.huc.edu/oneentry.php?cits=all&lemma=xp%2B+V).
- Verses 9–12 retain Satan's working, power, signs and lying wonders,
  deception among those perishing, not receiving love of the truth,
  God's sending deception, belief in a lie, and judgment of those who
  preferred unrighteousness. The coordinated clauses join across verses.
- Verse 13 has “our brothers,” distinct from “my brothers” in verses 1
  and 15. God's choosing, life, sanctification and faith remain. Verse
  14 retains plural “these things” and becoming our Lord's glory.
- Verses 15–17 retain commands, word and letter, our Lord Jesus Messiah
  and God our Father, loving and giving, eternal encouragement, good
  hope through his grace, hearts, and every word and every good deed.

Source checks for 2 Thessalonians 3:

- Verses 1–2 retain “our brothers,” the word of our Lord running and
  being glorified, deliverance from wicked people, and the causal
  statement that not everyone has faith. The prayer joins across verses.
- Verse 3 says that the Lord will guard the recipients, rather than
  substituting strengthening. Verse 4 retains confidence in our Lord,
  what the speakers command, and both completed and ongoing obedience.
  Verse 5 retains our Lord, God's love and Messiah's patient endurance.
- Verse 6 retains “my brothers,” our Lord Jesus Messiah, withdrawal
  from disorderly conduct, and the commandment that “they received”
  from the speakers, matching the third-person plural source verb.
- Verses 7–10 retain imitation, working with toil night and day, avoiding
  a burden, the right to support, setting an example, and the condition
  that anyone unwilling to work should not eat. The explanation about
  authority and example continues the preceding work statement.
- Verse 11 retains the explicit empty or vain things, rather than
  replacing them with meddling. Verse 12 retains working quietly and
  eating one's own bread. Verse 13 has “my brothers”; verse 14 has
  plural “our words,” marking, avoiding association and becoming ashamed.
- Verse 16 is a prayer. Its Lord-of-peace construction has an anticipatory
  third-person possessive, while the following “Our Lord” has the genuine
  first-person plural suffix. Verses 17–18 retain Paul's handwritten
  greeting and sign, how he writes, grace with everyone, and Amen.

Source checks for 1 Timothy 1:

- Verses 1–2 retain God as our life-giver, Messiah Jesus as our hope,
  Timothy as the speaker's true son, our Father and our Lord.
- Verses 3–4 retain the first-person request, remaining in Ephesus,
  Macedonia, the prohibition on different teaching, fables and endless
  genealogies, and the intensifier in the contention/edification contrast.
  The coordinated instructions join across the verse boundary.
- Verses 5–8 retain love from a pure heart, good conscience and sincere
  faith, straying from these things, empty words, the causal explanation
  about would-be law teachers, both negatives, and the condition of
  conducting oneself lawfully in the law.
- Verses 9–11 keep the righteous and the contrastive “but” on their actual
  source positions. The lists retain striking parents, lying with males,
  kidnapping free people, lying and violating oaths. Sound teaching joins
  to the gospel of the glory of the blessed God; the glory suffix anticipates
  that God owner and does not require an additional independent possessor.
- Verses 12–14 preserve thanks, our Lord strengthening the speaker,
  counting him faithful and appointing him to service, his former conduct,
  mercy, ignorance and unbelief, and grace increasing in him with faith
  and love. The grace statement has one predicate and one grace noun.
- Verses 15–17 retain the trustworthy saying, acceptance, Jesus Messiah
  at their own positions, giving life to sinners, the speaker's emphatic
  first-person statement, all patience, believers and eternal life.
  The doxology retains the explicit Amen.
- Verses 18–20 retain former prophecies and plural “in them,” faith and
  good conscience, rejecting this from themselves and becoming devoid of
  faith, rather than importing a nautical image. Hymenaeus, Alexander,
  handing over to Satan and learning not to blaspheme remain explicit.

Source checks for 1 Timothy 2:

- Verses 1–2 retain the first-person request, all four prayer nouns,
  prayer for kings and rulers, and the speakers' first-person plural
  living calmly and quietly in reverence for God and purity.
- Verses 3–5 retain God as our life-giver, the desire that all people
  live and turn to knowledge of truth, both actual “one” quantifiers,
  the mediator and human-being language, and Jesus Messiah.
- Verses 6–7 retain giving himself as a ransom, everyone, the testimony
  that came at its time, the appointment, herald and apostle, speaking
  truth without lying, and teaching the Gentiles in faith and truth.
- Verse 8 has men praying, with third-person plural “their hands,”
  distinct from the speakers' plural in verse 2. Anger and the second
  thought/reasoning prohibition remain separate negatives.
- Verses 9–10 retain modest clothing as well as modesty and self-control,
  the finite adornment predicate, the entire prohibited adornment list,
  and the contrasting “with good works, as is fitting” construction.
  The instruction joins across the verse boundary.
- Verses 11–12 retain learning quietly with submission, both prohibited
  infinitives, and the following quietness clause. The Aphel infinitive
  is rendered “act boldly”; compare the [CAL entry for mrh](https://cal.huc.edu/oneentry.php?cits=all&lemma=mrx+V).
- Verses 13–15 retain Adam's formation before Eve, both deception clauses,
  transgression concerning the commandment, the woman's life statement,
  “her children,” and the plural conditional “they remain,” with faith,
  love, holiness and modesty. The plural offspring reading follows the
  actual form; compare the [CAL child/offspring entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=yld+N).

Source checks for 1 Timothy 3:

- Verses 1–3 retain the conditional desire for an elder's office, good
  work, no blemish, one wife, watchful mind, sobriety, order, hospitality
  and teaching. The wine, swift striking hand, humility, contentiousness
  and money qualifications remain distinct.
- Verses 4–7 retain the elder's own house and children, the repeated
  guiding predicate, God's ecclesia, the recent disciple, passive
  exaltation and Satan's judgment. The testimony clause keeps having a
  good testimony and its source among outsiders in their own spans.
- Verses 8–10 retain plural deacons, pure conduct, the idiomatic
  “not double-tongued,” wine and gain prohibitions, holding the faith's
  mystery with a pure conscience, testing beforehand and then serving.
  “But” introduces the positive requirement after the prohibitions;
  compare the [CAL entry for ella](https://cal.huc.edu/oneentry.php?cits=all&lemma=%29l%29+c).
- Verse 11 has plural women and feminine-plural “their minds,” followed
  by faithfulness in everything and the separate slander prohibition.
  Verse 12 returns to a singular deacon; verse 13 retains procuring a
  good rank for themselves and boldness in the faith of Jesus Messiah.
- Verses 14–15 retain the speaker's writing, hope, coming and possible
  delay, the addressed person's conduct, God's house, the living God's
  ecclesia and the pillar/foundation description of truth.
- Verse 16 retains the mystery of this righteousness and all six linked
  passive events: revealed in flesh, justified in spirit, seen by angels,
  proclaimed among nations, believed in the world and taken up in glory.

Source checks for 1 Timothy 4:

- Verses 1–3 retain the Spirit's explicit speech, plural people departing
  and following errant spirits, plural teachings of demons, deceptive
  speech, seared consciences, the marriage and food prohibitions, and
  foods created for use and thanksgiving for believers who know truth.
  The description continues across verses 2–3.
- Verses 4–5 retain every created thing being good, the negative rejection
  clause, the thanksgiving condition, and sanctification by God's word
  and prayer. Both explanatory connectors remain in complete English.
- Verse 6 has “your brothers,” explicit “you” being nourished, plural
  “words of faith,” and the distinct singular good teaching learned.
  Verses 7–9 retain training, the bodily-exercise contrast, the promise
  of present and future life, and the saying's trustworthiness/acceptance.
- Verse 10 preserves the speakers' first-person plural labor, reproach
  and trust, the living God as life-giver of all people, and “especially”
  concerning believers. Verses 11–12 retain teaching, commanding, the
  youth prohibition, and word, conduct, love, faith and purity.
- Verse 13 specifically has reading, prayer and teaching. Verse 14
  retains the gift in the recipient, its being given to him, prophecy,
  and the singular hand-laying expression with eldership.
- Verse 15 expresses becoming known to everyone that the recipient is
  advancing. The directional phrase belongs to the progress idiom;
  compare the [CAL entry for lqdm](https://cal.huc.edu/oneentry.php?cits=all&lemma=lqdm+p).
- Verse 16 retains watching oneself and one's teaching, persevering,
  doing these things, and the recipient saving himself and those who
  hear him, with the source's explicit second-person address.

Source checks for 1 Timothy 5:

- Verses 1–4 retain the elder/younger comparisons, possessive brothers
  and sisters, purity, actual widows, sons or grandsons, and doing right
  among the members of their household. The household-member construction
  keeps the household's owner and its members in their source roles.
- Verses 5–8 retain the solitary widow's hope and prayers, the living/dead
  contrast, blamelessness, caring for one's own and especially the household
  of faith, and that person's denial of faith. Both source faith words
  remain. The long conditional binds the care verb to its actual objects
  in one grammatical span so that the English stays natural.
- Verses 9–12 retain sixty years, one husband, each good-work condition,
  the younger-widow instruction, Messiah, marriage and former faith.
  The derived verb in verse 11 is rendered “avoid”; Lesson 12 of
  [Younansardaroud's Syriac course book](https://refubium.fu-berlin.de/bitstream/handle/fub188/24490/Classical_Syriac_Course_Book_3-2019.pdf?isAllowed=y&sequence=3)
  gives the form as “decline,” and [Peshitta Titus 3:9](https://www.dukhrana.com/peshitta/analyze_verse.php?font=Estrangelo+Edessa&size=125%25&source=&verse=Titus+3%3A9)
  supplies the same derived verb with the from-preposition in an avoidance
  instruction.
- Verses 13–16 retain movement among houses, idle speech, marriage and
  household care, having begun to turn aside after Satan, the believing
  man/woman distinction, supporting widows and sparing the ecclesia's burden.
- Verses 17–22 retain the elders' double honor, both Scripture quotations,
  two or three witnesses, public rebuke, our Lord Jesus Messiah and his
  chosen angels, the mental-priority clause, partiality, hand-laying,
  other people's sins and personal purity.
- Verses 23–25 retain the from-now-on instruction, a little wine, both
  separate reasons, the recipient's stomach and continuing ailments,
  sins going before or following people to judgment, and the concluding
  visibility and concealment of good works.

Source checks for 1 Timothy 6:

- Verses 1–2 retain the yoke, their masters, God's name and his teaching,
  believing masters as brothers, and the benefit of service. The final
  teaching/exhortation command remains a separate sentence.
- Verses 3–6 retain approaching the sound words of our Lord Jesus Messiah,
  the fear-of-God teaching, arrogance and ignorance, disputes and their
  consequences, corrupted minds, deprivation of truth and supposed gain.
  The withdrawal command remains explicit. Verse 6 keeps the causal “For,”
  the speakers' gain, fear of God, use and their sufficiency.
- Verses 7–10 retain bringing nothing and carrying nothing away, sufficient
  food/clothing, the dangers of seeking riches, the root of these evils,
  love of money, straying from faith and the reflexive misery statement.
- Verses 11–12 retain “you, man of God,” fleeing from these things, all six
  pursuit objects, the contest, eternal life, calling, good confession and
  many witnesses. Verse 19 separately has true/steadfast life.
- Verses 13–16 form one charge and its linked clauses. God gives life to
  all, Jesus Messiah's testimony is before Pontius Pilate, and the command
  is kept until our Lord's manifestation. God reveals him in his time;
  blessing, unique might, kingship/lordship, deathlessness, inaccessible
  light, inability to see him, honor, power and Amen remain explicit.
- Verse 17 keeps the rich of this age and the negated exaltation in their
  minds in their actual source spans. The provision clause has all things
  as its object and “for our rest” as its purpose, with the first-person
  plural suffix retained. Verses 18–19 retain good deeds, sharing, a
  foundation for what is to come and attaining true life.
- Verses 20–21 retain Timothy's entrusted charge, empty speech, opposing
  principles of false knowledge, the feminine singular reference to that
  knowledge, straying from faith, the final addressed “you,” grace and Amen.

Source checks for 2 Timothy 1:

- 1:5 retains the source’s remembrance construction and second-person possessive: “the remembrance I have of your genuine faith.” The maternal sequence identifies Lois as Timothy’s grandmother; the running English makes that relation natural while preserving source order.
- 1:12 retains `באידוהי` as “in his hands,” between the ability predicate and “my deposit,” and retains the first-person object in “for me.”
- 1:13 renders the sound words as the subject and “a pattern” as the predicate: “Let the sound words which you heard from me be a pattern for you.”
- 1:15 renders `דאיתיהונ מנהונ` as the source-owned existential/partitive clause: “and among them are Phygelus and Hermogenes.”
- 1:17 retains the emphatic `אפ` in “But even when he came to Rome”; 1:18 renders `לות מרנ` as “with our Lord,” preserving the source relation.
- 1:10 renders `דמחיננ` as “life-giver” and `מתחבלנותא` as “incorruptibility”; ecclesia terminology is not present in this chapter.

Source checks for 2 Timothy 2:

- Verse 2 keeps the two relative clauses distinct: the things heard from Paul
  are committed to believing men, who are able to teach others.
- Verses 3–6 retain the service, lawful competition and ploughman imagery;
  verses 11–13 retain the repeated conditions and personal suffixes.
- The ethpeal form of שאל in verses 16 and 23 means “shun/avoid” in this
  context, as in the source checks for Acts 20. The exported lemma's “ask”
  gloss does not establish the sense of this derived form.
- Verse 17 keeps the copula and connective with the identification of
  Hymenaeus. Verse 20 keeps gold, only, or and silver together in their
  source-owned spans, followed by the wood and pottery contrast.
- Verse 25 retains the disputants, humility, God giving repentance and
  knowing truth in their actual clauses. The possibility particle is
  “perhaps”; “lest” would reverse the sense of the positive possibility.
- Verse 26 keeps the snare and Satan relation distinct from the locative
  clause, “in which,” and retains capture and the final will phrase.

Source checks for 2 Timothy 3:

- 3:2 keeps the negation/persuasion sequence separate from the following denier-of-grace phrase.
- 3:6 renders the participle as creeping house to house, captivating women buried in sins and led by diverse lusts.
- 3:7 retains always learning but never being able to come to knowledge of the truth.
- 3:9 retains the advance/foolishness/known comparison and source connective sequence.
- 3:14 renders the knowing-that clause and the from-whom learned clause.
- 3:15 preserves childhood, holy Scriptures, learning, wisdom, salvation, faith, and Jesus Messiah roles.
- 3:16-17 preserves Spirit-written Scripture's instructional uses and the man of God as perfect and complete for every good work.

Source checks for 2 Timothy 4:

- Preserved the repeated first-person forms in verse 6 and rendered the clause as an imminent pouring-out, rather than silently dropping either source token.
- Kept the causal/contrast particles and the full hearing clause in verses 3-4, including the itching of their hearing and the turn toward fables.
- Rendered verse 8's preserved crown, repayment by the Lord, righteous Judge, and beneficiaries in one grammatical sentence while retaining each source position.
- Kept personal suffixes explicit in the appeals to Timothy (come to me, bring him with you, beware of him) and in the greetings.
- Used “evangelist” and “ministry” in verse 5 and “Lord,” “Messiah,” and “kingdom” terminology consistently with the existing Peshitta translation.

Source checks for Titus 1:

- v1 renders `בהימנותא דגביא דאלהא` as “in the faith of God's chosen,” retaining the preposition, chosen plural, and genitive relation.
- v2–v5 retain the finite person/number distinctions, attached possessive suffixes, and relational d-prefixes, including “his word,” “its season,” “our proclamation,” and Titus-directed actions.
- v6 renders `אינא דדלא רשינ` idiomatically as “Who is blameless”; v7 uses “elder” for `קשישא`, not an imported “overseer.”
- v7–v8 preserve each prohibition, the hand/strike idiom, gain phrase, contrast, and moral qualifications; v15–v16 retain negation, plural suffixes, denial, and disqualification.
- v12 and v14 preserve source order and conjunctions through coherent grouped spans: “are always liars” and “to Jewish myths and to human commandments.”

Source checks for Titus 2:

- The chapter repeatedly uses finite forms of the Syriac verb “be” (הוא) as copulas with plural subjects (vv. 2–6); these are rendered as English infinitival or coordinated clauses without dropping the predicates. In v. 8, the source adjective is “sober,” distinct from the following “not corrupted.”
- In vv. 4–5, the source explicitly makes the young women modest and then directs them to love their husbands and children; the feminine-plural group and possessive suffixes on husbands, children, households, and husbands are retained as “their.”
- Verse 6 retains the opening conjunction in “And as for young men,” while v. 7 renders דינ as “however” (but/yet), rather than introducing an unsupported temporal “then.”
- Verse 8 contains two separate negatives before “corrupted” and “despise,” followed by a relative clause about the person standing against “us”; the translation keeps both prohibitions and the first-person plural object.
- In vv. 9–10, servants are told to obey masters, please them, avoid argumentative contradiction, and not steal; their displayed faithfulness is the good quality that is good in everything and adorns God's teaching.
- Verse 12 distinguishes rejecting ungodliness and worldly desires from the three positive ways of life: sensible conduct, uprightness, and fear of God in the present world.
- Verses 13–14 preserve the linked sequence “blessed hope,” revelation of the great God's glory and our Life-giver Jesus the Messiah, followed by his self-giving, redemption, purification of us, and a new zealous people.

Source checks for Titus 3:

- Verse 1 retains the two governing groups (rulers and authorities) and distinguishes being subject from obeying, while preserving the readiness clause.
- Verse 3 keeps the first-person plural subject throughout the sequence, including the paired negatives, the former condition, and the reciprocal hatred at the close.
- Verses 4–7 preserve the causal progression from God's kindness and compassion, through mercy, new birth, renewal, and the Spirit poured out through Jesus the Messiah, to justification and heirship in hope of eternal life.
- Verse 8 separates “and in these things” from the first-person desire verb, then preserves the relation between those who have believed in God and diligence in good works, with the concluding assessment that these works are good and beneficial to people.
- Verse 9 keeps the repeated avoidance/prepositional particles and assigns the final conjunction and vanity clause to the correct source positions; verse 10 likewise retains the source's “from” construction and once-and-twice admonition sequence.
- Verses 12–13 retain the dative “to you” before Artemas, the contrastive “but” before “the scribe,” and the complete “something/no/lack/to them” sequence as natural “lack nothing.”
- Verse 15 preserves the third-person plural subject “all who are with me” as the ones who greet you, followed by the imperative “greet with peace”; the four closing sentences retain both relative groups and the grace benediction.
- Verse 14 preserves both negative elements in “may not be without fruit,” rather than collapsing the source's two negative markers into one.

Source checks for Philemon 1:

- The opening preserves the distinct people and roles: Paul is a prisoner of Jesus the Messiah, Timothy is a brother, Philemon is beloved, and the servant designation carries its “with us” suffix.
- Verse 5 keeps both directional links: Philemon’s faith and love are toward the Lord Jesus and toward all the holy ones; the plural holy-ones form is not reduced to a singular recipient.
- Verse 6 retains the fellowship/faith subject and the source’s fruit-in-works expression before the knowledge of every good thing in Jesus the Messiah.
- Verses 8–9 preserve the demonstrative “because of this,” the source’s “boldness is mine” construction, and the sequence “I, Paul, who am aged”; the appeal’s “from you” relation is retained within the appeal span.
- Verses 11–14 preserve the former useless/present useful contrast, the “thus receive him” connective, and Paul’s refusal to act without counsel so that goodness is not by compulsion but by will.
- Verse 13 keeps the “to/with me” relation with keeping Onesimus and the source’s “that he might be” serving me instead of Philemon; verse 18 retains the indefinite “in anything” object.
- Verse 15 renders the idiom as Onesimus departing for an hour so that Philemon might receive him forever; “from the age” is not treated as a literal geographic departure.
- Verse 22 renders the opening בחדא דינ idiomatically as “And herewith,” then retains the lodging request, hope, prayer basis, and passive return to Philemon.

Source checks for Hebrews 1:

- Verses 1–2 retain the twofold “portions/forms” and “last days” contrasts, with the Son as the appointed heir and the agent through whom the worlds were made.
- Verse 3 keeps the copular/explanatory opening, the Son’s own person, the cleansing of sins, and his sitting at the right hand on high; the possessive suffixes are rendered rather than omitted.
- Verses 4–8 preserve the explicit contrast between the Son and angels, including the Son’s inherited superior name, the command for all God’s angels to worship him, and the direct address “O God.”
- Verse 9 retains the causal force (“because”) and the repeated “God, your God” construction before the anointing and comparison with companions.
- Verses 10–12 preserve the second-person subjects, the fold-up/changed contrast, and the negated completion of the Son’s years.
- Verses 13–14 retain the angel question, the “from among” phrase, the footstool image, and the description of angels as ministering spirits sent for those prepared to inherit life.

Source checks for Hebrews 2:

- 2:1 keeps מטל + הנא as “Therefore,” חיביננ as “we must,” and the explicit negative דלא before “fall”; the warning is “we must be exceedingly careful ... so that we do not fall away.”
- 2:2 retains the opening conditional אנ ... גיר as “If indeed,” rather than flattening it to a causal “for”; the confirming verb and the final retribution/justice phrase remain linked to their actual source spans.
- 2:3 renders the complete source clause naturally as “we despise these things which are our life. These things began to be spoken by our Lord and were confirmed among us by those who heard from him.” This keeps מרנ “our Lord,” מנ “from,” and בנ “among us” owned by the final source spans.
- 2:4 follows the source's actual subject order by rendering סהד ... עליהון ... אלהא as “God testified about them,” and keeps חילא as “mighty works” and פולגא as “distributions.”
- 2:6 keeps דמסהד + כתבא as “the Scripture testifies,” matching the actual verb-then-subject source order.
- 2:8 distinguishes the negative-subjection phrase from the contrastive present: “nothing that was not subjected. But now we do not yet see ...”; השא is “now” and דינ is “but/yet.”
- 2:9 identifies the lowered one as “this one ... Jesus” (הו ... ישוע), then gives “because of the suffering of death”; its final clause is “on behalf of everyone ... tasted death,” with God's grace kept explicit.
- 2:10-11 preserve the distinct roles and suffixes: many sons are brought to glory, the prince of their life is perfected through suffering, the sanctifier and sanctified are from one, and 2:11's מטל + הנא remains “Therefore.”
- 2:14 and 2:17 retain actual source ownership for the participation/likeness clauses: “also he in the same likeness ...,” and “in everything he should be like his brothers.” The active Pael participle מחסא is rendered “might make expiation,” not as a noun “atonement.” 2:16 follows the passive syntax: מותא is explicit “death,” and both משלט predicates mean “authorized,” not “assumed a nature.”

Source checks for Hebrews 3:

- In 3:1, אחי is plural with a first-person singular suffix, so “my holy brothers” preserves both the plural addressees and possessive suffix; חזאוהי is an imperative “consider/see him” with a masculine singular object suffix.
- In 3:3–4, טב is comparative in the glory/building comparison (“greater”), while גיר supplies “for”; the explanatory clause identifies God as the one who builds everything.
- In 3:5 and 3:10, the possessive suffix in בכלה is rendered “his house,” and מאנת ... לי is rendered as the first-person “I was weary of this generation,” rather than reversing the Syriac subject and object.
- In 3:6, the house statement is rendered “his house are we,” retaining the copular verb and the explicit first-person plural pronoun; the conditional span keeps “if we hold until the end” with the perseverance object.
- In 3:7–10, the causal particle and speaking Spirit are retained, and the fathers’ testing, seeing the works, forty years, generation, wandering hearts, and lack of knowledge are all separately represented.
- In 3:12–15, דלמא is the negative-purpose “lest,” not a positive “perhaps”; the warnings retain the evil unbelieving heart, departure from the living God, deception of sin, and the quoted “today” exhortation.
- In 3:17–19, the questions preserve the contrast between those whose bones fell in the wilderness and those who did not believe; the final causal clause explicitly links inability to enter with unbelief.

Source checks for Hebrews 4:

- The chapter repeatedly uses ניחתא for the promised divine “rest”; the translation keeps “rest” in the promise, oath, Sabbath-rest, and exhortation contexts, while משבתו in verse 9 is rendered “Sabbath-rest” because the verse explicitly makes the Sabbath observance/rest remain for God’s people.
- Verse 3 preserves the oath as direct speech: דימית is first-person “I swore,” ברוגזי carries the first-person suffix “my wrath,” and the דלא נעלונ construction means “they shall not enter” (נעלונ itself is the plural imperfect entry verb). The following “works of God” clause is kept separate and retains “from the foundation of the world.”
- Verse 8 names ישוע בר נונ; in this contextual contrast with the later day, the referent is Joshua son of Nun, so the English uses “Joshua” rather than the New Testament name “Jesus.”
- Verse 12 links each predicate to its own source span: the word is living and fully effective, sharper than a sword with two edges, and reaches the division of soul/spirit and joints/marrow/bones before judging heart-thoughts and intentions.
- Verse 14’s רב כומרא רבא is one great high-priest title, followed by Jesus the Messiah, God’s Son, with the ascent-to-heaven clause; the three source words are therefore grouped as “a great high priest” rather than splitting the two רב forms across roles.
- Verse 15 retains both source negations and the subject/object roles: “we have not a high priest who is not able to feel with our weakness,” but one tested in every respect like us, apart from sin; verse 16 keeps the mercy and grace-for-help objects of approaching the throne.

Source checks for Hebrews 5:

- The high-priest compound is retained as “high priest” at verses 1, 5, and 10; the repeated רב + כומרא forms were checked against their actual contiguous spans rather than treating רב as an isolated title.
- Verse 1 preserves the human origin and representative role: the priest is from humans, appointed over the things that belong to God, and is to offer both an offering and a sacrifice for sin.
- In verse 5, דברי carries the first-person suffix (“my Son”), while ילדתכ carries the second-person suffix (“begotten you”); the appointment/quotation wording therefore keeps the one who speaks and the addressee distinct.
- Verse 7 keeps the prayer sequence, strong crying, tears, the one able to save him from death, and the final “he was heard” as separate source-linked clauses.
- Melchizedek is preserved as a proper name in verses 6, 10, and 11; דמותה in verses 6 and 10 is rendered with its third-person possessive (“his likeness”).
- Verse 4 retains the reflexive “himself” before the contrast with the one called by God; verse 11 retains both “concerning him” and “concerning this Melchizedek,” the discourse object, and the object suffix in “explain it.”
- Verses 12–14 preserve the teacher obligation, “your time in instruction,” plural first written lines of the beginning, milk/copula sequence, and mature/solid-food contrast; the relative, causal, negative, and trained-senses clauses remain linked to their actual source positions.

Source checks for Hebrews 6:

- In 6:1–2, the source joins the foundation question to repentance from dead works and faith in God, then continues the list through baptism, laying on of hands, resurrection from the dead, and eternal judgment. The laying-on-of-hands compound is kept together while retaining both source positions.
- In 6:4–6, the repeated “again/from the beginning” wording is retained rather than smoothed away: those who have received the heavenly gift, Holy Spirit, good word, and coming-age power cannot return to sin and renewal, nor crucify the Son of God again and despise him.
- In 6:7–8, the earth imagery preserves the causal particle for the rain-bearing ground and the adversative turn before the thorn-bearing ground. The prefixed relative perfect דאשתית is rendered “which has drunk”; the suffix on דמטלתהונ is third-person masculine plural, referring to the preceding “those,” so the cultivation relation is rendered “those for whose benefit it is cultivated.” 6:8 retains רחיקא as “far” in “not far from a curse.”
- In 6:9–12, the contrastive דינ particles are rendered as “however”/“but,” including the exhortation to show diligence and not become discouraged. “Ecclesia” terminology is not present in this chapter.
- In 6:13–18, Abraham, God, the promise, the greater one, the oath, the two unchangeable things, refuge, comfort, and hope are all retained. The source’s oath sequence in 6:13 is compressed into natural English while preserving the self-swearing clause.
- In 6:19–20, the anchor, soul, unshaken condition, veil, Jesus’ prior entrance, priesthood forever, and Melchizedek likeness are each linked. The contiguous אפי תרעא is an attested sanctuary-entry curtain/veil compound (CAL, https://cal.huc.edu/oneentry.php?cits=all&lemma=%29pyn+N); with לגו מן it is rendered “within the veil,” retaining the spatial construction without inventing a second “inner” adjective.

Source checks for Hebrews 7:

- Verses 1–4 retain the copula identifying Melchisedec as king and priest, the later subject who meets Abraham, the blessing and tithe, and the discourse contrasts in “however” (7:1, 4). The genealogy clause in 7:3 says that his parents, beginning, and end are not written, rather than asserting that he literally had no parents.
- Verses 5–10 preserve the causal “for,” the Levitical priesthood and law command, the people/brothers and Abraham-loins relations, and the temporal “when” (not “although”). Scripture is explicitly the witness that the one receiving tithes lives (7:8), and the Levi-through-Abraham argument remains intact.
- Verses 11–15 preserve the conditional perfection question and both priesthood likenesses. The source’s אמר ... דינ ... likeness-of-Aaron clause is rendered as a complete counterfactual sentence, “It would have said instead: he would be according to the likeness of Aaron,” without importing an unsupported “not be called”; 7:13 keeps the born-of-another-tribe and no-altar-service argument, and 7:14 retains “our Lord” and the Moses contrast.
- Verses 16–22 retain the two-part fleshly-precepts/life-power contrast, the weakness and no-benefit rationale, the better hope and approach to God, and the oath mediated through David. Jesus remains the guarantee of the better covenant.
- Verses 23–28 preserve the mortal-versus-enduring priesthood contrast, life and intercession, the priest’s purity and separation from sins, the one-time self-offering, and the law/oath appointment of the perfected Son. Verse 26 says “such a priest”; “high priest” remains only where the source has the chief-priest compound in 7:27.

Source checks for Hebrews 8:

- The opening contrast particle דינ is rendered as “but,” while the repeated causal גיר is retained as “for” in 8:3, 8:7, and 8:8.
- The priest terminology distinguishes רב כומרא as “chief priest,” and כומרא in the plural as “priests”; the offering nouns קורבנא and דבחא are kept as “gifts” and “sacrifices.” In 8:6, בה מצעיא is retained as “in it as mediator” rather than dropping the preposition.
- The relative and copular sequence in 8:2 is rendered “the true tabernacle that God has fixed and not a human being,” retaining both the divine actor and the negative human contrast.
- The Moses instruction in 8:5 preserves the imperative “See, make everything,” the pattern relative, and the suffix “to you,” rather than treating the citation as an unlinked quotation.
- The covenant quotation in 8:8–12 retains the family/house wording, the hand-taking and exodus sequence, the negative covenant-abiding clause, the “to me a people” relation, and the paired youngest-to-oldest range.
- In 8:10, the source order is rendered with the law in their minds, writing it on their hearts, and the paired “I will be their God / they will be my people” relation; 8:11 keeps the finite verb and its first-person object together as “all of them will know me.”
- In 8:13, the source distinguishes the first covenant being made old from what has itself grown old and decayed; the final clause therefore reads “is near to decay.”

Source checks for Hebrews 9:

- Verses 1–5 retain the sanctuary structure and furnishings from the actual source: the earthly holy house, first and inner tabernacles, second veil, golden incense house, ark and covenant overlay, golden urn, manna, Aaron's budding rod, covenant tablets, cherubim, mercy seat, and the source's statement that there is no time to speak about each arranged item.
- In 9:3, אפי תרעא is retained as the sanctuary curtain/veil compound rather than reduced to the bare “door” gloss for תרעא. The Comprehensive Aramaic Lexicon records the compound sense “curtain or cover of something” in its אפי entry, with the sanctuary-entrance parallel at https://cal.huc.edu/oneentry.php?cits=all&lemma=%29pyn+N; “second veil” also preserves דתרינ and the spatial relation from מנ.
- In 9:2, אית הוא בה is rendered as the complete past existential “had within it,” and ומתקרא הוא retains its conjunction as “and was called.” In 9:3, the source's דלגו מן אפי תרעא דתרינ construction is rendered as the relative spatial clause “which was within the second veil,” while preserving the established veil compound.
- Verses 8–10 are read as complete connected predicates: the Spirit makes known that the way of holiness was not yet revealed while the first tabernacle stood; this is a symbol for the time in which offerings were made, followed by the fleshly ordinances clause through correction. Sentence punctuation was checked across the complete chapter paragraph, including the conditional join from 9:13 into 9:14.
- Verse 10 supplies the English copula and antecedent (“they were only in food and drink...”) for the source's elliptical contrast, while retaining both kinds terms and the fleshly ordinances through correction. Verse 13 keeps postpositive גיר as “For” in “For if,” lets p12 הוו complete the plural relative “those who were defiled,” and assigns p13 ומקדש with its object p14 to the shared blood-and-ashes subject: “were sprinkled ... and sanctified them.” Verse 20 places the colon after “to them,” and verse 23 marks the contrast before the heavenly-things clause.
- Verses 6–10 preserve the priestly access restrictions and the source's grammar: priests enter the outer tabernacle continually, the high priest enters once yearly with blood for himself and the people's transgression, and the Spirit makes known that the holy way was not yet revealed while the first tabernacle stood. Offerings and sacrifices cannot perfect the conscience, while the fleshly ordinances last until correction.
- Verses 11–14 keep the Messiah's contrast with the former rites: his great and perfect tabernacle is not hand-made; he enters once with his own blood for eternal redemption; the cleansing contrast is from dead works so that we may serve the living God.
- Verses 15–17 retain the source's covenant/testament and death grammar. The mediator's death is redemption for those who transgressed the first covenant; where there is a testament, the maker's death is shown, and it has no usefulness while its maker lives. The causal For in 9:16 is rendered in the span owning גיר.
- Verses 18–23 retain the causal and negative relations: the first covenant was not established without blood; Moses' commanded sprinkling covers the scroll, all the people, the tabernacle, and its service vessels; without shedding blood there is no forgiveness; likenesses of heavenly things are distinguished from the heavenly things requiring a better sacrifice. In 9:19, גיר is included in the opening “For when it was commanded” span.
- Verses 24–28 preserve the Messiah's true-heaven contrast and all time/repetition roles: the chief priest repeatedly enters yearly with another's blood, whereas the Messiah offers himself once at the end of the age; humans die once and face judgment; the Messiah was offered once and appears a second time without sins for the life of those who await him, retaining the plural suffix in לחייהון and the final “to him” relation.

Source checks for Hebrews 10:

- In 10:1, the law had a shadow in it; it is not itself called a shadow. The repeated-pronoun construction at positions 18–20 identifies the same sacrifices. Their passive offering at 22–23 remains distinct from their inability to perfect at 24–27 and the active offering at 29–30.
- In 10:2, the long-ago adverb belongs to the consequent cessation clause, and the contrast particle introduces that consequence. The conscience predicate retains its negative and its affected persons; the final dative belongs to the cleansing construction rather than introducing another beneficiary.
- In 10:5, the source says that God clothed the speaker with a body. In 10:6 and 10:8, the whole-burnt-offering compound is one kind of offering, not two separate offerings. The repeated first-person forms in 10:7 complete the coming predicate and do not introduce a second speaker. In 10:10, both the explanatory particle and the possessive in “will of his” are retained; the one-time expression is rendered “once.”
- In 10:11, the chief priest’s repeated offerings retain their past auxiliaries. In 10:17, sins are not remembered against the people. The confidence expression in 10:19 and the veil compound in 10:20 are translated as complete idioms; the latter follows the sanctuary compound checked in Hebrews 9. The renewal verb in 10:20 remains a finite relative predicate.
- In 10:21, the noun followed by “great” is distinguished from the chief-priest compound in 10:11. In 10:24–25, the reciprocal expressions refer to one another; the final demonstrative belongs to “that day.” In 10:27, the adversaries are plural.
- In 10:29, the blood of the covenant is reckoned as anyone’s blood, while the sanctification relative is retained. In 10:30, no explanatory “for” is added to the opening knowing predicate. In 10:32, the source explicitly names baptism. In 10:34, the grief is for imprisoned people; in 10:35, the great reward is prepared for the confidence the readers have.
- In 10:37, the future coming predicate and its pronominal/relative subject are read together. In 10:38, “my faith” retains the first-person possessive; the feminine discouragement construction with an affected-person dative is an idiom also found in 1 Thessalonians 3:3, rather than a reason to make faith its subject. In 10:39, the final noun is singular “our soul,” with a first-person plural possessor, and the separate “for us” relation is retained.

Source checks for Hebrews 11:

- In 11:1, the existential construction and its dative are read together in the comparison with actual occurrence. In 11:3, the visible-things relative, past being predicate, and source preposition retain their own positions; the English copula is not assigned to the preposition alone.
- In 11:4, the comparative retains the intensifier, the testimony is concerning Abel, and God is the witness to his offering. Abel is not made the speaker of testimony to God. The removal, negation, and prior testimony clauses in 11:5 remain distinct.
- In 11:6, God remains the rewarder of those who seek him. The English retains the active rewarder predicate and its subject, with the seekers in the preceding dative phrase, rather than replacing the clause with a passive statement about being rewarded.
- In 11:9, the heirs/inheritance construction retains the singular possessive and promise genitive as “the heirs of his promise.” In 11:10, the dative completes the city's foundation/possession construction, while the following relative identifies its builder and maker.
- In 11:11, Sarah is the subject; her barren condition is subordinate to her receiving strength, and she gives birth outside the time of her years. The final clause says she firmly believed the promiser was faithful, rather than merely saying that she was strengthened by belief. In 11:13, the promise noun is singular with a plural possessor; dying, not receiving, the promise, and the following contrast are linked to their actual source positions.
- In 11:16, the desired city is better than the former city, namely the one in heaven; the heavenly city is not the inferior term of the comparison. In 11:17, the possessive in “his trial” is retained, and the source's only-son/receiving-by-promise clause is followed without importing the familiar Greek construction about receiving promises. In 11:19, the ability to raise from the dead belongs to God.
- In 11:23, Moses' parents hide him for three months after his birth; the description is of a beautiful boy. The source-order comparison in 11:26 keeps the Messiah's reproach and Egypt's treasure in their proper comparative relationship.
- The list introduced in 11:32 continues through the following relative clauses. In 11:34, the source says they were strengthened out of illnesses, rather than changing the from-phrase into a temporal “after.” In 11:35, the source's giving to women of their sons remains part of the preceding agents' deeds; the following tortured group and its better-resurrection clause remain separate.
- In 11:39–40, the negative reception of the promise is followed by the reason and purpose. The anticipatory looking construction in 11:40 is rendered “God looked ahead to our own advantage,” retaining both the help/advantage noun and the possessive emphasis before the clause about not being perfected without us.

Source checks for Hebrews 12:

- In 12:1, the first-person existential/copular construction is rendered “we also have,” with its dative understood grammatically. In 12:6, the Lord is the subject of loving and chastening; the postpositive explanatory particle is rendered once, and the delight-in construction is not changed into selecting sons from among a group.
- In 12:7, the otherwise unglossed `אינו` introduces the rhetorical question about what son his father does not chasten. It is not an assertion that such a son exists. In 12:9, the fleshly fathers are plural, but the Father of spirits is singular: the third-person feminine plural suffix anticipates the following spirits, rather than supplying an English “our.” The local Murdock reference's plural “spiritual fathers” is not followed.
- In 12:10, the earthly fathers' desire and chastening auxiliaries belong to their own predicates; God's contrasting purpose is the readers' advantage and participation in his holiness. In 12:11, “afterward” begins the fruit/result clause, not the preceding sorrow clause, and both contrast particles are retained.
- In 12:14, holiness supplies the antecedent of “without it,” and our Lord is the object of seeing. In 12:15, the root's sprouting/growth, harm, and resulting defilement remain distinct predicates. In 12:16, the figurative human sense of `רפא` is “dissolute,” explicitly attested for this verse in [CAL's rpy entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=rpy+A).
- In 12:17, the temporal `מן בתר` compound is rendered “thereafter,” rather than requiring an extra English “from”; [CAL lists the compound](https://cal.huc.edu/browseSKEYheaders.php?direction=1&sortkey=mn%40+ahtj+g) as after/hereafter/thereafter. The desire verb and its auxiliary are linked together. Rejection ends at source position 10; the following place/repentance/negative-finding clause gives the reason, without attaching an invented “from the place” to rejection.
- In 12:18, the indexed source has no word for Murdock's supplied mountain; the touching form is passive/reflexive. In 12:20, the endurance predicate is second-person plural, and the following animal/mountain/stoning clause retains its source terms.
- In 12:21, the sight is so fearful that Moses speaks; Moses is the subject of the resulting speech clause, not the possessor of the sight. The repeated first-person forms complete the fear/trembling predicates. In 12:22, the assemblies are plural, while 12:23 uses the established “ecclesia” rendering for `עדתא`.
- In 12:25, the repeated `שאל` forms are Ethpeel, not the Peal “ask” gloss. Nöldeke explicitly records the derived-stem senses “begged to be excused” and “declined” ([Compendious Syriac Grammar, p. 113, end of section 171](https://biblicalstudies.org.uk/pdf/e-books/noldeke_theodor/syriac-grammar_noldeke.pdf)). The linked text consistently renders the verb with its from-relation as “beg off from.” The earthly hearers' failure to escape remains the main predicate; the final comparison makes its elliptical negative polarity explicit.
- In 12:27, “this” statement is the subject of “shows.” The shaken things are made/created; the separate purpose clause says that the unshaken things may remain. The source's unglossed `עבידא` is read with its making lexeme and plural copula. In 12:29, `אכלתא` describes consuming fire; the unrelated machine gloss “weevil” is not inserted into the verse. Compare the fire/devouring sense in the [Syriac lexical entry for the eating derivative](https://www.assyrianlanguages.org/sureth/dosearch.php?language=id&searchkey=364).

Source checks for Hebrews 13:

- In 13:1–2, the continued brotherly love and the hospitality instruction are rendered as genuine English exhortations. The hospitality verse retains both the source's worthiness/privilege predicate and the receiving of angels without awareness, rather than assigning remembered English words to unrelated positions.
- In 13:4, the contrast particle introduces God's judgment of fornicators and adulterers; it does not contrast the two groups with one another. The bed retains its possessive. In 13:5, the mind/money negative exhortation, sufficiency construction, and two datives belong to their actual clauses. The following emphatic subject identifies the Lord himself as the speaker, without borrowing an extra “you” from the preceding clause. Both negatives and the hand image remain in his promise.
- In 13:6, the first-person confidence construction and its dative are interpreted together, while the quoted helper/fear/human-action clause retains its own persons. In 13:8, `הויו` functions as an identity/copular expression in the assertion about Jesus the Messiah; the English does not strand the dictionary's “i.e.” gloss before “forever.”
- In 13:9, the strengthening verb is active first-person plural and the hearts are “our hearts,” rather than a passive statement about “your hearts.” The final benefiting verb is passive: those who walked in the foods were not benefited. In 13:11, the animals/blood relative and the burned-flesh comment are joined without treating the first half as an independent sentence; the chief priest, sanctuary, sins, flesh, and encampment retain their source relationships.
- In 13:12–14, Jesus suffers outside the city and the readers are called to go to him outside the encampment. The source's coming-city expression is retained, with the explanatory particle introducing the lack of an abiding city here. In 13:15, praise, fruit, lips, confession, and his name remain linked to their actual words.
- In 13:16, compassion and sharing with the poor are followed by the source's approval construction: these sacrifices make a person pleasing before God. In 13:17, the leaders' watching, the readers' souls and account, and the warning about unprofitable groaning retain their actual subjects and suffixes.
- In 13:18, the explanatory particle belongs to “for we are confident,” followed by the conscience complement and conduct rationale. It is not moved into a new independent conscience clause. In 13:19, the quickness modifies the writer's return. In 13:20, the great Shepherd is identified as Jesus the Messiah, our Lord; the covenant is not made the antecedent of that identification.
- In 13:21, the second-person plural recipients of “make you complete” are distinct from `בנ`, “in us,” in the following working/accomplishing clause. The blessing retains its volitional force. In 13:23, “with him” belongs to the writer's seeing the readers with Timothy after his arrival, not to Timothy arriving with himself.
- In 13:24, the first greeting is an imperative, but the Italy greeting is a declaration with those from Italy as its subject. The final grace greeting retains its plural recipients and Amen.

Source checks for James 1:

- 1:1 retains “our Lord,” twelve tribes, dispersed nations, and peace greeting.
- 1:3 renders the testing of faith as producing patience; 1:4 keeps patience’s complete work and “lacking nothing.”
- 1:5–8 preserves asking in faith without doubting, the wind-stirred wave, and the relative “who is divided in his mind.”
- 1:12 uses temporal “when he is proved”; 1:13 preserves the direct temptation quotation, the statement that God is not tempted by evil, and the independent “no one” negative.
- 1:14 retains the personal temptation, the relation to one’s own desire, and the lusting/drawing predicates. In 1:15, desire conceives and gives birth to sin; completed sin gives birth to death.
- 1:17 has one gift noun with “good and complete” modifiers; 1:19 coordinates quick hearing, slow speaking, and slow anger.
- 1:21 preserves first-person-plural “our nature”; 1:23 retains “a doer of it”; 1:24 assigns “goes away” and “forgets” to their own source words.
- 1:25 retains demonstrative “this one”; 1:26 renders the heart as deceiving him; 1:27 retains masculine “keep himself,” orphans/widows, oppression, and the world.

Source checks for James 2:

- 2:1 retains the face-taking/partiality relation explicitly.
- 2:5 preserves the contrastive particle: God chose the poor, but rich in faith, as heirs.
- 2:7 retains the passive/name invocation: the good name called over the readers.
- 2:10 renders the one-point stumble without adding an unsupported commandment word.
- 2:13 positions 10-14 are second-person plural and passive; the imperfect is rendered “you will be raised by mercy above judgment,” rather than making mercy the grammatical subject.
- 2:20 retains the contrastive particle in “But do you want to know.”
- 2:21-25 preserve the work/faith argument and named Abraham, Isaac, Rahab relations in source order.

Source checks for James 3:

- 3:1 preserves the prohibition on many becoming teachers, the direct address to brothers, and first-person plural liability to a greater judgment.
- 3:2-4 retains the source's progression from many/for/all stumbling to the tongue, bridles, and the ship's small timber; causal particles and the singular mouth are represented.
- 3:5-6 preserves the tongue's small-member and raised/arrogant construction, the fire/forest image, the tongue's defiling, the course of generations, the wheel, and the final fire assertion.
- 3:7-9 keeps the passive subjection of animals, the tongue's inability to be subdued, and the human beings made in God's likeness.
- 3:10-12 retains the same-mouth contrast, the fountain question, and the fig-tree/vine/fruit and salty-water impossibilities.
- 3:13-16 preserves the wise person's conduct, bitter envy/contention, the prohibition on boasting against truth, and the causal confusion/evil result.
- 3:17-18 retains the source's wisdom qualities, including mercy, good fruits, no partiality, no respect of persons, and the peace-mediated sowing of righteous fruits.

Source checks for James 4:

- James 4:1 preserves the question's source order: "from where" asks about wars and strifes among you, and the final relative verb explicitly wages war in your members.
- James 4:2 links the negative come/hand construction as "nothing comes into your hands" and keeps the repeated second-person subjects and final causal "because you do not ask."
- James 4:4 distinguishes the source's adulterers, this-world love, enmity, friend, and enemy terms; the postpositive "therefore" remains visible between "whoever" and "desires," and "God" is linked at both occurrences.
- James 4:5 renders the source's contiguous jealousy-desire-spirit construction as "the jealous desire of the spirit that dwells in us" rather than importing a remembered verse wording.
- James 4:6 exposes the source roles as "But greater grace our Lord gives to us" (retaining the first-person plural suffix in "our Lord") and renders מטל הדא אמר as "for this reason he says" rather than making "this" the speaker.
- James 4:7 keeps the postpositive connective visible as "Submit yourselves therefore to God."
- James 4:8 retains both draw-near verbs, the hand/heart objects, and the final divided-in-soul description.
- James 4:11 maps both speak-against and judge-against-law clauses separately, then preserves the conditional distinction between judging the law and being its doer or judge.
- James 4:12 keeps one lawgiver and judge, the make-alive/destroy contrast, and the repeated second-person neighbor question.
- James 4:13-15 preserves the opening question "But what shall we say concerning those who say," the city/year/trading/gain sequence, the vapor that appears briefly and vanishes, and the Lord-willing condition.
- James 4:17 keeps the additive "And whoever" opening, the good-known/not-done construction, and the final "to him it is sin" mapping.

Source checks for James 5:

- James 5:4–5: `עלת` is rendered in the complete “has entered the ears of the Lord of hosts” phrase, and `גיר` is retained in the causal opening “For you have lived merrily on the earth.”
- James 5:8 and 5:10: `ולבותכונ שררו` is “and strengthen your hearts”; `דמותא לנביא סבו` owns “Take the prophets as an example”; `למגרת רוחא דאולצניכונ` retains long-suffering, spirit, and the plural 2mp “your afflictions”; and `הנון דמללו בשמה דמריא` explicitly resumes the prophets as “namely, those who spoke in the name of the Lord.”
- James 5:14: `עדתא` is rendered with the project’s established terminology as “ecclesia.”
- James 5:16: `חילה דצלותא` is the anticipatory genitive “the power of the prayer”; the final third-person feminine resumptive `לה` is absorbed into the complete relative “which a righteous man prays,” without a dangling “to.”
- James 5:16 grammar check: anticipatory suffix/genitive and relative resumptive analysis follows Mark Francois, *Classical Syriac Grammar*, chapter 7 §§7.3–7.4, [pp. 73–75](https://markfrancois.wordpress.com/wp-content/uploads/2023/01/classical-syriac-grammar-week-7.pdf#page=5).
- James 5:18: plural `פאריה` is rendered “its fruits.”
- James 5:11 and 5:19: `יהביננ טובא לאילין דסיברו` is rendered as giving blessedness to those who endured, and the contiguous `ונפניוהי אנש` phrase is rendered “someone turns him back.”

Source checks for 1 Peter 1:

- Position-sensitive revisions absorb auxiliaries and explicit pronouns into contiguous grammatical spans: `דנהוונ למשמעתא` is “become obedient”; `נטירינ אנתונ` is “you are kept”; and the repeated 2PL pronouns in vv. 6 and 8 are not duplicated in English.
- Relative and appositional clauses were kept source-led: v. 10 makes the prophets the subject of “inquired”; v. 11 makes the Messiah’s indwelling Spirit the showing/testifying subject and keeps the sufferings-of-Messiah relation; v. 19 attaches Messiah to the spotless Lamb; v. 20 keeps appointed-beforehand, pre-foundation, and last-time manifestation components.
- Negation and suffixes were retained explicitly: v. 12 “not for themselves,” “for us,” and “announced to you”; v. 14 “did not know”; v. 17 retains the second-person possessive in “your sojourning”; v. 18 both silver/gold negatives and the fathers suffix; v. 22 retains the pure/perfect-heart and one-another phrases.
- In v. 22, the verbs `נהוינ` and `ונהוינ` are third-person feminine plural imperfects agreeing with “your souls”; the coordinated rendering keeps nonpast “your souls become holy ... and become full”; `תהוונ מחבינ חד לחד` is second-person plural imperfect plus participle, rendered “be loving one another.” Verse 24 keeps the causal particle as “For” and joins the grass/flower clause with a semicolon rather than leaving a fragment.

Source checks for 1 Peter 2:

- 2:1 keeps the imperative at position 0 and `therefore` at 1; positions 7–8
  are one genuine face-taking idiom rendered “hypocrisy,” not “in the face.”
- 2:2 groups positions 8–10 as “pure spiritual milk” while retaining the
  source order through one grammatical adjective group.
- 2:3 joins the conditional to verse 2 with punctuation and groups positions
  3–5 as “that he, the Lord, is good,” so the good lexeme, copula, and Lord
  are all owned without a stranded subordinate sentence.
- 2:4 groups positions 0–4 as “to whom you draw near: he is,” preserving the
  third-person copula referent rather than making “you” singular; `לות` is
  rendered “toward God,” one of its actual source senses.
- 2:5 preserves plural temples and priests; neither is replaced by a singular
  temple or the abstract noun priesthood.
- 2:6 places causal `גיר` first in a grammatical “for it is said” group and
  groups the quoted put/place verb with its explicit first-person subject.
- 2:7 places the believing predicate at 5–6 and the contrast at 8, while one
  adjacent grammar group keeps “to you who believe ... this honor” complete.
  Its contrast continues into the stone predicate in 2:8 as one sentence.
- 2:8 repairs the stone/stumbling opening, keeps the `בדלא` negative causal
  connector, and renders positions 9–11 as “the word to which they were
  appointed,” retaining the plural appointed verb and demonstrative relation.
- 2:9 renders active participial `דמכהנא` as “officiating as priests” and
  `למלכותא` as “for the kingdom”; redeemed ecclesia, praise/calling relation,
  and the 3sg “his” suffix on light remain explicit.
- 2:10 adds the finite “are” to the now/current people-of-God predicate while
  preserving the formerly/now contrast and both mercy clauses.
- 2:11 retains first-person request, 2pl “from you,” abstention, and the
  demonstrative `הלין` as “those” followed by its relative “that wage war.”
- 2:12 groups the copular exhortation as “let your conduct be good,” retains
  the 2pl “against you” suffix, and renders `נחזונ` as third-person “they may
  see,” followed by “and praise.”
- 2:13 retains the two causal `because` words separately: God, then kings,
  then their authority. The authority suffix is plural “their.”
- 2:14 keeps the third-person plural “they are sent,” with separate purposes
  for offender punishment and good-doer praise.
- 2:15 uses “your good deeds” from the deed suffix and keeps singular “mouth,”
  the foolish relative, explicit negation, and God once.
- 2:16 renders the adjacent freedom phrase as people who make freedom for
  themselves a covering for their wickedness; the third-plural suffix on
  wickedness is explicit.
- 2:17 uses one contiguous idiom group for `ומנ אלהא דחלו`: “and fear God.”
  This is the natural governed fear-God construction, while retaining all
  three source links and the imperative parallelism.
- 2:19 uses the existential+dative possession idiom “For these have grace before
  God,” then identifies them as those who, because of a good conscience, endure
  distress that comes upon them wrongfully. The final `בעולא` is retained.
- 2:20 resolves the rhetoric as: those suffering from their own wrongdoing have
  no praise; when the addressed hearers do good, third-person plural actors
  pressure them, and they endure it, their praise is great with God. The
  second-person and third-person plural subjects are kept distinct.
- 2:21 keeps the source's first-person plural “for us” (not “for
  you”) and the example/footsteps sequence.
- 2:22 keeps the guile noun before the “was found” verb and preserves both
  negative elements.
- 2:23 renders the final roles as Christ entrusting his judgment/cause to the
  righteous Judge; no reflexive “himself” is inserted.
- 2:24 keeps all our sins, plural “them,” first-person plural “we are dead / we
  live,” his own righteousness, plural wounds, and second-person plural healing.
- 2:25 combines the astray and “you were” forms into one natural “You were
  astray,” then keeps the present return to Shepherd and Overseer of your souls.

Source checks for 1 Peter 3:

- 3:1 preserves the relative clause after “husbands”: those not persuaded by the word may be won through the wives’ good conduct, without importing a separate recipient or extra action.
- 3:2 keeps `מתדברנ` as the explicit second-person feminine plural verbal predicate: “you conducting yourselves in reverence and modesty,” rather than nominalizing it as “your conduct.”
- 3:3 retains all three source adornment categories: outward hair-braiding, a gold ornament, and excellent clothing.
- 3:5 renders `מן קדימ` as the temporal phrase “in former times,” followed by the subject “holy women”; it does not make “from old” an origin or age modifier of the women.
- 3:6 retains the source’s copular auxiliaries around Sarah’s submission/calling and the second-person “you are her daughters”; `מתתזיענ` is rendered “you are not shaken” rather than confused with the following fear noun.
- 3:7 reads the final clause from its actual morphology: `תהוונ` is second-person plural and `מתתקלינ` is a plural passive participle, while `בצלותכונ` means “in your prayers”; therefore it is rendered “so that you may not be hindered in your prayers.”
- 3:9 keeps the source order “to no one evil for evil do not repay” and the distinct reviling-for-reviling pair; the contrast is plural “opposite of these,” followed by the positive action “bless.”
- 3:10 renders `הכיל` as inferential “therefore,” not temporal “then,” and attaches “to see” to “good days” in a complete maxim: whoever desires life and loves to see good days.
- 1 Peter 3:12 keeps the source's causal opening and contrasts the Lord's ears (hearing them) with his face against the evil; “evil” follows the source adjective rather than importing “wicked.”
- 3:13's מנו is rendered as “who is he,” and the imperfect verb as a prospective “will do”; the object is “evil” before “to you,” preserving source order.
- 3:14 preserves the conditional suffering and source “blessedness,” but renders מנ אילין דמדחלין לכון as “of those who frighten you”; the separate final prohibition is “do not be troubled.”
- 3:15 places “in your hearts” before “the Lord Messiah” because that is the source order. מפק ברוחא is rendered “for a defense” as the fixed defensive-response noun compound (CAL, [mappaq brūḥā](https://cal.huc.edu/oneentry.php?cits=all&lemma=mpq%40brwx%29+N)); it is not read as a literal “spirit” phrase. The final source phrase is humility followed by fear.
- 3:16 assigns תארתא טבתא to “a good conscience,” keeps איכנא דהנונ as “so that those,” preserves the explicit “evil men” comparison, and renders טלם as “reject” (not an inferred “slander”).
- 3:17 uses the source's profit/advantage predicate and retains both temporal “while” clauses and the explicit “if this is God's will.”
- 3:18 preserves “once,” the substitution contrast “the righteous for sinners,” the object “you” of bringing near, and the body/spirit contrast. Messiah is used consistently with the source glossary.
- 3:19-20 render שיול as “Sheol” (the source's explicit term), keep the relative link to the souls, and preserve the source's former non-obedience, Noah-day setting, divine longsuffering, ark command, repentance hope, eight souls, entry, and water-saving sequence.
- 3:21 follows the source's type/baptism explanation: active חיינ with its explicit second-person subject is rendered “you live in baptism,” not passive “are made alive.” The negative/contrast clauses distinguish washing bodily filth from confessing God with a pure conscience; the resurrection phrase remains attached to Jesus Messiah. The repeated copular subject pronoun is absorbed into “live,” retaining the initial “you” once in English.
- 3:22 keeps the source's relative continuation (“who”), ascension to heaven, right-hand location, and its genuine inversion “and subject to him are angels, authorities, and powers.”

Source checks for 1 Peter 4:

- 4:1: Rendered `כלמנ` (position 12, no exported gloss) as “every one,” matching the clause and Murdock context; rendered `לה מנ` (17–18) as the “from” relation before “all sins.”
- 4:2: Kept `נחא` as “live,” not “save,” and grouped the adjacent lust/human/live sequence into “to live for human lusts”; `כמא זבנא דאיתוהי` is rendered “as long as he is.”
- 4:3: Preserved the sequence of profane desire, luxury, drunkenness, lasciviousness, revelry/music, and demon worship; the final `פולחנא` is contextually “worship,” not generic work.
- 4:4: Rendered the imperfect plural verbs as the present “they marvel and blaspheme,” and retained the negative `בדלא` as “because you do not revel.”
- 4:5: Rendered `עתיד` in the relative clause as “is ready,” with `למדנ` as the infinitive “to judge”; the objects remain “the dead and the living.”
- 4:6: Rendered `אסתבר` (Ethpeal perfect) as “was preached,” and `נתדינונ` as the passive “might be judged”; `ונחונ` remains active “live,” not “are made alive.”
- 4:7 preserves contrastive `דינ` as “But” in the initial predicate. The separate `מטל הנא` supplies “therefore” before the exhortations “be sober” and “be watchful.”
- 4:8: Rendered `חובא` as “love,” `חריפא` as “fervent,” and `חדדא` as “one another”; the final clause is “love covers a multitude of sins.”
- 4:9: Rendered `אכסניא` as “strangers” and preserved `דלא` as “without,” yielding “without murmuring.”
- 4:10: Preserved the second-person plural suffix in `מנכונ` as “of you,” the feminine singular object suffix in `בה` as “with it” (the gift), and the third-person masculine singular suffix in `לחברוהי` as “his associates.” `רבי ביתא טבא` is rendered contextually as “good stewards.”
- 4:11: Rendered the two כל-מנ forms as “whoever” and kept the repeated speech/minister predicates. The 13–14 giving form includes its dative suffix as “has given him”; 18–19 preserve “God may be glorified” despite source verb-before-subject order. Positions 23–25 are the resumptive possessive/copula relationship, rendered “to whom belongs.”
- 4:12: Kept the plural address and suffixes in both “to you” phrases. The final 11–14 causal/copular group is rendered “because it was for your testing”; this avoids the ungrammatical source-order fragment while retaining the explicit “it was.”
- 4:13: The source places “at the revelation of his glory” before the two second-person rejoicing verbs; retained that order as “thus also, at the revelation of his glory, rejoice and exult.” The 4–5 phrase is Messiah’s sufferings, not a generic suffering.
- 4:14: Preserved the plural “you” in reproach and the dative plural “upon you.” Positions 8–10 are the actual noun phrase `רוחא משבחתא דאלהא`: “the praiseworthy Spirit of God”; no causal “because” is inserted. “The name of the Messiah” follows the construct relationship.
- 4:15: The source closes with the suffering predicate (13–14), after the three role descriptions. Rendered “as one who does evil suffer” to keep all classifications and avoid moving the verb across source positions.
- 4:16: Retained the conditional and contrast, the Christian designation, negated shame, and the demonstrative “this name.”
- 4:17: Grouped the source-order copular/relative clause 0–4 as “because it is time for judgment to begin,” and 8–11 as “if it begins with us.” “Those who do not obey the Gospel of God” retains the relative plural and negation.
- 4:18: Kept “scarcely lives” (not “is made alive”); the source has the ungodly and sinner before the “where ... be found” predicate, so the final span preserves that source ordering while remaining readable.
- 4:19: Rendered `מטל הנא` as the causal-demonstrative “For this reason,” retaining both source words before the relative subject. `בעבדא שפירא` is singular/mass “in good work,” not plural “works.” The final `איכ דלברויא מהימנא` is rendered “as to the faithful Creator,” attaching the comparison/recipient to the preceding “to him” while preserving the Creator noun and faithful adjective.

Source checks for 1 Peter 5:

- 5:1 — Render the opening verb with its first-person subject as “I entreat”; place the source’s adversative connector naturally as “however”; retain both explicit first-person forms, the elder identity, associate, witness, Messiah’s sufferings, and the future revelation of his glory.
- 5:2 — “Feed” is rendered “tend” for the flock, while the passive source form is “entrusted”; preserve the spiritual oversight, the force/willingness contrast, and the filthy-gain/whole-hearted contrast as “with all your heart.”
- 5:3 — The negative comparison is “not as lords over the flock”; the following exhortation is to become a good example for them, not to rule them.
- 5:4 — Keep the temporal clause and the source’s chief-shepherd subject; “crown of glory” is received from him and is explicitly unfading.
- 5:5 — “Young men” and “elders” retain their ecclesial terminology; the reflexive submission and clothing metaphor lead into humility of mind toward one another, followed by God’s opposition to the self-exalting and the source-ordered “to the humble he gives grace.”
- 5:6 — Preserve the therefore inference, the “under the mighty hand of God” image, and the purpose/result relation “that he may exalt you” at the fitting time.
- 5:7 — Keep the addressed plural command and the “all your care” object; the final clause keeps the source’s God-directed construction and his care concerning the addressees.
- 5:8 — The clause keeps the second-person plural imperatives, the possessive suffix in “your adversary,” and the distinct source title “Satan” at position 4; commas make “like a lion” modify the roaring action rather than the name.
- 5:9 — “Stand therefore against him” preserves the suffix object; the causal/comparative particle and explicit second-person plural pronoun are rendered as “as you are steadfast,” and the final clause keeps “these same sufferings” as the subject that befalls the brothers.
- 5:10 — The first relative (3–4) modifies the God of grace, while the independent הו at 10 is rendered as the equative “is the one.” This keeps דיהב at 11 as the relative “who has granted,” its recipient “us” at 12, and דכד at 13 introducing the granted content: “that, while we endure” the little afflictions, we may be strengthened, confirmed, and established. The perfect gift and ongoing endurance are therefore both retained without inventing a separate gift noun. The God/postpositive “But” pair is grouped together, and the complete sentence ends before the doxology in 5:11.
- 5:12 — Commas make “as I consider” a parenthetical while the second demonstrative remains appositional: “this is the true grace of God—this in which you stand,” without duplicating “in.” The sentence closes before the following greeting.
- 5:13 — The colon makes the postposed subject explicit: the chosen ecclesia in Babylon greets the addressees, with Mark coordinated afterward; the first-person possessive suffix remains “my son,” and “ecclesia” follows established terminology. A period separates it from the imperative in 5:14.
- 5:14 — The reciprocal phrase is kept as “one another,” and the final relative/resumptive sequence is rendered “those who are in the Messiah” without duplicating the addressees.

Source checks for 2 Peter 1:

- 1:1 retains the plural finite worthiness predicate. The Lord and Saviour
  possessives are first person plural.
- 1:3 uses a topical introduction and fronted object to preserve the late
  giving verb at position 11. `מנ` is the relative form, not the preposition
  "from". Recognition and calling retain their own source positions.
- 1:4 distinguishes the initial feminine plural reference to the preceding
  glory/excellence from the later reference to the promises. The plural
  promises are the fronted object of giving, not its means. Positions 8-11
  retain becoming, partakers, nature, and divine in order.
- 1:5-7 keep the additive virtues as an elliptical imperative list. In 1:8,
  the causative `מקימנ` and its object describe the virtues rendering the
  addressees neither idle nor unfruitful, rather than standing in them.
- 1:9 retains the passive lack of these qualities and forgotten cleansing of
  former sins. In 1:10, calling and choice belong to the addressees. The noun
  `גביתא` supports choice as well as collection; see the
  [CAL index](https://cal.huc.edu/browseSKEYheaders.php?direction=1&sortkey=cb+g).
  The intensity adverb and possessive on brothers are retained.
- 1:13-15 preserve the judgment of what is proper, causal knowledge of bodily
  departure, and making remembrance afterward. In 1:14, the disclosure verb
  has a first-person singular recipient, while the Lord title has a
  first-person plural possessive despite Murdock's singular title.
- 1:16 retains first-person plural going and making known, the second-person
  plural recipient, and the first-person plural Lord title. The fables are
  described as fashioned artfully, not as a preceding temporal event.
- 1:17-18 form a continuous sentence around the voice report. The declaration
  keeps the copula, "my Son", and the speaker's pleasure. The glory's feminine
  possessive differs from the Son's masculine referent. In 1:18, the first
  `ל` pronoun anticipates the following voice object; it is absorbed into the
  English object. The later recipient of the voice's coming remains explicit.
- 1:19 keeps the additive particle and the relative connection to attending
  to the prophetic word. The place noun and its following adjective form a
  small phrase. The knowing construction in 1:20 continues this sentence.
- 1:20 retains writing and its possessive, rather than substituting a claim
  about private interpretation. The verbal noun `שריא` is contextually an
  interpretation or exposition: its root has a metaphorical expounding sense
  in [CAL](https://cal.huc.edu/oneentry.php?cits=all&lemma=%24ry+V), and
  [Murdock's translation and note](https://aramaicnewtestament.org/peshitta/murdock/epistle/2peter_1.htm)
  support this reading. CAL's noun gloss "release" alone does not settle the
  metaphorical meaning.
- 1:21 distinguishes being moved from speaking. The Holy Spirit occupies
  positions 11-13; the speakers are third-person plural holy people of God.

Source checks for 2 Peter 2:

- 2:1 retains the people noun, rather than Murdock's world, and preserves the
  Lord buying the false teachers, their denial, and their bringing destruction
  upon themselves. In 2:3 the demonstrative introduces the teachers whose
  judgment follows; it is not a second object after the exploited addressees.
- 2:4-9 keep the extended conditional sentence. The explanation of Lot's
  distress in 2:8 is parenthetical, so the Lord's knowledge in 2:9 completes
  the conditional. The lower places in 2:4 remain source vocabulary, and
  2:5 retains Noah's ordinal description.
- 2:6 ends with the third-singular perfect placement verb `סמ`. It establishes
  the example for future ungodliness; it is not part of a future passive
  description of people being placed. In 2:7 the plural copula completes the
  lawless people's description, before the separate active delivery of Lot.
- 2:8 retains the past dwelling construction and the active Pael tormenting
  predicate. The righteous soul is the object, not a passive or locative
  subject. The upright man is identified appositionally in the English.
- 2:10 retains singular glory and its relation to fear, rather than importing
  plural glorious beings. The source preposition `מן` with `זוע` can introduce
  the object of fear; the English uses "of glory". Compare the fear sense and
  its pronominal objects in [CAL](https://cal.huc.edu/oneentry.php?cits=all&lemma=zw%28+V).
- 2:12 keeps the explicit past copula and the subsequent future destruction
  predicate. In 2:13 the singular copula belongs to the iniquity/reward
  construction. The plural defilement descriptions qualify the people,
  rather than the singular daytime revelry.
- 2:14 distinguishes possessing the eyes and trained heart from being
  children of the curse. It does not say that these teachers possess cursed
  offspring. The curse description is an English nominal predicate.
- 2:16 preserves the donkey's lack of ordinary speech, her human-voice
  speaking, and the restraint of the prophet's foolishness. The two actions
  remain active, and the possessive on foolishness is absorbed into its
  following prophet genitive.
- 2:17 treats the repeated demonstrative as a topic: the gloom is reserved
  for these people; the people are not equated with darkness. In 2:18 the
  barely escaping description follows the supplied source and Murdock.
- 2:19 `זכא` is marked as a passive participle: the man is vanquished, not
  the conqueror of the thing that enslaves him. The English active equivalent
  makes that thing the agent overcoming the man. Both anticipatory/resumptive
  objects and the final enslavement relation are retained.
- 2:20 absorbs the anticipatory possessives in the world and Lord genitives.
  Entanglement and being overcome remain distinct, and the evil-plus-`מן`
  comparison is English "worse than". The result is declarative, not a question.
- 2:21 keeps the causal particle and the separate backward/turning elements.
  In 2:22 the things of the proverb are plural, the dog's return is past,
  and the sow's bathing is active. The final wallowing phrase uses the
  coordinated proverb's understood return rather than inventing a source verb.

Source checks for 2 Peter 3:

- 3:1 `מן כדו` is the idiom "already/now", rather than a statement that
  something suffices. Compare the lexical adverb in the
  [Classical Syriac course book](https://refubium.fu-berlin.de/bitstream/handle/fub188/24490/Classical_Syriac_Course_Book_3-2019.pdf?isAllowed=y&sequence=3).
  The plural reference to the letters is retained, and the purpose clause
  in 3:2 continues the sentence. In 3:3 the prefixed `ד־` introduces the
  content known beforehand; it does not make knowing a temporal condition
  for the mockers' arrival. Their walking and the preceding "while" keep
  their own source positions. The quoted question and argument in 3:4 are
  punctuated as the mockers' speech.
- 3:5 has a singular feminine subject, "this", with plural experiencers:
  it escapes their notice by their choice. The plural willing verb is
  distinct from the singular initial predicate. The earth's rising from
  and through water remains explicit. In 3:6, the flooding verb permits an
  intransitive inundation rendering; it does not make the world the agent
  flooding something else.
- 3:7 `אסינינ` is a passive participle, not an active storing action.
  The following reservation for fire shares the heavens/earth subject.
  In 3:8 the singular fact is the subject and the addressees are the object
  of the forgetting/escaping construction; "my beloved" retains its suffix.
- 3:9 keeps plural promises. The long-spirit expression is idiomatic
  patience, and the closing repentance clause remains parallel with the
  preceding clause about perishing. In 3:10 `מן שלי` is the suddenly idiom;
  "suddenly" is also among the supplied gloss alternatives.
- 3:10 ends with `תשתכח`, a future Ethpeal "will be found". There is no
  negative particle in the supplied verse, although Murdock's reference
  says "will not be found". The affirmative rendering follows the source;
  compare the [Syriac text and analysis](https://dukhrana.com/peshitta/analyze_verse.php?font=Estrangelo+Edessa&size=125%25&source=ubs&verse=2Peter+3%3A10).
  The works are described as being in the earth, without adding a separate
  possessive to the works noun. Burning remains intransitive, and the
  dissolution predicates remain passive.
- 3:11-12 form one question: the expectation of the day continues the
  question about fitting conduct. The fire-testing predicate in 3:12 is
  passive, while the elements' melting is future. In 3:13 the new heavens
  and earth are fronted objects of first-person plural expectation.
- 3:14 keeps the care/striving imperative, the purpose clause, the spot and
  blemish negations, the passive finding, and its personal recipient.
  In 3:15 the source says "the Lord", without the first-person possessive
  found in the closing title. Patience is the idiom; `פורקנא` includes
  redemption, salvation, and deliverance among its glosses.
- 3:15-16 form a continuous writing/speaking comparison. Paul's speaking
  verb is past. The difficult things, the people without instruction, their
  unstable condition, and their distortion of the writings retain distinct
  source roles; the stability participle is not an active supporting action.
  The final possessive belongs to their own destruction.
- 3:17 distinguishes the lawless people from their error and retains the
  addressees' own steadfastness. The closing 3:18 includes both the Lord/
  Saviour title and the explicit genitive "and of God the Father". The
  Father phrase is present in this Syriac source and is retained.

Source checks for 1 John 1:

- 1:1 retains the source's opening first-person plural declaration. The
  seeing verb's resumptive object is absorbed into the English relative
  construction, rather than duplicating its object. The coordinated seeing,
  looking, and touching predicates retain the same first-person participants.
- 1:2 distinguishes the past manifestation and seeing from the present
  testimony and proclamation. Positions 3-4 are active participles without
  a past auxiliary; they are not rendered as past actions. The later
  existential/copula and past auxiliary are a small combined construction.
- 1:3 retains the postpositive contrastive particle and both possessive
  forms as "and yet our own fellowship". It does not turn the contrastive
  particle into a relative pronoun. The possession/existence construction
  describing the addressees' fellowship is rendered as a small grammatical unit.
- 1:4 has first-person plural "our joy" and second-person plural "in you";
  neither participant is substituted for the other.
- 1:5 combines the light noun and its actual following copula. It does not
  assign an unsupported "indeed" to that copula. The conjunction before
  darkness and the full emphatic negation remain explicit. In 1:6 the
  conjunction coordinating the negative walking clause is likewise retained.
- 1:6, 1:8, and 1:10 retain the initial "and if" prefix. The postpositive
  contrast in 1:7 and 1:9 is English "but if". The conditional subjects,
  first-person objects, and the negative scope in all five clauses are kept.
- 1:9's trustworthiness expression is a passive-participial/adjectival
  construction with its copular pronoun, rather than an active believing
  action. Forgiveness and cleansing retain their third-person agent and
  first-person plural recipients. In 1:10 the source says the word is not
  "with us", which is retained rather than replaced from another text.

Source checks for 1 John 2:

- 2:1 uses the existential/first-person dative as "we have", without adding
  a second locative "with us". In 2:2 the identifying expression and
  postpositive causal particle form "For he is". `על אפי` is a bound
  prepositional expression; its face noun is not a displaced possessive.
  The sins retain "our", and the following conjunction/negation is explicit.
- 2:3 keeps the initial conjunction. In 2:6 `דאיכ` is the according-to
  expression, the following walkings are a noun, and final `נהלכ` is the
  walking verb. The verb is not moved onto the preposition. In 2:7 and
  2:18 the first-person singular possessives remain "my beloved/children".
- 2:12-14 retain the source's present writing forms and subsequent perfect
  writing forms in their actual verse positions. The causal knowing clauses
  use "because you have known"; the correlative/copular descriptions retain
  their own positions. The source's verse boundary does not move the final
  children/Father clause to match the Murdock reference's numbering.
- 2:16's anticipatory masculine reference points to the world, followed by
  a plural pronoun/copula. It is "they are from the world itself", rather
  than a personal "from him" with the Father as an apparent antecedent.
  2:18 retains the source's false-Messiah title and its plural counterpart.
- 2:21 retains both the negative writing statement and the negative knowing
  clause. The feminine all-quantifier belongs to the following falsehood,
  not to knowing the masculine truth. The resulting clause is "and that no
  falsehood is from the truth", with its anticipatory origin phrase absorbed.
- 2:22 keeps the rhetorical question and renders the denial as a quoted
  negative statement. This retains the source's `לא` while avoiding the
  opposite English meaning produced by "denies that Jesus is not ...".
  Compare the [Syriac text and translations](https://dukhrana.com/peshitta/analyze_verse.php?font=Estrangelo+Edessa&size=125%25&source=&verse=1John+2%3A22).
  The following Father/Son denial sequence stays in the source's order.
- In 2:21–22, the negative constructions `לא הות` and `לא הוא` retain
  present assertions, rather than mechanically importing past time from
  their perfect-form being verbs. Compare the non-inherent-predicate
  and negative-cleft uses in [Pat-El on Syriac negation](https://doi.org/10.1093/jss/fgl006).
- 2:23 has an active believing/trusting predicate with the Father, and the
  later Father noun belongs to a second explicit confession predicate.
  2:24 retains the message's remaining with the addressees, the "for if"
  connective, and the addressees' corresponding remaining in Father and Son.
- 2:25 keeps both the promise noun and the separate perfect promised verb.
  2:27 likewise keeps the conditional remaining verb and its "with you"
  phrase, rather than replacing those positions with only the anointing noun.
- 2:27's feminine pronoun and teaching participle share the explicit anointing
  subject. The negative existential, feminine locative, and falsehood noun
  form a small grammatical phrase. The final masculine "in him" is distinct
  from that feminine "in it". The comparative teaching clauses lead into
  the separate imperative "remain".
- 2:29 `דעו` is imperative "know", followed by "also that". The concluding
  origin statement is "is from him"; it does not add a birth verb absent
  from this supplied source.

Source checks for 1 John 3:

- In 3:1, `מטל הנא` is the causal demonstrative construction “Therefore,”
  followed by “the world.” The calling and making predicates both retain
  their first-person plural objects; the final knowing verb is perfect.
- In 3:2, “My beloved” preserves the first-person singular possessive.
  The revelation clause introduces the expected event, while the separate
  first-person plural being predicate carries “in his likeness.”
- In 3:4, the postpositive contrast and “for” are retained, as are both
  sin nouns and both iniquity nouns. In 3:5, the separate second sin noun
  belongs to “and sin is not in him,” rather than to “our sins.”
- In 3:8, the two copular pronouns belong to the Satan/sinner clause;
  neither supplies an extra subject before the following “Son of God.”
  The loosening verb is rendered “undo” in relation to Satan's works.
  In 3:9, “and he is not able to sin” explicitly returns to the person
  born of God after the intervening clause about God's seed.
- In 3:12, the masculine emphatic evil adjective is substantivized as
  “the evil one” in the preceding Satan contrast. Its comparison continues
  the command in 3:11. In 3:14, the separate “by this” demonstrative
  explicitly introduces the evidence of loving our brothers.
- In 3:15, the plural ability and abiding participles agree with the
  Syriac plural life noun. English uses the mass noun “life,” with the
  source-order construction “cannot have abiding in him life eternal.”
- In 3:16, the soul/self noun is rendered “life/lives” in the giving-up
  context. `על אפי` is the idiom “for/on behalf of,” not physical faces.
  In 3:17, closing one's mercies is “closes his heart”; in 3:21, open
  faces before God express “we have confidence.”
- In 3:19, the supplied source explicitly says “before he comes,” with
  a third-person singular coming verb and its following subject pronoun.
  In 3:24, the keeping predicate is passive (“is kept by him”), followed
  by the distinct dwelling predicate and its explicit “and he” subject.
  The later third-person dwelling verb is also rendered with “he.”

Source checks for 1 John 4:

- In 4:1, the second-person plural imperfect is a prohibition; the
  following being/discerning construction supplies the contrasting command
  to discern spirits. In 4:2, “the Spirit of God” is the subject of “is
  known”; the separate “every spirit” begins the following test.
- In 4:3, the supplied confession clause names Jesus without repeating
  the Messiah title found in 4:2. The later source explicitly names the
  false Messiah. `מנ כדו` is the temporal idiom “already.” In 4:4, the
  comparative “than” is distinct from the following “in the world.”
- In 4:5, `מטל הנא` together means “therefore.” In 4:6, the two negated
  predicates remain distinct, and the repeated prepositional demonstrative
  construction supplies “By this.”
- In 4:9, the Son precedes the sending verb and its subject, God.
  English uses a local passive while retaining God as the sender.
  The anticipatory possessives in “the love of God” are absorbed naturally.
- In 4:10, the two occurrences of unvocalized `אחבנ` have different
  vocalizations and morphology: “we loved” and “he loved us.” `על אפי`
  is the on-behalf-of idiom. In 4:12, the fronted God is the object of
  “no one has ever seen”; the verb's resumptive object suffix refers to him.
- In 4:13, the Spirit phrase retains its partitive “from,” rather than
  making the Spirit the cause or subject of giving. In 4:14, the Saviour
  is “for the world.” In 4:16, the believing and knowing verbs are perfect;
  the supplied source ends with abiding in God without another reciprocal
  dwelling clause.
- In 4:17, confidence/open faces is a possession idiom, rendered “that we
  may have confidence.” The being verb there is imperfect, while the later
  “as he was” has an explicit perfect. In 4:18, the separate fear nouns
  retain their object and subject roles, and the supplied noun is danger.
- In 4:19, the first-person plural imperfect is hortative, “let us love.”
  In 4:20, “I love God” is first-person speech; the following hatred is
  third-person narration about that speaker. In 4:21, the two love verbs
  have distinct roles: whoever loves God should also love his brother.

Source checks for 1 John 5:

- In 5:1, the two birth forms differ: the first is a passive participle
  and the second a perfect. In 5:4, the first overcoming verb is an active
  participle, while the victory “has overcome” uses a perfect. The final
  first-person plural possessive belongs to “our faith.”
- In 5:5, the Son noun precedes its copula; the small linked predicate
  retains both rather than assigning “Son” to the copular pronoun.
  The supplied 5:7 has the Spirit's testimony and truth; 5:8 names the
  Spirit, water, and blood, whose agreement is expressed as “in one.”
- In 5:6 and 5:16–17, `לא הוא` is read as a negative/emphatic construction,
  without mechanically importing past time from the perfect-form `הוא`.
  This distinction follows the negative and negative-cleft uses discussed
  in [Pat-El's Syriac negation study](https://doi.org/10.1093/jss/fgl006).
- In 5:9, `חד כמא` supplies the comparative degree in the question about
  God's testimony. In 5:10, the believer's present participle is distinct
  from the later perfect “has not believed.” The possessive/existential
  construction is “has this testimony in himself.”
- In 5:11, English passive wording preserves the source's fronted life
  object, with God still explicitly the giver and us the recipients.
  In 5:12, passive-classified `אחיד` describes the holding/possessing state;
  it does not mean that the person is held by the Son. In 5:13, the
  believing verb is perfect, “you ... have believed.”
- In 5:15, “we have received” is perfect, and `מנ כדו` is the temporal
  “already.” The later asking verb is also perfect. In 5:16, life is given
  “to him,” with the following plural recipients retained as a clarification.
  The negative sin and prayer clauses keep their separate scopes.
- In 5:18, the born-of-God person “keeps himself”; the self noun is not
  replaced with a different person as object. In 5:19, the passive placing
  state is rendered “lies in the evil one.” In 5:20, the two first-person
  plural imperfects express knowing and being as purposes of the gift.
- In 5:21, `דחלת פתכרא` is religious reverence or worship of idols, not
  merely being frightened by them. The broader religious sense of the
  fear/awe noun is documented in the [Comprehensive Aramaic Lexicon](https://cal.huc.edu/getlex.php?coord=620540603&word=15),
  including Syriac religion, cult, and devotion senses. The noun and the
  separate plural idols remain individually linked.

Source checks for Revelation 1:

- In 1:1, the passive giving construction with the being infinitive is
  rendered “is appointed to happen.” The opening English supplies a
  presentative copula; the later making-known and sending predicates keep
  their third-person subjects. John's testimony continues into 1:2.
- In 1:3, the reader is singular and the hearers plural; English “are”
  agrees with their coordinated subject. The final drawing-near verb is
  perfect, “has drawn near.” In 1:5, loving and releasing are active
  participles, while the following making verb in 1:6 is perfect.
- In 1:6, the supplied source has a priestly kingdom: `כהניתא` is a
  singular adjective modifying the kingdom, not a separate plural priests.
  In 1:7, eyes and tribes retain their respective seeing and mourning roles;
  local English passive wording keeps the source's postposed agents.
  The piercing form `דדקרוהי` follows plural `אילינ`; its vocalized ending
  and antecedent support “those who pierced him” despite the singular raw tag.
- In 1:8, the source names the Syriac letters Olaph and Taw. In 1:9,
  the son/partner construction means a fellow partner, not a separate son.
  The source has tribulation and endurance in Jesus without another
  Messiah title there; the title is present later in the same verse.
- In 1:10, `חדבשבא` explicitly names Sunday. In 1:12, the infinitive is
  knowing/recognizing the voice, and the speaking verb is perfect.
  The seeing clause continues into the human-form figure in 1:13, where
  the actual garment noun is ephod. No down-to-the-feet wording is added.
- In 1:14, one white adjective describes head and hair, followed by wool
  and snow comparisons. In 1:15, the source's Lebanese brass is heated:
  the heating form is passive, not an active heating agent. In 1:16, the
  weapon is a sharp spear, and the showing/strength description retains
  the feminine third-person suffix as English “its.”
- In 1:18, the Living One continues the preceding first/last titles;
  “I was dead” has its own first-person perfect. The key noun is singular
  in both the supplied vocalization and morphology, also visible in the
  [Syriac source analysis](https://dukhrana.com/peshitta/analyze_verse.php?font=Estrangelo+Edessa&size=125%25&source=&verse=Revelation+1%3A18).
  Death and Sheol remain in the source's order.
- In 1:19, the present things and the future/prepared things remain
  coordinated separately. In 1:20, the stars and lampstands have distinct
  explanations, both final seven-quantifiers are retained, and gold belongs
  to the later lampstand phrase rather than the earlier one.

Source checks for Revelation 2:

- Reviewed all 29 verses and 441 source positions against the exact forms,
  complete gloss alternatives, numeric morphology, and continuous English.
  Possessive datives, anticipatory genitives, and copular/resumptive pronouns
  are rendered idiomatically. Short contiguous constructions group the words
  needed for English grammar, including the four-word tree-of-life infinitive
  in 2:7 and the four-word negative copular clause in 2:9.
- The supplied text differs materially from the Murdock reference: 2:1 has
  neither “all things,” “right,” nor a second “seven”; 2:5 says “went out,”
  omits an early repentance command, “quickly,” and “from its place”; 2:7
  says “God,” without “my”; 2:9 and 2:13 omit “your works.” These words
  are not imported from familiar English versions.
- Revelation 2:8 has perfect “came to life.” In 2:10 the opening second-person
  singular changes to plural for the prisoners, testing, exhortation, and
  promised crown. The hearing formulas retain plural “ears.” The imperative
  “repent” belongs to 2:16, following “Likewise” at the end of 2:15.
- Revelation 2:13 is difficult in the supplied text. The vocalized verb at
  position 14 is second-person singular perfect, rendered “you contended.”
  Both witness phrases remain. The first is interpreted as a coordinated
  nominal predicate with an implicit copula and inherited second-person
  subject: “and were my own faithful witness.” In the second, “is faithful”
  is read as the predicate, followed by an extraposed restrictive relative,
  “who from among you was killed.” This is an uncertain syntactic reading,
  not an unambiguous identification of the witness. No “Antipas,” “spectacle,”
  “with,” or additional statement of Satan's dwelling is supplied.
  [Dukhrana's verse analysis](https://dukhrana.com/peshitta/analyze_verse.php?font=XenoType+SY+Darmasuq&size=125%25&source=&verse=Revelation+2%3A13)
  corroborates the forms and records Etheridge's “with” variant; that
  preposition is absent from the source used here.
- In 2:17, “white tally” is a contextual rendering of *hushbana*, whose
  supplied glosses are “reckoning,” “calculation,” and “account.” The
  [Comprehensive Aramaic Lexicon](https://cal.huc.edu/oneentry.php?cits=all&lemma=xw%24bn+N)
  also records a concrete account item and a possible written account slip.
  That lexical evidence supports the contextual inference of a tally, but
  does not establish a stone or any specific ritual or civic function.
  Both occurrences remain linked. “In writing” renders the noun *ktaba*;
  it is not described as a passive participle. The final verb is perfect,
  “has received.” No eating verb is supplied for the manna in this verse.
- Revelation 2:18 has singular “eye” and “Lebanese brass.” In 2:20 the text
  says “much” and “your woman”; the genuine possessive is retained without
  deciding whether marriage is meant. The teaching, leading astray, and
  sacrifices of idols each retain their own source positions. The warnings
  in 2:5 and 2:22 preserve their conditional “unless” construction.
- Revelation 2:23 retains “kill with death,” plural “kidneys,” and singular
  “heart”; no disease is inferred. The ecclesias' knowing is read as the
  predicted result of the warning. In 2:24 the recipients do not “have” the
  teaching; no receiving verb is present. In 2:26 both overcoming and keeping
  are perfect, and no “unto the end” phrase occurs. The infinitive “to tend”
  in 2:27 continues the grant of authority in 2:26; the breaking verb is
  active second-person plural, followed by the speaker's perfect “received.”

Source checks for Revelation 3:

- Reviewed the 22 verses and 361 source positions against exact forms,
  vocalization, complete gloss alternatives, numeric morphology, and running
  English. Genuine possessives remain; possessive datives, anticipatory
  genitives, and resumptive pronouns are rendered idiomatically. The hearing
  formulas retain plural “ears” and the established ecclesias terminology.
- Revelation 3:1 retains the source's coordination of works, a name the
  addressee has, and the alive/dead nominal clauses. This preserves the
  explicit source sequence rather than silently assigning the name phrase
  to remembered English wording.
- Revelation 3:2 is syntactically difficult. The vocalized *qayem* is read as
  a command coordinated with “be awake,” despite the export's perfect tag.
  The “you were about to die” construction is second-person singular, not
  a statement that plural things were dying. Its relative/causal prefix is
  interpreted causally: “the remainder of those things, for you were about
  to die.” This is an interpretive resolution of the awkward clause.
  “I have not found you” retains the object suffix; the following completed
  works remain plural, rendered in a short two-word source construction as
  “with your works completed.” The final God noun has no “my.”
- In 3:3, “heard” precedes “received,” and the next command is “take heed.”
  The last bare “coming” participle has the same vocalized form as the first;
  the speaker's first person is inherited from context even though the raw
  export assigns its unaccompanied occurrence a third-person tag. No new
  third-person actor is introduced. In 3:4 the source says “I have” and
  “before me,” not the reference's “you have” and “with me.”
- Revelation 3:7 has vocalized and tagged plural “keys.” In 3:8 the suffix
  on “close it” resumes the door; English absorbs it into the relative
  “which ... to close.” The word and name belong to the speaker, and the
  keeping and non-denial to the addressee. In 3:9 “some from” expresses
  partitive *min*, not an added count; the first giving verb is participial,
  followed by the future making them come, pay homage, and know.
- Revelation 3:10 has no “hour” noun. The speaker's patience and promise to
  keep the addressee remain distinct from the trial coming upon the world.
  The inhabitants' suffix anticipates “of the earth,” rather than adding
  another possessor. In 3:12 the first “God” lacks “my,” while the later
  two occurrences have it. There is no “heaven” word. Both the new city
  and the speaker's own new name remain. In 3:14 the creation suffix
  anticipates God, and *rishitha* is rendered “beginning.”
- Revelation 3:15 keeps cold before hot and renders the evaluative
  participle with *hwa* as “it would be fitting,” rather than inserting
  “I wish.” In 3:16 the lukewarm clause begins with “and”; no causal
  “because” is imported. The imminent vomiting retains its object suffix.
- Revelation 3:17 has an actual person shift: second-person perfect “you
  said” and “that you are rich,” then first-person “I have grown rich” and
  “I have no need,” then second-person “you do not know.” Punctuation marks
  the shift from indirect report to quoted continuation, rather than
  silently changing source pronouns. The causal sentence continues with
  “I counsel you” in 3:18. The source has no “blind” adjective in 3:17.
- Revelation 3:18 retains the passive tested gold, white garments, shame of
  nakedness, and eyesalve. There is no separate “eyes” noun. The applying
  verb is imperative, with eyesalve fronted as its object. In 3:19 the
  emphatic initial “I” is rendered “As for me,” followed by the fronted
  object and repeated first-person subject of rebuking and chastising.
  In 3:20 perfect “I have stood” precedes future “I will knock”; no “to
  him” is added after entering. In 3:21 overcoming and sitting with the
  Father are perfect, and the two throne possessors remain distinct.

Source checks for Revelation 4:

- Reviewed all 11 verses and 209 source positions against exact forms,
  vocalization, all gloss alternatives, numeric morphology, source roles,
  and continuous English. Short contiguous constructions render numbers,
  possession, and grammatical subject placement; no long source clause is
  collapsed into a single link. Spatial anticipatory suffixes before an
  explicit throne noun are absorbed into natural English rather than
  duplicated as “around it of the throne.”
- Revelation 4:1 has no “first” adjective for the voice. “Spoke” is perfect,
  and the quoted voice, rather than the seer, promises “I will show you.”
  The final passive “is given” and infinitive “to take place” preserve the
  supplied construction rather than importing an unsourced necessity verb.
  Narrative copulas and participial descriptions are rendered in the past
  where they describe the vision introduced by “I saw” and “I was”; this is
  a contextual temporal choice, not a claim that participles are perfect.
- Revelation 4:3 retains the distinct form, appearance, stone, jasper,
  sardius, cloud-rainbow, throne, and emerald expressions. English supplies
  copulas for the nominal predicates. In 4:4 both twenty-four counts remain,
  along with the elders, white garments, their heads, and crowns of gold.
- In 4:5 the first throne noun is plural, while the later throne is singular.
  The source order is thunders, lightnings, and voices. The seven lights are
  burning, with no separate “fire” noun supplied. The final relative
  identifies the lights as the seven Spirits of God. In 4:6 the sea and
  glass are distinct words. The eyes' spatial phrases refer to the creatures'
  front and rear; their suffixes do not introduce other participants.
- Revelation 4:7 uses the ordinary numerical ordinal idiom for the second,
  third, and fourth creatures. The third creature's possessive existential
  is “had a face”; the source's plural-shaped face noun is idiomatic singular
  English here. The flying participle describes the eagle comparison.
- Revelation 4:8 explicitly includes each creature's standing, its claws,
  “and upward,” six wings around it, and the creatures' interior eyes.
  None is omitted to match the shorter reference English. The negative
  possessive existential is “they have no cessation,” followed by the
  saying infinitive. “The holder of all” retains both words of the divine
  title. The final existential-plus-perfect construction is “the one who
  was,” followed by distinct “and is” and “and is coming.”
- In 4:9, *qobbal taybutha* is a two-word idiom rendered “thanksgiving,”
  not independent “acceptance” and “favor.” The
  [Brock–Kiraz dictionary entry](https://sedra.bethmardutho.org/lexeme/get/1203)
  explicitly records the compound as gratitude/thankfulness.
  The perfect giving form precedes a plural four-creature subject. The
  English uses plural agreement and a completed temporal condition,
  “whenever ... have given,” followed by the future falling, worshiping,
  and casting of 4:10. This resolves the unusual agreement/context without
  silently treating the perfect as a participle.
- Both “Amen” words in 4:9–10 remain as parenthetical affirmations. The
  temporal clause continues across the verse boundary. “Down will fall”
  permits grammatical inversion while keeping the verb before the elders;
  worship's duration and Amen precede its delayed living-one object.
  The final saying clause retains the explicit “while” relation. In 4:11
  “You are worthy” accounts for the second-person worthy form and copular
  pronoun; “our” belongs to both Lord and God. There is no “Holy” adjective
  or separate “by you they exist” clause. The conclusion is “through your
  will they existed and were created,” with both final perfect forms kept.

Source checks for Revelation 5:

- Reviewed all 14 verses and 221 positions against exact source forms,
  all gloss alternatives, numeric morphology, suffix roles, and continuous
  English. Nominal copulas and participial descriptions use the observed
  narrative context; perfects, infinitives, prohibitions, and future
  promises remain distinct. Anticipatory object/genitive suffixes are
  absorbed naturally into their following explicit book, hand, or Lamb
  referent instead of duplicated in English.
- Revelation 5:1 retains the right-hand expression, inside and outside
  inscription, sealing, and seven seals. In 5:2 the source says “another”
  mighty angel. Revelation 5:3 includes opening, loosening its seals, and
  seeing the book; 5:4 instead ends with opening and loosening its seals.
  The weeping participle with past auxiliary is “I was weeping.”
- Revelation 5:5 puts the perfect overcoming verb before the Lion. English
  resumes its subject in apposition: “he has triumphed—the Lion ... the Root
  of David.” The later verb is the distinct future “He will open.” There
  is no “seven” adjective before the seals in this verse. In 5:6 the throne,
  four living creatures, and elders all precede the standing Lamb; none is
  dropped to match the shorter reference. The seven horns, seven eyes,
  seven Spirits of God, and sending throughout the earth remain. In 5:7
  the source says “hand,” without the reference's additional “right.”
- Revelation 5:8 has a singular bowl for each one, with a harp and gold.
  The distributive construction is rendered “there was for each one of
  them,” using short spans. The possession clause is parenthesized so the
  plural praising/speaking relative at 5:9 resumes the creatures and elders.
  The source's plural-shaped *basma* is English mass “incense”; its relative
  therefore takes English “is,” with the prayers as its metaphorical
  predicate. The [Comprehensive Aramaic Lexicon's bsm entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=bsm+N)
  records incense as often plural-only. This does not change the singular
  bowl into plural bowls or invent a second set of containers.
- In 5:9 the cognate praise/praise construction is rendered in its musical
  context as “singing a new song of praise,” after the preceding harps.
  The slaughtering is passive and second-person; the buying is active,
  second-person, with first-person plural “us,” followed by “your blood”
  and “for God.” Tribes, peoples, and nations remain plural. There is no
  tongue/language noun in this source verse.
- Revelation 5:10 retains third-person “them,” first-person “our God,”
  singular “a kingdom,” and both plural “priests” and “kings,” in that order.
  The last verb is third-person plural imperfect, “they will reign.” The
  [CAL mlk entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=mlk+V)
  explicitly records the causative stem's intransitive Syriac sense “to
  rule”; it is not forced into an unsourced act of making others kings.
- Revelation 5:11 keeps both seeing and hearing, the comparison before the
  voice, its many angels, the throne/creatures/elders, and both large-number
  expressions. The existential plus past auxiliary belongs to “their number
  was.” Revelation 5:12 has the same vocalized second-person worthy form as
  5:9, rendered as direct address: “You are worthy, Lamb who was slaughtered.”
  All seven objects of receiving remain separate.
- Revelation 5:13 fronts the creation and all-in-them topic. English supplies
  the topic marker “as for,” and renders the additive hearing clause as “I
  also heard.” The postposed source copula remains “exists,” following the
  relative clause's spatial phrases. The seer hears the creation's plural
  voices speaking; the throne's occupant and the Lamb are joint recipients
  of blessing, honor, glory, and dominion. The reference's throne occupant
  speaking to the Lamb is not imported. In 5:14 the creatures' saying is
  participial, while the elders' falling and worshiping are perfect. No
  additional twenty-four count or living-forever phrase is supplied there.

Source checks for Revelation 6:

- Reviewed all 17 verses and 268 source positions against exact forms,
  vocalization, complete gloss alternatives, numeric morphology, suffix
  roles, and running English. Short contiguous grammatical constructions
  render ordinals, passives, possession, and reciprocal objects. Narrative
  descriptions use contextual past English without relabeling participles
  as perfect forms. The fig-tree comparison retains generic present time.
- In 6:1 the Lamb is the subject of the opening verb, not its object. In 6:2
  both hearing and seeing remain, as do the three separate expressions for
  being victorious, conquering, and the purpose of conquering. The recipient
  passives “he was given” preserve the rider's receiving role. In 6:4 the
  source has no “another” adjective for the red horse, and the reciprocal
  killing clause remains separate from taking peace from the earth.
- The third seal's opening in 6:5 is passive. The commands in 6:3, 6:5, and
  6:7 are only “Come,” without an additional seeing command. There is no
  extra “I looked” before the black horse in 6:5. The fourth-seal sentence
  in 6:7 does not repeat “fourth” for its living creature. In 6:6 no “four”
  modifies the creatures, wine precedes oil, and the prohibition addresses
  both. The grain unit is the source's cab measure, not an assumed Greek
  choenix or a modern volume conversion; the price remains a denarius.
- Revelation 6:8 has no “behold” word. The pale horse's rider is Death,
  with Sheol following him. Authority is given to a singular recipient over
  a fourth part of the earth. The final beast/animal is singular and has no
  additional “ravenous” adjective. Anticipatory possessives in the rider's
  name and the earth's fourth part are rendered as ordinary English genitives.
- In 6:9 the testimony is “of Jesus,” rather than the reference's “to the
  Lamb.” The final relative is past possession, “which they had.” In 6:10
  the actual form *daman* is interpreted as the blood noun with first-plural
  possession, “our blood,” despite the export's homographic MN “who” lemma
  and absent suffix annotation. The blood-avenging syntax and the following
  “from/upon the earth's inhabitants” support that reading. The
  [CAL blood entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=dm+N)
  confirms the DM noun; it does not itself disambiguate this inflected
  occurrence. The raw annotation remains unchanged, and the interpretation
  is documented rather than silently treating the machine gloss as authority.
  The past possessive dative in 6:9 is idiomatic English “they had.” In
  6:10 the blood-recompense construction uses Syriac “from” for those
  answerable for the blood; English “avenge ... upon” preserves that role.
  The [CAL demand/avenge entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=tb%28+V)
  documents the vengeance sense and its blood/source constructions.
  The judging/avenging participles form an ongoing appeal rendered with
  prospective English “will you not,” not past acts.
- Revelation 6:11 retains the distributive white robe, the instruction to
  rest, both time nouns, and the fellow servants and brothers with their
  possessives. The completion verb is nominalized as “completion is reached
  also for ...,” preserving the people whose completion is in view while
  keeping separate word owners. No explicit “number” noun is invented.
  The final killing comparison supplies English ellipsis “they too were”
  for the earlier martyrs, retaining the source's additional “also.”
- In 6:12 the hair-sackcloth comparison precedes the sun's becoming black;
  the earthquake, blackness, and whole moon's blood comparison each retain
  their own copular/result construction. In 6:13 the fig tree casts its own
  unripe figs because of the strong wind when shaken; the causal wind phrase
  remains before the temporal clause. Revelation 6:14 has plural writings/
  scrolls and a plural rolling verb. Its first separating verb is tagged
  singular with the plural-shaped heavens; English uses plural concord.
  Every mountain and every island remain, with the collective/distributive
  “their place” before the movement verb.
- Revelation 6:15 preserves the coordinated ranks and the free-person idiom.
  “Armies” applies the military sense of *hayla* in context; the
  [CAL army/force entry](https://cal.huc.edu/oneentry.php?cits=all&lemma=xyl+N)
  supports that sense beyond the export's power/strength glosses. The hiding
  verb and selves form a reflexive English expression; rocks are not replaced
  by an unsourced cleft noun. In 6:16 only the face of the Lamb is named:
  no throne-occupant or wrath phrase is imported from the longer reference.
  The unmarked English imperative “hide” does not require choosing between
  the export's singular tag and its plural addressees. Revelation 6:17 keeps
  the plural possessor “their wrath” and the question about ability to stand.

Source checks for Revelation 7:

- 7:1-3: The seer is first person; four angels stand and hold four winds.
  The demonstrative in 7:1 is singular, and no second "of the earth" follows
  "four winds." The wind is prevented from blowing on the earth, sea, or any
  tree. The other angel ascended (perfect), has the living God's seal through
  an existential possessive dative, and cries to the four angels who were
  granted permission to harm. The prohibition addresses them in the second
  plural; "we seal" is first plural. The servants' anticipatory possessive is
  resolved as "the servants of God," with no unsupported "our" on God.
  The seal's location is the source's "between their eyes."
- 7:4-8: The total is 144,000 from all Israel's tribes, with twelve thousand
  in each named group. The source order in 7:7 is Simeon, Issachar, Levi.
  Asher and the tribal Joseph follow the contextual names, rather than
  Murdock's spelling or a lexeme's unrelated biographical gloss. The list
  continues across verse boundaries; "sealed" occurs at 7:4 and finally
  7:8, without inventing a separate sealing verb for each entry. Repeated
  anticipatory tribe suffixes are idiomatic "the tribe of X."
- 7:9-12: The seer sees a great multitude; the negative existential plus
  ability construction makes its number beyond anyone's ability to count.
  The short four-position link retains that grammatical unit. Peoples,
  tribes, nations, languages, standing, white robes, and palms in their hands
  are retained. A repeated English "from" keeps the mixed singular and
  plural nouns grammatical while sharing the source preposition.
  Anticipatory suffixes become "before the Lamb" and "around
  the throne." The coordinated "and to him who sits" remains explicit.
  Angels stand around the throne, elders, and four living creatures, then
  fall on their faces before the throne; no absent worship verb or possessive
  on that throne is imported. The following saying clause continues the
  sentence. Praise precedes blessing; "thanksgiving" renders the compound
  qobbal taybutha, and both occurrences of "our God" retain first-plural
  possession. Participles describe the ongoing scene in contextual English
  past, while the seated figure's description remains present.
- 7:13-14: One elder answers and speaks to the seer. English passive grammar
  preserves the answer's actor while keeping source order.
  The white-robed people's identity and origin remain questions. "My lord"
  and "you know" address the elder, with the repeated second-person subject
  expressed as one emphatic English clause. The people came from great
  affliction and actively washed their own robes and made them white in the
  Lamb's blood; the plural actors and robe object remain distinct.
- 7:15-17: "Therefore" renders "because of this"; they serve him day and
  night in his temple. The seated one will shelter them: the verb's covering
  and protective sense with al is independently supported by CAL's gnn
  causative entry (https://cal.huc.edu/oneentry.php?cits=all&lemma=gnn+V).
  English fronting preserves the sun clause's source order, every negative,
  and "any heat," with no unsupported "any more." The Lamb feeds and leads
  them to life and to fountains of water, two coordinated destinations,
  rather than an imported "living water" modifier. The last wiping verb is
  third singular with an implicit subject; no absent "God" noun or explicit
  change of actor is added. All tears and their eyes are retained.

Source checks for Revelation 8:

- 8:1-2: The seventh seal is opened by the implicit third-person subject;
  silence follows for about half an hour. The comparative particle with a
  measure gives the approximate duration. The seer is first person; seven
  angels were standing before God and received seven trumpets. The standing
  participle and past auxiliary form one ongoing past description; the
  following passive and plural dative retain the trumpet recipients. Short
  links keep the ordinal seal phrase and passive relative grammatical.
- 8:3-4: Another angel came and stood at the altar. Existential possession is
  idiomatic "he had," rather than a fabricated holding verb. Gold modifies
  the censer; no second gold adjective is supplied for the later altar.
  Much incense was given to him so that he might offer it with the prayers
  of all the saints. The imperfect complement expresses purpose, and its
  understood offering object is the incense. "All the saints" resolves the
  anticipatory suffix without inventing "their saints." Incense is an
  English mass noun despite the Syriac plural shape. The smoke rises with
  the prayers from the angel's hand before God, retaining that source order.
- 8:5-7: English passive inversion retains the taking verb before the angel
  and censer, while preserving the angel as actor. He fills the censer with
  fire on the altar and casts it onto the earth; partitive min becomes
  English "with" for the filling material. Peals of thunder, voices, flashes
  of lightning, and an earthquake retain the actual list order and the
  source plurals. The trumpets are on the angels, expressed as possession;
  they prepare themselves. The first sounds; hail and fire are mingled with
  water, not blood, and are cast onto the earth. A third of the earth and
  trees, and all the grass of the earth, burn. The intransitive burning
  predicates retain their affected subjects; the first angel is not made
  the actor of those burning verbs.
- 8:6-13: The cry/sound root is rendered in the explicit trumpet context as
  "sound." This contextual interpretation is supported by CAL's z(q entry,
  which records horn sounding in the simple stem and a horn being sounded
  in the corresponding passive causative, while its causative entries also
  cover loud cries and summons (https://cal.huc.edu/getlex.php?coord=5610110631&word=10).
  This does not claim that every stem has a separate dictionary gloss
  "blow a trumpet." The ordinal second, third, and fourth identify the
  successive participants without importing an absent angel noun each time.
- 8:8-11: Something like a great burning mountain falls into the sea; there
  is no separate fire noun and no passive casting verb in 8:8. A third of
  the sea becomes blood. The short three-position copular units in 8:8 and
  8:11 keep the fraction and its referent together with the becoming verb;
  their final predicates remain separately linked. A third of the living
  sea creatures dies and a third of the ships is destroyed. The relative
  existential "life in it" is idiomatic "which had life." The plural creation
  noun means creatures here; CAL independently includes creation/creature
  and Syriac examples (https://cal.huc.edu/cal_entry_web.php?lemma=bryyh+N).
  A great star falls from heaven, burning like a flame, not a lamp; it falls
  upon rivers and springs of water. CAL's eye/spring entry confirms the
  spring sense with water (https://cal.huc.edu/oneentry.php?cits=all&lemma=%28yn+N).
  Its name is Wormwood. The waters become like wormwood, retaining the
  comparative particle. Many people die because the waters have been made
  bitter; the inverted passive keeps the waters as the postverbal subject.
  No absent "from the waters" phrase is added before the cause clause.
- 8:12-13: The affected fractions of sun, moon, and stars are struck, and a
  third of them is darkened; the source has one darkening verb. English
  singular agreement follows the collective fraction. The day does not show
  light for a third of its duration, and the night likewise; light is
  understood from the luminaries and darkness, while the negative, showing
  verb, fraction, and day suffix remain represented. Anticipatory fraction,
  star-name, and earth-inhabitant suffixes are resolved through their
  following referents, without duplicate possessors. The seer only hears
  one eagle flying in heaven and saying three woes. The source contains no
  separate seeing, tail, blood, loud voice, or remaining modifier here.
  Singular sound belongs to plural trumpets of three angels, who are ready
  to sound; readiness is a state rather than a completed sounding action.

Source checks for Revelation 9:

- 9:1-4: The fifth sounds in the established trumpet sequence; the seer is
  first person. The star had fallen from heaven, and the key is given to him.
  The pits/wells are plural throughout 9:1-2; the abyss suffix is anticipatory.
  This source has no pit-opening clause in 9:2. Smoke rises like smoke from
  a great kindled furnace and darkens sun and air. Locusts come from it and
  receive the power that scorpions have, with no additional comparison word.
  The locusts must not harm earth's grass, any green herb, or even trees;
  the exception is people without God's seal between their eyes. "Except"
  renders the but-if construction, and existential non-possession is "did
  not have." Negative scope gives English "or any" for the coordinated herb.
- 9:5-6: The locusts may not kill the people, but the people are to be
  tormented for five months. The torment verb is passive in stem 4, not an
  active command to torment themselves; its English subject is made explicit
  from the preceding object. Their torment resembles a scorpion's when it
  falls upon a person. The imperfect predictions retain seeking death,
  failing to find it, longing to die, and death fleeing from them. Small
  subject/verb links keep the postverbal people and death grammatical in
  English, while the death object suffix remains on "find it."
- 9:7-10: All the appearance/form and comparison words are retained.
  Prepared horses are the battle comparison, a singular crown resembles
  gold, and the locusts' faces resemble human faces. Hair possession is
  idiomatic object-fronted "hair they had"; their teeth resemble lions'.
  The breastplate is singular, like a breastplate of iron; wing sound is
  compared to chariots of many horses running into battle. Tails resemble
  the form of a singular scorpion. "Stings, however, were in their tails"
  retains the postpositive contrastive particle and actual nominal order.
  Their power harms people for five months. Existential possession and
  nominal comparisons receive English copulas in the ongoing vision scene.
- 9:11: The king is the angel of the abyss, named Abaddon in Hebrew.
  The second name is the source participle shara, rendered "Destroyer,"
  rather than an imported Apollyon noun. CAL's shry simple-stem entry
  explicitly includes destroying/breaking with Syriac examples
  (https://cal.huc.edu/oneentry.php?cits=all&lemma=%24ry+V).
  The vocalized armayit is rendered "in Gentile usage" in this naming
  context; it is not silently changed to an Aramaic or Greek language label.
  CAL distinguishes pagan usage from the related Syriac-language adverb
  (https://cal.huc.edu/cal_entry_web.php?lemma=%29rmy%232+A).
  The name/dative/existential phrase becomes idiomatic "his name is."
- 9:12-16: One woe has passed; two more are coming. "After these things"
  begins source verse 13 and completes the prediction across the boundary.
  The sixth angel sounds and the seer hears one voice from four horns of
  the gold altar before God. Its saying clause continues into 9:14, with
  second-singular "Release" addressing the trumpet-bearing sixth angel.
  The four angels bound at the great Euphrates are released; they were
  prepared for the hour, day, month, and year to kill a third of humankind.
  Passive English inversion keeps the release verb before its subject.
  The forces of horsemen number two myriads of myriads; the source's numeric
  terms remain explicit, as does the seer's hearing of their number.
- 9:17-19: The vision gives the seated riders a singular breastplate.
  The difficult material phrase retains fire, chalcedony, and the actual
  genitive "of sulfur" after chalcedony, without inserting a third
  coordinating conjunction or an unsupported color adjective. This is a
  literal visionary description; the material relation is uncertain. The
  same genitive reading occurs in the independently published Syriac text
  (https://www.textgridrep.org/browse/49fh6.0?mode=gallery).
  "Their horses" keeps its genuine possessive, and their heads resemble
  lions' heads. Mouth remains the source's singular collective with a plural
  possessor. Fire, sulfur, and smoke issue in that order in both 9:17-18.
  These are the three plagues killing a third of humankind; "and specifically
  by" marks the explanatory repetition of the three causes. The horses'
  power is in their mouth and also their tails. There is no serpent-tail,
  head, or striking clause in source 9:19.
- 9:20-21: The surviving remainder is not killed by the plagues and does
  not repent. English makes the source relative description a coordinated
  finite statement, preserving both negative clauses and their conjunction.
  The work of their hands and the purpose of ceasing demon/idol worship
  remain explicit. Wood precedes stone. The four-position link in 9:20
  renders the shared delayed ability predicate with its coordinated negative
  infinitives as "able to see or hear," governed by "are not"; this retains
  both negative scopes in grammatical English. "Or to walk" also remains.
  All five explicit negative particles are represented. The final verse
  retains murders, sorceries, and fornication, with their possessors and
  three coordinated from/of phrases; this source has no theft phrase.

Source checks for Revelation 10:

- 10:1-2: The seer sees another angel descending from heaven and clothed
  with a cloud. There is no source adjective "mighty." The rainbow's
  heaven relation is retained, and the sun comparison is his appearance,
  not an imported face noun. His feet resemble fire pillars. Existential
  possession is idiomatic "he had" an open booklet in his hand. The right
  foot is placed upon the sea; the left, however, is upon land. The
  postpositive contrastive particle remains after the left-foot phrase.
- 10:3-4: The angel cries with a loud voice like a roaring lion. The seven
  thunders speak with their plural voices. Short subject/verb units keep
  the postverbal thunders grammatical in English without losing the
  independent voice noun. The seer's preparation and hearing are first
  singular. The source's "of seven" after heaven in 10:4 is read as "the
  seventh heaven," using the same noun-plus-cardinal ordinal construction
  as "the seventh angel" in 10:7. This is an explicit syntactic reading;
  the raw gloss remains "seven." The independently published Syriac text
  confirms the extra word (https://betaportal.textgrid.d.sub.uni-goettingen.de/browse/49fh6.0).
  It is not omitted to match the reference English. Both seal and do-not-
  write commands address the seer in the second singular; the writing
  verb's object suffix is singular "it," referring to what was spoken.
- 10:5-7: The angel raises his hand and swears; no unsupported "right"
  adjective is added to the hand. The first verse continues into the oath
  across the boundary. He swears by the living one, creator of heaven,
  its contents, earth, and its contents; this source has no separate sea
  creation clause. The creation verb's anticipatory object suffix resolves
  into its following object. The negative copular time clause uses one
  short four-position unit so again/further, time, not, and be all remain
  represented in grammatical English. In the seventh angel's days, when
  ready to sound, God's mystery is to be completed. The ma d- construction
  has its established temporal sense "when," rather than the isolated
  pronominal gloss "what" (https://cal.huc.edu/oneentry.php?cits=all&lemma=mh%40d_+c).
  The source's perfect
  completion is rendered prospectively within the oath's future setting,
  and its resumptive conjunction becomes English "then." It is not made
  into an already completed event in the narrator's present. God declared
  this mystery to his servants, the prophets; the active declarer and the
  appositional mystery reference remain distinct.
- 10:8-9: The voice again speaks with the seer and gives second-singular
  go/take commands. The booklet has no repeated open adjective in 10:8.
  The angel's locations are land then sea in that verse, preserving the
  source order. Hand-of-angel suffixes resolve idiomatically rather than
  becoming duplicated possessors. The seer goes to the angel while telling
  him to give the booklet; the participle plus explicit pronoun is first
  person, not an imported third-person speaker. The angel's take/eat
  commands are second singular, with the booklet as "it." The causative
  bitter verb, affected-person dative, and possessed belly form a short
  three-position unit: "it will make your belly bitter." This absorbs the
  repeated possessor without inventing a separate beneficiary. Belly is
  singular. In the mouth it will be like honey; no absent sweet adjective
  is supplied in 10:9.
- 10:10-11: The seer takes the booklet from the angel's hand and eats it.
  The explicit sweet state follows the honey comparison in the source;
  their short three-position comparative unit gives "sweet as honey."
  English inversion retains the bitter predicate before "my belly," with
  the seer as eater and the belly as the affected singular subject. The
  final speech retains passive-participial "given," the second-person
  recipient, again, and the actual time noun, rather than replacing the
  whole commission with an unsupported modal. The prophecy concerns
  peoples, nations, languages, and many kings; the source has no princes
  noun. The final adjective is kept with the immediately preceding kings
  in a short noun/adjective unit.

Source checks for Revelation 11:

- 11:1: The reed's likeness/form and rod are separate source nouns. Standing is an active participle with a past auxiliary; saying is another active participle coordinated with it. English inversion preserves the angel as the standing speaker. The commands address one person; the final singular “in it” refers to the temple.
- 11:2: The actual text has `דלגו מנ`, inward/inside, not Murdock's outer court. The CAL entry for `gw` explicitly lists `lgw mn` as “inward from” (https://cal.huc.edu/oneentry.php?cits=all&lemma=gw+N); “inside of” retains the compound relation without harmonizing the text. The separate `מן לבר` supplies outside, and the causative imperative is rendered “leave outside.” The feminine object of do-not-measure is the court. The holy city remains the object of trampling for forty-two months.
- 11:3: Give/grant governs an infinitive to prophesy; “permission” supplies the English complement of that construction rather than inventing a second finite prophecy verb. The genuine first-person suffix is “my witnesses.” A four-position unit keeps the duration, 1,260 days, together in English. The witnesses, not the granting speaker, are clothed in sackcloth during their prophesying.
- 11:4: The two olive trees and two lampstands stand before the Lord of all the earth. The feminine singular suffix on all anticipates earth; it does not add an independent “his.” The relative clause fronts the before-phrase to retain ascending source links.
- 11:5: The fire is the subject of goes-out and consumes; mouth is singular with plural possessors, and enemies is plural with the same possessors. The second attacker clause is singular/generic, but the actual final dative `להונ` is third plural. English “to such people” interprets that plural as the generic attackers just mentioned, rather than changing it to singular “him.” “It is granted” retains GIVEN and “to be killed” retains the passive infinitive. This contextual referent resolution is an interpretation of an unusual source construction.
- 11:6: Both existential-plus-dative constructions express having power. Close governs heaven; the subordinate three-position unit retains the negative, rain and descend. Heaven and sackcloth elsewhere use their normal English singular/mass forms despite Syriac plural forms. Their prophecy remains explicit. Every plague and as-often-as-they-wish retain the universal and quantity/relative construction.
- 11:7: `מא ד` is temporal when; the perfect completed testimony is anterior to the predicted battle. The creature ascends from the sea, not an abyss. The short make-war-with-them unit retains the participants and the noun war. Overcoming and killing have their own explicit plural objects.
- 11:8: The bodies and streets are plural. English supplies a locative predicate, “will lie on,” for the nominal location following the future killing. Spiritually, both city names, their Lord and the passive crucifixion remain explicit; the source has no additional “also.”
- 11:9: SEE is an active participle. “There will be observers ... of their dead bodies” nominalizes that seeing predicate to keep the viewers' origins, nations/tribes/languages/peoples, in source order without a sentence fragment. The prospective setting comes from the surrounding prediction. The repeated bodies, conjunction and negated permission are retained, as is passive placement in plural graves. Three days and a half is the idiomatic duration, not half of the whole three-day period.
- 11:10: Both rejoice predicates remain, with make-merry distinguishing the second. Gifts precedes send in the source; the short two-position unit keeps that object fronted. The inhabitants' feminine suffixes anticipate earth. The two prophets' torment is third plural perfect, not an imperfect or singular verb.
- 11:11: The first phrase contains a living adjective: “a living spirit.” The second is genuinely “a spirit of life.” Both clauses are retained: entering them and falling upon them, separated by their standing on their feet. Great fear WAS upon the observers. Narrative perfects supply the setting for the relative active participle “who saw.” There is no additional “these” before the duration.
- 11:12: The plural imperative come-up-here is distinguished from the subsequent perfect ascent. A three-position unit gives normal English order to enemies watching them while retaining the genuine plural possessor on enemies and the plural object.
- 11:13: The hour has an explicit that. One out of ten expresses the fraction of the city; “parts” supplies English fraction grammar. The passive deaths retain both source nouns, names and persons, in the metonymic “persons counted by name,” followed by seven thousand. Counted explains the enumeration, not a separate narrated counting event. The rest were in fear and gave praise to God IN HEAVEN, the latter phrase being present in this source.
- 11:14: The source has two woes with a plural departed verb, followed by the third woe coming immediately. It is not a singular second woe.
- 11:15: The seventh angel cried out; great voices are in heaven, with no additional thunders. A three-position copular unit renders the world's kingdom becoming our God's and his Messiah's. The kingdom suffix anticipates world, while our God and his Messiah retain their genuine possessors. The reign verb is third singular perfect, form 5, rendered “has reigned” in the proclamation's completed perspective, not mislabeled a participle or silently replaced with a future. The age of ages preserves the source's singular/plural expression.
- 11:16: The twenty-four elders sit before God on their own plural thrones, then fall on their faces and worship God. No throne is added before God. The relative clause fronts before-God, preserving the location of the sitting. The saying infinitive at the start of the next verse continues this sentence.
- 11:17: We-give-thanks is first plural, addressed to singular you. Holder of all retains both source words. The two third-person singular existential expressions plus past auxiliary become the appositive “the one who is and who was,” referring to the God being addressed. There is no coming clause. Taking up the addressee's great power and reigning are second-singular perfects.
- 11:18: The nations were angry; the addressee's wrath and the time of the dead share came. English repeats the shared verb at time to clarify the coordinated clause, without adding another event. The dead are to be judged passively. Giving a reward and destroying are second-singular imperfects, contrasted with the third-plural perfect “who destroyed” the earth. Prophets, saints and those fearing the addressee's name all remain explicit, including the small with the great.
- 11:19: The opened temple is in heaven, without an added God noun. The ark is seen in the temple; the standalone his belongs to covenant and supplies no additional temple possessor. The final list follows the actual order: lightnings, thunders, voices, earthquake and great hail. Short copular/passive units keep English grammatical while every source position occurs once in ascending order.

Source checks for Revelation 12:

- 12:1: The great sign is seen passively in heaven. Clothed is a passive participle; the
  moon is under her plural feet, and a crown of twelve stars is on her head. The crown
  and stars remain separate source nouns in their actual order.
- 12:2: Pregnant, crying aloud, labour, also, torment and giving birth all remain
  explicit. The labour form `ומחבלנ` is encoded as a feminine plural active participle
  amid predicates about one woman. English follows the singular woman context without
  changing that source encoding or inventing other women. CAL's labour homonym
  explicitly cites the same Revelation 12:2 form under pael “to be in labor”:
  https://cal.huc.edu/oneentry.php?cits=all&lemma=xbl%232+V . The narrative setting
  supplies past auxiliaries; the source also/`אף` remains explicit.
- 12:3: Another sign is seen, followed by a great dragon of fire. The source supplies
  fire, not a separate red adjective. Existential plus dative gives who-had. The dragon
  has seven heads, ten horns and seven diadems on his HEADS, plural despite Murdock's
  singular head.
- 12:4: Dragging is an active participle, rendered with a narrative past auxiliary
  beside the perfect casting. The third of the stars and the standing dragon are
  distinct predicates. Ready-to-give-birth retains the prepared participle; `מא ד` is
  temporal when within the dragon's purpose, not a free-standing what. The masculine
  object suffix on devour anticipates the explicit son; her on son is a genuine feminine
  possessor.
- 12:5: The male son is destined/prepared to shepherd all the nations, with a rod of
  iron. Shepherd preserves the feed/tend lexeme rather than replacing it with an
  unrelated ruling verb. All's plural suffix anticipates nations. The son is snatched
  away passively; her son and God's his-throne remain distinct genuine possessors.
- 12:6: The first place/region expression functions as where. The existential, past
  auxiliary and feminine dative form the short three-position grammar unit “she had.” A
  two-position place/there unit gives normal English order to the actual place noun and
  its location. Prepared is passive, with God as its source/agent. Nourish is third
  plural with a feminine object, retained as they-might-nourish-her. A separate
  four-position duration unit renders 1,260 days in normal English number order.
- 12:7: War is in heaven. Michael and HIS angels are fighting with the dragon; the
  dragon and HIS angels fought. The first fight is an active participle, placed in the
  established past narrative; the second is perfect. The sentence continues into the
  next verse.
- 12:8: Both negatives remain: they could not prevail, and no place was found for them
  in heaven. Could-not-prevail supplies the understood fight complement of be-able, not
  an additional source event. The source has no separate any-more expression.
- 12:9: The dragon is cast out passively. `רישא` is rendered “of the beginning” with
  serpent, using the beginning/origin sense also listed by CAL for this lexeme:
  https://cal.huc.edu/oneentry.php?cits=all&lemma=ry%24+N . Accuser and Satan are
  explicit names. Deceived is perfect, form 5, not an active participle. All's feminine
  singular suffix anticipates earth. The second casting has an explicit on-earth
  location, and his angels are cast with him.
- 12:10: The voice is FROM heaven and said is perfect. Behold is retained; “there has
  been” renders the perfect existential in the proclamation of established deliverance,
  power and God's kingdom. This source has no Messiah/dominion clause here. The causal
  prefix on cast introduces the passive expulsion of our brothers' accuser. Accusing is
  an active participle with a past auxiliary; the source order is night AND day, before
  OUR God.
- 12:11: They overcame by the Lamb's blood and through the word of HIS testimony. The
  testimony suffix is genuinely third masculine singular, not Murdock's their; the
  English leaves that possessor as his, with the Lamb the nearest named antecedent. The
  explicit negative governs loving their own life/soul, idiomatically plural lives for
  the plural possessors, even unto death.
- 12:12: The surface `מטלהנא` combines because-of (`מטל`) and this (`הנא`), supplying therefore despite its missing dictionary gloss.
  The plural imperative addresses heavens and those lodged in them; the latter is a
  passive-participle state, rendered are-lodged. Woe is to earth and sea, because the
  Accuser is descending TO THEM: the suffix is third plural, not Murdock's second-person
  you. Having great wrath and knowing are distinct; having attaches the wrath to the descending Accuser without a distant relative clause; while renders the temporal particle.
  The final existential/dative expresses that little time is available to him,
  preserving the amount, time and possessor without importing a separate shortness verb.
- 12:13: When the dragon saw is followed by a passive perfect casting, rendered as
  anterior had-been-cast in the narrative. The same dragon persecuted the woman who bore
  the male. No additional child noun is supplied where the source only has male.
- 12:14: Two wings of the great eagle are given to the woman. English plural agreement
  follows wings although the preceding Syriac passive is encoded singular. She might fly
  to her place in the wilderness TO BE NOURISHED: this is a passive infinitive, not a
  second finite event. There, a time, plural times and half a time all remain; there is
  no source conjunction before times. From-before plus the face expression is rendered
  away-from-the-presence of the serpent, with the face suffix anticipating the serpent
  rather than adding another possessor.
- 12:15: The serpent casts waters from his mouth after the woman, like a river. The
  final feminine carrying/bearer noun, water and make-her predicate form a short
  three-position causative unit: “so that he might cause her to be carried away by
  water.” This interprets the river's intended effect on the woman in context; it does
  not turn the waters into a plural object of carry or drop the explicit feminine object
  of make.
- 12:16: Earth helps the woman and opens ITS mouth; the river is swallowed. The object
  suffix on swallowed anticipates the explicit river. The source names the DRAGON as the
  one who cast the river from HIS mouth, rather than Murdock's serpent. These mouth
  possessors remain distinct.
- 12:17: The dragon is angry with the woman and goes to make war with the rest of her
  offspring. They keep God's commandments and have Jesus' testimony. The commandment
  suffix anticipates God; the testimony suffix anticipates Jesus. The existential/dative
  supplies have. The actual source chapter ends here, without Murdock's final
  standing-on-the-sand sentence.

Source checks for Revelation 13:

- 13:1: The source opens with FIRST-person singular stood on the sand of the sea, then
  I-saw, rather than Murdock's previous-chapter third-person standing sentence. The
  ascending beast has ten horns, seven heads and ten diadems. Name is SINGULAR on its
  plural heads, not Murdock's names. Short ascending/beast and existential/dative units
  give normal English order and possession.
- 13:2: Likeness is an actual noun, retained in was-in-the-likeness of a leopard. The
  feet are compared with a BEAR, not Murdock's wolf; lions is plural in the mouth
  comparison. English supplies the understood feet/mouth in those comparisons. The
  give/to-it/dragon unit retains the dragon as giver and the beast as recipient. HIS
  power and HIS throne belong to the dragon, distinct from the beast's feminine suffixes
  on its feet and mouth.
- 13:3: One of its heads is as though battered to death. The export has no gloss for
  `פעיעתא`; Ginoskos lists the exact lemma `ܦܥܝܥܐ` as battered/beaten/bruised/worn and
  the related `ܦܥ` as beat/wound
  (https://ginoskos.com/dictionary?o%5Bascension%5D=desc&o%5Bcolumn%5D=lemma&p%5Bpage%5D=74&p%5Bsize%5D=20).
  Battered follows that lexical evidence, with to-death supplied by the next source
  word, rather than guessing from an empty gloss. The separate wound-of-its-death
  construction is idiomatically its-mortal-wound, healed passively. All's suffix
  anticipates earth. Following renders the after/behind relation of the earth's
  amazement toward the beast.
- 13:4: Both worship clauses remain: worship of the dragon because he gave authority,
  then worship of the beast. The saying infinitive introduces BOTH questions, including
  who resembles this beast, absent from the reference. Fight-with-it uses the fighting
  sense of the infinitive's root in context; ability is a separate active participle.
- 13:5: A mouth and authority are given to the beast in two separate passive clauses.
  Speaking is an active participle modifying mouth; great things is plural, and
  blasphemy is a mass noun. Authority is to act for forty-two months, with the
  three-position duration retaining both numerals and months.
- 13:6: Opened is a feminine singular perfect referring to the beast. Blaspheme occurs
  twice: an infinitive, then an imperfect purpose clause. Name and lodging have NO
  possessive suffixes; lodging belongs to THOSE WHO ARE LODGED in heaven. The latter is
  a passive-participle state. The English keeps the against/in relations with both
  objects without importing Murdock's his-name/his-tabernacle or a separate and-them
  object.
- 13:7: The permission to make war with the saints and overcome THEM precedes the giving
  of authority, unlike the reference order. All's feminine plural suffix anticipates the
  list of plural tribes, nations, languages and peoples. The two giving clauses remain
  passive; the two actions granted are infinitives.
- 13:8: The future worship predicate becomes English passive worship-will-be-offered TO
  the beast BY all the inhabitants, preserving both actor and recipient without a long
  reordered span. Those who are not inscribed are the people themselves: this source has
  no names noun here. The book of life is further identified as that of the Lamb.
  `קטילא` is slain/killed, despite the erroneous SEDRA stain gloss; CAL explicitly lists
  the derived adjective qṭīl as murdered/killed
  (https://cal.huc.edu/oneentry.php?cits=all&lemma=q%2Bl+V). This nominal adjective is
  not mislabeled as numeric tense 6 in the export. The source says BEFORE the plural
  foundations of the world, not Murdock's from; the foundation suffix anticipates world.
- 13:9: The existential plus dative is idiomatic has; ears is plural. The final
  third-singular imperfect has the exhortative force let-him-hear, rather than an
  invented second-person imperative.
- 13:10: Lead-away is an ACTIVE participle in a generic captor clause; others supplies
  its understood transitive object. The captor goes into captivity, and whoever kills
  with the sword will be killed by the sword. The latter is passive imperfect, not a
  passive participle. Here is FAITH then ENDURANCE of the saints, in the actual source
  order.
- 13:11: The second beast ascends from earth and has two horns. Resemble is feminine
  SINGULAR, so the BEAST itself resembles a lamb; the source does not attach that
  comparison merely to plural horns. The existential/past/dative unit supplies it-had.
  Speaking is an active participle with an explicit past auxiliary, like the dragon.
- 13:12: The first beast's authority is fronted as object, with all-of-it resuming
  authority. Exercise and cause are imperfects, rendered prospectively will in the
  vision's prediction; the preceding past vision and this prospective description remain
  distinct. Before-HIM has a genuine masculine singular suffix: the English leaves its
  antecedent unresolved rather than silently changing the source form or confidently
  identifying a different actor. The complete earth-and-dwellers worship clause, omitted
  by the reference, remains. English normalizes the coordinated result as
  cause-them-to-worship. A four-position relative/passive unit gives
  whose-mortal-wound-was-healed, retaining relative, healing, wound and its-death
  relation.
- 13:13: The beast will perform great signs; fire is fronted as the object it will cause
  to descend. Both make/do predicates are imperfects; descend is infinitive. From
  heaven, on earth, and before human beings are three distinct relations. The
  great-signs noun/adjective unit does not hide another source actor or event.
- 13:14: Deceive is imperfect, rendered prospectively. The means are signs granted to
  the second beast TO PERFORM before the first beast, a full clause absent from the
  reference. English plural were-granted agrees with signs although the preceding Syriac
  passive is encoded singular. Telling is the continuing force of the saying infinitive.
  The earth-dwellers are told to make an image FOR the beast which has the sword wound
  AND LIVED; lived is perfect, not a future recovery verb.
- 13:15: Granted and give are separate source verbs; the gift is BREATH/spirit to the
  beast's image, not an independent life lexeme. The following cause verb is feminine
  singular, identifying the SECOND BEAST as the one causing the deaths, rather than the
  masculine image. The source has no image-speaking clause. The anticipatory masculine
  object at to-it/the-image is rendered as one short object unit. Those refusing worship
  are to be killed passively.
- 13:16: The prospective cause predicate is bring-it-about, with both the
  for-all-of-them scope and the later to-them recipient retained. Small precedes great;
  rich and poor are followed by MASTERS AND SERVANTS, not Murdock's bond and free. A
  mark is to be given passively on their RIGHT HANDS, plural, or on the space BETWEEN
  THEIR EYES. Space supplies normal English grammar for on-between, without substituting
  an absent forehead noun.
- 13:17: This clause continues the mark arrangement: no one may buy or sell AGAIN except
  the marked person. The buy/sell distinction follows peal versus pael vocalization of
  the same root; there is no independent ability word. The mark is OF THE NAME of the
  beast, not mark-OR-name. On-him retains the actual locative suffix, while its-name
  refers to the beast.
- 13:18: Here-is-wisdom has an anticipatory feminine singular existential suffix, not a
  second owner of wisdom. And-whoever-has-within-him understanding retains the relative
  prefix, conjunction and in-him relation. Let-him-calculate is a third-singular
  imperfect exhortation; its object suffix anticipates the explicit number. A short
  number/is/for unit moves postpositive FOR into natural English order. The number is of
  a human being, followed directly by SIX HUNDRED AND SIXTY AND SIX; no extra
  its-number-is clause is imported.

Source checks for Revelation 14:

- 14:1: The Lamb is standing on Mount Zion, with 144,000. Both HIS name and HIS Father's
  name remain. The existential/on-them relative becomes on-whom-were, with normal
  English agreement for the coordinated names, followed by written. The location is
  on-between-their-eyes; space supplies English grammar for that relation. The
  conjunction on with-him is retained.
- 14:2: The voice/sound is compared with many waters and great thunder, then with ONE
  harpist striking HIS HARPS, plural. These are not plural harpists with one harp each.
  Sound is the contextual audible sense of voice. English supplies was in the final
  nominal comparison; the striking participle remains linked to the singular musician.
- 14:3: Praise is both the action and the new hymn. The explicit as/like particle
  remains as-with. CAL's šbḥ entry includes singing a hymn
  (https://cal.huc.edu/oneentry.php?cits=all&lemma=%24bx+V); hymn is the musical praise
  in this scene, not a separate imported event. Before is repeated for the throne, four
  living creatures and elders. No one was able to learn it; the object suffix
  anticipates the hymn. Except-for renders the short `אלא אנ` exception idiom. The
  144,000 are purchased from earth, a passive-participle description.
- 14:4: These/are/those is explicit. With-women precedes the negative passive
  defilement. The short virgins/for/they-are unit gives grammatical English while
  retaining the postpositive causal particle. Followed is perfect, with its object
  suffix anticipating Lamb. Would-go gives the following imperfect a habitual setting
  within that past following. Purchased is passive perfect; this source has NO by-Jesus
  agent. The human group is firstfruits to God AND the Lamb.
- 14:5: The verse opens with a d-prefixed in-their-mouth, continued as
  for-in-their-mouth, without adding an absent and. Mouth is singular with plural
  possessors. The negative and passive found retain falsehood as subject. The short
  four-position without/blemish/for/they-are unit gives for-they-are-without-blemish; no
  negation or causal particle is lost.
- 14:6: The angel flies in the MIDST of heaven and has an everlasting gospel UPON HIM.
  There is no blood word here despite the reference. The gospel's eternity expression is
  rendered with the adjective everlasting in a short unit. The audience list is people,
  nations, tribes and language in the actual order and numbers; every scopes the list,
  and the final every repeats that scope for grammatical English. The proclamation
  continues into the next verse's saying infinitive.
- 14:7: The first command is FEAR God, not Murdock's worship; the from particle belongs
  to the fear-God construction and is rendered with the verb in a short unit. Giving him
  glory and worshiping the one WHO MADE remain separate commands. His judgment's hour
  HAS COME. Springs is the water-source sense of `עינא`, not literal anatomical eyes;
  CAL lists both eye and spring/fountain
  (https://cal.huc.edu/oneentry.php?cits=all&lemma=%28yn+N).
- 14:8: Another, the second, is following him; the source leaves angel implicit here.
  The following participle has a past auxiliary, and saying shares that narrative
  setting. Both fallen predicates remain before Babylon the great. The source says she
  gave the nations drink FROM THE WRATH OF HER HARLOTRY, with no wine noun here. All's
  plural suffix anticipates nations.
- 14:9: The third angel followed THEM. Worshiped and took are both PERFECT, rendered as
  completed acts in the generic whoever clause. The beast's image and mark retain their
  genuine feminine possessors. The location is between HIS eyes, the worshiper's, with
  NO or-on-his-hand alternative. The sentence and quotation continue into the judgment
  in the next two verses.
- 14:10: Also HE will drink. The wrath suffix anticipates THE LORD; the source does not
  name God at this position. Mixed and without-admixture are both explicit and are
  retained as the source's metaphor rather than replacing mixed with an absent poured
  verb. The cup's HIS indignation is a genuine possessor. Torment is passive imperfect,
  with fire and sulphur, before the holy angels AND THE LAMB, not the reference's
  throne.
- 14:11: Smoke of THEIR torment WILL ASCEND, imperfect, for the age of ages. Rest is
  negated for them by day AND night. Those worshiping the beast and its image are
  followed by the full additional AND-FOR-WHOEVER-TOOK-THE-MARK-OF-ITS-NAME clause
  absent from the reference. Took is perfect; its-name belongs to the beast. The third
  angel's quotation closes here.
- 14:12: Here-is-endurance has an anticipatory feminine singular existential suffix, not
  another possessor. The saints HAVE KEPT the commandments, a perfect rather than a
  present participle. The commandment suffix anticipates God and the faith suffix
  anticipates Jesus; English keeps the explicit genitives without duplicate his
  pronouns.
- 14:13: The voice says write, a second-singular imperative. Blessed-are-the-dead
  renders the beatitude construction idiomatically. DEPARTED is perfect and the Lord has
  a genuine FIRST-PLURAL suffix: OUR Lord. Dashes keep the completed-departure relative
  parenthetical so from-now-on can qualify the blessing. Yes, says the Spirit, because
  they will rest from THEIR TOILS: the rest verb is imperfect in its causative/passive
  stem, with the restful outcome rendered in normal English. This source has NO
  their-deeds-follow-them clause.
- 14:14: The source opens AND BEHOLD, without another I-saw verb. A white cloud bears a
  sitting LIKENESS OF A HUMAN BEING; the source noun likeness remains, without importing
  a separate Son title. English inversion preserves the sitting figure as subject. The
  possessive existential gives he-had, followed by a gold crown ON HIS HEAD and a sharp
  sickle ON HIS HAND. These genuine masculine possessors refer to the figure.
- 14:15: Another angel came from the temple AND CRIED ALOUD, both perfects. The seated
  figure on the cloud receives the full second-singular command: SEND YOUR SICKLE AND
  REAP, because the hour has come TO REAP. The reference omits these source words. No
  extra ripe-harvest clause is added beyond the actual ending.
- 14:16: The one sitting on the cloud is explicit and casts HIS SICKLE on earth. English
  uses passive inversion, there-was-cast BY-THE-ONE ... HIS-SICKLE, preserving that
  actor and object while keeping source links ascending. The final passive clause is the
  earth was reaped; it is not another act by an unspecified reaper.
- 14:17: Another angel came out of the temple IN HEAVEN. On-him plus the existential
  gives upon-him-was a sharp sickle in the narrative setting, without adding the
  reference's separate also. The adjective and sickle form a short English-order unit.
- 14:18: The angel from the altar has authority OVER FIRE and cries to the one with the
  sharp sickle. Both existential/dative possession expressions remain. The explicit
  second-person YOU is retained in the short You-send imperative unit, followed by YOUR
  sharp sickle. The clusters belong to earth's vineyard: the vineyard suffix anticipates
  earth. ITS GRAPES HAVE GROWN retains the perfect and the masculine singular possessor
  referring to vineyard, not a feminine possessor referring directly to earth.
- 14:19: The angel casts his sickle and gathers/plucks earth's vineyard; harvested is
  the English verb for that vineyard-gathering metonym. The implied object of the
  following cast is made explicit as the harvest, the grapes just gathered. GREAT
  modifies the feminine WINEPRESS, not masculine God. The wrath suffix anticipates God,
  so there is no independent extra his possessor.
- 14:20: The full source has a winepress trampled OUTSIDE THE CITY and BLOOD COMING OUT
  FROM THE WINEPRESS, clauses omitted by the reference. The blood reaches the bridles of
  plural horses. The distance is A THOUSAND AND TWO HUNDRED STADES, 1,200, not Murdock's
  1,600. Stades preserves the historical unit without an unwarranted modern conversion.

Source checks for Revelation 15:

- 15:1: Another sign is great and wondrous; the following angels carry SEVEN PLAGUES.
  This source has no seven numeral modifying angels in this verse, though seven angels
  are explicit later. The existential/on-them unit becomes upon-whom-were. The last ones
  refers to the plagues, as does the feminine plural in-these suffix. Completed is
  passive perfect, retaining the vision's completed perspective. The wrath suffix
  anticipates God rather than adding an independent his possessor. Wondrous describes a
  sign worthy of wonder, not an inanimate sign feeling astonishment.
- 15:2: The vision has something LIKE a sea of glass mingled with fire, then those who
  had prevailed. Prevail-OVER renders the victory construction with from; beast, image
  and number of its name all remain. The genuine feminine image/name possessors refer to
  the beast. Standing is an active participle placed within the past vision, while
  prevailed is perfect and anterior to that standing. LʿL MIN is ABOVE, retained as a
  short compound relation rather than on-from. The harps are UPON THEM; the harp suffix
  anticipates God, so no additional independent his is added to God's harps.
- 15:3: Singing the hymn is the musical praise sense of the verb and noun, followed by
  AND-WERE-SAYING. The servant suffix anticipates God. Great and wondrous are YOUR
  DEEDS; the second predicate again says YOUR DEEDS, not the reference's ways. Wondrous
  renders the passive-shaped description as things wondered at. KʾANIN is the
  just/proper adjective despite the incomplete exported verb-gloss set: CAL's kʾn entry
  explicitly gives just/honest and the proper/fair sense for inanimates
  (https://cal.huc.edu/oneentry.php?cits=all&lemma=k%29n+A). It is not translated as a
  created/existing event merely from the root glosses. Holder-of-all retains both title
  words; King-of-ages retains the plural age/world noun. The speech continues into the
  next verse.
- 15:4: The rhetorical question retains not, fear YOU, Lord, glorify and YOUR name. All
  three causal particles remain as For, for and because. You-alone-are uses a short
  copula/adverb unit; holy has NO added and-just predicate. All nations will come and
  worship BEFORE YOU. The ending is because YOU ARE UPRIGHT, a second-singular
  predicate, not the reference's revealed righteousnesses. CAL's trīṣ entry gives
  upright/honest and right/correct
  (https://cal.huc.edu/oneentry.php?cits=all&lemma=tryc+A), corroborating this actual
  form without importing an absent reveal verb or righteousness noun.
- 15:5: From-after is the temporal after compound, followed by plural these-things.
  Looked is the intransitive English rendering of seeing the following scene. The temple
  WAS OPENED passively; inversion keeps the complete
  temple-of-the-tabernacle-of-testimony genitive chain in source order, followed by
  in-heaven. No extra behold is supplied.
- 15:6: Here SEVEN ANGELS is explicit. They come out from the temple, identified as
  those bearing seven plagues. While, clothed and girded remain distinct, with both
  clothing predicates passive participles. The linen is CLEAN AND BRIGHT; the noun and
  adjectives form a short English-order unit. Their chests/breasts are plural with
  genuine plural possessors. The source belt is singular, retained as a-belt-of-gold in
  the normal distributive sense for the angels, rather than silently recoding the source
  as plural belts.
- 15:7: One of the four living creatures GAVE seven BOWLS to the seven angels. Filled is
  a passive participle; the bowls contain the wrath OF GOD, with the wrath suffix
  anticipating that explicit possessor. There is NO gold word describing these bowls.
  God IS LIVING, with both existential and living adjective retained,
  for-the-age-of-ages. The final AMEN, absent from the reference, remains explicit.
- 15:8: The temple was filled WITH SMOKE; with is the idiomatic English complement of
  fill for the source's from/partitive relation. The smoke-of-the-glory genitive is read
  as source/origin, hence smoke FROM THE GLORY OF GOD, coordinated with the explicit AND
  FROM HIS POWER. The glory suffix anticipates God; his-power is a genuine possessor.
  No-one-was-able preserves the negative existential, ability participle and explicit
  past auxiliary, followed by enter-the-temple. Until-the-plagues-were-completed is the
  passive until-condition backshifted within that past frame; inversion keeps both seven
  numerals and the angels' genitive contiguous. The completion verb is imperfect in the
  source, unlike the completed perfect in verse 1.

Source checks for Revelation 16:

- 16:1: A LOUD VOICE FROM THE TEMPLE is explicit, though absent from the local
  reference. Saying is an active participle; go and pour-out are second-plural
  imperatives. Both seven numerals remain. The vessels are BOWLS, not the distinct cup
  noun of verse 19. The wrath suffix anticipates God, without an extra independent his.

- 16:2: The first went and poured HIS BOWL. Ulcer, foul and painful form a short
  three-source noun/adjective unit. The people renders collective ANASHA in its plural
  human context; who-had is the existential plus genuine plural possessive dative. AND
  THOSE WHO WORSHIPED remains explicit, followed by ITS IMAGE, with the genuine feminine
  suffix referring to the beast. Worshiped locates the active participle in the narrated
  scene.

- 16:3: The bowl is poured INTO THE SEA, and the repeated sea is the subject of became.
  There is NO BLOOD WORD in this verse: the actual comparison is the sea becoming LIKE A
  DEAD PERSON. Every living soul DIED in the sea, retaining the perfect and distributive
  singular. No blood or extra things clause is imported from Murdock.

- 16:4: The bowl is poured INTO rivers AND INTO springs of water, and they became BLOOD.
  Springs is the water-source sense of the eye lexeme, confirmed by CAL's ayn entry,
  sense 2 (https://cal.huc.edu/oneentry.php?cits=all&lemma=%28yn+N). The plural sources
  and plural became remain; blood is present here, unlike verse 3.

- 16:5: Righteous-are-you addresses one second-singular YOU. The one WHO IS AND WHO WAS
  is an appositive; the past auxiliary belongs to the second existential. AND HOLY
  remains another predicate of the addressee. Because-you-judged-these is a short
  two-source unit, retaining the plural demonstrative and second-singular perfect judge
  verb, without inventing a judgment noun.

- 16:6: The blood belongs to PROPHETS AND HOLY ONES in that source order. Fronted
  objects preserve ascending links while distinguishing THEY POURED OUT from YOU GAVE
  THEM blood TO DRINK. For renders the causal connector as an explanatory conjunction,
  keeping a complete English sentence. WORTHY ARE THEY preserves the participial
  predicate and plural pronoun without adding a second causal particle.

- 16:7: The ALTAR ITSELF is heard saying; no one-from-the-altar is added. Yes, Lord, God
  and Almighty remain. Almighty renders the short holder-of-all title. TRUE AND
  RIGHTEOUS are plural predicates of YOUR JUDGMENTS, also plural, with a genuine
  second-singular possessor; the reference's singular judgment is not followed.

- 16:8: The fourth ANGEL is explicit. It-was-granted retains the passive perfect, and TO
  HIM the genuine masculine singular dative without deciding between the angel and sun
  as referent. To-scorch is the English purpose complement for an imperfect in the
  causative stem; form 5 is not an active-participle tense. People and fire remain.

- 16:9: People WERE SCORCHED with GREAT HEAT and blasphemed THE NAME OF GOD. Who-has is
  existential possession, followed by authority OVER THESE PLAGUES, not an extra plague
  possessor. The negative plus perfect remains THEY DID NOT REPENT, followed by TO GIVE
  HIM GLORY with its genuine dative.

- 16:10: The fifth ANGEL pours his bowl on the THRONE OF THE BEAST; the throne's
  feminine suffix anticipates that named possessor. ITS KINGDOM has a genuine feminine
  possessor and became DARKNESS. THEY WERE GNAWING preserves the active participle plus
  past auxiliary; THEIR TONGUES retains the plural owner, while FROM PAIN has no added
  possessor.

- 16:11: THE NAME OF THE GOD OF HEAVEN is explicit, including name omitted by the
  reference. Both causes remain: BECAUSE OF THEIR PAINS AND BECAUSE OF THEIR ULCERS. The
  perfect negation remains THEY DID NOT REPENT OF THEIR DEEDS. All three genuine plural
  possessors are retained; heaven is the normal English collective.

- 16:12: The SIXTH ANGEL pours his bowl on the GREAT RIVER EUPHRATES. ITS WATERS is
  plural with a genuine river possessor, and dried-up is perfect.
  So-that-a-way-might-be-prepared is a short predicate/subject unit rendering the
  passive imperfect purpose in the past narrative. The kings come FROM THE RISING OF THE
  SUN: the collective plural east/rising idiom is confirmed by CAL's mdnh entry, senses
  1 and 2 (https://cal.huc.edu/oneentry.php?cits=all&lemma=mdnx+N).

- 16:13: I SAW is first-singular perfect. Each repeated FROM THE MOUTH belongs to its
  named owner: DRAGON, BEAST and FALSE PROPHET. The mouth suffixes anticipate these
  owners, without redundant English his/her. The postposed object is THREE SPIRITS,
  UNCLEAN, LIKE FROGS. Unclean retains the explicit not-clean pair; no separate issuing
  verb is imported.

- 16:14: FOR THEY ARE retains the existential with feminine plural suffix and causal
  particle. SPIRITS OF DEMONS PERFORM SIGNS AND GO OUT, both verbs active participles.
  They go to the KINGS OF THE INHABITED WORLD TO GATHER THEM, the object being genuinely
  plural. The battle belongs to THAT GREAT DAY OF GOD ALMIGHTY. No all
  numeral/quantifier absent from this source is added.

- 16:15: The source has BEHOLD and a bare masculine singular coming participle, encoded
  third singular, with NO first-person ANA pronoun. He-is-coming preserves that source
  reading rather than importing the reference's I. Robinson/Coakley section 10, pages
  46-47, explains first/second-person participial pronouns and third-person omission
  (https://www.syriac-eschool.com/wp-content/uploads/2022/02/Coakley_Robinson_Syriac_Grammar.pdf).
  The beatitude's suffix is idiomatic in BLESSED IS THE ONE. Awake is a passive-shaped
  state, followed by active KEEPS HIS GARMENTS. LEST HE WALK NAKED is a short
  negative/predicate unit; AND THEY SEE HIS SHAME stays under lest, preserving the
  change to a plural observer.

- 16:16: HE WILL GATHER retains the imperfect THIRD-SINGULAR verb, distinct from plural
  THEM. The source does not say they-gathered. The place IS CALLED IN HEBREW MAGDU,
  retaining the actual name rather than expanding it to the reference's Armageddon. The
  place's passive calling is contextual.

- 16:17: The SEVENTH ANGEL pours his bowl INTO THE AIR. THERE CAME OUT A LOUD VOICE is
  grammatical existential inversion, with both FROM THE TEMPLE and FROM BEFORE THE
  THRONE. The final perfect HWA is read as the completed-event announcement IT HAS
  HAPPENED, following saying. CAL's hwy entry, sense 3, confirms come-to-pass/happen
  (https://cal.huc.edu/oneentry.php?cits=all&lemma=hwy+V); no separate finished/done
  verb is asserted to exist in the source.

- 16:18: The opening lists LIGHTNINGS AND THUNDERS, then an EARTHQUAKE; there is no
  voices word. Every source copula is retained as there-were, occurred,
  had-not-occurred, there-were-humans and final it-was. The great quake has NO precedent
  since humans were on earth. The second quake noun and repeated comparison/degree
  remain in the emphatic SUCH WAS THIS EARTHQUAKE; SO GREAT IT WAS. Short English
  copular inversion keeps the links ascending without shifting source positions.

- 16:19: The GREAT CITY BECAME THREE PARTS, then the CITIES OF THE NATIONS FELL: cities
  and fell are plural despite the reference's singular city. BABYLON THE GREAT WAS
  REMEMBERED passively BEFORE GOD, TO GIVE HER the CUP OF THE WINE. The genuine feminine
  recipient and both genuine masculine possessors remain in OF HIS WRATH AND OF HIS
  INDIGNATION; the two anger nouns are not collapsed into one.

- 16:20: EVERY ISLAND is distributive singular and FLED is perfect. THE MOUNTAINS are
  plural and WERE NOT FOUND is passive perfect, with the explicit negative retained.

- 16:21: GREAT HAIL descends FROM HEAVEN ON HUMANS. LIKE A TALENT IN WEIGHT explains the
  actual weight noun without inserting a modern conversion or a new source noun for
  hailstones. Humans BLASPHEMED GOD BECAUSE OF THE PLAGUE OF HAIL. The second BECAUSE
  remains, followed by a short four-source predicate/copula/subject/degree unit: ITS
  PLAGUE IS VERY GREAT. This unit keeps the genuine feminine possessor, the copula and
  the final much/very intensifier without shifting them onto earlier source positions.

Source checks for Revelation 17:

- 17:1-2: One of SEVEN ANGELS came and spoke, both perfects. Upon-whom-were renders the
  existential plus on-them relation, followed by SEVEN BOWLS. COME AFTER ME retains the
  imperative and genuine first-singular after-me suffix, absent from the reference.
  I-WILL-SHOW-YOU is first-singular imperfect with a second-singular object. There is NO
  great adjective on harlot in verse 1. The relative WITH WHOM continues into verse 2;
  English passive FORNICATION WAS COMMITTED BY THE KINGS retains the active source's
  kings as agents while preserving link order. Kings and inhabitants have anticipatory
  feminine suffixes referring to EARTH; HER FORNICATION has a genuine harlot possessor.
  ALL BECAME DRUNK is followed by its appositive THE INHABITANTS OF THE EARTH, retaining
  the all quantifier and FROM THE WINE relation.

- 17:3: He LED ME OUT is a perfect causative with a genuine first-singular object. INTO
  THE WILDERNESS precedes IN THE SPIRIT. The woman is SITTING on a SCARLET BEAST FILLED
  WITH NAMES OF BLASPHEMY. The possession existential refers to the beast and gives
  SEVEN HEADS. HORNS, BUT, TEN become BUT THE HORNS WERE TEN in short English copular
  units, preserving the adversative rather than substituting the reference's and.
  Sitting remains an active participle, filled a passive one.

- 17:4: The woman WAS THERE, CLOTHED IN PURPLE AND SCARLET. The clothing predicate is
  feminine singular; the following GILDED predicate is feminine PLURAL and is read of
  the purple/scarlet clothing, not of the singular woman. English WHICH WERE makes that
  agreement explicit. GOLD, PRECIOUS STONES and PEARLS remain; precious is the
  good/valuable sense in the stone idiom. SHE HAD is existential possession in the
  vision, followed by a CUP OF GOLD ON HER HAND, with a genuine feminine possessor.
  FILLED is masculine singular and describes the cup; IMPURITY AND ABOMINATION are
  singular complements of HER FORNICATION. No earth phrase is added at this verse's end.

- 17:5: ON plus BETWEEN renders the location BETWEEN HER EYES, preserving the eye noun
  and genuine feminine possessor without replacing the whole expression by forehead.
  WRITTEN is a masculine singular passive participle, followed by MYSTERY; there is NO
  NAME noun in this source. BABYLON THE GREAT is MOTHER OF HARLOTS AND OF THE
  ABOMINATIONS OF THE EARTH. Abominations is plural; its suffix anticipates earth,
  without an independent extra her.

- 17:6: I SAW the woman DRUNK, a passive-shaped state, FROM THE BLOOD OF THE HOLY ONES
  AND FROM THE BLOOD OF THE WITNESSES OF JESUS. Both from/blood phrases remain. The
  witness suffix anticipates Jesus, rather than adding a separate his before the named
  owner. I MARVELED WITH GREAT WONDER precedes WHEN I SAW HER, retaining both
  first-singular perfects and the genuine feminine object on the final seeing verb.

- 17:7: AND IT WAS SAID TO ME BY THE ANGEL is an English passive inversion of the active
  said, with the angel kept as its agent. WHY HAVE YOU MARVELED retains the
  second-singular perfect. I AM TELLING YOU renders the explicit emphatic I,
  first-person speaking participle plus pronoun, and genuine second-singular dative; it
  does not import a future will. The beast IS LADEN WITH HER, retaining the
  passive-shaped carrying/load state and genuine woman object. CAL's tn entry gives
  carrying and loading, including loaded animals
  (https://cal.huc.edu/oneentry.php?cits=all&lemma=%2B%28n+V); the form can also bear
  the idiomatic active carrying sense. The following WHICH HAS is a separate existential
  possession with SEVEN HEADS AND TEN HORNS.

- 17:8: The first beast WAS AND IS NOT, with the existential plus past auxiliary and
  negative existential kept distinct. The following past HWA controls ITH as a single
  past existential, not two coordinate present/past assertions; Robinson/Coakley section
  14, page 66, explains this construction
  (https://www.syriac-eschool.com/wp-content/uploads/2022/02/Coakley_Robinson_Syriac_Grammar.pdf).
  READY is passive-shaped prospective description, followed by TO ASCEND FROM THE SEA:
  sea is explicit, not the reference's abyss. TO DESTRUCTION IT IS GOING retains the
  active participle. The earth-dwellers WILL BE AMAZED; English predicate inversion
  keeps them the plural subject. Their NAMES ARE NOT WRITTEN is a short
  negative/passive/subject unit with the genuine plural possessor. The BOOK OF LIFE is
  FROM THE FOUNDATIONS OF THE WORLD: foundations is plural and its suffix anticipates
  world. AS THEY SEE renders the seeing relative participle as the circumstance of their
  wonder. The final beast again WAS AND IS NOT, and HAS DRAWN NEAR is perfect, rather
  than the reference's present approaches.

- 17:9: HERE IS UNDERSTANDING is FOR THE ONE WHO HAS WISDOM, retaining the possessive
  dative rather than changing it to the mind's possession. SEVEN HEADS ARE SEVEN
  MOUNTAINS retains both numerals and the explicit plural copula. WHERE plus SITS, WOMAN
  and ON THEM forms a short four-source relative unit, ON WHICH THE WOMAN SITS. This
  gives grammatical English to the source's resumptive locative without losing the
  plural on-them relation.

- 17:10: AND THEY ARE SEVEN KINGS is a short noun/numeral/copula unit continuing the
  heads' explanation. FIVE HAVE FALLEN is perfect, ONE EXISTS, and THAT OTHER ONE HAS
  NOT YET COME retains the demonstrative, negation, yet and perfect come. AND WHEN HE
  HAS COME is the temporal ma d- construction with its completed-event perspective. A
  LITTLE WHILE IS GIVEN TO HIM TO REMAIN retains the duration idiom, passive giving
  participle, genuine masculine singular recipient and infinitive; no separate must verb
  is asserted to exist.

- 17:11: The source unusually names both DRAGON AND BEAST, followed by singular
  resumptions. English AS FOR introduces this loose coordinated topic, with THAT ONE and
  IT TOO preserving the singular resumption of the nearest beast. This keeps the
  source's ambiguity without asserting that the two names identify one entity or
  deleting dragon. The source says WHICH IS AND IS NOT: there is NO past HWA auxiliary
  here, unlike the was sequences in verse 8. EIGHTH, FROM THE SEVEN, the explicit
  singular copula and TO DESTRUCTION IT IS GOING all remain; going is a feminine
  singular active participle.

- 17:12: TEN HORNS WHICH YOU SAW ARE TEN KINGS retains both ten numerals and the
  explicit plural copula. BY WHOM A KINGDOM HAS NOT YET BEEN RECEIVED is English passive
  inversion of their active receiving; the kings remain the agents and both negative and
  yet are retained. AUTHORITY, AS KINGS, FOR ONE HOUR THEY RECEIVE preserves the fronted
  object, comparison, time and active receiving participle. WITH THE BEAST has a
  singular beast, not the reference's beasts.

- 17:13: THESE HAVE ONE PURPOSE uses a short four-source numeral/noun/existential/dative
  possession unit; purpose renders will/desire in this shared-intent context. POWER AND
  AUTHORITY OF THEIR OWN retains both nouns, their conjunctions and the genuine plural
  ownership applying to both. TO THE BEAST THEY GIVE preserves the actual active
  participle in the present, without automatically importing the reference's future
  will.

- 17:14: THESE AGAINST THE LAMB WILL MAKE WAR AND THE LAMB WILL OVERCOME THEM keeps the
  imperfect conflict verbs and plural object. Against is the adversarial sense of with
  in the fighting construction. CAL qrb, causative stem sense 2, gives
  carry-on-war/attack with an example using the same am preposition
  (https://cal.huc.edu/oneentry.php?cits=all&lemma=qrb+V); the Lamb overcoming THEM
  confirms that opposition here. HE IS LORD OF LORDS AND KING OF KINGS retains both
  titles and the explicit singular copula. The actual vocalized MARAWATHA is the
  irregular plural lords, despite the exported dominion lemma gloss and feminine
  metadata; it is not the abstract MARUTHA. The Classical Syriac dictionary explicitly
  gives this plural of lord/master
  (https://assyrianlanguages.org/sureth/dosearch.php?language=id&searchkey=8443). THOSE
  WITH HIM ARE CALLED AND CHOSEN AND FAITHFUL retains the genuine masculine singular
  with-him suffix and all three plural descriptions. Faithful is supported by CAL mhymn,
  sense 1, alongside believer in sense 2
  (https://cal.huc.edu/oneentry.php?cits=all&lemma=mhymn+A); the incomplete export gloss
  set does not require substituting a different lemma.

- 17:15: AS FOR THE WATERS introduces the source's topic; WHICH YOU SAW is
  second-singular perfect, ON WHICH SITS THE HARLOT keeps the plural prepositional
  suffix and active sitting participle. PEOPLES AND MULTITUDES AND NATIONS AND LANGUAGES
  are all plural and remain separately linked. The final THEY ARE retains the explicit
  plural existential after the fronted predicate list.

- 17:16: The TEN HORNS WHICH YOU SAW ON THE BEAST are the topic resumed by THESE and the
  feminine plural HATE verb. On-the-beast renders the l-phrase's association/possession;
  it is NOT an and-the-beast coordinated subject. Thus the horns remain the actors. They
  WILL HATE THE HARLOT and MAKE HER DESOLATE AND NAKED, EAT HER FLESH and BURN HER WITH
  FIRE. Both genuine feminine object suffixes and the flesh possessor remain. The
  destruction verbs are imperfect; burning is in the causative stem, not an automatic
  active participle.

- 17:17: FOR GOD PUTS INTO THEIR HEARTS renders the active giving participle with its
  placement idiom in the present, not a past has-put. The purpose clause is THAT THEY
  SHOULD DO HIS WILL AND MAKE THEIR WILL ONE AND GIVE THEIR KINGDOM TO THAT BEAST. HIS
  WILL and THEIR WILL are distinct genuine possessors, followed by one as their will's
  resulting unity. THAT remains the feminine demonstrative of beast. UNTIL THERE ARE
  FULFILLED THE WORDS OF GOD keeps passive completion in the until-condition and English
  inversion for ascending links. The words suffix anticipates God, without adding an
  extra independent his.

- 17:18: The WOMAN WHOM YOU SAW IS THE GREAT CITY retains the identification and
  second-singular perfect. WHICH HAS is the existential plus genuine feminine possessive
  dative. KINGSHIP renders the royal reign/kingdom noun in its rule-over relation to the
  KINGS OF THE EARTH. The kings' feminine suffix anticipates earth, rather than adding a
  separate her referring to the city.

Source checks for Revelation 18:

- 18:1: AFTER THESE THINGS is the temporal from-after compound. I SAW is first-singular
  perfect, and WHO DESCENDED is perfect, not an active descending participle. WHO HAD is
  existential possession backshifted within the vision, followed by GREAT AUTHORITY. The
  earth SHONE FROM HIS GLORY, retaining its active shining verb and the genuine
  masculine singular glory possessor.

- 18:2: The angel CRIED ALOUD with a LOUD VOICE. Both FALLEN perfects remain before
  BABYLON THE GREAT. She became a HABITATION FOR DEMONS AND A PRISON FOR ALL SPIRITS
  THAT ARE UNCLEAN AND DETESTABLE. Prison renders custody as the place of confinement.
  The explicit not-clean pair remains in unclean; both adjectives are plural. This
  source has NO BIRD OR BEAST clauses beyond these spirits.

- 18:3: FROM THE WINE OF HER FORNICATION SHE POURED FOR ALL THE NATIONS retains the
  feminine singular perfect subject, not a plural nations-drank clause. CAL mzg gives
  mixing and the pouring/serving-liquid sense
  (https://cal.huc.edu/oneentry.php?cits=all&lemma=mzg+V). Her fornication is not
  changed to the reference's wrath. The kings' feminine suffix anticipates EARTH; WITH
  HER is a genuine Babylon reference. The merchants GREW RICH FROM THE POWER OF HER
  MADNESS. Madness retains SHNAYA rather than silently importing luxury; CAL's shny noun
  entry confirms insanity/madness
  (https://cal.huc.edu/oneentry.php?cits=all&lemma=%24ny+N).

- 18:4: COME OUT is second-plural imperative, followed by FROM WITHIN HER and MY PEOPLE,
  with genuine feminine and first-singular suffixes. SO THAT YOU MAY NOT SHARE IN HER
  SINS retains the negative purpose construction. LEST YOU RECEIVE SOME OF HER PLAGUES
  renders the following prohibitive clause, with some-of giving normal English to the
  partitive from relation. Sins and plagues have genuine feminine possessors.

- 18:5: SINS HAVE CLUNG TO HER is a short verb/complement/subject unit retaining the
  explicit to-her relation, followed by UP TO HEAVEN. It does not drop that pronoun in
  favor of only sins-reaching-heaven. GOD REMEMBERED HER INIQUITIES retains God as the
  singular semantic subject of the middle/reflexive remember verb and the genuine
  feminine possessor on plural iniquities.

- 18:6: REPAY HER, DOUBLE IT and MIX are three second-plural imperatives. AS ALSO SHE
  REPAID retains the explicit also and feminine pronoun. DOUBLE IT FOR HER is followed
  by the explanatory A DOUBLE MEASURE ACCORDING TO HER DEEDS: the second double noun
  describes the resulting repayment, not another doubling of an already doubled measure.
  IN THAT CUP WHICH SHE MIXED retains the demonstrative and perfect; MIX FOR HER A
  DOUBLE MEASURE retains the second genuine recipient and final double noun. It supplies
  the understood repayment object in double-it, not a new source actor or event.

- 18:7: The proportional clause describes how she GLORIFIED HERSELF AND EXALTED HERSELF,
  both perfects. IN LIKE MEASURE SHALL BE THE TORMENT AND MOURNING supplies an English
  copula to the matching-degree predicate; there is NO give verb in this source verse.
  IN HER HEART SHE SAYS is active participle speech. I SIT AS A QUEEN retains the
  first-person sitting participle and explicit I; A WIDOW I AM NOT keeps the negative
  existential, and MOURNING I WILL NOT SEE keeps the second negative and first-singular
  imperfect.

- 18:8: METULHANA has null exported glosses but is the recognizable
  because-of-this/THEREFORE compound. IN ONE DAY is an actual numeral phrase. Plagues
  WILL COME UPON HER: death AND mourning AND famine, with all three named. WITH FIRE SHE
  WILL BURN retains the intransitive basic-stem imperfect. MIGHTY IS THE LORD WHO JUDGED
  HER retains the genuine object and perfect judging, without importing a separate God
  noun or changing judged to present judges.

- 18:9-10: The two future lament verbs and both feminine references become WEEPING FOR
  HER AND MOURNING OVER HER BY THE KINGS OF THE EARTH. This English nominal inversion
  keeps the kings explicit as agents while maintaining source links. Their perfect
  fornication and self-exaltation remain distinct, followed by WHEN THEY SEE THE SMOKE
  OF HER BURNING and WHILE STANDING FAR AWAY FROM FEAR OF HER TORMENT across the verse
  boundary. THEY WILL SAY retains the imperfect. There are THREE separate WOES in verse
  10. GREAT CITY, BABYLON and MIGHTY CITY all remain. IN ONE HOUR THERE HAS COME YOUR
  JUDGMENT preserves the perfect and genuine second-feminine-singular judgment
  possessor.

- 18:11: The merchants WILL WEEP AND MOURN OVER HER. AND THEIR CARGO HAS NO BUYER
  ANYMORE renders cargo, the negative existential, the nominalized buying participle and
  again as grammatical English. Their is a genuine plural owner, not her. The
  conjunction is AND, not an invented because. The colon introduces the repeated cargo
  noun and the catalogue continuing through verses 12-13.

- 18:12-13: Every actual commodity remains: gold, silver, precious stones, pearls,
  linen, purple, scarlet silk, aromatic wood, ivory vessels, vessels of precious wood,
  brass, iron, marble, cinnamon, ointments, perfume, frankincense, wine, oil, fine
  flour, sheep, horses, chariots, bodies and souls of humans. The source's
  silk-OF-scarlet is one short scarlet-silk unit, not a silently added and-scarlet item.
  Aromatic-wood renders wood-of-perfume. All three all/every expressions in verse 12
  remain. Verse 13 has NO amomum, wheat or separate beasts-of-burden item. Bodies AND
  souls are both plural, with OF HUMANS attached to the pair.

- 18:14: YOUR FRUIT and YOUR SOUL both retain genuine second-feminine-singular
  possessors. THE DESIRE OF YOUR SOUL is appositional to the fruit desired. HAS DEPARTED
  occurs twice; the second subject is ALL THAT IS RICH AND PRAISEWORTHY. NOT AGAIN WILL
  YOU SEE THEM is second-feminine-singular imperfect with a separate plural object; the
  source does not say the traders will obtain them.

- 18:15: AND THEY WILL NOT FIND THEM retains the negative plus third-plural active
  imperfect and plural object, completing the preceding loss theme. A sentence boundary
  then introduces THE MERCHANTS OF THESE THINGS, WHO GREW RICH FROM HER, FROM AFAR WILL
  STAND. This preserves the source's basic-stem find rather than recoding it as a
  passive merchants-will-be-found clause. FROM FEAR OF HER TORMENT keeps the genuine
  feminine possessor. WHILE WEEPING AND MOURNING uses the active weeping participle and
  passive-shaped mourning state, continuing into saying in verse 16.

- 18:16-17: Two WOES precede the GREAT CITY CLOTHED IN LINEN AND PURPLE AND SCARLET.
  Clothed is feminine singular; WHICH WERE GILDED is feminine PLURAL and describes the
  clothing, followed by GOLD, PRECIOUS STONES AND PEARLS. The opening of verse 17
  continues that lament: FOR IN ONE HOUR THERE WAS EMPTIED WEALTH SUCH AS THIS. The
  emptying is perfect in its passive/reflexive stem. Then a new narrative sentence names
  ALL PILOTS OF SHIPS, ALL WHO TRAVEL IN SHIPS TO PLACES, MARINERS AND ALL WHO WORK AT
  SEA. The three all quantifiers and plural ships/places remain; AT A DISTANCE STOOD
  retains the final perfect, rather than automatically carrying the earlier future stand
  into this scene.

- 18:18: THEY WEPT FOR HER is perfect with a genuine feminine object, not the
  reference's unsuffixed cried. WHILE WATCHING and AND SAYING retain the participial
  circumstances of that past lament. WHO IS IT THAT RESEMBLES THE GREAT CITY retains the
  interrogative, copular pronoun and relative resembling verb. No separate other or this
  word is inserted.

- 18:19: They CAST DUST ON THEIR HEADS AND CRIED ALOUD, both perfects, followed by WHILE
  WEEPING AND MOURNING AND SAYING. Heads has a genuine plural possessor. Both WOES
  remain. IN WHICH GREW RICH THOSE WHO HAD A SHIP AT SEA keeps the locative relative,
  perfect enrichment and possessive existential. Ship is SINGULAR, naturally
  distributive for these owners; no extra all is placed before those. FROM HER SPLENDOR
  renders the genuine possessor and honor/glory noun in the enrichment context. The
  final d-prefix introduces the causal FOR IN ONE HOUR SHE BECAME DESOLATE, a perfect
  result.

- 18:20: REJOICE OVER HER is second-plural imperative, addressed to HEAVENS AND HOLY
  ONES AND APOSTLES AND PROPHETS. The source has holy ones, not angels. GOD HAS JUDGED
  YOUR CASE AGAINST HER renders the perfect judge verb and cognate judgment noun as the
  legal case settled for the plural addressees. Against-her gives idiomatic English to
  the adverse/retributive relation of the source's from-her; it does not erase the
  feminine pronoun or turn your plural claim into her judgment.

- 18:21: English passive inversion, THERE WAS TAKEN UP BY ONE OF THE MIGHTY ANGELS A
  GREAT STONE, preserves the taking actor and all source nouns while keeping ascending
  links. Mighty is plural with angels; GREAT modifies STONE, not the following
  millstone. HE CAST IT INTO THE SEA AND SAID retains both perfects, followed by THUS
  WITH A VIOLENT RUSH THERE WILL BE CAST DOWN BABYLON THE GREAT CITY. The final verb
  TESHKAḤ is a feminine third-singular basic-stem imperfect, unlike the passive cast
  verb. SHE WILL NOT PREVAIL AGAIN uses the independently attested overcoming/capability
  sense: CAL shkh, G sense 2, independent-verb subsection, gives overcome/conquer with
  Syriac examples (https://cal.huc.edu/oneentry.php?cits=all&lemma=%24kx+V). This
  contextual loss of power after overthrow is distinguished from the reference's passive
  be-found reading; the actual source form is retained.

- 18:22: SOUND OF A HARP AND OF A TRUMPET AND OF KINDS OF MUSIC AND OF TRUMPETERS
  preserves the actual instrument and performer inventory. Harp and trumpet are
  singular, kinds and trumpeters plural. WILL NOT BE HEARD IN YOU AGAIN is passive
  imperfect with a genuine second-feminine-singular location suffix. No pipers or
  additional artisan clause is imported from the reference.

- 18:23: LIGHT OF A LAMP WILL NOT BE VISIBLE TO YOU AGAIN renders the passive visual
  verb with its actual TO-YOU dative; the following voices use IN YOU. VOICE occurs
  twice, for BRIDEGROOM AND BRIDE, and both not/again pairs remain. YOUR MERCHANTS WERE
  THE GREAT ONES OF THE EARTH retains the past existential, not separate present and
  past assertions; the great-ones suffix anticipates earth. The printed merchant suffix
  is second person and the export encodes feminine plural, despite the singular city
  context and form; English your preserves the address without silently altering the
  source metadata. Both causal particles remain. BY YOUR SORCERIES YOU DECEIVED ALL THE
  NATIONS is active causative perfect with second-feminine-singular subject, not an
  agentless passive nations-were-deceived clause.

- 18:24: IN HER WAS FOUND THE BLOOD OF PROPHETS AND HOLY ONES WHO WERE SLAIN ON THE
  EARTH preserves the past passive finding and plural slain participle in its past
  narrative setting. There is NO extra all-other-people slain clause in this source. The
  blood belongs to the named prophets and holy ones; the source is not expanded to the
  reference's universal final group.

Source checks for Revelation 19:

- Revelation 19 was checked against all 332 actual source positions, every exported
  gloss alternative, inflected forms, morphology, suffixes, and the final English
  owners. All 21 verses retain every source position once in ascending order. Short noun
  phrases, postpositive particles, possessive constructions, and title units use two or
  three positions; no whole long clause is hidden in a combined link. Supplied English
  copulas, articles, relative constructions, and inversion make the running text
  grammatical without treating dictionary glosses as inflected English.

- In 19:1 and 19:6 the crowds are plural. There is no additional honor noun in 19:1. The
  three source nouns deliverance, glory, and power are kept in their actual order; the
  dative supplies the English possession predicate "belong to our God." Both causal
  particles in 19:2 remain. K'ANIN at 19:2 position 2 is "just," despite the export's
  incomplete generic KWN glosses; CAL's k'n adjective confirms just/fair/proper. His
  judgments and his servants retain their masculine singular possessor; her fornication
  and her plural hands retain their feminine singular possessor. Requiring the servants'
  blood from her hands preserves TBA and the source's MIN. In 19:3 D'TARTEN is "a second
  time" and SLĒQ is perfect, "has ascended."

- In 19:4 the source explicitly says "our God," rather than Murdock's unsuffixed God.
  Twenty-four elders and four living creatures remain separate subjects. The sitting
  participle is present; "were saying" places the saying participle in the past vision
  without mislabelling it as a perfect. In 19:5 the source has a voice from the throne,
  without a come-forth verb. Both ALL quantifiers, "his name," the little, WITH, and the
  great remain. In 19:6 all three voice comparisons are retained; AMLEK is a
  third-person singular perfect (form 5), not an active participle. "Has reigned"
  preserves its completed aspect. There is no OUR suffix on Lord or God there. AHID KUL
  is the short title "Almighty," not an extra "entirely" after an already complete
  Almighty rendering.

- In 19:7 the first two first-person plural forms are current rejoicing and exulting;
  the first-person imperfect gives "let us give." The feast has come, and HIS WIFE has
  made ready HERSELF. The feast's suffix anticipates "of the Lamb" rather than
  introducing another possessor. In 19:8 the permission is passive and the clothing
  clause gives its content. The linen is clean AND bright. The feminine plural nominal
  TRĪṢĀTHĀ is rendered "upright deeds" with its implicit nominal head; CAL tryc confirms
  upright/correct. The plural pronominal copula agrees with that predicate in Syriac;
  English "is" agrees with mass-noun linen.

- In 19:9 the first speech verb is explicitly plural "they said," whereas the second is
  singular "he said." The beatitude keeps its called plural subject and supper, wedding
  feast, and Lamb chain. Its final statement retains THESE WORDS, TRUE, OF GOD, and the
  plural existential; there is no MY suffix. In 19:10 the narrator worshiped him (a
  perfect action, not an infinitive purpose). The response is "No!" and a separate
  assertion of being the hearer's fellow servant and that of his brothers; no SEE/LOOK
  verb is added, and this assertion is not changed into a prohibition against worshiping
  a fellow servant. The testimony belongs to Jesus. The God-directed imperative retains
  YATIRAIT's especially/above-all force, and the following GAR is causal "for," not part
  of the Jesus name link.

- In 19:11 MHAYMNA is "Faithful," supported by CAL mhymn's trustworthy/faithful sense
  beyond the export's believer gloss. The rider judges and wages war, with present
  participles. In 19:12 his eyes, his head, many diadems, and a singular written name
  remain; no extra names or written-on-him clause is imported from Murdock. The except
  construction combines ELLA EN, and the generic negative knowing construction gives "no
  one knows except himself." In 19:13 the garment has no possessive suffix. ZLĪʿ is a
  passive participle; Payne Smith, Compendious Syriac Dictionary p. 117, explicitly
  gives "mingled, infected" for this form, supporting "mingled with blood" beyond
  SEDRA's limited draw-out gloss. Both clothing and naming are states; his NAME is
  called Word of God.

- In 19:14 the plural armies were following HIM; HWA is the actual third-person plural
  perfect auxiliary, not an additional event. NQYPYN has passive-shaped morphology but
  the lexical idiom "follow" gives active English following. White horses and white AND
  clean linen remain. In 19:15 the actual suffix says "their mouth," and NEQTLUN says
  "they will kill"; neither is silently changed to the singular rider from the
  reference. There is no two-edged adjective. The separate singular subject will tend
  THEM, the explicit plural object at position 10, with an iron rod; the next singular
  subject tramples (a participle). The wrath suffix anticipates God, and AHID KUL again
  gives Almighty.

- In 19:16 the garments and thighs are both plural, each with HIS as possessor, and both
  ON phrases remain without an invented conjunction. The inscription is a singular
  written name, King of kings and Lord of lords. The vocalized MĀRAWĀTHĀ is the plural
  of lord/master, confirmed by the Classical Syriac dictionary's mara entry; the generic
  dominion gloss and feminine plural metadata do not override that actual form.

- In 19:17 ANOTHER angel is explicit. He cried and said are perfects, whereas standing
  and flying are participles. The bird noun is a feminine singular collective, naturally
  English "birds" alongside the plural come/gather imperatives; no ALL or THIS is
  imported. In 19:18 all five flesh nouns remain, with commanders of THOUSANDS (the
  lexical numeral is plural), mighty people, horses and their riders, free and servants,
  little and great. No additional ALL or free-BORN is supplied.

- In 19:19 the beast's armies and the kings' soldiers remain distinct, with their actual
  possessors. Gathering is an active participle, and making war is the purpose
  infinitive. War WITH these opponents gives idiomatic English "against"; no white
  adjective is supplied for the horse. In 19:20 the capture and casting are passive
  perfects; the false prophet performed signs before the beast and deceived people BY
  those signs. The mark, worshipers, and its image remain. BOTH DESCENDED is a separate
  explicit perfect action before they were cast; there is no ALIVE word. The short final
  noun phrase retains fire, its burning participle, AND OF sulfur as coordinated
  genitives. In 19:21 the rest were killed, with the adversative DEYN retained. The
  sword's suffix anticipates its named rider; the relative instrument repeats that
  sword. Here HIS mouth is singular, distinct from THEIR mouth in 19:15. The collective
  bird noun and feminine singular perfect are English plural "all the birds were
  satisfied," and THEIR flesh retains its genuine plural possessor.

Lexical support: https://cal.huc.edu/oneentry.php?cits=all&lemma=k%29n+A ;
https://cal.huc.edu/oneentry.php?cits=all&lemma=tryc+A ;
https://cal.huc.edu/oneentry.php?cits=all&lemma=mhymn+A ;
https://dukhrana.com/lexicon/PayneSmith/page.php?p=117 ;
https://assyrianlanguages.org/sureth/dosearch.php?language=id&searchkey=8443 .

Source checks for Revelation 20:

- All 15 verses and 237 actual source positions were checked against the inflected
  forms, every supplied gloss alternative, numeric morphology, suffixes, and final
  English owners. Each source position occurs once in ascending order. Two- and
  three-position noun phrases, compound prepositions, negative verb units, and short
  verb/subject units give grammatical English without hiding long clauses in combined
  links. The source's five ITH constructions have no HWA auxiliary: none is inherently a
  past existential, and no extra past/present assertion is manufactured. Supplied
  articles, copulas, and relative constructions support English grammar without
  replacing the actual lexical owners.

- In 20:1 ANOTHER angel is explicit. He descended is perfect; the following relative
  existential preserves the key UPON HIM and the separate great chain IN HIS HAND. In
  20:2 the seizure suffix anticipates the dragon object rather than adding another
  person. The FIRST serpent, the identifying existential, ACCUSER, Satan, bound HIM, and
  a thousand years remain. There is no source clause about deceiving the whole habitable
  world to import from the reference.

- In 20:3 the casting, closing, and sealing are perfect actions. English "it" supplies
  the understood abyss object for closing and sealing; it does not introduce a door
  noun. L'EL MIN is the short compound "above," with HIM as its genuine pronominal
  complement, not an extra independent "from him" clause. The negative, AGAIN, and
  imperfect deception verb form the short unit "so that he would not again deceive,"
  followed by ALL THE NATIONS. There is no source "until these thousand years are completed"
  clause. AFTER THESE THINGS is followed by YIHIV, a passive participle, and the
  infinitive TO RELEASE HIM for a little time. "It is granted" retains an impersonal
  passive permission construction rather than inventing an active giver or turning the
  following infinitive into an independent future event. CAL yhb G sense 3 confirms
  permit/allow beyond the simple give gloss.

- In 20:4 the parenthetical sitting and grant of judgment preserve the initial I SAW as
  governing seats AND THE SOULS; the souls noun has no recipient L-prefix, so no "and TO
  the souls" is supplied. The English "of those" gives the souls' human referents,
  retaining the demonstrative and following relative construction; AND OF THOSE at
  position 16 remains a second explicit genitive group. ETPSEQ is encoded as a masculine
  singular passive perfect; English "who were cut off" agrees with the plural human
  referents without inventing a HEAD noun or silently claiming the source verb is
  plural. Both causal phrases, testimony of Jesus and word of God, remain. All three
  worship/image/mark negatives are kept. ON BETWEEN THEIR EYES is rendered "on the
  space between their eyes," with a supplied English nominal head for the locative,
  rather than replacing EYES with forehead. THEIR HANDS are plural. D'HAYAW gives the
  relative WHO LIVED, followed by the perfect AND REIGNED; the Messiah has no THEIR
  suffix, and the thousand years have no THESE demonstrative. Verse 20:5 has only "And
  this is the first resurrection"; no rest-of-the-dead clause is imported from another
  textual tradition.

- In 20:6 blessed and holy, the singular person with a share, the first resurrection,
  and OVER THESE retain their actual owners. LAYT explicitly negates the authority
  available to the second death. "They will be" is imperfect, not an additional present
  "are"; priests belong to God AND THE MESSIAH, with no HIS suffix on Messiah. They WILL
  REIGN WITH HIM for a thousand years. In 20:7 MA D- is temporal WHEN, and the perfect
  completion form presents the years as completed before the future release. English
  uses "have been completed" in the temporal clause; a short three-position unit gives
  the verb and count phrase in grammatical English order. Satan WILL BE RELEASED is
  passive imperfect. His imprisonment retains its genuine singular possessor; no THESE
  is added to the years.

- In 20:8 he WILL GO OUT, with two purpose infinitives, to deceive ALL the nations and
  to gather THEM for battle. The corners suffix anticipates earth, and the numeral FOUR
  remains. L-Gog and L-Magog are definite direct-object markers in apposition to the
  nations, not new recipients requiring English "to Gog." Syriac's L-prefix can mark
  definite direct objects (Robinson/Coakley, sections 7 and 18). The final relative
  keeps THOSE, THEIR NUMBER, and the comparison with the sand of the sea; no plural
  actor replaces singular Satan.

- In 20:9 the breadth suffix anticipates EARTH and the surrounding suffix anticipates
  the named CITY. The source explicitly has THE CITY OF THE ENCAMPMENT OF THE HOLY ONES
  and then THE BELOVED CITY; the first CITY is not omitted as in the reference, and
  encampment is a noun, not an adjective modifying holy ones. The fire DESCENDED FROM
  HEAVEN, FROM GOD, in that order, and CONSUMED THEM. In 20:10 the Accuser is THEIR
  DECEIVER, an owned noun rather than a new past deception verb. He WAS CAST into the
  lake of fire AND sulfur; WHERE introduces the beast and false prophet, without an
  additional ALSO. The torment is a passive imperfect, WILL BE TORMENTED, day and night
  for the age of ages. Both source age nouns remain.

- In 20:11 the great white throne and the one SITTING ABOVE IT retain the actual
  participle and L'EL MIN compound. The resumed THE ONE introduces FROM BEFORE WHOSE
  FACE the earth and heaven FLED; FACE has the idiomatic plural form with a singular
  possessor, naturally singular English face. A PLACE WAS NOT FOUND FOR THEM is passive
  perfect; there is no possessive suffix on place or THIS demonstrative. In 20:12 the
  dead STOOD (perfect, not an active participle) before the throne. BOOKS plural were
  opened, then ANOTHER BOOK singular was opened, WHICH IS OF JUDGMENT, not the
  reference's life. The dead WERE JUDGED from the written things IN THE BOOK, singular
  in the actual vocalization and metadata, according to THEIR DEEDS. No source noun is
  moved to own the judgment verb or FROM particle.

- In 20:13 the sea GAVE UP its dead, and DEATH AND SHEOL GAVE UP those WITH THEM. The
  passive judgment verb is singular, followed by the distributive HAD HAD, EACH ONE OF
  THEM; the final possessor remains THEIR DEEDS, not the reference's his deeds. In 20:14
  death and Sheol WERE CAST into the lake of fire, followed only by THIS IS THE SECOND
  DEATH. There is no repeated "namely, this lake of fire" clause. In 20:15 AND WHOEVER
  introduces the negative passive finding, with MARKED as a passive participle, IN THE
  BOOK OF LIFE; the main casting verb is a separate passive perfect. No IF or THIS
  before the lake is imported from the reference.

Lexical and grammatical support:
https://cal.huc.edu/oneentry.php?cits=all&lemma=yhb+V ;
https://cal.huc.edu/cal_entry_web.php?lemma=l%28yl+X ;
https://www.syriac-eschool.com/wp-content/uploads/2022/02/Coakley_Robinson_Syriac_Grammar.pdf .

Source checks for Revelation 21:

- All 27 verses and 411 actual source positions were checked against the inflected
  forms, vocalization, every supplied gloss alternative, numeric morphology, suffixes,
  and final English owners. Each source position occurs once in ascending order. Two-
  and three-position noun phrases, postpositive particles, possessive constructions,
  negative verb units, and short predicates give grammatical English without putting
  long clauses into combined links. Actual forms control incomplete or inconsistent
  metadata. Articles, copulas, English nominal heads, and relative constructions are
  supplied where the languages require them; these do not replace named actors or move
  lexical words to other links.

- In 21:1 new heavens and new earth are distinct from the former heavens and former
  earth. The postpositive GAR belongs with the former-heavens phrase, and AZALU is a
  plural perfect. LAYT with TOB gives the sea's present negative existence, "is no
  more," not an additional past event. In 21:2 the holy city, new Jerusalem, is the
  fronted object of I SAW; the seeing suffix resumes that object without introducing a
  second city. DESCENDING is an active participle, PREPARED a passive participle, and
  ADORNED an adjective. The order is FROM HEAVEN, FROM THE PRESENCE OF GOD, then the
  bride comparison and HER HUSBAND. TSĒD denotes presence/proximity, not a new travel
  destination.

- In 21:3 the dwelling belongs to God and is WITH HUMANS. He DWELLS is an active
  participle, not an automatic future. THEY WILL BE HIS OWN PEOPLE is a short
  nominal-predicate/possessor/future-copula unit; it does not assert a separate present
  and future being. The final nominal God-with-them phrase supplies English "is with
  them," followed by the explicit AND WILL BE THEIR GOD. In 21:4 HE WILL WIPE AWAY ALL
  TEARS retains the active singular agent and plural tear noun, rather than making the
  wiping agentless. Their eyes have a genuine plural possessor. Death will not be
  HENCEFORTH, and mourning, UPROAR, and pain have their own negatives. AGAIN belongs
  with the final imperfect existence verb. The verse ends ON HER FACE: the actual suffix
  is third-feminine-singular, not their. Its antecedent is not silently replaced with a
  new noun. There is no former-things-passed causal clause in this source.

- In 21:5 AND I WENT is an explicit first-person perfect before the speech. English
  passive transposition "I was told by the one sitting" preserves the source's active
  speaker and indirect object while allowing the subject to follow the speech verb in
  source order. SITTING is an active participle, and ON THE THRONE remains. NEW + MAKE +
  I forms the short unit "I am renewing," followed separately by ALL THINGS; the present
  making is not converted into a perfect. The second HE SAID TO ME, the singular WRITE
  imperative, THESE WORDS, FAITHFUL AND TRUE, and the plural identifying existential
  remain. There is no BECAUSE particle or OF GOD ending to import from the reference.
  CAL mhymn confirms faithful/trustworthy beyond the export's limited believer gloss.

- In 21:6 HWAY is a separate third-feminine-plural perfect, "They have come to pass,"
  not a copula belonging to the next I. Both explicit first-person pronouns remain in I
  AM OLAPH AND I AM TAW. The actual Syriac letter names are retained, consistent with
  Revelation 1:8, rather than replacing them with Greek letters. Beginning and end are
  separate nouns. L-D-TSĀHE introduces THE THIRSTY ONE: despite the export's
  first-person coding, the bare stative form has no attached first-person pronoun and
  functions as the nominal recipient of I WILL GIVE. No extra assertion "I am thirsty"
  is introduced. The explicit I and first-person imperfect GIVE remain at positions
  10-11. EINA means SPRING in the waters/life context (CAL ayn), beyond the isolated eye
  gloss. FROM, the waters, life, and FREELY remain. In 21:7 the conquering verb is
  perfect, "the one who conquered," followed by the explicit HE and future inheritance.
  The two datives give natural English HIS GOD and MY SON, with both future copulas.

- In 21:8 the fearful list keeps the adversative DEYN, the separate unbelieving
  negation, unrighteous and defiled people, murderers, SORCERERS before FORNICATORS,
  worshipers OF IDOLS, and ALL LIARS. THEIR PORTION is a genuine plural possessor; the
  all suffix anticipates liars instead of adding another group. The burning lake, OF
  FIRE AND sulfur, and the second-death identification retain their actual source words.
  In 21:9 ONE OF SEVEN ANGELS is explicit. The relative existential and ON THEM give
  those upon whom are seven bowls, filled with the SEVEN LAST PLAGUES. He SPOKE is
  perfect; SAYING renders the following speech infinitive. Come and show-you retain
  their singular addressee. The wife's suffix anticipates the named Lamb; it does not
  introduce an additional independent owner.

- In 21:10 he LED ME AWAY in the spirit and SHOWED ME are third-person perfect actions
  with genuine first-person objects. The mountain is explicitly GREAT AND HIGH. The city
  is HOLY JERUSALEM, DESCENDING from heaven, from the presence of God. In 21:11 ITH plus
  the city's dative gives "it has" the glory of God. ITS LIGHT is a genuine possessive
  noun, followed by the actual repeated comparisons and both likeness nouns: precious
  stone, jasper, and CRYSTAL. No bright adjective or extra VERY is supplied from the
  reference.

- In 21:12 the city HAS a great and high wall, HAS twelve gates, and ON THE GATES are
  TWELVE ANGELS, which the reference omits. THEIR NAMES are written and identified as
  the names of TWELVE TRIBES OF ISRAEL; there is no children noun. The plural pronoun is
  retained without substituting a new explicit possessor. In 21:13 all four directions,
  east, north, south, and west, each have THREE GATES; FROM remains, with supplied
  English existential copulas for the nominal descriptions. In 21:14 the wall HAS twelve
  foundations, and ON THEM are twelve names of the apostles OF THE SON, not Lamb. The
  apostles' suffix anticipates the named Son, and the only twelve before names is not
  repeated as an additional apostle numeral.

- In 21:15 the speaking participle plus HWA gives WHO WAS SPEAKING. The separate ITH HWA
  ON HIM construction gives one past possession, HAD UPON HIM, not an extra present and
  past existence. Reed, measure, and gold form the short golden-measuring-reed noun
  phrase. The measuring infinitive's object suffix anticipates the city; the purpose is
  to measure THE CITY AND ITS WALL, with no gates noun. In 21:16 foursquare layout is a
  passive participial state; its length is like its breadth. The measuring action is
  perfect and its suffix anticipates the city. TWELVE THOUSAND STADES is the actual
  count and unit, without a second twelve or an unverified metric conversion. The final
  length, breadth, and height all retain their city possessor; the equal participle and
  plural copula give ARE EQUAL.

- In 21:17 the measuring verb's masculine suffix anticipates WALL, while the wall's
  feminine suffix identifies the city as possessor. The count is ONE HUNDRED AND
  FORTY-FOUR CUBITS. BY THE MEASURE OF A HUMAN, WHICH IS THE ANGEL'S retains the
  explicit genitive comparison of the measuring standard, without adding a second cubit
  unit. In 21:18 the wall's structure is jasper, and the city is OF PURE GOLD, IN THE
  LIKENESS OF PURE GLASS. In 21:19 the foundations of the wall of the city are adorned
  WITH PRECIOUS STONES, plural; there is no EVERY. The adornment is a passive
  participle, not a perfect event. The first foundation is jasper, followed in order by
  the second sapphire, third chalcedony, and fourth emerald.

- In 21:20 the fifth has SARDIUS AND ONYX as two actual coordinated source words. They
  are not collapsed into the reference's single sardonyx: CAL tpr explicitly cites this
  verse's additional AND ONYX and contrasts it with the Greek's single stone. The
  identical SARDON word at the sixth is also sardius. The seventh's KEPH DAHBA,
  literally stone-of-gold, is the recognizable name CHRYSOLITE, supported by the Syriac
  dictionary's specific compound entry. The two-position lexical unit retains both
  constituents; it does not assert a uniquely identified modern mineral specimen. Eighth
  beryl, ninth topaz, tenth chrysoprase, eleventh jacinth, and twelfth amethyst retain
  their actual numerals and lexical owners. The last two numerals have no conjunction
  prefix, so no extra AND is supplied.

- In 21:21 twelve gates AND twelve pearls are ONE FOR ONE: a supplied English copula
  expresses the source's nominal correspondence without adding a second gate/pearl
  equation. EACH ONE OF THE GATES WAS OF ONE PEARL retains the single past existential
  ITH HWA and the material MIN complement. The street OF THE CITY retains DEYN, OF PURE
  GOLD, LIKE GLASS, and the final present ITH BAH, IS IN IT. No bright adjective or
  extra AND before IS is imported. In 21:22 the fronted temple object is governed by I
  DID NOT SEE IN IT. GAR gives FOR; Lord, God, AHID KUL/ALMIGHTY, the explicit HE, IS,
  and ITS TEMPLE retain their source order and owners. There is no Lamb token in verse
  22.

- In 21:23 W-L-LAMB and W-L-CITY are parallel dative complements of passive METHBEA: FOR
  THE LAMB AND FOR THE CITY the sun is not required, nor the moon, that THEY might shine
  FOR IT. The two prefixes are not discarded to make a bare compound subject or move the
  Lamb into verse 22. The glory suffix anticipates God; ILLUMINATED IT is an active
  causative perfect with glory as its feminine singular agent and the city as object,
  not a present generic shining or an agentless passive. ITS LAMP IS THE LAMB retains
  the singular lamp and existential identification.

- In 21:24 nations WALK and kings BRING are present active participles, not
  automatically future. The source has neither a SAVED adjective nor THEIR on glory, nor
  a wealth-of-nations clause. TO IT remains. In 21:25 the city's gates WILL NOT BE
  CLOSED BY DAY is negative passive imperfect, and FOR NIGHT WILL NOT BE THERE keeps
  GAR, the second negation, future copula, and THERE. In 21:26 they WILL BRING TO IT
  GLORY AND HONOR OF THE NATIONS. In 21:27 the actual negative is WILL NOT BE THERE, not
  an ENTER verb. ANYTHING UNCLEAN, anyone DOING ABOMINATION AND FALSEHOOD, and ELLA EN,
  BUT ONLY, remain in the exclusion/exception construction. The qualifying people are
  WRITTEN IN THE BOOK OF THE LAMB. The book's suffix anticipates the Lamb, and there is
  no LIFE word in this source.

Lexical support:
https://cal.huc.edu/oneentry.php?cits=all&lemma=mhymn+A ;
https://cal.huc.edu/oneentry.php?cits=all&lemma=%28yn+N ;
https://cal.huc.edu/oneentry.php?cits=all&lemma=%2Bpr+N ;
https://www.assyrianlanguages.org/sureth/dosearch.php?language=id&searchkey=48938 .

Source checks for Revelation 22:

- All 21 verses and 283 actual source positions were checked against the inflected
  forms, vocalization, every gloss alternative, morphology, suffixes, and final English
  owners. Every position occurs once in ascending order. Short noun phrases, negative
  units, and predicates use two or three positions; long clauses are not hidden in
  combined links. The final chapter had two full independent source reviews and a
  continuous-English review, with the final narrow wording changes checked against the
  frozen baseline. Supplied English articles, copulas, nominal heads, and resumptive
  constructions express grammar without replacing actual actors or lexical owners.

- In 22:1 he SHOWED ME is perfect, while the river PROCEEDS is an active participle. The
  waters, life, PURE, ALSO BRIGHT, and crystal comparison remain distinct source words.
  The throne suffix anticipates God, with AND OF THE LAMB retained. In 22:2 the city's
  STREETS are plural; paired MEKA gives ON THIS SIDE AND ON THAT, along the river, with
  the tree of life. BEARS and GIVES are present participles. TWELVE FRUITS is not
  expanded to twelve kinds, and EVERY MONTH remains; ITS FRUITS and ITS LEAVES retain
  their genuine singular tree possessor and plural noun forms. No ONE is supplied before
  the monthly fruits. The leaves are FOR THE HEALING OF THE NATIONS.

- In 22:3 ALL CURSE under negation is the short unit NO CURSE, with WILL BE THERE
  retained. The throne OF GOD AND OF THE LAMB WILL BE IN IT, and HIS SERVANTS WILL SERVE
  HIM; the latter keeps both the genuine servant possessor and the verb's object suffix.
  In 22:4 HIS FACE has the idiomatic plural form, naturally singular English, and HIS
  NAME is ON BETWEEN THEIR EYES. A supplied English nominal head gives "on the space
  between their eyes," without replacing EYES with a forehead noun. The name's future
  copula is supplied from the connected future description, not claimed as an extra
  source HWA word.

- In 22:5 night WILL NOT BE THERE, and the second negative governs passive imperfect
  REQUIRE: light AND a lamp AND the light OF THE SUN will not be required FOR THEM. The
  source's separate light, lamp, and sunlight nouns are not collapsed into light OF a
  lamp. The sun's light suffix is anticipatory. The Lord God GIVES LIGHT TO THEM renders
  the active causative participle and its pronominal complement, not a future or passive
  event. The final word at position 16 is the noun THEIR KING, not a they-will-reign
  verb; English "and is their King" supplies the identifying copula, with FOR THE AGE OF
  AGES retained.

- In 22:6 THESE WORDS ARE FAITHFUL AND TRUE retains the recognizable absolute predicate
  MHAYMNAN, despite the export's generic believe/trust gloss. CAL mhymn confirms
  trustworthy/faithful. SPIRITS is plural, and HOLY modifies PROPHETS. The Lord God SENT
  HIS ANGEL: there is no ME object to import from the reference. The angel is to SHOW
  HIS SERVANTS WHAT IS GRANTED TO COME TO BE QUICKLY; YIHIV is a passive participle and
  HWA an infinitive. No independent MUST word is added. In 22:7 I AM COMING is a
  first-person active participle plus explicit I, followed by QUICKLY. The blessedness
  suffix anticipates the singular keeper; words, prophecy, and THIS BOOK remain
  separately owned.

- In 22:8 I AM JOHN identifies the narrator. The relative SAW and HEARD at positions 2-3
  are third-person singular perfects referring to John; the later I SAW and HEARD at 6-7
  are first-person perfects. Both sequences retain SEE before HEAR. I FELL TO WORSHIP is
  a perfect followed by a genuine purpose infinitive. The feet suffix anticipates the
  named ANGEL, whose SHOWING is a present participle, with ME and THESE THINGS retained.
  In 22:9 LOOK is an affirmative singular imperative, followed by NO, the elliptical
  refusal of the worship. It is not a prohibition against looking. Syriac normally
  expresses negative commands with LA plus an imperfect, as the grammar explains; the
  actual following verse has that construction. YOUR FELLOW SERVANT I AM and the
  genitive extensions to YOUR BROTHERS, THE PROPHETS, and the keepers of THESE WORDS OF
  THIS BOOK retain the separate identity assertion. TO GOD PAY HOMAGE preserves the
  object-fronted imperative and the second-person singular command.

- In 22:10 DO NOT SEAL renders LA plus the second-person imperfect prohibition. The
  words OF THE PROPHECY OF THIS BOOK remain, followed by GAR and THE TIME HAS DRAWN
  NEAR: QREB is perfect, not a nominal IS NEAR. In 22:11 the wrongdoer is a present
  participle, whereas the first filthy-state verb is perfect, "has become filthy." All
  four AGAIN tokens remain with their imperfect hortatives: do wrong, become filthy,
  practice RIGHTEOUSNESS, and BE SANCTIFIED. In 22:12 coming is present participial, AT
  ONCE is its own adverb, MY REWARD and WITH ME are separately linked, and AND I WILL
  GIVE is an active first-person imperfect, not an infinitive recompense clause.
  EVERYONE, ACCORDING TO, and HIS WORK retain their owners.

- In 22:13 both explicit I pronouns give I AM OLAPH AND I AM TAW, consistent with
  Revelation 1:8 and 21:6. The first, last, beginning, and end remain separate
  predicates. In 22:14 DOING HIS COMMANDMENTS is present participial; THEIR AUTHORITY
  WILL BE OVER THE TREE OF LIFE retains the plural possessor, singular authority noun,
  and future existence. The actual GATE is singular, through which THEY WILL ENTER THE
  CITY. In 22:15 fornicators, murderers, and worshipers OF IDOLS precede OUTSIDE.
  Supplied English copulas and resumption connect the subsequent unclean people,
  sorcerers, and ALL OBSERVERS AND DOERS OF FALSEHOOD. OBSERVERS is a plural noun, not
  the reference's lovers; no dogs word occurs in this source.

- In 22:16 I, JESUS, SENT MY ANGEL is perfect with the genuine first-person angel
  possessor. The purpose clause is THAT HE MIGHT TESTIFY AMONG YOU THESE THINGS BEFORE
  THE ECCLESIAS, retaining the second-person plural audience. Both following I pronouns
  give the emphatic I MYSELF AM. The lineage suffix anticipates DAVID; the ROOT AND
  LINEAGE OF DAVID are not given an extra independent HIS. AND WITH HIM is a genuine
  separate pronominal phrase and is not omitted or assigned a new named antecedent. The
  following coordinated predicate retains the STAR, MORNING, and BRIGHT in the short
  bright-morning-star noun phrase.

- In 22:17 the Spirit and bride SAY, with an active participle; COME is a singular
  imperative. The hearer LET HIM SAY and the thirsty one LET HIM COME AND TAKE are
  imperfect hortatives. The source has no separate whoever-is-willing clause. The
  waters, life, and FREELY remain. In 22:18 I TESTIFY is present participial, and the
  audience hears THE WORD, singular, OF THE PROPHECY OF THIS BOOK. The next relative
  introduces anyone ADDING TO THEM, with the genuine feminine plural pronominal
  complement preserved even though the earlier word noun is singular. GOD is the
  explicit active subject of WILL PLACE UPON HIM THE PLAGUES WRITTEN IN THIS BOOK. A
  short verb/prepositional-object/subject unit makes that English clause grammatical
  without inventing another agent.

- In 22:19 the first decreasing verb is a present participle with FROM, "takes away
  from" the words of the book OF THIS PROPHECY; THIS is feminine and belongs to
  prophecy. GOD WILL DIMINISH is an active causative imperfect, not an agentless
  passive. HIS PORTION retains its genuine possessor, FROM THE TREE OF LIFE AND FROM THE
  HOLY CITY. The final plural THOSE THINGS WRITTEN IN THIS BOOK is a resumptive
  apposition, not a singular relative modifying city alone. In 22:20 HE SAYS and WHILE
  TESTIFYING retain the two participles and temporal KAD; YES, I AM COMING QUICKLY
  follows. COME, LORD JESUS is a separate imperative. There is no AMEN in verse 20. In
  22:21 the grace suffix anticipates OUR LORD JESUS THE MESSIAH, and the final
  benediction is WITH ALL HIS HOLY ONES. HIS is a genuine singular possessor on the
  plural holy ones. The explicit final AMEN is retained in its actual verse.

Lexical and grammatical support:
https://cal.huc.edu/oneentry.php?cits=all&lemma=mhymn+A ;
https://cal.huc.edu/oneentry.php?cits=all&lemma=yhb+V ;
https://www.syriac-eschool.com/wp-content/uploads/2022/02/Coakley_Robinson_Syriac_Grammar.pdf
(sections 10 and 13-14 on participles, negative commands, and infinitives).
