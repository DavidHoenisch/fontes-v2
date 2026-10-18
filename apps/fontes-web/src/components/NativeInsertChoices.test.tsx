// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import {
  NativeInsertChoices,
  clearInsertContextCache,
} from "./NativeInsertChoices";
import type { Live, Response, Workspace } from "../native";
const state = {
  content_id: "snapshot-a",
  session: { selected_ref: 20016003, translation: "kjv" },
  books: [{ id: 20, name: "Proverbs" }],
  passage: [
    { id: "v3", refs: [20016003], text: "Commit thy works unto the LORD." },
  ],
} as unknown as Workspace;
const resource = {
  id: "mh:verse",
  title: "Matthew Henry",
  author: "Matthew Henry",
  kind: "commentary",
  excerpt: "A summary",
};
const reply = (data: Record<string, unknown>): Response => ({ state, data });
function deferred() {
  let resolve!: (r: Response) => void;
  const promise = new Promise<Response>((r) => {
    resolve = r;
  });
  return { promise, resolve };
}
afterEach(cleanup);
beforeEach(clearInsertContextCache);
it("offers thought and passage immediately while a slow context request is pending", () => {
  const context = deferred(),
    send = vi.fn(() => context.promise),
    onInsert = vi.fn();
  render(
    <NativeInsertChoices
      live={{ state, send, busy: false, error: "" }}
      onInsert={onInsert}
    />,
  );
  expect(screen.getByRole("button", { name: /Your thought/ })).toBeTruthy();
  expect(
    screen.getByRole("button", { name: /Scripture passage/ }),
  ).toBeTruthy();
  expect(screen.getByRole("status").textContent).toContain("Finding");
  fireEvent.click(screen.getByRole("button", { name: /Your thought/ }));
  expect(onInsert).toHaveBeenCalledWith(
    expect.objectContaining({ kind: "thought" }),
  );
  expect(send).toHaveBeenCalledTimes(1);
  expect(send).toHaveBeenCalledWith({
    command: "study.context",
    params: { ref_id: 20016003 },
  });
});
it("fetches only the chosen resource and retains exact source provenance", async () => {
  const selected = deferred(),
    send = vi
      .fn()
      .mockResolvedValueOnce(reply({ resources: [resource] }))
      .mockReturnValueOnce(selected.promise),
    onInsert = vi.fn();
  render(
    <NativeInsertChoices
      live={{ state, send, busy: false, error: "" }}
      onInsert={onInsert}
    />,
  );
  const option = await screen.findByRole("button", { name: /Matthew Henry/ });
  expect(send).toHaveBeenCalledTimes(1);
  fireEvent.click(option);
  expect(send).toHaveBeenLastCalledWith({
    command: "study.resource",
    params: { id: resource.id },
  });
  expect(onInsert).not.toHaveBeenCalled();
  await act(async () =>
    selected.resolve(
      reply({
        resource: {
          id: resource.id,
          title: resource.title,
          author: resource.author,
          source: "mh-source",
          content: "A real excerpt 😀.",
        },
      }),
    ),
  );
  expect(onInsert).toHaveBeenCalledWith(
    expect.objectContaining({
      text: "A real excerpt 😀.",
      native: expect.objectContaining({
        resource_id: resource.id,
        quote: "A real excerpt 😀.",
        content_id: "snapshot-a",
      }),
    }),
  );
});
it("reuses context for a repeat open but invalidates it for a new snapshot", async () => {
  const send = vi.fn(async () => reply({ resources: [resource] })),
    onInsert = vi.fn();
  const first = render(
    <NativeInsertChoices
      live={{ state, send, busy: false, error: "" }}
      onInsert={onInsert}
    />,
  );
  await screen.findByRole("button", { name: /Matthew Henry/ });
  first.unmount();
  const second = render(
    <NativeInsertChoices
      live={{ state, send, busy: false, error: "" }}
      onInsert={onInsert}
    />,
  );
  await screen.findByRole("button", { name: /Matthew Henry/ });
  expect(send).toHaveBeenCalledTimes(1);
  second.unmount();
  render(
    <NativeInsertChoices
      live={{
        state: { ...state, content_id: "snapshot-b" },
        send,
        busy: false,
        error: "",
      }}
      onInsert={onInsert}
    />,
  );
  await waitFor(() => expect(send).toHaveBeenCalledTimes(2));
});
it("does not add a late selected excerpt after the menu closes", async () => {
  const selected = deferred(),
    send = vi
      .fn()
      .mockResolvedValueOnce(reply({ resources: [resource] }))
      .mockReturnValueOnce(selected.promise),
    onInsert = vi.fn();
  const view = render(
    <NativeInsertChoices
      live={{ state, send, busy: false, error: "" }}
      onInsert={onInsert}
    />,
  );
  fireEvent.click(await screen.findByRole("button", { name: /Matthew Henry/ }));
  view.unmount();
  await act(async () =>
    selected.resolve(
      reply({
        resource: { ...resource, source: "mh", content: "Late content" },
      }),
    ),
  );
  expect(onInsert).not.toHaveBeenCalled();
});
