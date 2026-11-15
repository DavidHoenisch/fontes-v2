import { mkdtemp, writeFile, mkdir, readFile, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import type { Resource } from "./contracts";
import type { IntakeState } from "./intake-contracts";
import { validateSource } from "./intake-service";
const escape = (s: string) =>
  s
    .replaceAll("&", "&amp;")
    .replaceAll('"', "&quot;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll("'", "&apos;");
export type SourceResource = Pick<
  Resource,
  "id" | "version" | "kind" | "language" | "title" | "description"
>;
export function sourceFiles(
  resource: SourceResource,
  state: IntakeState,
): Record<string, string> {
  const f = state.fields,
    e = escape,
    ns = 'xmlns="urn:fontes:resource:1"';
  const extension = state.cover!.mediaType === "image/png" ? "png" : "jpg";
  return {
    "metadata.xml": `<?xml version="1.0" encoding="UTF-8"?>\n<resource ${ns} format-version="1" id="${e(resource.id)}" version="${e(resource.version)}" kind="${resource.kind}" language="${e(resource.language)}"><title>${e(resource.title)}</title><description>${e(resource.description)}</description><contributors><contributor role="author">${e(f.author)}</contributor></contributors><publisher id="${e(resource.id.slice(0, resource.id.lastIndexOf(".")))}">${e(f.publisher)}</publisher>${f.edition ? `<edition>${e(f.edition)}</edition>` : ""}${f.publicationDate ? `<publication-date>${e(f.publicationDate)}</publication-date>` : ""}<cover href="assets/cover.${extension}" media-type="${state.cover!.mediaType}" width="1200" height="1800" alt="${e(f.coverAlt)}" license-id="${e(f.coverLicenseId)}" rights-holder="${e(f.coverRightsHolder)}" attribution="${e(f.coverAttribution)}"/><references canonical-registry="fontes-protestant-66-v1" source-versification="${e(f.sourceVersification)}"/><coverage mode="selected"><range start="${e(f.coverageStart)}"${f.coverageEnd ? ` end="${e(f.coverageEnd)}"` : ""}/></coverage><rights license-id="${e(f.licenseId)}" rights-holder="${e(f.rightsHolder)}" license-file="rights/LICENSE.txt" attribution-file="rights/ATTRIBUTION.txt"><copyright>${e(f.copyright)}</copyright><attribution>${e(f.attribution)}</attribution><permissions ${Object.entries(
      f.permissions,
    )
      .map(([k, v]) => `${k}="${v}"`)
      .join(
        " ",
      )}/></rights><provenance><identifier>EPUB SHA-256: ${state.source!.sha256}</identifier><identifier>Fontes prose EPUB importer v1; publisher-reviewed conversion</identifier></provenance></resource>\n`,
    "index.xml": `<?xml version="1.0" encoding="UTF-8"?>\n<index ${ns} format-version="1" resource-id="${e(resource.id)}">${state.documents.map((d) => `<document id="${d.id}" title="${e(d.title)}" href="content/${d.id}.xml"/>`).join("")}</index>\n`,
    "rights/LICENSE.txt": f.licenseText + "\n",
    "rights/ATTRIBUTION.txt":
      f.attribution + "\nArtwork: " + f.coverAttribution + "\n",
    ...Object.fromEntries(
      state.documents.map((d) => [
        `content/${d.id}.xml`,
        `<?xml version="1.0" encoding="UTF-8"?>\n<document ${ns} id="${d.id}" resource-id="${e(resource.id)}"><title>${e(d.title)}</title>${d.blocks
          .map((b) => {
            const proposals = state.proposals.filter(
              (p) => p.blockId === b.id && p.decision === "accepted",
            );
            return `<block id="${b.id}" kind="${b.kind}"><text>${e(b.text)}</text>${proposals.length ? `<annotations>${proposals.map((p) => `<scripture start="${p.start}"${p.end ? ` end="${p.end}"` : ""} relation="cites" basis="author"/>`).join("")}</annotations>` : ""}</block>`;
          })
          .join("")}</document>\n`,
      ]),
    ),
  };
}
export async function buildSource(
  resource: SourceResource,
  state: IntakeState,
  cover: { buffer: Buffer },
  overrides?: {
    files: Record<string, string>;
    binaries: Record<string, Buffer>;
  },
) {
  const temp = await mkdtemp(path.join(os.tmpdir(), "fontes-authoring-")),
    source = path.join(temp, "source");
  try {
    const files = overrides?.files ?? sourceFiles(resource, state);
    for (const [name, content] of Object.entries(files)) {
      await mkdir(path.dirname(path.join(source, name)), { recursive: true });
      await writeFile(path.join(source, name), content, { mode: 0o600 });
    }
    await mkdir(path.join(source, "assets"), { recursive: true });
    await writeFile(
      path.join(
        source,
        "assets",
        `cover.${state.cover!.mediaType === "image/png" ? "png" : "jpg"}`,
      ),
      cover.buffer,
      { mode: 0o600 },
    );
    for (const [name, bytes] of Object.entries(overrides?.binaries ?? {})) {
      await mkdir(path.dirname(path.join(source, name)), { recursive: true });
      await writeFile(path.join(source, name), bytes, { mode: 0o600 });
    }
    await validateSource(source, true);
    const filename = `${resource.id}--${resource.version}.fontes-source.zip`,
      output = path.join(temp, filename);
    const cli =
      process.env.FONTES_RESOURCE_CLI ??
      path.resolve(process.cwd(), "../../target/debug/fontes-resource");
    await promisify(execFile)(
      cli,
      ["pack", source, "--output", output, "--deny-warnings"],
      {
        timeout: 30_000,
        maxBuffer: 1024 * 1024,
        env: {
          NODE_ENV: "development",
          PATH: process.env.PATH,
          LANG: "en_US.UTF-8",
        },
      },
    );
    return { buffer: await readFile(output), filename };
  } finally {
    await rm(temp, { recursive: true, force: true });
  }
}
