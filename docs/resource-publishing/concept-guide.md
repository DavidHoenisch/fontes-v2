# Concepts and topics

Status: proposed author and editorial workflow. The [example taxonomy](../../specs/resource-packages/v1/taxonomy.example.xml) demonstrates the format; it is not a published authoritative registry. A searchable registry and proposal tools remain implementation work.

A **concept** is a stable metadata identity with a defined meaning. A **topic** is its visible browsing label. Labels can change or be translated without changing the concept ID. Scripture references, concepts, word links, and a user's private tags remain distinct.

## Select an existing concept

Search preferred labels, alternative labels, language, scope notes, and relationships. Reuse an ID only when its meaning fits the passage. Do not select a similarly named concept merely to avoid a proposal. Attach it to the smallest useful block using `discusses`, `defines`, or `illustrates`.

| Situation | Representation |
| --- | --- |
| Two names mean the same thing | Preferred and alternative labels on one concept |
| One subject is narrower than another | Separate IDs linked by `broader`; child-to-parent direction |
| Subjects are associated but neither contains the other | `related` relationship |
| A publisher's concept has the same scope as a registry concept | Reviewed equivalence resolution preserving both identities |
| Scopes overlap but differ | Close-match review suggestion; no automatic equivalence |

“The Flood” and “Noah's Flood” can label the same Genesis 6–9 narrative. A concept about a global-extent interpretation has a more specific scope; it may be narrower or related. Equating those meanings automatically would misrepresent resources that discuss the narrative without making that claim. Similarly, the desert of Sin and moral sin need different IDs even when a label overlaps.

## Propose a concept

Use `taxonomy/concepts.xml` and your verified publisher namespace. This complete example follows [concepts.xsd](../../specs/resource-packages/v1/concepts.xsd):

```xml
<concepts xmlns="urn:fontes:resource:1" format-version="1"
          publisher-id="org.example" scope="publisher">
  <concept id="urn:fontes:publisher:org.example:concept:messianic-ancestry">
    <label language="en" kind="preferred">Messianic ancestry</label>
    <label language="en" kind="alternative">Ancestry of the Messiah</label>
    <scope-note>Discussion of ancestry as presented in texts about the Messiah.</scope-note>
    <related id="urn:fontes:concept:incarnation"/>
    <evidence start="Rom.1.3"/>
  </concept>
</concepts>
```

Provide one preferred label for each included language, optional alternatives, a scope note distinguishing nearby meanings, relevant scripture evidence, and suggested relationships. Refer to this provisional ID in your content. Publishers may not define or modify central `urn:fontes:concept:*` entries. Additional parents are allowed; parent cycles are rejected. Several words need not mean several concepts, and a different spelling alone is not evidence of a new meaning.

## Review and standardize

Maintainers compare scopes and evidence, then record one of these decisions: retain a distinct publisher concept, register a new central concept, add an alias, approve equivalence, or retain separate meanings with a relationship. Duplicate-label detection offers suggestions; it never silently merges submissions.

A decision records proposal identity, submitting publisher, reviewer, evidence, target ID, relationship, and registry revision. The signed taxonomy snapshot includes reviewed mappings. Stored assertions retain the publisher's submitted ID; queries can resolve it to the current effective ID. Mapping targets are flattened and cycle checked. Parent, related, and close-match links do not become equivalence mappings. A correction publishes a new auditable revision and rebuilds affected derived indexes.

An author can adopt a central ID in a later source update. Existing installations benefit from approved mappings without rewriting or republishing their source archives. The original assertion remains available for provenance and reversal.

## Expand discovery responsibly

Exact and alias matches resolve to one concept. Broader/narrower expansion is a visible search choice, with deduplication for multiple parents. Related-topic results should be labeled accordingly. A topic match never invents a scripture citation or implies an author's agreement with every interpretation under that topic.

Adding a taxonomy term does not automatically tag old books. Fontes editorial enrichment must be a separate, reviewed assertion layer with evidence, target resource/block identities and hashes, and contributor provenance. Its signed exchange format remains to be defined. User-created private tags stay in `user.sqlite`; public registry submission requires a separate explicit contribution workflow.
