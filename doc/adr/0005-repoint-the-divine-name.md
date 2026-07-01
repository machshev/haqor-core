# 5. Re-point the divine name on import

Date: 2026-06-28

## Status

Accepted

## Context

The Tetragrammaton — the four consonants יהוה — is the most frequent word in the
Hebrew Bible (~6,800 occurrences across all forms). In the Masoretic text it is
never written with its own spoken vowels. The Masoretes pointed the consonants
with the vowels of the word to be **read in its place** (a *qere perpetuum*):

- usually אֲדֹנָי "Adonai" → **יְהוָה**,
- אֱלֹהִים "Elohim" where Adonai already stands adjacent → **יְהוִה**,
- with plene **יְהֹוָה / יְהֹוִה** and a hataf-segol **יֱהוִה** variant,
- and the same on proclitic forms — לַיהוָה, וַיהוָה, בַּיהוָה, מֵיְהוָה,
  וּבַיהוָה …

These points are not the pronunciation of the name; they are a reading
instruction. A pipeline that treats the surface vocalisation as the word's sound
therefore produces wrong output for the single most common word in the text.

This first surfaced in the spaced-repetition **reading tutor**: its "read this
word" card romanises the surface niqqud,
and for יְהוָה that yields the non-word **"yehva"** — actively miseducating the
learner and obscuring the name. The same artifice degrades any consumer that
reasons over the vocalisation (transliteration, syllabification, the morphology
generator, search by vowel pattern).

Two places the fix could live:

1. **Downstream, per consumer** — e.g. a Dart-side reading override in the app
   keyed on the divine-name surfaces. Local and reversible, but every consumer
   that cares about vocalisation must carry the same special case independently,
   and the source data still asserts a vocalisation the manuscripts do not.
2. **Upstream, at import** — re-point the consonants once, in the UXLC parser, so
   every derived artifact (`bible.db`, and `hebrew.db`'s `surface` / `verse_word`
   built from it) and every downstream consumer sees a single, consistent form.

This is a deliberate, eyes-open departure from a faithful transcription of the
Leningrad Codex. The justification: the qere-perpetuum pointing is itself an
editorial substitution layered over the consonantal text, not the name's own
vocalisation. Restoring the reconstructed pronunciation removes a known
distortion rather than introducing one, and serves this project's purpose — to
teach people to *read* the Hebrew Bible. The cost is that the displayed text
diverges from printed editions exactly at the divine name, in every view
(reader, occurrences, and the tutor's verse-reading reward), not only on the card
that prompted it.

## Decision

Re-point the divine name to its reconstructed pronunciation **יַהְוֶה**
("Yahweh") at import time, in `uxlc::repoint_divine_name`, applied per `<w>` in
`parse_book` alongside the existing `strip_internal_maqaf` and `split_glued_word`
source-normalisations.

Detection is by consonant skeleton: the four name-consonants יהוה, optionally
behind one or two proclitics (conjunctive vav and the inseparable prepositions
lamed / bet / kaf / mem). The proclitic letters and their pointing are kept; only
the four name-consonants are re-pointed; a word-joining maqaf is preserved. The
match is exact — יְהוּדָה "Judah" (with a dalet) and the short form יָהּ "Yah"
(two consonants) do not match. In the Masoretic text the bare consonant string
יהוה is reserved for the divine name, so the rule has no false positives on the
corpus.

Because every artifact derives from `uxlc::parse_all`, the change propagates
consistently with no per-consumer special-casing.

## Consequences

- The reading tutor, transliteration, reader, occurrences and morphology all see
  one honest, pronounceable form. "yehva" is gone.
- The re-pointed form יַהְוֶה is a homograph of the III-he imperfect it derives
  from ("he will be / he causes to be" — the name's own etymology), so the verb
  parser now resolves it directly. We deliberately do **not** special-case it: if
  it parses, it parses. The previous proper-noun prefilter entry (needed only
  because the qere pointing had no verb reading) is removed.
- The shipped text diverges from a strict Leningrad transcription **at the divine
  name only**. This is intentional and documented here; anyone diffing against a
  printed edition or another WLC-derived dataset will see it.
- The original qere pointing (and thus the explicit *Adonai* / *Elohim* reading
  tradition it encodes) is no longer recoverable from the data. If a future view
  wants to *teach* the qere ("written YHWH, read Adonai") it must reintroduce the
  substituted pointing rather than recover it — accepted, as the project's aim is
  reading the name, not the substitution.
- Cantillation accents on the bare name are dropped together with its old
  pointing (re-anchoring a ta'am onto a re-spelled word is ill-defined). Accents
  on proclitics and on every other word are untouched.
- The choice of **Yahweh** over Adonai / "the LORD" is a reconstruction; should
  scholarship or project preference change, only `YAHWEH` in `uxlc.rs` and a DB
  regeneration are needed.
