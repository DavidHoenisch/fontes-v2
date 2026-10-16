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
| `annotation.save` | optional existing `id`, `start_ref`, `end_ref`, `kind` (`note`/`highlight`), optional `color`, `content`; UUID on create, inclusive range |
| `annotation.delete` | existing annotation `id`; durable local deletion |
| `reference.favorite` | `ref_id`, `enabled: boolean`; idempotently add/remove favorite |
| `preferences.set` | complete `preferences` object: `text_size`, `line_spacing`, `verse_numbers`, `scripture_serif` |

Workspace `passage` entries now include additive `word_links` (default empty when reading older serialized models). Each link has `start`/`end` Unicode scalar offsets against that entry’s exact `text`, `surface`, `witness`, `strongs` and source/reviewed `method`. Codes come only from explicit existing alignment records, with canonical source-token compatibility and valid spans; they are not inferred. SQL/schema and command IDs are unchanged. The current released KJV/WEB corpus supplies no English links. UI lookup/panel support is not evidence of a completed aligned corpus.

Context additionally includes `cross_references` (default empty in older serialized models), each with `start_ref`, `end_ref`, display `label`, optional active-edition first-verse `preview` and `resource_id`. These are explicit target links from directly applicable cross-reference articles; missing text remains null. More specific source coverage sorts first. Existing article/token fields, command IDs and SQL contracts are retained.

Unknown commands/arguments and wrong types are rejected. Reference IDs must actually exist, not merely satisfy their arithmetic shape. `start_ref <= end_ref`; notes need nonblank text (maximum 100,000 UTF-8 bytes), highlights need a color. Colors are `amber`, `sage`, `blue`, `rose`. Text size is 16–36 inclusive and line spacing 1.2–2.2 inclusive; nonfinite numbers are invalid. References are at most 256 UTF-8 bytes and search text at most 512. Exact names/OSIS/USFM are preferred; other prefixes must resolve uniquely. Explicit shorthand aliases are `j`/`jn` → John and `psalm` → Psalms; `Jo` remains ambiguous. A book without numbers opens chapter 1, verse 1. Range notation/natural-language questions are not reference-parser syntax in this slice.

## JSON control / CLI

One request object per line:

```json
{"command":"reading.navigate","params":{"reference":"Rom 1:3"}}
```

The JSON transport replies with `{"ok":true,"result":{...}}` or `{"ok":false,"error":{"code":"...","message":"..."}}`. Successful result objects contain `protocol_version: 1`, a `state` workspace and a tagged `data` value (`workspace`, `search`, `context`, `resource`, `word`, `saved_annotation`). JSON field ordering is not significant. Search includes `offset`/`has_more`; context includes its reference; resource links retain their typed basis. Stable error categories are `invalid_command`, `invalid_json`, `invalid_input`, `not_found`, `incompatible_data`, `storage`.

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
