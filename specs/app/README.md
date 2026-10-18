# Application command protocol v1

Initial implemented contract, separate from snapshot update-protocol v1 and draft resource-library v2. Authority: [`Command`](../../crates/fontes-engine/src/commands.rs), generated [command catalog/input schemas](commands-v1.json), and [`Response`/view models](../../crates/fontes-engine/src/model.rs). The engine test and application workflow detect registry/schema drift. IDs are stable within this contract; incompatible semantics must not silently reuse them. This is the initial development protocol, not a long-term external API compatibility guarantee for every view-model field.

| ID | Parameters / behavior |
| --- | --- |
| `workspace.get` | `{}`; current reading state, installed profile, books/translations, passage, annotations and saved references |
| `reading.navigate` | `reference: string`; resolve a book/chapter/optional verse; preserve location on error |
| `reading.select` | `ref_id: integer`; select an actual canonical anchor and its chapter |
| `reading.chapter` | `delta: -1 or 1`; chapter movement across books; refuse movement beyond canon |
| `reading.translation` | installed translation `id`; retain canonical selection/annotations |
| `search.query` | `query`, optional `limit` (default 30, 1–100), `offset` (default 0, 0–10000); literal AND terms |
| `study.context` | `ref_id`; up to 100 typed article matches, 2,000 source analysis/Strong's rows and 100 distinct cross-reference ranges |
| `study.resource` | article `id`; full plain text, source/rights-review and up to 500 typed links |
| `study.word` | `code`, e.g. G25/G0025 or H7225; definitions and first 100 distinct canonical occurrence locations, total count |
| `study.occurrences` | `code`, optional `book_id`, `offset` (default 0, maximum 100000), `limit` (default 8, 1–50); complete distinct verse counts by book plus filtered, paged active-edition previews |
| `annotation.save` | optional existing `id`, `start_ref`, `end_ref`, `kind` (`note`/`highlight`), optional `color`, `content`; UUID on create, inclusive range |
| `annotation.delete` | existing annotation `id`; durable local deletion |
| `reference.favorite` | `ref_id`, `enabled: boolean`; idempotently add/remove favorite |
| `preferences.set` | complete `preferences` object: `text_size`, `line_spacing`, `verse_numbers`, `scripture_serif`, optional `theme` (default `sea_glass`) |

Workspace `passage` entries now include additive `word_links` (default empty when reading older serialized models). Each link has `start`/`end` Unicode scalar offsets against that entry’s exact `text`, `surface`, `witness`, `strongs` and source/reviewed `method`. Codes come only from explicit existing alignment records, with canonical source-token compatibility and valid spans; they are not inferred. SQL/schema and command IDs are unchanged. The current released KJV/WEB corpus supplies no English links. UI lookup/panel support is not evidence of a completed aligned corpus.

Context additionally includes `cross_references` (default empty in older serialized models), each with `start_ref`, `end_ref`, display `label`, optional active-edition first-verse `preview` and `resource_id`. These are explicit target links from directly applicable cross-reference articles; missing text remains null. More specific source coverage sorts first. Existing article/token fields, command IDs and SQL contracts are retained.

Word results additionally include `headwords` (default empty when reading older responses), with `resource_id`, exact source-supplied `text` and optional `transliteration`. These come only from a definition-linked resource's structured `dictionary_entry` display metadata. Missing, blank, malformed or differently typed metadata yields no headword and does not hide definitions. A dictionary headword is not an inferred verse lemma or English alignment. Existing command inputs, IDs, SQL and protocol version are unchanged.

Unknown commands/arguments and wrong types are rejected. Reference IDs must actually exist, not merely satisfy their arithmetic shape. `start_ref <= end_ref`; notes need nonblank text (maximum 100,000 UTF-8 bytes), highlights need a color. Legacy `annotation.save` colors are `amber`, `sage`, `blue`, `rose`. The additive `highlight.save` command supports richer styles and exact edition-bound selection; see [rich highlights v1](highlights-v1.md). Text size is 16–36 inclusive and line spacing 1.2–2.2 inclusive; nonfinite numbers are invalid. References are at most 256 UTF-8 bytes and search text at most 512. Exact names/OSIS/USFM are preferred; other prefixes must resolve uniquely. Explicit shorthand aliases are `j`/`jn` → John and `psalm` → Psalms; `Jo` remains ambiguous. A book without numbers opens chapter 1, verse 1. Range notation/natural-language questions are not reference-parser syntax in this slice.

## First-run bootstrap

The separate `library.setup` lifecycle operation is implemented by `fontes-engine::bootstrap`, with GUI confirmation and `fontes-cli setup --data-dir DIRECTORY --profile reading|study`. It runs before an initialized Engine exists, downloads only a confirmed signed profile, stores versioned app-owned receipts/generations and opens the library. It does not add a command to the 17-command registry or expose pre-initialization JSON/MCP access. Exact state/trust/compatibility/activation requirements are in [first-run setup](../../docs/app/first-run-setup.md). A future remote bootstrap transport needs its own authorization/lifecycle contract.

## JSON control / CLI

One request object per line:

```json
{"command":"reading.navigate","params":{"reference":"Rom 1:3"}}
```

The JSON transport replies with `{"ok":true,"result":{...}}` or `{"ok":false,"error":{"code":"...","message":"..."}}`. Successful result objects contain `protocol_version: 1`, a `state` workspace and a tagged `data` value (`workspace`, `search`, `context`, `resource`, `word`, `occurrences`, `saved_annotation`). JSON field ordering is not significant. Search includes `offset`/`has_more`; context includes its reference; resource links retain their typed basis. Stable error categories are `invalid_command`, `invalid_json`, `invalid_input`, `not_found`, `incompatible_data`, `storage`.

Frames are bounded to 1 MiB including an optional newline. Malformed/invalid ordinary commands produce an error and permit the next line. A frame exceeding the limit closes the stream with a stderr diagnostic; the server does not allocate or drain an unbounded frame. No TCP/HTTP endpoint, shell execution, arbitrary SQL, content write, package installer, or AI generation command exists.

CLI `run <id> --params '<object>'` parses that exact command and prints the successful result object directly; failures use stderr and a nonzero status. `commands` needs no library/user file. The UI uses the same Rust enum and dispatcher without serializing its own SQL or reinterpreting IDs.

## MCP stdio

Launch the explicit `fontes-cli ... mcp` mode. It implements JSON-RPC 2.0 line framing, MCP protocol version **2025-06-18**, `initialize`, `ping`, `tools/list`, `tools/call`, and response-free notifications. Newer protocol clients must accept negotiation to the supported version or disconnect. There is no remote transport, resource/prompt/subscription API or live-GUI IPC. Each tool name is the exact command ID and its input schema comes from the same generated registry. Successful calls include text JSON plus `structuredContent`; domain failures use `isError: true`, malformed parameters use JSON-RPC `-32602`, unknown methods `-32601`, malformed JSON `-32700`.

```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"example-client","version":"1"}}}
{"jsonrpc":"2.0","method":"notifications/initialized"}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"reading.navigate","arguments":{"reference":"John 3:16"}}}
```

The client chooses which user database to expose. Tools can read private notes and persist mutations; user-facing host confirmation/access policy belongs to that client. Stdout is reserved for protocol messages. A future authenticated GUI control or remote transport needs a separate explicit capability/security contract; do not publish this process on a network port as a shortcut.

## Source word-to-dictionary tags

`passage.strongs_links` is additive (default empty when deserializing older replies). Each item contains exact Unicode-scalar `start`/`end`, a Strong’s code and `source_snapshot_id`. This is a dictionary association, without a claimed original-token identity, surface, lemma or morphology. `word_links` retains its distinct source/reviewed original-token alignment meaning. The context menu can use both to open dictionary definitions; an original-language surface is promoted only from a known token alignment or chosen source word, otherwise the dictionary headword is labeled as such.

Optional stored metadata uses the new [verse Strong’s tags v1 schema](verse-strongs-v1.schema.json) inside existing `verse_texts.display_json.source_strongs`; published SQL is unchanged. The engine checks the exact edition/verse ID, SHA-256 of UTF-8 verse text, valid non-overlapping scalar offsets, source snapshot existence and installed dictionary codes. Corrupt metadata fails closed. The pinned GPL KJV source is owner-approved and enabled in the ten-source Study build. One reviewed text mismatch (1 John 2:23) is withheld; WEB receives no KJV tags. Eight- and nine-source complete Study profiles remain accepted explicitly. Source code/credits and full license accompany builds; existing installations need both a compatible client and the updated published Study library.

## Selectable reading themes

`preferences.set` retains its command ID and adds optional `preferences.theme`: `sea_glass`, `ink_saffron`, `charcoal_raspberry`, `cobalt_citron`, `terracotta_linen`, `aubergine_apricot`, or `midnight_mint`. Omission defaults to Sea glass; unknown values are rejected. The generated catalog is authoritative. The same field is included in workspace preferences and saved in the existing app-session JSON. No published content/user SQL changes. Legacy sessions without this field load with Sea glass; older binaries may reject sessions saved with the new field because session decoding is strict. Notes/highlights are independent of this presentation preference.

New app sessions default to 24px Scripture at 1.63 line spacing (D038). Existing explicitly serialized typography values are preserved; this does not rewrite saved preferences or change the accepted 16–36 / 1.2–2.2 ranges.

The additive `note.save` and read-only `note.passage` commands implement [note documents v1](notes-v1.md), including exact source validation, optional text selection and backward-compatible annotation fallback. Workspace `notes` defaults empty.


### Parent library catalogue (2026-10-17)

`library.catalog` is additive to protocol v1: optional `source_id` and default-zero `offset` (0–100000). Without a source it lists installed study sources grouped by source ID and resource type, with article counts/author; translations remain in workspace metadata. With a source it additionally returns at most 50 deterministic title/ID-ordered resource summaries and `has_more`. `Data::Library` carries sources, resources, offset and has_more. It never changes reading session or user data. No published SQL changes. The generated command catalogue now has 18 entries. Existing commands/IDs are unchanged.


### In-place passage preview (2026-10-17)

`reading.preview` accepts canonical inclusive `start_ref` and `end_ref`; both must be valid and ordered. `Data::Preview` contains `verses` (`ref_id`, display `label`, exact active-edition `text`) and `has_more`. It returns at most 100 distinct source verse segments in canonical order; merged source rows render once. Missing edition text yields an empty list. It does not call navigation or add recent history. The command catalogue now contains 19 commands. No SQL migration or repurposed command IDs.
