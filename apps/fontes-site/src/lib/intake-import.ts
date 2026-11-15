import { createHash } from "node:crypto";
import path from "node:path";
import { parse } from "parse5";
import registry from "../../../../data/canonical/registry-v1.json";
import { DomainError } from "./contracts";
import { array, xml, xmlText } from "./intake-archive";
import {
  emptyIntake,
  type IntakeDocument,
  type IntakeState,
  type Proposal,
} from "./intake-contracts";

export const hash = (value: string | Buffer) =>
  createHash("sha256").update(value).digest("hex");
const stableId = (prefix: string, identity: string) =>
  `${prefix}-${hash(identity).slice(0, 24)}`;
export function canonicalRef(ref: string) {
  const match = /^([A-Za-z0-9]+)\.([1-9]\d*)\.([1-9]\d*)$/.exec(ref);
  if (!match) return false;
  const book = registry.books.find((b) => b.osis === match[1]);
  return (
    !!book && Number(match[3]) <= (book.verse_counts[Number(match[2]) - 1] ?? 0)
  );
}
export function validRange(start: string, end?: string) {
  if (!canonicalRef(start) || (end && !canonicalRef(end))) return false;
  const order = (ref: string) => {
    const [osis, c, v] = ref.split(".");
    return (
      registry.books.find((b) => b.osis === osis)!.id * 1e6 +
      Number(c) * 1000 +
      Number(v)
    );
  };
  return !end || order(start) <= order(end);
}
export function detectReferences(documents: IntakeDocument[]): Proposal[] {
  const aliases = registry.books
    .flatMap((b) =>
      [...new Set([b.name, b.osis])].map((alias) => ({ alias, book: b })),
    )
    .sort((a, b) => b.alias.length - a.alias.length);
  const proposals: Proposal[] = [];
  for (const doc of documents)
    for (const block of doc.blocks) {
      // Conservative explicit references only; no guessed allusions, themes or verse text.
      const pattern = new RegExp(
        `\\b(${aliases.map((a) => a.alias.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")).join("|")})\\.?\\s+([1-9]\\d*):([1-9]\\d*)(?:[–-]([1-9]\\d*))?(?![\\d:–-])`,
        "gi",
      );
      for (const match of block.text.matchAll(pattern)) {
        const book = aliases.find(
          (a) => a.alias.toLowerCase() === match[1].toLowerCase(),
        )!.book;
        const start = `${book.osis}.${match[2]}.${match[3]}`,
          end = match[4] ? `${book.osis}.${match[2]}.${match[4]}` : undefined;
        if (!validRange(start, end)) continue;
        proposals.push({
          id: stableId("reference", `${block.id}:${match.index}:${match[0]}`),
          blockId: block.id,
          evidence: match[0],
          start,
          ...(end ? { end } : {}),
          method: "literal-reference-v1",
          decision: "pending",
        });
      }
    }
  return proposals;
}
function localPath(base: string, href: string) {
  if (/^[a-z][a-z0-9+.-]*:|^\/|\\|[?#]/i.test(href))
    throw new DomainError(
      400,
      "external_epub_path",
      "EPUB reading files must be local archive paths without query strings or fragments.",
    );
  let decoded: string;
  try {
    decoded = decodeURIComponent(href);
  } catch {
    throw new DomainError(
      400,
      "invalid_epub_path",
      "An EPUB path has invalid encoding.",
    );
  }
  const result = path.posix.normalize(
    path.posix.join(path.posix.dirname(base), decoded),
  );
  if (
    result.startsWith("../") ||
    result.startsWith("/") ||
    decoded.includes("\\")
  )
    throw new DomainError(
      400,
      "unsafe_epub_path",
      "An EPUB path escapes its container.",
    );
  return result;
}
export function importEpub(files: Map<string, Buffer>): IntakeState {
  if (files.get("mimetype")?.toString() !== "application/epub+zip")
    throw new DomainError(
      400,
      "invalid_epub",
      "The archive is missing the EPUB media-type marker.",
    );
  if (files.has("META-INF/encryption.xml"))
    throw new DomainError(
      400,
      "encrypted_epub",
      "Encrypted/obfuscated EPUBs are unsupported. Supply an authorized unencrypted edition or prepare a Fontes source package.",
    );
  const container = xml(
    files.get("META-INF/container.xml")?.toString("utf8") ?? "",
  );
  const root = array<any>(container.container?.rootfiles?.rootfile)[0];
  const opfPath = root?.["@_full-path"];
  if (!opfPath || !files.has(opfPath))
    throw new DomainError(
      400,
      "invalid_epub",
      "The EPUB package document is missing.",
    );
  const opf = xml(files.get(opfPath)!.toString("utf8")).package;
  const manifest = array<any>(opf?.manifest?.item),
    spine = array<any>(opf?.spine?.itemref);
  if (!spine.length || spine.length > 128)
    throw new DomainError(
      400,
      "epub_spine",
      "This prototype requires 1–128 reading-order documents.",
    );
  if (
    manifest.some((item) =>
      String(item["@_properties"] ?? "")
        .split(/\s+/)
        .includes("remote-resources"),
    )
  )
    throw new DomainError(
      400,
      "remote_epub",
      "Remote-resource EPUBs need offline source preparation.",
    );
  const state = emptyIntake();
  const seen = new Set<string>();
  let characters = 0;
  for (const ref of spine) {
    const item = manifest.find((i) => i["@_id"] === ref["@_idref"]);
    if (!item || item["@_media-type"] !== "application/xhtml+xml")
      throw new DomainError(
        400,
        "unsupported_epub_content",
        "Only XHTML prose documents are supported in the current EPUB importer.",
      );
    const file = localPath(opfPath, item["@_href"]);
    if (!files.has(file) || seen.has(file))
      throw new DomainError(
        400,
        "invalid_epub_spine",
        "Missing or duplicate reading-order document.",
      );
    seen.add(file);
    const source = files.get(file)!.toString("utf8");
    if (/<!ENTITY/i.test(source))
      throw new DomainError(
        400,
        "unsafe_epub_xml",
        "Custom XML entities are unsupported.",
      );
    const tree: any = parse(source);
    const pending: { node: any; depth: number }[] = [{ node: tree, depth: 0 }];
    while (pending.length) {
      const { node, depth } = pending.pop()!;
      if (depth > 64)
        throw new DomainError(
          400,
          "epub_depth",
          "EPUB markup is too deeply nested.",
        );
      for (const child of node.childNodes ?? [])
        pending.push({ node: child, depth: depth + 1 });
    }
    const blocks: IntakeDocument["blocks"] = [];
    let body: any;
    let title = "";
    const text = (node: any): string =>
      node.nodeName === "#text"
        ? node.value
        : node.tagName === "br"
          ? "\n"
          : (node.childNodes ?? [])
              .filter((n: any) => !["script", "style"].includes(n.tagName))
              .map(text)
              .join("");
    const locate = (node: any) => {
      if (node.tagName === "body") body = node;
      if (node.tagName === "title") title = text(node).trim();
      for (const child of node.childNodes ?? []) locate(child);
    };
    locate(tree);
    const walk = (node: any, depth = 0) => {
      if (node.nodeName === "#text" && node.value?.trim())
        throw new DomainError(
          400,
          "unsupported_epub_text",
          "Unwrapped text outside a heading, paragraph, or list item needs offline preparation to preserve its structure.",
        );
      if (depth > 64)
        throw new DomainError(
          400,
          "epub_depth",
          "EPUB markup is too deeply nested.",
        );
      if (
        [
          "table",
          "img",
          "svg",
          "math",
          "audio",
          "video",
          "iframe",
          "object",
          "canvas",
        ].includes(node.tagName)
      )
        throw new DomainError(
          400,
          "unsupported_epub_markup",
          `This EPUB contains ${node.tagName} content. Use the offline package tools to preserve that material.`,
        );
      if (["script", "style"].includes(node.tagName)) {
        state.warnings.push(
          `Non-reading ${node.tagName} content was excluded from ${file}.`,
        );
        return;
      }
      if (
        /^h[1-6]$/.test(node.tagName ?? "") ||
        ["p", "li"].includes(node.tagName)
      ) {
        const value = text(node).replace(/\s+/g, " ").trim();
        if (!value) return;
        characters += value.length;
        if (characters > 2_000_000 || value.length > 32_000)
          throw new DomainError(
            413,
            "epub_text_limit",
            "Extracted text exceeds the prototype limits.",
          );
        // Unsupported nested material must be detected even inside a paragraph/list item.
        const check = (n: any) => {
          if (
            [
              "img",
              "svg",
              "math",
              "table",
              "audio",
              "video",
              "iframe",
              "object",
              "canvas",
            ].includes(n.tagName)
          )
            throw new DomainError(
              400,
              "unsupported_epub_markup",
              `Unsupported embedded ${n.tagName} content.`,
            );
          for (const c of n.childNodes ?? []) check(c);
        };
        check(node);
        const sourceElementId = node.attrs?.find(
          (a: any) => a.name === "id",
        )?.value;
        blocks.push({
          id: stableId(
            "block",
            `${file}:${blocks.length}:${sourceElementId ?? ""}`,
          ),
          kind: /^h/.test(node.tagName)
            ? "heading"
            : node.tagName === "li"
              ? "list-item"
              : "paragraph",
          text: value,
          sourcePath: file,
          ...(sourceElementId ? { sourceElementId } : {}),
        });
        return;
      }
      for (const child of node.childNodes ?? []) walk(child, depth + 1);
    };
    if (body) walk(body);
    if (!blocks.length)
      throw new DomainError(
        400,
        "epub_empty_document",
        `No supported reading blocks in ${file}.`,
      );
    state.documents.push({
      id: stableId("document", file),
      title:
        title ||
        blocks.find((b) => b.kind === "heading")?.text ||
        `Section ${state.documents.length + 1}`,
      sourcePath: file,
      blocks,
    });
  }
  if (state.documents.reduce((n, d) => n + d.blocks.length, 0) > 5000)
    throw new DomainError(
      413,
      "epub_block_limit",
      "The EPUB has more than 5,000 supported blocks.",
    );
  state.warnings.push(
    "Prose import preserves reading order and visible words, but normalizes whitespace and flattens inline styling and navigation. Review every section against the original before approving.",
  );
  state.warnings.push(
    "Themes and implicit allusions have not been generated. AI assistance is not connected; explicit-reference candidates use a conservative detector and need your review.",
  );
  state.proposals = detectReferences(state.documents);
  return state;
}
export function importPrepared(files: Map<string, Buffer>): IntakeState {
  const metadata = xml(files.get("metadata.xml")!.toString("utf8")).resource;
  const state = emptyIntake("prepared");
  const fieldText = (value: any) =>
    typeof value === "object"
      ? String(value?.["#text"] ?? "")
      : String(value ?? "");
  const rights = metadata.rights,
    cover = metadata.cover;
  state.fields = {
    ...state.fields,
    author: array<any>(metadata.contributors?.contributor)
      .map(fieldText)
      .join("; "),
    publisher: fieldText(metadata.publisher),
    edition: fieldText(metadata.edition),
    publicationDate: fieldText(metadata["publication-date"]),
    sourceVersification: metadata.references["@_source-versification"],
    coverageStart: array<any>(metadata.coverage.range)[0]["@_start"],
    coverageEnd: array<any>(metadata.coverage.range)[0]["@_end"] ?? "",
    licenseId: rights["@_license-id"],
    rightsHolder: rights["@_rights-holder"],
    copyright: fieldText(rights.copyright),
    attribution: fieldText(rights.attribution),
    licenseText: files.get("rights/LICENSE.txt")!.toString("utf8"),
    coverAlt: cover["@_alt"],
    coverLicenseId: cover["@_license-id"],
    coverRightsHolder: cover["@_rights-holder"],
    coverAttribution: cover["@_attribution"],
    permissions: Object.fromEntries(
      Object.entries(rights.permissions)
        .filter(([key]) => key.startsWith("@_"))
        .map(([key, value]) => [key.slice(2), value]),
    ) as IntakeState["fields"]["permissions"],
  };
  const index = xml(files.get("index.xml")!.toString("utf8"), true);
  const collect = (nodes: any[]) => {
    for (const node of nodes) {
      if (node.document && node[":@"]?.["@_href"]) {
        const href = node[":@"]["@_href"],
          document = xml(files.get(href)!.toString("utf8"), true).find(
            (n: any) => n.document,
          );
        const blocks = document.document
          .filter((n: any) => n.block)
          .map((n: any) => ({
            id: n[":@"]["@_id"],
            kind: n[":@"]["@_kind"],
            text:
              n[":@"]["@_status"] === "omitted"
                ? `[Explicit source omission: ${n[":@"]["@_source-ref"] ?? "verse"}]`
                : xmlText(n.block.filter((x: any) => x.text)),
            sourcePath: href,
            sourceAssertions: n.block
              .flatMap((child: any) => child.annotations ?? [])
              .map((annotation: any) =>
                Object.entries(annotation)
                  .filter(([key]) => key !== ":@")
                  .map(
                    ([key]) =>
                      `${key}: ${Object.entries(annotation[":@"] ?? {})
                        .map(([name, value]) => `${name.slice(2)}=${value}`)
                        .join(", ")}`,
                  )
                  .join("; "),
              ),
          }));
        state.documents.push({
          id: node[":@"]["@_id"],
          title: node[":@"]["@_title"],
          sourcePath: href,
          blocks,
        });
      } else
        for (const [key, value] of Object.entries(node))
          if (key !== ":@" && Array.isArray(value)) collect(value);
    }
  };
  collect(index);
  state.warnings.push(
    "The validated source archive is retained byte-for-byte. The readable preview flattens inline presentation; downloading the source returns your original ZIP.",
  );
  return state;
}
