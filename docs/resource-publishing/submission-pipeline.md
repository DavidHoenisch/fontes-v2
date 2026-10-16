# Submission validation and source releases

The implemented pipeline accepts publisher source submissions, validates them, creates reproducible source packages and cover variants, and publishes approved packages as signed GitHub **prereleases**. It operates in this repository now and exposes reusable workflows for another repository later.

These are **source-stage releases**: editable `.fontes-source.zip` content, standalone XML metadata, artwork, notices, and review records. They are not `.fontes.zip` app delivery packages, compiled SQLite, or a live store catalog. This distinction is part of the signed protocol, artifact names, and release notes. The delivery compiler/installer remains a separate implementation phase in [SPEC.md](../../SPEC.md).

## Repository layout

```text
library/
  policy.json
  taxonomy.xml
  trust/source-release-1.json
  approvals/<resource-id>--<version>.json
  submissions/<resource-id>/
    submission.json
    source/                         # unpacked XML/rights/cover/media
```

The [CC0 Romans demonstration](../../library/submissions/org.example.romans-study/submission.json) is the first submission. Its `org.example` publisher is a reserved fictional fixture, not verified publisher registration. Its [approval](../../library/approvals/org.example.romans-study--1.0.0.json) covers only the original example prose and placeholder artwork. The [example taxonomy](../../library/taxonomy.xml) is a test snapshot, not a production registry.

Submitters add a resource directory and a strict `submission.json` with identity, publisher, numeric version, positive build revision, source, and release notes. Source can be:

```json
{"type":"path","path":"submissions/org.example.romans-study/source"}
```

For a larger public archive use a descriptor rather than putting every ZIP in Git:

```json
{"type":"https","url":"https://github.com/OWNER/REPOSITORY/releases/download/TAG/source.zip","sha256":"REPLACE_WITH_64_LOWERCASE_HEX_DIGITS","bytes":12345}
```

The second snippet is a template, not a valid hash or live endpoint. Remote input requires an exact byte length/hash, HTTPS on port 443, no URL credentials/fragment, and a hostname in trusted policy. Every redirect is checked against the same allowlist; redirects are limited, requests time out, and actual downloaded bytes are capped at 256 MiB. No GitHub/private token is sent to source URLs. Restricted sources must use repository-local paths in a private repository.

Paths resolve within the selected library checkout. Source XML/ZIP validation, extraction limits, rights files, and original-language restrictions remain those of the [CLI guide](cli-guide.md). The publisher normalizes inputs into a deterministic source ZIP. `source_sha256` means those canonical ZIP bytes; a remote descriptor's hash means the submitted upstream archive bytes. Both are retained as identities in the release manifest/submission descriptor. The original upstream ZIP container is not a separate release asset; the normalized ZIP preserves the complete validated source file content.

## PR feedback and maintainer approval

1. Open a PR changing `library/submissions/`. [Resource submission checks](../../.github/workflows/resource-submissions.yml) invokes the reusable read-only build workflow.
2. Tooling and policy/approvals are checked out separately at trusted base commits. The PR checkout supplies data only. A proposed approval or weakened policy in the PR cannot authorize that PR's publication.
3. Validation checks strict source schemas and semantics, then packs the frozen validated content, extracts metadata/notices, and renders 200 × 300 and 600 × 900 PNG covers from the verified master. It repeats preparation and compares every output byte.
4. The Actions run has a JSON review summary and a five-day `resource-review` artifact with diagnostics and source previews. XML diagnostics retain file/line/column and corrective help. Validation failures make the check fail. A valid submission without approval is `awaiting_approval`; it is not an error and does not publish.
5. After identity, text/cover rights, fidelity, references, and concept review, a maintainer adds an approval under `library/approvals/` and, for a new verified publisher, updates the policy allowlist. The approval records resource/version, **exact normalized ZIP SHA-256**, distribution class, and rights/publisher review IDs. The contributor's metadata declaration is not approval.
6. Merge the approved source/policy change. The trusted default-branch workflow rebuilds approved versions and passes same-run artifacts to the separate publisher job. No signing secret is passed to validation/build jobs.

An existing version's approval/hash mismatch fails validation: use a new resource version for changed source bytes. A new version can validate before its approval is merged. Build revisions can change tooling/registry/policy release material while retaining the same source version/hash. Existing published source versions cannot be repurposed under another build revision.

[CODEOWNERS](../../.github/CODEOWNERS) names maintainers for policy, approvals, trust, taxonomy, workflows, and tooling. **CODEOWNERS alone does not enforce review.** Enable required code-owner reviews and status checks in repository branch rules; GitHub environment reviewers are an additional optional production control. Do not treat a path in Git as proof of legal rights. Keep private agreements outside public submissions and use non-secret review identifiers.

## Workflows and release behavior

| Workflow | Responsibility and permissions |
| --- | --- |
| [resource-submissions.yml](../../.github/workflows/resource-submissions.yml) | PR/manual preview entry point; read-only, no signing secret |
| [resource-pipeline.yml](../../.github/workflows/resource-pipeline.yml) | Reusable trusted tooling/policy + separate submitted-data checkout, validation/tests, preparation twice, summaries/artifacts; read-only |
| [resource-release.yml](../../.github/workflows/resource-release.yml) | Default-branch push/manual entry point; builds approved submissions, then invokes publisher only when approved outputs exist |
| [resource-publish.yml](../../.github/workflows/resource-publish.yml) | Reusable publisher, default-branch/event guard, scoped write credentials/signing secret, environment, serialized destination publication |

Validation and publication are separate reusable workflows so a PR caller never needs `contents: write` merely to call a workflow containing a privileged job. There is no `pull_request_target` execution or privileged `workflow_run` consuming a PR artifact. The publisher downloads only its caller run's successful trusted build. No source-provided adapter or build script is executed. Reviewable code changes still need normal code-owner/branch protection; workflow checks cannot replace repository governance.

Tag format: `resource-source-<resource-id>--<version>--b<build_revision>`. All current source-stage releases are prereleases and use `latest=false`. Each release has:

- The normalized `<resource-id>--<version>.fontes-source.zip`.
- `metadata.xml`, the original `cover.png` or `cover.jpg`, `cover-card.png`, `cover-detail.png`.
- `NOTICE.txt`, `submission.json`, `approval.json`, and `validation-report.json`.
- `resource-source-manifest.json` and its detached raw 64-byte Ed25519 signature.

The nine payload files are bound by exact path/length/SHA-256 in the manifest. The signature covers `FONTES-RESOURCE-SOURCE-RELEASE-V1` followed by **one newline byte**, followed by the exact original UTF-8 manifest bytes. This domain is deliberately separate from snapshot, catalog, and app delivery manifests. Verify signatures against an externally configured key before trusting identity or inventory; reject unknown/missing/special files and changed assets.

The manifest additionally binds source/destination repository, source/tooling/tag-target commits, resource/publisher/version/build, source hash, policy/taxonomy hashes, and approval status. For a separate storage repository, the tag points at that repository's resolved HEAD while source provenance retains the original submission commit. The release artifact itself, not tag metadata, carries authoritative source provenance.

The publisher checks live destination visibility before writing. `private_restricted` requires both source and destination repositories to be private; a public destination is refused. It checks the current trusted policy and exact approval again, signs only approved outputs, creates a draft, downloads/verifies **all** draft assets, publishes, then downloads/verifies the published assets again.

A successful retry verifies and reuses identical existing assets without uploading/replacing them. The current trusted policy and approval must still permit publication. Adding an unrelated publisher or updating the registry does not force every unchanged resource to be republished; existing signed provenance remains the record of its original build. A complete signed draft can resume publication. A changed payload under the same tag is refused; a changed source under the same resource version is refused even under a new build revision. For an incomplete draft, inspect it and use a new build revision or deliberately remove the failed draft after confirming it was never published. The tool does not delete or overwrite releases automatically. A private/free policy change cannot silently relabel an existing release.

Removing an approval prevents new builds of that version; it does not delete an existing release. A production catalog's withdrawal/revocation flow remains separate. GitHub operations are the implemented storage backend; object storage/CDN publishing requires another reviewed backend with equivalent inventory/verification guarantees.

## Local feedback loop

Build/install the CLI as described in [the CLI guide](cli-guide.md). Run from the repository root, with new output directories:

```sh
fontes-resource submissions check \
  --root library --policy-root library --output dist/resource-review \
  --repository DavidHoenisch/fontes-v2 --source-repository DavidHoenisch/fontes-v2 \
  --source-commit "$(git rev-parse HEAD)" --tooling-commit "$(git rev-parse HEAD)"

fontes-resource submissions build \
  --root library --policy-root library --output dist/resource-approved \
  --repository DavidHoenisch/fontes-v2 --source-repository DavidHoenisch/fontes-v2 \
  --source-commit "$(git rev-parse HEAD)" --tooling-commit "$(git rev-parse HEAD)"
```

`check` prepares all valid submissions, including unsigned pending previews. `build` skips unapproved versions. Both write a versioned aggregate report and return exit 1 for invalid submissions; global operational failures return 2. Per-resource validation reports remain available when XML validation fails. An output directory is never reused.

After a release, download to a new directory and authenticate it:

```sh
gh release download resource-source-org.example.romans-study--1.0.0--b1 \
  --repo DavidHoenisch/fontes-v2 --dir dist/downloaded-source
fontes-resource submissions verify --dir dist/downloaded-source \
  --trust library/trust/source-release-1.json --repository DavidHoenisch/fontes-v2
```

`submissions publish --dir BUILD --policy-root LIBRARY --repository OWNER/REPO` requires `gh` authorization and a matching `FONTES_RESOURCE_SIGNING_KEY`. Ordinary authoring and preview builds do not require credentials. Publication returns release URLs; verification authenticates source assets, not SQLite or installation behavior.

## Keys, migration, and private repositories

The entry workflow prefers `FONTES_RESOURCE_SIGNING_KEY`; this repository currently falls back to its configured `FONTES_DATA_SIGNING_KEY`. The public key in `library/trust/source-release-1.json` must match. Domain separation makes the current reuse explicit; a new repository should configure its own appropriate key/trust policy. Private seeds never belong in source, reports, previews, or release assets.

The reusable build inputs select `tooling_repository`/`tooling_ref`, `library_path`, `policy_repository`/`policy_ref`/`policy_path`, `source_ref`, `release_repository`, and `approved_only`. Policy defaults to the **caller** repository, so a private library does not need to publish its approval records in the public tooling repository. The publisher receives the build's exact resolved policy SHA. Configure `source_read_token` only when cross-private-repository checkout requires it; configure `publication_token` when the destination differs from the caller and its default token cannot write there. The publication token must also be able to read source-repository visibility for the private-source check. These tokens are scoped to their relevant jobs.

To move the pipeline, copy the wrappers/tooling/contracts, or call the public reusable build and publisher from a new repository using a reviewed full tooling/workflow commit SHA. Keep private policy, review evidence, and content in that private repository; set `distribution` to `private_restricted`, use a matching private approval/key, and choose a private destination. The executable validation enforces visibility; the code does not implement paid entitlements, checkout, or application access to private release assets.

The two-workflow caller shape is shown in [the portable workflow example](workflow-example.yml), pinned to a reviewed full tooling/workflow commit. It is documentation, not an active workflow; update the refs together when adopting a later reviewed revision. For PR preview, call only the read-only build workflow with `approved_only: false` and the caller's base policy commit. For release, use a trusted default-branch event, `approved_only: true`, and pass only the publisher its signing/write secrets. The source workflow design follows GitHub's [reusable workflow](https://docs.github.com/en/actions/how-tos/reuse-automations/reuse-workflows) and [untrusted PR execution](https://docs.github.com/en/actions/reference/security/securely-using-pull_request_target) guidance.

## Initial verification record

The original CC0 example completed [trusted publication](https://github.com/DavidHoenisch/fontes-v2/actions/runs/38857998838) and a separate [read-only validation run](https://github.com/DavidHoenisch/fontes-v2/actions/runs/38858045577). Its [signed source prerelease](https://github.com/DavidHoenisch/fontes-v2/releases/tag/resource-source-org.example.romans-study--1.0.0--b1) was downloaded and authenticated outside Actions. All nine payload hashes/lengths and the 1200 × 1800, 200 × 300, and 600 × 900 cover dimensions were checked. The normalized archive is 5,105 bytes with SHA-256 `511c4be5dba051437a8b1787290d457232d9dd5e32e0c0ba3510ea6ef66254e4`. The review artifact was downloaded and its successful submission report checked separately. These are source-stage execution results; they do not verify a client installer, paid entitlement system, or compiled resource SQLite.

A subsequent [publication run using the pinned tooling revision](https://github.com/DavidHoenisch/fontes-v2/actions/runs/38858389618) also succeeded. It reused the existing release; the archive/manifest asset IDs and signed manifest bytes remained unchanged, confirming the retry did not overwrite or duplicate those assets.
