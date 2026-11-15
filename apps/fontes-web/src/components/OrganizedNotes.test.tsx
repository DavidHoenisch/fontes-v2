import { describe, it, expect } from "vitest";
import { filterOrganizedNotes, type Organization } from "./OrganizedNotes";
import type { Note } from "../data/preview";
const note = (id: string, anchor: string, text: string): Note => ({
  id,
  title: id,
  anchor,
  quote: "",
  updated: "2026-10-17",
  pinned: false,
  blocks: [{ id: "thought", kind: "thought", text }],
});
const notes = [
  note("a", "Romans 8:28", "Providence and hope"),
  note("b", "Romans 8:1", "Grace"),
  note("c", "John 3:16", "Love"),
];
const org: Organization = {
  folders: [{ id: "sermons", name: "Sermons" }],
  entries: [
    { note_id: "a", folder_id: "sermons", tags: ["grace", "prayer"] },
    { note_id: "b", folder_id: "sermons", tags: ["grace"] },
  ],
};
describe("combined note discovery", () => {
  it("intersects folder, tags, chapter and text without duplicating notes", () => {
    expect(
      filterOrganizedNotes(
        notes,
        org,
        "sermons",
        "hope",
        ["grace", "prayer"],
        "Romans",
        "8",
        "28",
        false,
        "recent",
      ).map((n) => n.id),
    ).toEqual(["a"]);
    expect(
      filterOrganizedNotes(
        notes,
        org,
        "sermons",
        "",
        ["grace", "prayer"],
        "John",
        "",
        "",
        false,
        "recent",
      ),
    ).toEqual([]);
  });
  it("places legacy notes in Inbox and finds tags in search", () => {
    expect(
      filterOrganizedNotes(
        notes,
        org,
        "inbox",
        "",
        [],
        "",
        "",
        "",
        false,
        "recent",
      ).map((n) => n.id),
    ).toEqual(["c"]);
    expect(
      filterOrganizedNotes(
        notes,
        org,
        "all",
        "prayer",
        [],
        "",
        "",
        "",
        false,
        "recent",
      ).map((n) => n.id),
    ).toEqual(["a"]);
  });
  it("sorts by canonical book, chapter and verse", () => {
    expect(
      filterOrganizedNotes(
        notes,
        org,
        "all",
        "",
        [],
        "",
        "",
        "",
        false,
        "scripture",
      ).map((n) => n.id),
    ).toEqual(["c", "b", "a"]);
  });
});
