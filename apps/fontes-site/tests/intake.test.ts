import { describe, it, expect } from "vitest";
import { openapi } from "../src/lib/openapi";
import { readArchive, xml } from "../src/lib/intake-archive";
import {
  importEpub,
  validRange,
  detectReferences,
} from "../src/lib/intake-import";
import { emptyIntake, intakeSaveSchema } from "../src/lib/intake-contracts";
import { epubEntries, zip } from "./intake-fixtures";
describe("EPUB draft preparation", () => {
  it("generates machine-readable schemas for preparation requests and responses", () => {
    expect(openapi.components.schemas.Preparation).toHaveProperty(
      "properties.state",
    );
    expect(openapi.paths).toHaveProperty("/api/v1/resources/{id}/intake");
  });
  it("rejects unwrapped text instead of silently dropping it", async () => {
    const files = await readArchive(
      await zip(epubEntries("<p>Visible</p>Unwrapped words")),
    );
    expect(() => importEpub(files)).toThrow("Unwrapped text");
  });
  it("uses spine order instead of manifest/filename order, preserves text, and creates unapproved candidates", async () => {
    const result = importEpub(await readArchive(await zip(epubEntries())));
    expect(result.documents.map((d) => d.title)).toEqual([
      "First section",
      "Last section",
    ]);
    expect(result.documents[0].blocks[0].text).toBe("Grace & peace");
    expect(result.proposals.map((p) => p.start)).toEqual([
      "Rom.1.3",
      "John.3.16",
    ]);
    expect(
      result.proposals.every(
        (p) => p.decision === "pending" && p.method === "literal-reference-v1",
      ),
    ).toBe(true);
  });
  it("keeps deterministic document/block identities for identical input", async () => {
    const files = await readArchive(await zip(epubEntries()));
    expect(importEpub(files)).toEqual(importEpub(files));
  });
  it("does not execute scripts or expose source HTML", async () => {
    const result = importEpub(
      await readArchive(
        await zip(
          epubEntries('<script>alert("bad")</script><p>Visible prose.</p>'),
        ),
      ),
    );
    expect(result.documents[0].blocks.map((b) => b.text)).toEqual([
      "Visible prose.",
    ]);
    expect(result.warnings.join(" ")).toContain("script");
  });
  it.each(["img", "table", "svg", "math"])(
    "rejects unsupported %s instead of silently deleting it",
    async (tag) => {
      const files = await readArchive(
        await zip(epubEntries(`<p>Before</p><${tag}></${tag}><p>After</p>`)),
      );
      expect(() => importEpub(files)).toThrow("EPUB contains");
    },
  );
  it("rejects encryption/obfuscation", async () => {
    const files = await readArchive(
      await zip({
        ...epubEntries(),
        "META-INF/encryption.xml": "<encryption/>",
      }),
    );
    expect(() => importEpub(files)).toThrow("Encrypted");
  });
  it("rejects a missing spine target", async () => {
    const files = await readArchive(await zip(epubEntries()));
    files.delete("EPUB/first.xhtml");
    expect(() => importEpub(files)).toThrow("Missing or duplicate");
  });
  it("rejects duplicate case-colliding archive paths", async () =>
    await expect(
      readArchive(await zip({ "A.xml": "one", "a.xml": "two" })),
    ).rejects.toThrow("case-colliding"));
  it("bounds actual expanded input", async () =>
    await expect(
      readArchive(
        await zip({ "too-large.txt": Buffer.alloc(8 * 1024 * 1024 + 1) }),
      ),
    ).rejects.toThrow("8 MiB"));
  it("rejects entity declarations before XML parsing", () =>
    expect(() => xml('<!DOCTYPE a [<!ENTITY a "boom">]><a>&a;</a>')).toThrow(
      "DTD",
    ));
  it("uses actual canonical verse counts and rejects reversed ranges", () => {
    expect(validRange("John.3.16")).toBe(true);
    expect(validRange("John.3.999")).toBe(false);
    expect(validRange("John.3.17", "John.3.16")).toBe(false);
  });
  it("does not infer allusions, themes or partial cross-chapter ranges", () => {
    const p = detectReferences([
      {
        id: "doc",
        title: "Test",
        sourcePath: "x",
        blocks: [
          {
            id: "b",
            kind: "paragraph",
            sourcePath: "x",
            text: "Romans 1:3-2:5; the prodigal son and divine love.",
          },
        ],
      },
    ]);
    expect(p).toEqual([]);
  });
  it("starts every permission denied and does not accept client-supplied extracted prose", () => {
    const state = emptyIntake();
    expect(Object.values(state.fields.permissions)).toEqual(
      Array(6).fill("deny"),
    );
    expect(
      intakeSaveSchema.safeParse({
        revision: 1,
        method: "guided",
        fields: state.fields,
        documents: [],
      }).success,
    ).toBe(false);
  });
});
