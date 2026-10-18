# Notes at scale — three interactive proposals

Design study only; not integrated with the app, native databases, or real notes. Three interfaces share 1,248 deterministic generated sample notes. Counts reflect the in-memory sample collection. This illustrates organization and retrieval flows, not production search/performance validation.

1. Notebooks: shelf, Inbox, collection search, sorting, selection, bulk move/archive with Undo.
2. Search & tags: combined tags, passage/text search, book/source/pin refinements and example saved views.
3. Scripture atlas: book → chapter → passage → notes; testament filter and unattached thoughts.

All directions open note previews and allow pinning or creating an unattached sample note. Everything resets on reload. KJV quotations are sample Scripture; generated personal thoughts and collected-piece counts are illustrative, with no fabricated attributed commentary quotations. No account, remote API or external font is required.

Current limitations: the notebook list is fixed; tag creation/renaming, persistent saved-view management, editing rich blocks, true indexed full-text search, scalable database queries, accessibility/performance audits and native installation are not implemented here. Shared dataset rows are generated repetitions to compare density and flows. Existing production `notes-v1` contracts are unchanged. No proposal has been selected for implementation.

Browser checks: selecting two notes and moving them changed the Romans notebook count from 351 to 349; Undo restored 351. Combined Grace/Romans/collected-source filters returned 99 sample notes; a named view could be saved, cleared and reopened. The atlas expanded Romans → Chapter 8 → Romans 8:28 into its note list. Creating an unattached sample note and searching its title returned it with a Today date. These are prototype interaction checks, not tests of native persistence or database scalability.
