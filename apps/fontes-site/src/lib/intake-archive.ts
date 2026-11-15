import yauzl from "yauzl";
import { XMLParser, XMLValidator } from "fast-xml-parser";
import { DomainError } from "./contracts";

export const MAX_UPLOAD = 20 * 1024 * 1024;
export async function readArchive(
  buffer: Buffer,
): Promise<Map<string, Buffer>> {
  if (buffer.length > MAX_UPLOAD)
    throw new DomainError(
      413,
      "upload_too_large",
      "This prototype accepts archives up to 20 MiB. Larger works can use the offline CLI.",
    );
  return new Promise((resolve, reject) => {
    yauzl.fromBuffer(
      buffer,
      { lazyEntries: true, validateEntrySizes: true, strictFileNames: true },
      (error, zip) => {
        if (error || !zip)
          return reject(
            new DomainError(
              400,
              "invalid_archive",
              "The archive is not a readable, unencrypted ZIP.",
            ),
          );
        const files = new Map<string, Buffer>(),
          paths = new Set<string>();
        let bytes = 0,
          entries = 0,
          settled = false;
        const fail = (message: string) => {
          if (!settled) {
            settled = true;
            zip.close();
            reject(new DomainError(400, "unsafe_archive", message));
          }
        };
        zip.on("error", () =>
          fail("The ZIP has invalid paths, sizes, or compressed data."),
        );
        zip.on("end", () => {
          if (!settled) {
            settled = true;
            resolve(files);
          }
        });
        zip.on("entry", (entry: yauzl.Entry) => {
          const name = entry.fileName;
          if (
            ++entries > 2000 ||
            name.length > 240 ||
            name.startsWith("/") ||
            /^[A-Za-z]:/.test(name) ||
            name.split("/").length > 32 ||
            name.includes("\\") ||
            name.split("/").some((p) => p === ".." || p === ".") ||
            /[\x00-\x1f]/.test(name) ||
            paths.has(name.toLowerCase())
          )
            return fail(
              "Unsafe, duplicate, case-colliding, or excessive archive paths.",
            );
          paths.add(name.toLowerCase());
          const type = (entry.externalFileAttributes >>> 16) & 0o170000;
          if (
            (type && type !== 0o100000 && type !== 0o040000) ||
            entry.isEncrypted()
          )
            return fail(
              "Encrypted entries, symlinks, and special files are unsupported.",
            );
          if (name.endsWith("/")) {
            zip.readEntry();
            return;
          }
          if (entry.uncompressedSize > 8 * 1024 * 1024)
            return fail(
              "An archive file exceeds the prototype's 8 MiB expanded-file limit.",
            );
          zip.openReadStream(entry, (err, stream) => {
            if (err || !stream)
              return fail("An archive entry could not be read.");
            const chunks: Buffer[] = [];
            let fileBytes = 0;
            stream.on("data", (chunk: Buffer) => {
              bytes += chunk.length;
              fileBytes += chunk.length;
              if (bytes > 64 * 1024 * 1024 || fileBytes > 8 * 1024 * 1024) {
                stream.destroy();
                fail("The archive exceeds the actual expanded-byte limits.");
              } else chunks.push(chunk);
            });
            stream.on("error", () =>
              fail("An archive entry failed decompression."),
            );
            stream.on("end", () => {
              if (!settled) {
                files.set(name, Buffer.concat(chunks));
                zip.readEntry();
              }
            });
          });
        });
        zip.readEntry();
      },
    );
  });
}
export function xml(text: string, ordered = false): any {
  if (
    text.length > 8 * 1024 * 1024 ||
    /<!DOCTYPE|<!ENTITY|<\?xml-stylesheet/i.test(text)
  )
    throw new DomainError(
      400,
      "unsafe_xml",
      "DTD, entity declarations and executable XML instructions are unsupported.",
    );
  if (XMLValidator.validate(text) !== true)
    throw new DomainError(
      400,
      "invalid_xml",
      "The source contains malformed XML.",
    );
  return new XMLParser({
    ignoreAttributes: false,
    removeNSPrefix: true,
    processEntities: true,
    preserveOrder: ordered,
    trimValues: false,
    parseTagValue: false,
  }).parse(text);
}
export function xmlText(nodes: any): string {
  if (typeof nodes === "string" || typeof nodes === "number")
    return String(nodes);
  if (Array.isArray(nodes)) return nodes.map(xmlText).join("");
  if (nodes && typeof nodes === "object")
    return Object.entries(nodes)
      .filter(([k]) => k !== ":@" && !k.startsWith("@_"))
      .map(([, v]) => xmlText(v))
      .join("");
  return "";
}
export const array = <T>(value: T | T[] | undefined): T[] =>
  value === undefined ? [] : Array.isArray(value) ? value : [value];
