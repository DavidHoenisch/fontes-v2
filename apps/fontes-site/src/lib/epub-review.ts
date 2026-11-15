import path from "node:path";
import sharp from "sharp";
import { XMLParser, XMLValidator } from "fast-xml-parser";
import { array, xml, xmlText } from "./intake-archive";
import { hash, detectReferences } from "./intake-import";
import { DomainError } from "./contracts";
import type { z } from "zod";
import type { Proposal } from "./intake-contracts";
import type { publisherReferenceSchema } from "./epub-review-contracts";
export type ReviewSegment = {
  text: string;
  marks: string[];
  lineBreak?: true;
  target?: { documentId: string; blockId?: string };
  unresolved?: string;
};
export type ReviewBlock = {
  id: string;
  kind: "heading" | "paragraph" | "list-item" | "note" | "image";
  text: string;
  segments: ReviewSegment[];
  sourceElementId?: string;
  image?: string;
  alt?: string;
  poetry?: boolean;
  references: z.infer<typeof publisherReferenceSchema>[];
};
export type ReviewDocument = {
  id: string;
  title: string;
  sourcePath: string;
  blocks: ReviewBlock[];
};
export type ReviewSummary = {
  profile: "epub-review-v1";
  title: string;
  authors: string[];
  contributors: string[];
  publisher: string;
  language: string;
  declaredRights: string;
  publicationNotice: string;
  documents: {
    id: string;
    title: string;
    sourcePath: string;
    blocks: number;
    characters: number;
    sha256: string;
  }[];
  statistics: {
    documents: number;
    blocks: number;
    characters: number;
    footnotes: number;
    footnoteReferences: number;
    sourceLinks: number;
    resolvedLinks: number;
    unresolvedLinks: number;
    scriptureCandidates: number;
    images: number;
    poetryTables: number;
  };
  fidelity: {
    normalizedTextMatches: boolean;
    sourceTextSha256: string;
    extractedTextSha256: string;
  };
  warnings: string[];
  rightsReviewRequired: true;
  exportable: false;
};
export type ReviewManifest = ReviewSummary & {
  sourceSha256: string;
  sourceBytes: number;
  assets: {
    sha256: string;
    mediaType: string;
    width: number;
    height: number;
  }[];
};
const id = (prefix: string, key: string) =>
  `${prefix}-${hash(key).slice(0, 24)}`;
function fail(code: string, message: string, status = 422): never {
  throw new DomainError(status, code, message);
}
const attr = (n: any, name: string) =>
  n.attrs?.find((a: any) => a.name === name || `${a.prefix}:${a.name}` === name)
    ?.value as string | undefined;
const children = (n: any): any[] => n.childNodes ?? [];
const find = (n: any, p: (n: any) => boolean): any =>
  p(n)
    ? n
    : children(n)
        .map((c) => find(c, p))
        .find(Boolean);
const descendants = (n: any): any[] => [n, ...children(n).flatMap(descendants)];
const plain = (n: any): string =>
  n.nodeName === "#text"
    ? n.value
    : ["script", "style"].includes(n.tagName)
      ? ""
      : children(n).map(plain).join("");
const compact = (s: string) => s.replace(/\s+/g, "");
function targetPath(base: string, href: string) {
  if (/^[a-z][a-z0-9+.-]*:|^\/|\\|[\x00-\x1f?]/i.test(href)) return null;
  const [relative, fragment, ...rest] = href.split("#");
  if (rest.length) return null;
  let decoded: string, anchor: string | undefined;
  try {
    decoded = decodeURIComponent(relative);
    anchor = fragment ? decodeURIComponent(fragment) : undefined;
  } catch {
    return null;
  }
  if (/\\|[\x00-\x1f?#]|^[a-z][a-z0-9+.-]*:|^\//i.test(decoded)) return null;
  const file = relative
    ? path.posix.normalize(path.posix.join(path.posix.dirname(base), decoded))
    : base;
  if (file.startsWith("../") || file.startsWith("/") || file === "..")
    return null;
  return { file, anchor };
}
// Evaluation model, deliberately separate from the narrower source-package converter.
// Uploaded HTML is parsed as data and never embedded or executed in the browser.
export async function reviewEpub(files: Map<string, Buffer>) {
  if (files.get("mimetype")?.toString() !== "application/epub+zip")
    fail("invalid_epub", "Missing EPUB media type.");
  if (files.has("META-INF/encryption.xml"))
    fail(
      "encrypted_epub",
      "Encrypted or font-obfuscated EPUBs are unsupported.",
    );
  const container = xml(files.get("META-INF/container.xml")?.toString() ?? "");
  const opfPath = array<any>(container.container?.rootfiles?.rootfile)[0]?.[
    "@_full-path"
  ];
  if (!opfPath || !files.has(opfPath))
    fail("invalid_epub", "Missing package document.");
  const opf = xml(files.get(opfPath)!.toString()).package,
    metadata = opf?.metadata ?? {};
  const manifest = array<any>(opf?.manifest?.item),
    spine = array<any>(opf?.spine?.itemref);
  if (!spine.length || spine.length > 256)
    fail("epub_spine", "Evaluation requires 1–256 reading-order documents.");
  const manifestIds = manifest.map((i) => i["@_id"]);
  if (new Set(manifestIds).size !== manifestIds.length)
    fail("invalid_epub", "Duplicate manifest IDs.");
  if (
    manifest.some((i) =>
      String(i["@_properties"] ?? "")
        .split(/\s+/)
        .includes("remote-resources"),
    )
  )
    fail("remote_epub", "Remote-dependent EPUBs are unsupported.");
  const warnings = new Set<string>();
  const parseDocument = (source: string) => {
    const clean = source.replace(/<!DOCTYPE\s+html\s*>/i, "");
    if (/<!DOCTYPE|<!ENTITY|<\?xml-stylesheet/i.test(clean))
      fail(
        "unsafe_epub_xml",
        "Custom entities, external DTDs and executable XML instructions are unsupported.",
      );
    if (XMLValidator.validate(clean) !== true)
      fail(
        "invalid_epub_xhtml",
        "A reading document is not well-formed XHTML.",
      );
    const ordered = new XMLParser({
      ignoreAttributes: false,
      preserveOrder: true,
      trimValues: false,
      parseTagValue: false,
      processEntities: true,
      cdataPropName: "#cdata",
    }).parse(clean);
    let count = 0;
    const convert = (nodes: any[], depth = 0): any[] => {
      if (depth > 64) fail("epub_complexity", "Excessive markup depth.");
      return nodes.flatMap((entry) =>
        Object.entries(entry)
          .filter(([key]) => key !== ":@" && !key.startsWith("?"))
          .flatMap(([key, value]) => {
            if (++count > 200_000)
              fail("epub_complexity", "Excessive markup node count.");
            if (key === "#text")
              return [{ nodeName: "#text", value: String(value) }];
            if (key === "#cdata") return convert(value as any[], depth + 1);
            if (key.startsWith("#")) return [];
            return [
              {
                nodeName: key,
                tagName: key.split(":").at(-1),
                attrs: Object.entries(entry[":@"] ?? {}).map(
                  ([name, value]) => ({
                    name: name.replace(/^@_/, ""),
                    value: String(value),
                  }),
                ),
                childNodes: convert(value as any[], depth + 1),
              },
            ];
          }),
      );
    };
    return { nodeName: "#document", childNodes: convert(ordered) };
  };
  // EPUB navigation supplies descriptive chapter labels and their enclosing book.
  const navigation = new Map<string, string>();
  const nav = manifest.find((i) =>
    String(i["@_properties"] ?? "")
      .split(/\s+/)
      .includes("nav"),
  );
  if (nav) {
    const location = targetPath(opfPath, nav["@_href"]);
    if (!location || !files.has(location.file))
      fail("epub_navigation", "Missing navigation file.");
    const tree = parseDocument(files.get(location.file)!.toString()),
      toc = find(
        tree,
        (n) =>
          n.tagName === "nav" &&
          (attr(n, "epub:type") ?? "").split(/\s+/).includes("toc"),
      );
    const visit = (node: any, parents: string[]) => {
      if (node.tagName === "li") {
        const anchor = children(node).find((n) => n.tagName === "a");
        const label = anchor ? plain(anchor).replace(/\s+/g, " ").trim() : "";
        if (anchor) {
          const t = targetPath(location.file, attr(anchor, "href") ?? "");
          if (t && label && !navigation.has(t.file))
            navigation.set(
              t.file,
              [...parents, label].join(" › ").slice(0, 1000),
            );
        }
        for (const c of children(node))
          if (c !== anchor) visit(c, label ? [...parents, label] : parents);
      } else for (const c of children(node)) visit(c, parents);
    };
    if (toc) visit(toc, []);
  }
  const documents: ReviewDocument[] = [],
    assets = new Map<
      string,
      { buffer: Buffer; mediaType: string; width: number; height: number }
    >();
  const anchors = new Map<string, { documentId: string; blockId: string }>(),
    sourceLinks: { base: string; href: string; segments: ReviewSegment[] }[] =
      [];
  const ambiguousAnchors = new Set<string>();
  const sourceTexts: string[] = [];
  let characters = 0,
    blockCount = 0,
    poetryTables = 0,
    footnoteReferences = 0;
  const seen = new Set<string>();
  for (const ref of spine) {
    const item = manifest.find((i) => i["@_id"] === ref["@_idref"]);
    if (!item || item["@_media-type"] !== "application/xhtml+xml")
      fail("epub_content", "Reading-order item is not XHTML prose.");
    const target = targetPath(opfPath, item["@_href"] ?? "");
    if (
      !target ||
      target.anchor ||
      !files.has(target.file) ||
      seen.has(target.file)
    )
      fail("epub_spine", "Missing, duplicate or unsafe reading file.");
    const file = target.file;
    seen.add(file);
    const tree = parseDocument(files.get(file)!.toString()),
      body = find(tree, (n) => n.tagName === "body");
    if (!body) fail("epub_empty_document", "Missing reading body.");
    const document: ReviewDocument = {
      id: id("document", file),
      sourcePath: file,
      title:
        navigation.get(file) ||
        plain(find(tree, (n) => n.tagName === "title") ?? {}).trim() ||
        `Section ${documents.length + 1}`,
      blocks: [],
    };
    const ids = new Set<string>();
    for (const n of descendants(body)) {
      const a = attr(n, "id");
      if (a) {
        if (ids.has(a)) {
          ambiguousAnchors.add(`${file}#${a}`);
          warnings.add(
            `Duplicate source anchors in ${file}; ambiguous links remain unresolved.`,
          );
        }
        ids.add(a);
      }
    }
    sourceTexts.push(compact(plain(body)));
    let waiting: string[] = [];
    function bind(node: any, block: ReviewBlock) {
      for (const n of descendants(node)) {
        const a = attr(n, "id");
        if (a)
          anchors.set(`${file}#${a}`, {
            documentId: document.id,
            blockId: block.id,
          });
      }
      for (const a of waiting)
        anchors.set(`${file}#${a}`, {
          documentId: document.id,
          blockId: block.id,
        });
      waiting = [];
    }
    function segments(
      node: any,
      marks: string[] = [],
      link?: { base: string; href: string; segments: ReviewSegment[] },
    ): ReviewSegment[] {
      if (node.nodeName === "#text") {
        const segment: ReviewSegment = { text: node.value, marks };
        if (link) link.segments.push(segment);
        return [segment];
      }
      if (["script", "style"].includes(node.tagName)) {
        warnings.add(
          "Script/style content was excluded; uploaded HTML is never executed.",
        );
        return [];
      }
      if (node.tagName === "br")
        return [{ text: "\n", marks, lineBreak: true }];
      if (node.tagName === "a") {
        const href = attr(node, "href");
        if (href) {
          link = { base: file, href, segments: [] };
          sourceLinks.push(link);
        }
        if ((attr(node, "epub:type") ?? "").includes("noteref"))
          footnoteReferences++;
      }
      const mark = ["em", "i"].includes(node.tagName)
        ? "emphasis"
        : ["strong", "b"].includes(node.tagName)
          ? "strong"
          : node.tagName === "sup"
            ? "superscript"
            : undefined;
      return children(node).flatMap((c) =>
        segments(c, mark ? [...marks, mark] : marks, link),
      );
    }
    function emit(node: any, kind: ReviewBlock["kind"], poetry = false) {
      if (
        descendants(node).some(
          (n) =>
            n !== node &&
            [
              "img",
              "table",
              "svg",
              "math",
              "audio",
              "video",
              "iframe",
              "object",
              "canvas",
            ].includes(n.tagName),
        )
      )
        fail(
          "unsupported_epub_markup",
          "Embedded media inside an inline or footnote block needs a richer profile.",
        );
      const spans = segments(node),
        text = spans.map((s) => s.text).join("");
      if (!text.trim()) {
        for (const n of descendants(node)) {
          const a = attr(n, "id");
          if (a) waiting.push(a);
        }
        return;
      }
      if (
        text.length > 64_000 ||
        (characters += text.length) > 8_000_000 ||
        ++blockCount > 25_000
      )
        fail(
          "epub_text_limit",
          `Evaluation limit in ${file}: block ${text.length}, total ${characters}, blocks ${blockCount}.`,
          413,
        );
      const elementId = attr(node, "id");
      const block: ReviewBlock = {
        id: id("block", `${file}:${document.blocks.length}:${elementId ?? ""}`),
        kind,
        text,
        segments: spans,
        ...(elementId ? { sourceElementId: elementId } : {}),
        ...(poetry ? { poetry: true } : {}),
        references: [],
      };
      bind(node, block);
      document.blocks.push(block);
    }
    async function walk(node: any, poetry = false): Promise<void> {
      if (["script", "style"].includes(node.tagName)) {
        warnings.add(
          "Script/style content was excluded; uploaded HTML is never executed.",
        );
        return;
      }
      if (
        [
          "svg",
          "math",
          "audio",
          "video",
          "iframe",
          "object",
          "canvas",
          "form",
          "input",
          "template",
        ].includes(node.tagName)
      )
        fail(
          "unsupported_epub_markup",
          `Evaluation does not support ${node.tagName} content.`,
        );
      if (node.tagName === "table") {
        const cells = descendants(node).filter((n) =>
          ["td", "th"].includes(n.tagName),
        );
        if (
          cells.length !== 1 ||
          descendants(cells[0]).some((n) => n.tagName === "table") ||
          Number(attr(cells[0], "colspan") ?? 1) !== 1 ||
          Number(attr(cells[0], "rowspan") ?? 1) !== 1
        )
          fail(
            "unsupported_epub_table",
            "Multi-cell or nested tables need a richer conversion profile.",
          );
        const anchor = attr(node, "id");
        if (anchor) waiting.push(anchor);
        poetryTables++;
        warnings.add(
          "Single-cell layout tables are rendered as ordered poetry paragraphs; table layout is not retained.",
        );
        for (const c of children(cells[0])) await walk(c, true);
        return;
      }
      if (node.tagName === "img") {
        const location = targetPath(file, attr(node, "src") ?? "");
        if (!location || location.anchor || !files.has(location.file))
          fail("epub_image", "An image is missing or has an unsafe source.");
        const buffer = files.get(location.file)!;
        if (buffer.length > 4 * 1024 * 1024)
          fail("epub_image", "An image exceeds 4 MiB.");
        const info = await sharp(buffer, { limitInputPixels: 16_000_000 })
          .metadata()
          .catch(() => null);
        if (
          !info ||
          !["png", "jpeg"].includes(info.format ?? "") ||
          (info.pages ?? 1) !== 1 ||
          (info.orientation ?? 1) !== 1
        )
          fail(
            "epub_image",
            "Only upright single-frame PNG/JPEG images are supported.",
          );
        await sharp(buffer, { limitInputPixels: 16_000_000 }).raw().toBuffer();
        const digest = hash(buffer);
        assets.set(digest, {
          buffer,
          mediaType: info.format === "png" ? "image/png" : "image/jpeg",
          width: info.width!,
          height: info.height!,
        });
        const block: ReviewBlock = {
          id: id("block", `${file}:${document.blocks.length}:image`),
          kind: "image",
          text: "",
          segments: [],
          image: digest,
          alt: attr(node, "alt") ?? "Source image",
          references: [],
        };
        bind(node, block);
        document.blocks.push(block);
        blockCount++;
        return;
      }
      if (/^h[1-6]$/.test(node.tagName ?? "")) {
        emit(node, "heading", poetry);
        return;
      }
      if (
        node.tagName === "aside" &&
        (attr(node, "epub:type") ?? "")
          .split(/\s+/)
          .some((t) => ["footnote", "endnote"].includes(t))
      ) {
        emit(node, "note", poetry);
        return;
      }
      if (node.tagName === "p") {
        if (
          descendants(node).some(
            (n) =>
              n !== node &&
              [
                "img",
                "table",
                "svg",
                "math",
                "audio",
                "video",
                "iframe",
                "object",
                "canvas",
              ].includes(n.tagName),
          )
        )
          fail(
            "unsupported_epub_markup",
            "Embedded media inside a paragraph needs a richer profile.",
          );
        emit(node, "paragraph", poetry);
        return;
      }
      const a = attr(node, "id");
      if (a) waiting.push(a);
      // Split nested list/container runs without duplicating descendants' words.
      let run: any[] = [];
      const flush = () => {
        if (run.length) {
          emit(
            { childNodes: run },
            node.tagName === "li" ? "list-item" : "paragraph",
            poetry,
          );
          run = [];
        }
      };
      for (const child of children(node)) {
        if (
          [
            "h1",
            "h2",
            "h3",
            "h4",
            "h5",
            "h6",
            "p",
            "ol",
            "ul",
            "li",
            "div",
            "section",
            "article",
            "aside",
            "table",
            "nav",
            "img",
            "header",
            "footer",
            "blockquote",
            "figure",
            "figcaption",
          ].includes(child.tagName)
        ) {
          flush();
          await walk(child, poetry);
        } else if (child.tagName === "hr") {
          flush();
        } else run.push(child);
      }
      flush();
    }
    await walk(body);
    if (!document.blocks.length)
      fail("epub_empty_document", `No readable content in ${file}.`);
    for (const a of waiting)
      anchors.set(`${file}#${a}`, {
        documentId: document.id,
        blockId: document.blocks.at(-1)!.id,
      });
    // Footnote and image emitters do not recursively walk; reject hidden unsupported material there too.
    for (const n of descendants(body))
      if (
        [
          "svg",
          "math",
          "audio",
          "video",
          "iframe",
          "object",
          "canvas",
          "form",
          "input",
          "template",
        ].includes(n.tagName)
      )
        fail("unsupported_epub_markup", `Unsupported ${n.tagName} content.`);
    documents.push(document);
  }
  const byFile = new Map(documents.map((d) => [d.sourcePath, d]));
  let resolvedLinks = 0;
  for (const link of sourceLinks) {
    const t = targetPath(link.base, link.href);
    const doc = t ? byFile.get(t.file) : undefined;
    const target = t?.anchor
      ? ambiguousAnchors.has(`${t.file}#${t.anchor}`)
        ? undefined
        : anchors.get(`${t.file}#${t.anchor}`)
      : doc
        ? { documentId: doc.id }
        : undefined;
    if (target) {
      for (const s of link.segments) s.target = target;
      resolvedLinks++;
    } else {
      for (const s of link.segments) s.unresolved = link.href.slice(0, 500);
    }
  }
  if (sourceLinks.length - resolvedLinks)
    warnings.add(
      "Some source links could not be resolved; they are shown as inert text with their source target and are never fetched.",
    );
  const proposals = detectReferences(
    documents.map((d) => ({
      ...d,
      blocks: d.blocks
        .filter((b) => b.kind !== "image")
        .map((b) => ({
          ...b,
          sourcePath: d.sourcePath,
          kind: b.kind as "paragraph",
        })),
    })),
  );
  if (proposals.length > 25_000)
    fail("epub_reference_limit", "Too many reference candidates.", 413);
  const byBlock = new Map<string, Proposal[]>();
  for (const p of proposals) {
    const list = byBlock.get(p.blockId) ?? [];
    list.push(p);
    byBlock.set(p.blockId, list);
  }
  for (const d of documents)
    for (const b of d.blocks) b.references = byBlock.get(b.id) ?? [];
  const sourceTextSha256 = hash(sourceTexts.join("")),
    extractedTextSha256 = hash(
      documents.flatMap((d) => d.blocks.map((b) => compact(b.text))).join(""),
    );
  if (sourceTextSha256 !== extractedTextSha256)
    fail(
      "epub_fidelity",
      "Normalized source text differs from the extracted reading text. No draft was committed.",
    );
  warnings.add(
    "Evaluation preview only: original section order, readable text, emphasis, local links and footnotes are retained; CSS/pagination is not reproduced.",
  );
  warnings.add(
    "Scripture candidates are conservative literal matches and remain unapproved. Themes/allusions and AI processing are not connected.",
  );
  warnings.add(
    "Source rights notices are preserved as claims requiring review. This preview does not authorize distribution, source export or publication.",
  );
  const metadataValues = (value: any) =>
    array<any>(value)
      .map((v) => xmlText(v).trim())
      .filter(Boolean);
  const summary: ReviewSummary = {
    profile: "epub-review-v1",
    title: metadataValues(metadata.title)[0] ?? "Untitled EPUB",
    authors: metadataValues(metadata.creator),
    contributors: metadataValues(metadata.contributor),
    publisher: metadataValues(metadata.publisher)[0] ?? "",
    language: metadataValues(metadata.language)[0] ?? "",
    declaredRights: metadataValues(metadata.rights).join("\n"),
    publicationNotice: documents
      .filter((d) =>
        /copyright|publication.notice|rights/i.test(
          d.sourcePath + " " + d.title,
        ),
      )
      .flatMap((d) => d.blocks.map((b) => b.text))
      .join("\n")
      .slice(0, 40_000),
    documents: documents.map((d) => ({
      id: d.id,
      title: d.title,
      sourcePath: d.sourcePath,
      blocks: d.blocks.length,
      characters: d.blocks.reduce((n, b) => n + b.text.length, 0),
      sha256: hash(JSON.stringify(d)),
    })),
    statistics: {
      documents: documents.length,
      blocks: blockCount,
      characters,
      footnotes: documents
        .flatMap((d) => d.blocks)
        .filter((b) => b.kind === "note").length,
      footnoteReferences,
      sourceLinks: sourceLinks.length,
      resolvedLinks,
      unresolvedLinks: sourceLinks.length - resolvedLinks,
      scriptureCandidates: proposals.length,
      images: assets.size,
      poetryTables,
    },
    fidelity: {
      normalizedTextMatches: true,
      sourceTextSha256,
      extractedTextSha256,
    },
    warnings: [...warnings],
    rightsReviewRequired: true,
    exportable: false,
  };
  return { summary, documents, assets };
}
