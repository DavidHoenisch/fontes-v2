# Fontes specifications

Start with the root **[system specification](../../SPEC.md)**. It is the current implementation baseline for requirements, non-functional constraints, user/operational flows, visual choices, data contracts, library/publishing architecture, decision history, acceptance criteria, and open items.

| Document | Role |
| --- | --- |
| [System specification](../../SPEC.md) | Current cross-system entry point; retain original FR/NFR identifiers and update status as features ship |
| [Library/package detail specification](library-and-packages-v0.2.md) | Detailed resource, catalog, taxonomy, and recovery design, read with the system specification |
| [Original v0.1 Markdown transcription](archive/bible-study-software-v0.1.md) | Historical owner-supplied baseline with full prose/SQL/Rust and source Word SHA-256; superseded for implementation |
| [Versioned resource contracts](../../specs/resource-packages/v1/README.md) | Exact draft XML/JSON contracts, executable model, examples, and design conformance checks |
| [Pipeline architecture](../architecture/data-pipeline.md) | Pipeline/update implementation detail and planned operation |
| [Resource publishing guides](../resource-publishing/README.md) | Author, CLI, taxonomy, maintainer, and client integration instructions |

Schema source files remain the authority for the exact named wire/database version; the system specification links them and explains their relationships. Record and resolve discrepancies together rather than silently changing old published contracts. No local Downloads path or conversation history is required to follow the active specification.
