// @vitest-environment jsdom
import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { SearchWorkspace } from "./SearchWorkspace";
import type { Live, Workspace, Response } from "../native";
vi.mock("./Sheet", () => ({
  Sheet: ({
    open,
    title,
    children,
    onClose,
  }: {
    open: boolean;
    title: string;
    children: React.ReactNode;
    onClose: () => void;
  }) =>
    open ? (
      <div role="dialog" aria-label={title}>
        <button onClick={onClose}>Close search</button>
        {children}
      </div>
    ) : null,
}));
vi.mock('../data/preview',()=>({books:[{id:43,name:'John'}],passages:[{book:43,chapter:3,verses:['For God so loved the world.']}]}));
afterEach(cleanup);
const state = {
  content_id: "one",
  session: { translation: "kjv" },
  translations: [{ id: "kjv", name: "King James Version" }],
} as unknown as Workspace;
const verse = {
  id: "verse",
  ref_id: 43003016,
  title: "John 3:16",
  excerpt: "For God so loved the world.",
};
const resource = {
  id: "article",
  ref_id: null,
  title: "Faith",
  excerpt: "Trust in God",
};
const reply = (data: Record<string, unknown>): Response => ({ state, data });
const live = (send: Live["send"]): Live => ({
  state,
  send,
  error: "",
  busy: false,
});
function submit(q: string) {
  fireEvent.change(screen.getByRole("searchbox"), { target: { value: q } });
  fireEvent.submit(screen.getByRole("search"));
}
it("submits word searches and requests each scope from the engine, preserving scope during paging", async () => {
  const send = vi
    .fn()
    .mockResolvedValue(reply({ hits: [verse], has_more: true }));
  render(
    <SearchWorkspace
      open
      onClose={() => {}}
      onRead={() => {}}
      live={live(send)}
    />,
  );
  submit("trust");
  await screen.findByText(verse.excerpt);
  expect(send).toHaveBeenLastCalledWith({
    command: "search.query",
    params: { query: "trust", scope: "all", limit: 10, offset: 0 },
  });
  fireEvent.click(screen.getByRole("tab", { name: "Resources" }));
  await waitFor(() =>
    expect(send).toHaveBeenLastCalledWith({
      command: "search.query",
      params: { query: "trust", scope: "resources", limit: 10, offset: 0 },
    }),
  );
  await screen.findByText(verse.excerpt);
  fireEvent.click(screen.getByRole("button", { name: /Next/ }));
  await waitFor(() =>
    expect(send).toHaveBeenLastCalledWith({
      command: "search.query",
      params: { query: "trust", scope: "resources", limit: 10, offset: 10 },
    }),
  );
});
it("uses the arrow button, opens a verse at its canonical reference and closes after completion", async () => {
  const send = vi
      .fn()
      .mockResolvedValue(reply({ hits: [verse], has_more: false })),
    onRead = vi.fn().mockResolvedValue(undefined),
    onClose = vi.fn();
  render(
    <SearchWorkspace
      open
      onClose={onClose}
      onRead={onRead}
      live={live(send)}
    />,
  );
  fireEvent.change(screen.getByRole("searchbox"), {
    target: { value: "John 3:16" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Search" }));
  fireEvent.click(
    await screen.findByRole("button", { name: /John 3:16.*King James/ }),
  );
  await waitFor(() => expect(onRead).toHaveBeenCalledWith(43003016));
  expect(onClose).toHaveBeenCalledTimes(1);
});
it("loads full resources with attribution and returns to the retained search results", async () => {
  const send = vi
    .fn()
    .mockResolvedValueOnce(reply({ hits: [resource], has_more: false }))
    .mockResolvedValueOnce(
      reply({
        resource: {
          id: "article",
          title: "Faith",
          content: "Full original article",
          source: "Dictionary",
          author: "Author",
          links: [],
        },
      }),
    );
  render(
    <SearchWorkspace
      open
      onClose={() => {}}
      onRead={() => {}}
      live={live(send)}
    />,
  );
  submit("faith");
  fireEvent.click(
    await screen.findByRole("button", { name: /Faith.*Study resource/ }),
  );
  await screen.findByText("Full original article");
  expect(screen.getByText("Dictionary · Author")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: /Back to results/ }));
  expect(
    screen.getByRole("button", { name: /Faith.*Study resource/ }),
  ).toBeTruthy();
  expect(send).toHaveBeenCalledTimes(2);
});
it("invalidates pending results on edits, dismissal and edition changes", async () => {
  let resolve!: (r: Response) => void;
  const pending = new Promise<Response>((r) => {
    resolve = r;
  });
  const send = vi.fn(() => pending);
  const { rerender } = render(
    <SearchWorkspace
      open
      onClose={() => {}}
      onRead={() => {}}
      live={live(send)}
    />,
  );
  submit("trust");
  expect(screen.getByRole("status").textContent).toContain("Searching");
  fireEvent.change(screen.getByRole("searchbox"), {
    target: { value: "new query" },
  });
  await act(async () => resolve(reply({ hits: [verse], has_more: false })));
  expect(screen.queryByText(verse.excerpt)).toBeNull();
  rerender(
    <SearchWorkspace
      open={false}
      onClose={() => {}}
      onRead={() => {}}
      live={live(send)}
    />,
  );
  expect(screen.queryByRole("dialog")).toBeNull();
  rerender(
    <SearchWorkspace
      open
      onClose={() => {}}
      onRead={() => {}}
      live={{
        ...live(send),
        state: { ...state, session: { ...state.session, translation: "web" } },
      }}
    />,
  );
  expect((screen.getByRole("searchbox") as HTMLInputElement).value).toBe("");
});
it("shows empty and retry states and retains only actual recent searches", async () => {
  const send = vi
    .fn()
    .mockRejectedValueOnce(Error("Search unavailable"))
    .mockResolvedValue(reply({ hits: [], has_more: false }));
  render(
    <SearchWorkspace
      open
      onClose={() => {}}
      onRead={() => {}}
      live={live(send)}
    />,
  );
  submit("trust");
  await screen.findByRole("alert");
  fireEvent.click(screen.getByRole("button", { name: "Retry search" }));
  await screen.findByText("No matches for “trust”");
  fireEvent.click(screen.getByRole("button", { name: "Clear search" }));
  expect(screen.getByText("Recent searches")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "trust" }));
  await waitFor(() => expect(send).toHaveBeenCalledTimes(3));
});
