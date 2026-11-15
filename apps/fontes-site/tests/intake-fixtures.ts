import { ZipFile } from "yazl";
export async function zip(entries: Record<string, string | Buffer>) {
  const archive = new ZipFile();
  const chunks: Buffer[] = [];
  const result = new Promise<Buffer>((resolve, reject) => {
    archive.outputStream.on("data", (c) => chunks.push(c));
    archive.outputStream.on("end", () => resolve(Buffer.concat(chunks)));
    archive.outputStream.on("error", reject);
  });
  for (const [name, content] of Object.entries(entries))
    archive.addBuffer(
      Buffer.isBuffer(content) ? content : Buffer.from(content),
      name,
      {
        mtime: new Date("1980-01-01T00:00:00Z"),
        compress: name !== "mimetype",
      },
    );
  archive.end();
  return result;
}
export function epubEntries(
  body = "<h1>Grace &amp; peace</h1><p>Read Romans 1:3 and John 3:16. John 3:999 is invalid.</p>",
) {
  return {
    mimetype: "application/epub+zip",
    "META-INF/container.xml":
      '<container xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="EPUB/book.opf" media-type="application/oebps-package+xml"/></rootfiles></container>',
    "EPUB/book.opf":
      '<package xmlns="http://www.idpf.org/2007/opf" version="3.0"><metadata/><manifest><item id="last" href="last.xhtml" media-type="application/xhtml+xml"/><item id="first" href="first.xhtml" media-type="application/xhtml+xml"/></manifest><spine><itemref idref="first"/><itemref idref="last"/></spine></package>',
    "EPUB/first.xhtml": `<html xmlns="http://www.w3.org/1999/xhtml"><head><title>First section</title></head><body>${body}</body></html>`,
    "EPUB/last.xhtml":
      '<html xmlns="http://www.w3.org/1999/xhtml"><head><title>Last section</title></head><body><p>Final words.</p></body></html>',
  };
}
