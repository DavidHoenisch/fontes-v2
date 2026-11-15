import { describe, it, expect } from "vitest";
import { readFile, writeFile, mkdir } from "node:fs/promises";
import path from "node:path";
import { reviewEpub } from "../src/lib/epub-review";
import { readArchive } from "../src/lib/intake-archive";
import { hash, importEpub } from "../src/lib/intake-import";
import { epubEntries, zip } from "./intake-fixtures";
describe("bounded EPUB evaluation", () => {
  it("preserves footnote text, superscripts, back links and normalized source text", async () => {
    const result = await reviewEpub(
      await readArchive(
        await zip(
          epubEntries(
            '<p id="p">Original words<a id="ref" href="#note" epub:type="noteref"><sup>1</sup></a>.</p><aside id="note" epub:type="footnote"><a href="#ref">1</a><span> A note, not discarded.</span></aside>',
          ),
        ),
      ),
    );
    expect(result.summary.statistics.footnotes).toBe(1);
    expect(result.summary.statistics.footnoteReferences).toBe(1);
    expect(result.summary.statistics.unresolvedLinks).toBe(0);
    expect(result.summary.fidelity.normalizedTextMatches).toBe(true);
    const blocks = result.documents[0].blocks;
    expect(blocks[1].kind).toBe("note");
    expect(
      blocks[0].segments.find((s) => s.text === "1")?.target?.blockId,
    ).toBe(blocks[1].id);
  });
  it("respects XHTML self-closing anchors instead of HTML error correction", async () => {
    const result = await reviewEpub(
      await readArchive(
        await zip(
          epubEntries(
            '<span id="page"/><p id="first"><i><a id="empty"/>First</i> words.</p><p>Second paragraph.</p>',
          ),
        ),
      ),
    );
    expect(result.documents[0].blocks.map((b) => b.text)).toEqual([
      "First words.",
      "Second paragraph.",
    ]);
    expect(result.summary.fidelity.normalizedTextMatches).toBe(true);
  });
  it("leaves ambiguous duplicate source anchors unresolved", async () => {
    const result = await reviewEpub(
      await readArchive(
        await zip(
          epubEntries(
            '<p id="duplicate">One</p><p id="duplicate">Two</p><p><a href="#duplicate">Ambiguous</a></p>',
          ),
        ),
      ),
    );
    expect(result.summary.statistics.unresolvedLinks).toBe(1);
    expect(result.documents[0].blocks[2].segments[0].target).toBeUndefined();
  });
  it("rejects external DTDs before parsing XHTML", async () => {
    const files = await readArchive(await zip(epubEntries()));
    files.set(
      "EPUB/first.xhtml",
      Buffer.from(
        '<!DOCTYPE html SYSTEM "https://example.invalid/evil"><html><body><p>Words</p></body></html>',
      ),
    );
    await expect(reviewEpub(files)).rejects.toThrow("external DTDs");
  });
  it("does not duplicate nested list words or lose unwrapped prose", async () => {
    const result = await reviewEpub(
      await readArchive(
        await zip(
          epubEntries(
            "Before<ul><li>Outer<ol><li>Inner</li></ol>After</li></ul>End",
          ),
        ),
      ),
    );
    expect(result.documents[0].blocks.map((b) => b.text).join("")).toBe(
      "BeforeOuterInnerAfterEnd",
    );
    expect(result.summary.fidelity.normalizedTextMatches).toBe(true);
  });
  it("retains poetry paragraphs in a single-cell layout table", async () => {
    const result = await reviewEpub(
      await readArchive(
        await zip(
          epubEntries(
            "<table><tbody><tr><td><p>Line one.</p><p>Line two.</p></td></tr></tbody></table>",
          ),
        ),
      ),
    );
    expect(result.documents[0].blocks.map((b) => b.text)).toEqual([
      "Line one.",
      "Line two.",
    ]);
    expect(result.summary.statistics.poetryTables).toBe(1);
  });
  it("rejects multi-cell tables rather than flattening their meaning", async () => {
    await expect(
      reviewEpub(
        await readArchive(
          await zip(
            epubEntries("<table><tr><td>A</td><td>B</td></tr></table>"),
          ),
        ),
      ),
    ).rejects.toThrow("Multi-cell");
  });
  it("keeps external/script targets inert and never fetches them", async () => {
    const result = await reviewEpub(
      await readArchive(
        await zip(
          epubEntries(
            '<p><a href="javascript:alert(1)">Words</a> <a href="https://example.invalid">more</a></p>',
          ),
        ),
      ),
    );
    expect(result.summary.statistics.unresolvedLinks).toBe(2);
    expect(result.documents[0].blocks[0].segments.every((s) => !s.target)).toBe(
      true,
    );
  });
  it("rejects unsupported media even inside a footnote", async () => {
    await expect(
      reviewEpub(
        await readArchive(
          await zip(
            epubEntries(
              '<aside epub:type="footnote"><span>Note</span><iframe/></aside>',
            ),
          ),
        ),
      ),
    ).rejects.toThrow("Embedded media");
  });
});
describe.skipIf(!process.env.FONTES_EPUB_FIXTURE)(
  "supplied real-book fixture",
  () => {
    it("audits exact bytes without sending book text to a database", async () => {
      const buffer = await readFile(process.env.FONTES_EPUB_FIXTURE!);
      const files = await readArchive(buffer);
      let legacyFailure = "";
      try {
        importEpub(files);
      } catch (e) {
        legacyFailure = (e as Error).message;
      }
      const result = await reviewEpub(files);
      const out = path.resolve(".local/verification/calvin-review");
      await mkdir(out, { recursive: true });
      await writeFile(
        path.join(out, "audit.json"),
        JSON.stringify(
          {
            sourceSha256: hash(buffer),
            sourceBytes: buffer.length,
            legacyFailure,
            ...result.summary,
          },
          null,
          2,
        ),
      );
      console.log(
        JSON.stringify(
          {
            sourceSha256: hash(buffer),
            sourceBytes: buffer.length,
            legacyFailure,
            statistics: result.summary.statistics,
            fidelity: result.summary.fidelity,
          },
          null,
          2,
        ),
      );
      expect(result.summary.fidelity.normalizedTextMatches).toBe(true);
      if (
        hash(buffer) ===
        "b5b3f101794bf03696780c0b74aedafd1ce4c8a809c0b789370030bc24bc2382"
      ) {
        expect(result.summary.statistics.documents).toBe(108);
        expect(result.summary.statistics.footnotes).toBe(658);
        expect(result.summary.statistics.footnoteReferences).toBe(658);
        expect(result.summary.statistics.poetryTables).toBe(3);
        expect(result.summary.publicationNotice).toContain(
          "incorporated into another website",
        );
      }
    }, 30_000);
  },
);
