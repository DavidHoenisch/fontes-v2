# Fontes implementation reference

- Start with [SPEC.md](SPEC.md), the current system specification. It includes requirements, flows, design decisions, schema authorities, implementation status, acceptance criteria, and open decisions.
- Read the versioned contracts and detailed design/operating guides linked from the relevant specification section before changing that area. The original v0.1 transcription is historical, not the active SQL or Rust contract.
- Keep implementation, schemas, CLI behavior, examples, tests, and specification status consistent. Record a changed design decision and its compatibility/migration effect in the same change.
- Preserve published versioned contracts and permanent reference IDs. A draft schema/model or synthetic fixture is not a production migration, live registry, or signed release.
- Clearly distinguish implemented behavior, planned requirements, deferred features, and unresolved decisions. Use the acceptance criteria to select meaningful verification; do not claim app/installer behavior from data-model tests alone.
