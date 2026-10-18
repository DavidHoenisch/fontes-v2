// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { NoteContext, noteDate } from "./NoteContext";
import type { Live } from "../native";
vi.mock("../native", () => ({ refId: () => 20016002 }));
afterEach(cleanup);
const makeLive = () => ({ state: { content_id: "test", session: { translation: "kjv" }, books: [{ id: 20, name: "Proverbs" }], passage: [] }, send: vi.fn(async () => ({ data: { block: { quote: "All the ways of a man" } } })) }) as unknown as Live;
it("keeps saved selections ahead of current-edition context", () => {
  const live = makeLive();
  render(<NoteContext quote="Saved exact selection" anchor="Proverbs 16:2" live={live} />);
  expect(screen.getByText("Saved exact selection")).toBeTruthy();
  expect(live.send).not.toHaveBeenCalled();
});
it("loads missing passage context without navigation or saving", async () => {
  const live = makeLive();
  render(<NoteContext quote="" anchor="Proverbs 16:2" live={live} />);
  await waitFor(() => expect(screen.getByText("All the ways of a man")).toBeTruthy());
  expect(live.send).toHaveBeenCalledExactlyOnceWith({command: "note.passage", params: {ref_id: 20016002}});
});
it("formats metadata into compact local calendar dates", () => {
  const now = new Date(2026, 9, 17, 20);
  expect(noteDate(new Date(2026, 9, 17, 10).toISOString(), now)).toBe("Today");
  expect(noteDate(new Date(2026, 9, 16, 10).toISOString(), now)).toBe("Yesterday");
});
