// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { PassageAttachment } from "./PassageAttachment";
import type { Live } from "../native";
// jsdom omits CSS.escape, which React Aria uses to focus selected options.
vi.stubGlobal("CSS", {
  escape: (value: string) =>
    value.replace(/[^a-zA-Z0-9_-]/g, (char) => `\\${char}`),
});
afterEach(cleanup);
const books = [
  { id: 1, name: "Genesis", chapters: [{ number: 1, verses: 31 }] },
  { id: 43, name: "John", chapters: [{ number: 3, verses: 36 }] },
];
it("previews the chosen passage before insertion, preserving exact edition and reference", async () => {
  const send = vi.fn(async () => ({
    data: { block: { quote: "For God so loved the world" } },
  }));
  const onInsert = vi.fn();
  render(
    <PassageAttachment
      books={books}
      initialRef={1001001}
      live={
        { state: { session: { translation: "kjv" } }, send } as unknown as Live
      }
      onInsert={onInsert}
      onBack={() => {}}
    />,
  );
  fireEvent.click(screen.getByRole("button", { name: /Book/ }));
  fireEvent.click(await screen.findByRole("option", { name: "John" }));
  fireEvent.click(screen.getByRole("button", { name: /Verse/ }));
  fireEvent.click(
    await screen.findByRole("option", { name: /^16$/ }),
  );
  expect(send).not.toHaveBeenCalled();
  expect(onInsert).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "Preview passage" }));
  await waitFor(() =>
    expect(screen.getByText("For God so loved the world")).toBeTruthy(),
  );
  expect(send).toHaveBeenCalledExactlyOnceWith({
    command: "note.passage",
    params: { ref_id: 43003016 },
  });
  fireEvent.click(screen.getByRole("button", { name: "Add John 3:16" }));
  expect(onInsert).toHaveBeenCalledWith(
    expect.objectContaining({
      ref: "John 3:16",
      native: {
        type: "passage",
        ref_id: 43003016,
        translation_id: "kjv",
        quote: "For God so loved the world",
      },
    }),
  );
  fireEvent.click(screen.getByRole("button", { name: /Verse/ }));
  fireEvent.click(
    await screen.findByRole("option", { name: /^17$/ }),
  );
  expect(screen.queryByRole("button", { name: "Add John 3:16" })).toBeNull();
});
