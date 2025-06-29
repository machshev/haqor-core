# 3. Use a custom bible format

Date: 2025-05-13

## Status

Accepted

## Context

### Why yet another bible format?

The issue I'm coming across is that while there are lots of bible formats out there already.
None of the formats I've found meet the unique requirements of Haqor.
This is not all that unexpected as Haqor aims to be a lot more than just another SWORD front-end.
It's a fundamental paradigm shift in terms of being an original language and study first application.

So a key requirement for a bible format is good seporation of data and presentation.
Which is vital for machine readable parsing, for advanced searching and analytics.

### Ubiquitous format

While Haqor is the primary user, it would be nice if others could easily write scripts or extensions.
So it's important that the base format is easy to load using many existing programming languages.

The sword format is a binary format that can't be parsed without reimplementing a complex binary format.
To use SWORD modules you have to depend on a C library that doesn't appear to be well maintained, and stuck with many legacy requirmeents.
These days it's not as important to compress the bible data into as small a space as possible.

So I'd rather make use of an already well established container file format with well established built in language support.
Which means that we only have to implement the business logic, examples are JSON, YAML, XML, or a database format like sqlite.
All of these have good programming language support for serialisation and deserialisation.

### Presentation over data

MySword is a more modern module format based on sqlite, so it's easy to load using any programming language.
There seems to have been a lot of thought put into the [module format](https://mysword-bible.info/modules-format).
However it seems to be essentially taking existing SWORD modules and storing them in SQLite DB.

This issue with this is that it is designed for broad support for existing bible modules, in a variety of different presentations.
HTML is used to allow module writers to capture the presentation format along with the actual bible text.
This makes sense for a standard bible reading app, where the goal is to support many different translations in many different languages.
However for Haqor this makes it impossible support every mysword bible module.
Because reliably seporating the text from the presentation for each module would mean writing a new parser for each module.

Haqor cares about the actual words, while applications like mysword only cares about verses.
If a verse can be renderred with a WebView as valid HTML then that's all that matters.
It's up to the module developers to make sure they put in strongs number based tags in the right places.

### lexical indexing

SWORD based modules (including mysword) all assume strongs numbers as the primary index for lexicons.
While this works farily well for Hebrew and Greek, it doesn't work at all for Aramaic/Syriac where SEDRA DB would be more appropriate.
Also while many Hebrew lexicons have strongs based indexing, this isn't neceserily the ideal index.

## Decision

Develop our own haqor bible module format using SQLite as the base format taking inspiration from mysword.

SQLite is a well established format that has a fairly small footprint (as compared to something like XML).
It makes it easier to implement searching and analytics functionality that we will need.

The bible modules should have first class support for original languages:

- Hebrew
- Aramaic
- Greek

Rather than what appears to have happened with SWORD modules, where English is the primary language and optionally embedded strongs numbers are the primary link.

Within Haqor, bible text should be in the original language.
From there words can be indexed by lexical form and morphology.
Lexicons will be provided that are indexed by lexical form directly rather than some intermediate index number such as strongs numbers.
Morphology keys can be provided in formats similar to the various "Readers Hebrew Bibles" available in print form.

The exact format of these modules will be defined in further ADRs.

## Consequences

We will need to define our own bible module format and provide an easy way of interacting with those modules.
