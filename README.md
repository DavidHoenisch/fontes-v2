# Fontes

Offline Bible study data and application foundations. The Rust reading pipeline compiles locked public domain KJV and World English Bible Classic inputs into a versioned SQLite database with full-text search and permanent canonical references.

## Build the reading data

The pinned Rust toolchain is installed through rustup. The first build downloads approximately 10 MB of source archives and checks their exact SHA-256 digests and sizes.

```sh
cargo test --workspace --locked
cargo build --release --locked -p fontes-data-pipeline
target/release/fontes-data build \
  --profile reading \
  --output dist/reading \
  --builder-commit "$(git rev-parse HEAD)" \
  --mirror-repository DavidHoenisch/fontes-v2
target/release/fontes-data validate dist/reading/library.sqlite
```

The output directory must be new or empty. A verified content-addressed source cache lives in `.data-cache/`. Use `--offline` to rebuild from that cache without any network access. If upstream replaces a locked archive, the pipeline can recover that exact archive from its retained assets on earlier data releases. It fails if the required locked bytes cannot be recovered; it never substitutes a changed download.

The reading profile contains **31,102 canonical anchors and 62,204 verse rows**, across 66 books and two translations. `verses_by_reference` links source verse records to canonical IDs. The WEB Romans doxology is mapped from source Romans 14:24–26 to canonical Romans 16:25–27. Four empty WEB verses are represented explicitly as source omissions. The reviewed rules are in `data/mappings/reading-v1.json`; no text is invented to fill those entries.

The first adapter imports verse-per-line XML. It preserves verse text after trimming surrounding whitespace, including KJV paragraph symbols, but does not yet import USFM paragraph, poetry, or footnote structure. User notes and highlights belong in a separate database defined by `data/schema/user-v1.sql`.

## GitHub releases

[Reading data releases](https://github.com/DavidHoenisch/fontes-v2/releases) use tags such as `data-v1-stable-1`. Pushing a relevant compiler, source, schema, or workflow change to `main` runs `.github/workflows/data-release.yml`; the workflow can also be dispatched manually.

The workflow tests the workspace, builds real source data twice, compares SQLite and compressed bytes, and passes checked outputs to a separate publishing job. Only that job receives the signing secret and repository write permission. The publisher allocates the next data sequence, signs the manifest, creates a draft release, verifies every uploaded asset, publishes, and downloads/verifies the published assets again. It never overwrites published releases and sets `make_latest=false` to keep data releases separate from app releases. Identical content from the same compiler commit is a no-op.

Published assets include:

- `fontes-reading.sqlite.zst`, ready to decompress into the content database.
- `release-manifest.json` and its raw Ed25519 signature, covering the database hashes, lengths, versions, source identities, and capabilities.
- `sources.lock.json`, `NOTICE.txt`, and `validation-report.json`.
- `changes.json`, including verse additions, changes, and removals compared with the previous release.
- The exact KJV and WEB source archives, named by their SHA-256, for future rebuilds.

Verify a downloaded release using the public key committed in this repository:

```sh
gh release download data-v1-stable-1 --repo DavidHoenisch/fontes-v2 --dir dist/downloaded
target/release/fontes-data verify \
  --dir dist/downloaded \
  --trust data/trust/data-release-1.json \
  --repository DavidHoenisch/fontes-v2
```

The verifier authenticates the original manifest bytes before parsing, checks source provenance, caps decompression, checks both artifact hashes, and validates SQLite references, coverage, integrity, and search. It does not install content or change user data. A future shared app updater will add atomic activation and recovery.

The publisher expects the `FONTES_DATA_SIGNING_KEY` GitHub repository secret. Its value is a 32-byte Ed25519 seed encoded as hex, matching `data/trust/data-release-1.json`. Private keys belong outside Git. Key generation and repository-secret setup have been performed for the initial release; normal runs do not regenerate keys.

## Further development

The study profile is defined but deliberately rejected by the compiler until its source adapters are implemented. Scheduled source discovery, reviewed update PRs, and mobile/desktop installation are specified in the [architecture](docs/architecture/data-pipeline.md). See [source research](docs/research/public-domain-data-sources.md) for content and rights decisions.
