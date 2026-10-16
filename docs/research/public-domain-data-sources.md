# Public domain data sources for Fontes

Research checked October 15, 2026, against Bible Study Software Specification v0.1. The recommendation assumes an initial 66-book Protestant canon, KJV first, local SQLite storage, and public domain or CC0 content. Rights classifications below describe the identified edition and the provider's statements, not every digital edition of the same historical work.

Start with **KJV verse text, WEB, both Strong's dictionaries, Robinson's Textus Receptus Greek, Matthew Henry, Nave, and the Treasury of Scripture Knowledge**. These provide reading, dictionary lookup, Greek token analysis, commentary, and topical retrieval from downloadable files. Add Tidwell as source material for reviewed book overviews. Genealogies still need a small curated dataset with verse evidence. Complete Hebrew morphology remains a separate decision: the familiar OSHB dataset is CC BY, and the promising Berean tables need a clearer determination of the rights covering their annotation columns.

## Recommended sources

| Resource | Specific source and format | Rights evidence | Import decision |
| --- | --- | --- | --- |
| KJV base text | [eBible eng-kjv](https://ebible.org/bible/details.php?id=eng-kjv&all=1), [VPL ZIP](https://ebible.org/Scriptures/eng-kjv_vpl.zip) containing XML and UTF-8 text | Provider labels the Bible public domain, with a UK rights exception | First batch. Parse the XML verse records and retain the selected 66 books. Treat added word annotations separately. |
| World English Bible Classic | [eBible eng-web](https://ebible.org/bible/details.php?id=eng-web&all=1), [VPL ZIP](https://ebible.org/Scriptures/eng-web_vpl.zip) or USFM | Explicit public domain dedication; the WEB name is a trademark | First batch, as a modern English translation. This edition includes deuterocanonical books; select the initial canon explicitly. |
| Strong's Greek dictionary | [morphgnt/strongs-dictionary-xml](https://github.com/morphgnt/strongs-dictionary-xml), [XML](https://raw.githubusercontent.com/morphgnt/strongs-dictionary-xml/master/strongsgreek.xml) | README explicitly applies CC0 to this digital edition | First batch. Structured entries, Greek headwords, transliteration, definitions, derivation, and cross-references. |
| Strong's Hebrew dictionary | [Open Scriptures StrongHebrewG.xml](https://github.com/openscriptures/strongs/blob/master/hebrew/StrongHebrewG.xml), OSIS XML | The specific XML work header declares Public Domain | First batch. Use this XML edition, not a blanket assumption about all Open Scriptures exports. |
| Greek Textus Receptus with parsing | [byztxt/greektext-textus-receptus](https://github.com/byztxt/greektext-textus-receptus), per-book `.UTR` files | Official repository explicitly says Public Domain | First batch. A useful Greek witness for a KJV-oriented product. Convert its ASCII Greek convention and parse the embedded tags. Do not claim exact KJV token alignment. |
| Matthew Henry complete commentary | [CrossWire MHC](https://crosswire.org/sword/modules/ModInfo.jsp?modName=MHC), [module ZIP](https://www.crosswire.org/ftpmirror/pub/sword/packages/rawzip/MHC.zip) | Module page and downloaded configuration declare Public Domain | First batch. Open SWORD format, compressed OSIS content; needs a build-time export that preserves passage links. |
| Nave's Topical Bible | [CrossWire Nave](https://crosswire.org/sword/modules/ModInfo.jsp?modName=Nave), [module ZIP](https://www.crosswire.org/ftpmirror/pub/sword/packages/rawzip/Nave.zip) | Module page and downloaded configuration declare Public Domain | First batch. Topics and scripture references can support keyword and natural-language retrieval. Compressed TEI dictionary module. |
| Treasury of Scripture Knowledge | [CrossWire TSK](https://crosswire.org/sword/modules/ModInfo.jsp?modName=TSK), [module ZIP](https://www.crosswire.org/ftpmirror/pub/sword/packages/rawzip/TSK.zip) | Module page and downloaded configuration declare Public Domain | First batch. Source for passage cross-references; compressed ThML commentary module. |
| Book overview source material | [Tidwell, The Bible Book by Book](https://www.gutenberg.org/ebooks/16290), [UTF-8 text](https://www.gutenberg.org/cache/epub/16290/pg16290.txt) and HTML | Project Gutenberg identifies this edition as public domain in the USA | First batch editorial input. Split and review the historical introductions, outlines, and purposes into our overview fields. Several books share a chapter. Preserve attribution and uncertainty. |
| Easton's Bible Dictionary | [CrossWire Easton](https://crosswire.org/sword/modules/ModInfo.jsp?modName=Easton), [module ZIP](https://www.crosswire.org/ftpmirror/pub/sword/packages/rawzip/Easton.zip) | Module page and downloaded configuration declare Public Domain | Useful expansion for people, places, and background articles. Compressed TEI dictionary module. |
| International Standard Bible Encyclopedia | [CrossWire ISBE](https://crosswire.org/sword/modules/ModInfo.jsp?modName=ISBE), [module ZIP](https://www.crosswire.org/ftpmirror/pub/sword/packages/rawzip/ISBE.zip) | Module page and downloaded configuration declare Public Domain | Expansion for historical reference articles; select this old edition, not a modern revision. Compressed TEI dictionary module. |
| American Standard Version 1901 | [eBible eng-asv](https://ebible.org/bible/details.php?id=eng-asv&all=1), [VPL ZIP](https://ebible.org/Scriptures/eng-asv_vpl.zip) or USFM | Provider identifies it as public domain | Easy later translation through the same importer. Download route listed by provider; archive not inspected in this research. |
| Robinson–Pierpont Byzantine Greek | [Official repository](https://github.com/byztxt/byzantine-majority-text), Unicode CSV and TEI XML | README declares all repository text/code public domain; LICENSE uses the Unlicense | Good alternate Greek witness. A Unicode CSV sample was parsed. Keep distinct from Textus Receptus. |

## What the actual downloads contain

These checks establish file accessibility and structure. They do not constitute a full textual collation or a completed production importer. Download measurements and SHA-256 values are recorded in `source-validation.json`; a machine-readable acquisition list is in `source-manifest.json`.

- **KJV:** the VPL archive is 5,226,544 bytes and contains `<verseFile><v b="GEN" c="1" v="1">…</v></verseFile>`. Selecting the 66 standard book codes produces **31,102 unique verses, 1,189 chapters, and 66 books**, ending at Revelation 22:21. The archive also contains additional books. Its SQL file uses MySQL syntax, so it is not a ready-made SQLite database.
- **WEB:** the VPL archive downloaded successfully at 5,344,189 bytes. XML and text are suitable import inputs. Use USFM instead if paragraph breaks, poetry, and notes are required for the reading surface; VPL is useful for reference and text validation.
- **Greek Strong's:** XML parsed successfully into **5,624 entries**. Preserve mixed XML content and dictionary cross-references instead of concatenating only the first text node.
- **Hebrew Strong's:** OSIS XML parsed successfully into **8,674 entry divisions**. The header also describes the copyrighted modern TWOT reference work. Import the public domain Strong's entries and treat TWOT identifiers as external references; this is not permission to reproduce TWOT article content. The repository's JavaScript files carry different notices.
- **Textus Receptus:** the John `.UTR` sample has chapter/verse markers, wrapped lines, ASCII Greek forms, Strong's identifiers, additional numerical parsing identifiers, and brace-enclosed morphology such as `{V-IAI-3S}`. A token can have alternative analyses. It is not CSV, and a newline is not necessarily a verse boundary.
- **Byzantine Greek:** the John CSV sample has **879 verse rows**, plus a header, with fields `chapter,verse,text`. The `text` cell itself contains Greek forms, Strong's numbers, and morphology. It is not one token per row. The parsed sample is unaccented; an accented presentation requires the repository's separate CCAT representation and a checked mapping.
- **Matthew Henry:** a 15,195,123-byte module archive with `ModDrv=zCom4`, `SourceType=OSIS`, UTF-8, and KJV versification. Archive and configuration inspected; content export and link preservation remain to be implemented and verified.
- **Nave, TSK, Easton, ISBE:** all four archives downloaded, and their configurations confirm their markup, module drivers, and Public Domain distribution declarations. Their respective download sizes are approximately **1.30, 2.64, 1.44, and 9.99 MB** in decimal units. These are compressed module sizes, not normalized SQLite or index sizes.
- **Tidwell:** UTF-8 text downloaded successfully, approximately **302 KB**. Historical author/date/purpose/outline discussions are usable editorial inputs, not an already normalized 66-row dataset.

## Sources needing a separate decision

### Tagged KJV editions

The [eBible eng-kjv2006 page](https://ebible.org/bible/details.php?id=eng-kjv2006&all=1) labels its text public domain and offers a [USFM ZIP](https://ebible.org/Scriptures/eng-kjv2006_usfm.zip). The 2.46 MB download really contains word-level Strong's tags. The `eng-kjv` USFM also contains tags, despite its less explicit title.

However, the upstream [CrossWire KJV module](https://crosswire.org/sword/modules/ModInfo.jsp?modName=KJV) identifies its distribution license as GPL and explains the modern annotation work. These statements are not sufficient to classify the annotation layer as unambiguously public domain. Use the underlying KJV verse text now; hold imported KJV word tags until that edition's annotation rights are resolved. The previously advertised eBible KJV2006 OSIS URL returned 404 during this check; use the currently listed formats.

### Berean translation tables

The [official downloads page](https://berean.bible/downloads.htm) provides [TSV](https://bereanbible.com/bsb_tables.tsv), XLSX, USFM, USX, and USJ. The [terms](https://berean.bible/terms.htm) dedicate the Berean and Majority Bible texts to the public domain using CC0.

The TSV was downloaded and parsed: **85,525,373 bytes, 23 columns, 754,647 data rows, 443,625 rows with a source-language form, and 31,102 distinct explicit verse references**. It includes Hebrew, Aramaic, Greek, transliteration, short and expanded parsing, Strong's identifiers, translation phrases, ordering columns, and variants. Many rows are placeholders; verse references occur sparsely and need contextual carry-forward. There is no explicit lemma column. The two `Parsing` headers also need distinct internal names.

This is a strong technical candidate for OT/NT word study and BSB alignment. The general dedication names the Bible texts; this research did not establish an equally explicit grant covering every annotation and third-party textual column. Hold those columns from a strict public domain pack pending that determination. BSB English text itself can be imported under the published dedication. BSB phrase alignments do not become KJV alignments.

### Hebrew base text and morphology

[Tanach.us](https://www.tanach.us/Tanach.xml) offers Unicode/XML Leningrad Codex text; a [Genesis XML sample](https://www.tanach.us/Books/Genesis.xml) was downloaded and parsed. Its [current license](https://tanach.us/License.html) permits unrestricted copying of the biblical Hebrew text, while reserving rights in other site files. Label the edition **UXLC**, as requested by the provider, rather than calling it WLC. Import the Hebrew text only; do not assume the site code, fonts, and explanatory material share its status. The current notice is an unrestricted-text permission, not an explicit CC0 declaration for the whole XML package.

For strict public domain Hebrew base text, the underlying WLC is another route: [OSHB's README](https://github.com/openscriptures/morphhb) distinguishes its public domain WLC text from its **CC BY 4.0 lemma and morphology data**. Importing just the PD text is possible, but does not deliver the missing annotations.

### Genealogies

The most convenient ready-made datasets fall outside the strict public domain requirement:

- [STEPBible TIPNR](https://github.com/STEPBible/STEPBible-Data) is UTF-8 tab-separated data with differentiated persons, family relationships, and verse occurrences, under **CC BY 4.0**. Its README identifies some descriptions as AI-generated; those should not be treated as historical source text.
- [Theographic](https://github.com/robertrouse/theographic-bible-metadata) offers JSON and CSV under **CC BY-SA 4.0**. Its person records are useful technically, but its license is not public domain.
- [Wikidata structured data](https://www.wikidata.org/wiki/Wikidata:Licensing) is **CC0**, with [JSON and query access](https://www.wikidata.org/wiki/Wikidata:Data_access). It is a candidate for identifiers and relationship discovery; this research did not validate its biblical relationship coverage or per-edge scripture evidence.

For v0.1, curate the requested major lines from the public domain scripture: Genesis 5, Genesis 10–11, Ruth 4:18–22, Matthew 1, and Luke 3. Give every relationship its own verse evidence. Keep parentage, ancestry, spouse, and interpretive identifications distinct. This is recommended work still to do, not a completed genealogy dataset.

### Convenient collections that are not blanket public domain sources

[MorphGNT SBLGNT](https://github.com/morphgnt/sblgnt) applies CC BY-SA to its parsing and lemmatization and points separately to the text's terms. [STEPBible](https://github.com/STEPBible/STEPBible-Data) is CC BY 4.0 and is an excellent alternative if the content policy later permits attribution licenses. [OpenBible cross-references](https://www.openbible.info/labs/cross-references/) are CC BY; use the TSK edition above for the strict public domain batch.

[Scrollmapper](https://github.com/scrollmapper/bible_databases) has ready-made SQLite, JSON, CSV, and other exports, but includes works with differing source rights. Its repository's MIT license does not establish that every included translation or annotation is public domain. It is useful for prototyping format adapters after individual provenance checks.

[CCEL](https://www.ccel.org/ccel/henry/mhc.html) has attractive ThML XML downloads. Its [site policy](https://www.ccel.org/about/copyright.html) distinguishes underlying PD books from modern special content and requests permission for commercial republication. Prefer the specifically PD-labelled CrossWire modules for this batch instead of treating every CCEL download as unrestricted.

## Import design for the specification

1. **Keep immutable source provenance.** For each acquisition store the exact URL, edition, upstream commit when available, retrieval date, SHA-256, rights statement URL, and permitted content layer. A single `is_public_domain` boolean cannot represent these distinctions; avoid defaulting unknown resources to approved.
2. **Use a build-time conversion stage.** Parse verse XML and dictionary XML directly. Export SWORD content with its established tools, then normalize it into project-owned JSONL or SQLite records. SWORD is an open format, so this need not add a proprietary format or a C++ runtime to the Rust app.
3. **Preserve linked entries during SWORD export.** CrossWire's [module tooling documentation](https://wiki.crosswire.org/DevTools:Modules) warns that `mod2imp` skips link entries. A naive export can lose commentary coverage. Verify linked keys against the source module and deduplicate article content while preserving every supported reference.
4. **Retain reading structure.** VPL is excellent for a first reference/text import, but insufficient for all typography requirements. Preserve USFM paragraph, poetry, footnote, and translator-supplied-word information separately when adding the richer reading edition. Keep annotation rights separate from base text rights.
5. **Add edition and reference mappings.** Original tokens need a `text_witness_id` or equivalent; TR, RP, and Hebrew editions cannot share an undifferentiated token stream. Preserve source references and map them to canonical anchors. Hebrew/English verse numbering, titles, omitted verses, and variants require explicit mappings.
6. **Do not infer alignments from shared Strong's numbers.** A verse can contain the same lexeme multiple times, words can span multiple translation words, and a Strong's entry is not always a normalized lemma. Keep source Strong's codes, token positions, alternate analyses, and derived dictionary headwords distinct. Verified KJV word-to-original-token alignment remains additional work.
7. **Treat overviews as edited content.** Tidwell, Henry's introductions, and old encyclopedia articles supply source material. They do not consistently populate every required field. Record historical author/date claims as attributed claims, leave unsupported fields unset, and review summaries before publishing them.
8. **Budget after normalization.** The measured compressed files are modest enough to justify a prototype. They do not prove the spec's final size target: full commentary text, token tables, FTS, and embedding indexes must be measured separately. Verse text and reference-rich topic articles can seed retrieval before adding embeddings.

The immediate acquisition step can proceed for the clearly classified base texts, dictionary XML editions, official Robinson Greek, and PD-labelled commentary/reference modules. The remaining work is importer construction, text/link validation, curated genealogies and overviews, and explicit decisions on the conditional annotation datasets.
