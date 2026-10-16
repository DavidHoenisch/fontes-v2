# Authoring a Fontes resource

Status: guide to the draft source format. The [authoring CLI](cli-guide.md) implements scaffolding, validation, packaging, schema export, and the [source-submission pipeline](submission-pipeline.md); app delivery compilation and the catalog remain planned. Use the [complete Romans study example](../../specs/resource-packages/v1/examples/romans-study/) as a concrete starting point; its publisher is fictional.

## Prepare the source directory

One resource represents a work or independently maintained edition. A Bible normally includes its books in one resource. A collection is a catalog bundle of resource IDs.

```text
metadata.xml
index.xml
assets/cover.png
rights/LICENSE.txt
rights/ATTRIBUTION.txt
content/introductions/romans.xml
content/rom/001.xml
taxonomy/concepts.xml             # optional new concept proposals
media/illustration.png            # optional referenced artwork
```

Place these files directly at the ZIP root. Name the archive `<resource-id>--<version>.fontes-source.zip`. Use relative paths with `/`, portable ASCII letters/digits/underscores/hyphens, and dots within filename components; preserve case and avoid case-only distinctions. XML text is UTF-8, including non-Latin languages. Paths never contain `.` or `..` components, backslashes, or an absolute root.

Keep content under `content/`. A chapter per file is recommended; introductions and dictionary entry groups can use separate files. Every content file appears exactly once in the ordered index. Optional artwork belongs under `media/` and must be referenced. Do not include scripts, SQL, fonts, private agreements, or generated databases. The limits are 16 MiB per XML file, 10,000 archive entries, 1 GiB expanded total, XML depth 32, and navigation depth 8.

## Describe the work and its rights

Follow [metadata.xml](../../specs/resource-packages/v1/examples/romans-study/metadata.xml) and [metadata.xsd](../../specs/resource-packages/v1/metadata.xsd); XML element order matters. Replace the fictional identity, title, description, contributors, publisher, edition/date, coverage, and rights with accurate information. The publisher namespace must be verified before publication. A resource such as `org.example.romans-study` belongs to publisher `org.example`.

Use a numeric `major.minor.patch` version, initially such as `1.0.0`. Published identity/version bytes are immutable. A new text, metadata, artwork, or licensing revision needs a new resource version; Fontes can separately increment a compilation build revision.

Supply the actual distribution terms in `rights/LICENSE.txt` and required credits/source notices in `rights/ATTRIBUTION.txt`. Declare offline display, local keyword search, export, printing, embeddings, and AI-context permissions individually. A free download does not establish public-domain status. The first catalog requires permission for public redistribution, offline display, and keyword indexing. Restrictions that the initial mechanism cannot honor prevent publication until suitable support exists.

Provide exactly one master cover, `assets/cover.png` or `assets/cover.jpg`: **1200 × 1800 pixels, PNG/JPEG, at most 4 MiB**. Fit older artwork onto that canvas without stretching. Include meaningful alternative text, artwork rightsholder, license identifier, and attribution in the cover metadata. Text and cover permissions are reviewed separately. CI will generate the browsing images; authors do not submit thumbnails.

## Structure and link the content

Use [index.xml](../../specs/resource-packages/v1/examples/romans-study/index.xml) for navigation order and labels. Give every document and block a stable ID unique across the resource. Keep those IDs when wording or file locations change. New paragraphs receive new IDs.

This complete document illustrates an explicit citation, an authored implicit connection, and a word-study link:

```xml
<document xmlns="urn:fontes:resource:1"
          id="rom-001" resource-id="org.example.romans-study">
  <title>The opening of Romans</title>
  <block id="rom-001-p01" kind="paragraph">
    <text>Read <ref start="Rom.1.3">Romans 1:3</ref> alongside this discussion.</text>
  </block>
  <block id="rom-001-p02" kind="paragraph">
    <text>This paragraph discusses the Messiah's human ancestry.</text>
    <annotations>
      <scripture start="Rom.1.3" relation="alludes-to" basis="author"/>
      <strongs code="G5207" relation="discusses"/>
    </annotations>
  </block>
</document>
```

Use the most specific relevant paragraph or verse, rather than attaching every reference only to a whole chapter. Supported scripture relationships are:

| Relationship | Use |
| --- | --- |
| `text-of` | A translation verse's canonical mapping |
| `applies-to` | The passage covered by commentary or another discussion |
| `cites` | An explicit scripture reference |
| `quotes` | Quoted scripture, retaining its known edition/witness |
| `alludes-to` | An implicit connection explicitly supplied as metadata |

References use registry OSIS identifiers such as `Rom.1.3`, with `end` for an inclusive range. A chapter/book endpoint expands according to the frozen [canonical registry](../../data/canonical/registry-v1.json). Declare the source versification and map numbering differences explicitly. A translation verse uses `kind="verse"`, its original `source-ref`, and `text-of` annotation(s); an actual source omission uses `status="omitted"` with no invented text. Declare selected versus complete coverage accurately.

An inline `<ref>` creates a citation link automatically. `<strongs code="G5207">…</strongs>` and Strong's annotations use normalized, range-checked codes. They work without a dictionary installed; definitions appear when available. The draft prose format does not yet express full token-level morphology, interlinear alignment, tables, or rich footnotes. Do not flatten these into misleading prose or invent unsupported elements; those profiles need an extension before publisher use.

For topic annotations and new terms, follow the [concept guide](concept-guide.md). Attach the concept's ID, not merely a label.

## Validate, submit, and update

The [contract checks](../../specs/resource-packages/v1/README.md#validate-and-package-the-example) run against the fixed checked-in example. They are useful for reviewing the design, not a claim that your own ZIP passed validation. You can check a known local content file structurally with:

```sh
xmllint --nonet --noout \
  --schema specs/resource-packages/v1/content.xsd \
  specs/resource-packages/v1/examples/romans-study/content/rom/001.xml
```

XSD validation alone does not check complete coverage, all cross-file references, or archive safety; use the implemented `fontes-resource validate` command for semantic checks and `pack` for deterministic source archives. Its file/line/column diagnostics include corrective instructions; see the [CLI guide](cli-guide.md). Publisher authority and rights still require human review. Submit authorized free sources under this repository's `library/submissions/` with the descriptor defined by the [submission pipeline](submission-pipeline.md); CI provides reports, source previews, and signed releases after exact-hash maintainer approval. Interactive preview and submission-helper tools remain planned. A dedicated repository/private intake contact can be added later. Do not upload a copyrighted or uncertain-rights work to a public PR while waiting for review.

For an update, preserve resource/document/block IDs, increment the source version, describe text/link/rights changes, and provide a reviewed anchor map for splits, merges, and retired blocks. The anchor-map interchange contract is still pending; it is a release blocker for updates requiring remapping. Retired content must leave user notes recoverable. Maintainers can suggest adopting approved concept IDs in your next version, while registry mappings allow earlier installed versions to benefit immediately.
