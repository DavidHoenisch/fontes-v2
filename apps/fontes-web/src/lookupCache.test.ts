import { expect, it, vi } from "vitest";
import { createLookupCache } from "./lookupCache";
it("deduplicates pending reads and isolates snapshot/edition keys", async () => {
  const cache = createLookupCache();
  let resolve!: (data: string) => void;
  const load = vi.fn(
    () =>
      new Promise<string>((done) => {
        resolve = done;
      }),
  );
  const first = cache.load("snapshot:kjv:word:G4416", load);
  expect(cache.load("snapshot:kjv:word:G4416", load)).toBe(first);
  resolve("headword");
  await expect(first).resolves.toBe("headword");
  expect(load).toHaveBeenCalledOnce();
  await cache.load("snapshot:web:word:G4416", async () => "other edition");
  await expect(cache.load("snapshot:kjv:word:G4416", load)).resolves.toBe(
    "headword",
  );
  cache.clear();
  await expect(
    cache.load("snapshot:kjv:word:G4416", async () => "reloaded"),
  ).resolves.toBe("reloaded");
});
it("evicts the least-recently-used result and permits retries after rejection", async () => {
  const cache = createLookupCache(2);
  const reload = vi.fn(async () => "replacement");
  await cache.load("one", async () => "one");
  await cache.load("two", async () => "two");
  await cache.load("one", reload);
  await cache.load("three", async () => "three");
  await expect(cache.load("two", reload)).resolves.toBe("replacement");
  expect(reload).toHaveBeenCalledOnce();
  await expect(
    cache.load("failed", async () => {
      throw Error("Offline");
    }),
  ).rejects.toThrow("Offline");
  await expect(cache.load("failed", async () => "retried")).resolves.toBe(
    "retried",
  );
});
