// @vitest-environment jsdom
import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import type { Live, Response, Workspace } from "../native";
import { CrossReferences } from "./CrossReferences";
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
        <button onClick={onClose}>Close</button>
        {children}
      </div>
    ) : null,
}));
afterEach(cleanup);
const state = {
  content_id: "one",
  session: { selected_ref: 1001001, translation: "kjv" },
  translations: [{ id: "kjv", abbr: "KJV" }],
  books: [{ id: 1, name: "Genesis" }],
} as unknown as Workspace;
const reference = { id: 1001002, label: "Genesis 1:2" };
const commentary = {
  id: "henry",
  title: "Creation",
  author: "Matthew Henry",
  kind: "commentary",
  basis: "applies_to",
  excerpt: "Commentary excerpt",
};
const target = {
  start_ref: 1001003,
  end_ref: 1001003,
  label: "Genesis 1:3",
  preview: "Let there be light",
};
const context = {
  resources: [
    commentary,
    { ...commentary, basis: "citation" },
    {
      id: "easton",
      title: "Earth",
      author: "Easton",
      kind: "dictionary",
      basis: "citation",
      excerpt: "Dictionary excerpt",
    },
    {
      id: "nave",
      title: "Creation topic",
      kind: "topic",
      basis: "citation",
      excerpt: "Topic excerpt",
    },
  ],
  cross_references: [target],
};
const reply = (data: Record<string, unknown>): Response => ({ state, data });
function live(send: Live["send"]): Live {
  return { state, send, error: "", busy: false };
}
function deferred() {
  let resolve!: (value: Response) => void;
  const promise = new Promise<Response>((r) => {
    resolve = r;
  });
  return { promise, resolve };
}

it("shows all returned resource kinds and preserves distinct link reasons without duplicate articles", async () => {
  const send = vi.fn().mockResolvedValue(reply(context));
  render(
    <CrossReferences
      reference={reference}
      onClose={() => {}}
      live={live(send)}
    />,
  );
  const group = await screen.findByRole("region", { name: "Commentary" });
  expect(
    within(group)
      .getByRole("button", { name: /Commentary/ })
      .getAttribute("aria-expanded"),
  ).toBe("false");
  fireEvent.click(within(group).getByRole("button", { name: /Commentary/ }));
  expect(within(group).getAllByRole("button")).toHaveLength(2);
  expect(
    within(group).getByText(/Covers this verse · Cites this verse/),
  ).toBeTruthy();
  expect(screen.getByRole("region", { name: "Dictionaries" })).toBeTruthy();
  expect(screen.getByRole("region", { name: "Topics" })).toBeTruthy();
  expect(screen.queryByRole("button", { name: /Genesis 1:3/ })).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: /Cross-references/ }));
  expect(screen.getByRole("button", { name: /Genesis 1:3/ })).toBeTruthy();
  expect(send).toHaveBeenCalledExactlyOnceWith({
    command: "study.context",
    params: { ref_id: 1001002 },
  });
});

it("loads a selected article in place, retains attribution and returns to the same list", async () => {
  const send = vi
    .fn()
    .mockResolvedValueOnce(reply(context))
    .mockResolvedValueOnce(
      reply({
        resource: {
          id: "henry",
          title: "Creation",
          source: "Henry source",
          author: "Matthew Henry",
          content: "Full article text",
          links: [],
        },
      }),
    );
  render(
    <CrossReferences
      reference={reference}
      onClose={() => {}}
      live={live(send)}
    />,
  );
  fireEvent.click(await screen.findByRole("button", { name: /Commentary/ }));
  fireEvent.click(
    screen.getByRole("button", { name: /Creation.*Matthew Henry/ }),
  );
  await screen.findByText("Full article text");
  expect(screen.getByText("Henry source · Matthew Henry")).toBeTruthy();
  expect(send).toHaveBeenLastCalledWith({
    command: "study.resource",
    params: { id: "henry" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Back to Genesis 1:2" }));
  expect(screen.getByRole("region", { name: "Dictionaries" })).toBeTruthy();
  expect(send).toHaveBeenCalledTimes(2);
});

it("retains in-place cross-reference previews without navigating", async () => {
  const send = vi
    .fn()
    .mockResolvedValueOnce(reply(context))
    .mockResolvedValueOnce(
      reply({
        verses: [
          { ref_id: 1001003, label: "Genesis 1:3", text: "Full verse text" },
        ],
        has_more: false,
      }),
    );
  render(
    <CrossReferences
      reference={reference}
      onClose={() => {}}
      live={live(send)}
    />,
  );
  fireEvent.click(await screen.findByRole("button", { name: /Genesis 1:3/ }));
  await screen.findByText("Full verse text");
  expect(send).toHaveBeenLastCalledWith({
    command: "reading.preview",
    params: { start_ref: 1001003, end_ref: 1001003 },
  });
  fireEvent.click(screen.getByRole("button", { name: "Back to Genesis 1:2" }));
  expect(screen.getByRole("region", { name: "Commentary" })).toBeTruthy();
  expect(
    send.mock.calls.some(([r]) =>
      ["reading.navigate", "reading.select"].includes(r.command),
    ),
  ).toBe(false);
});

it("ignores late article replies after Back and reloads when content changes", async () => {
  const pending = deferred();
  const send = vi
    .fn()
    .mockResolvedValueOnce(reply(context))
    .mockReturnValueOnce(pending.promise)
    .mockResolvedValue(reply({ resources: [], cross_references: [] }));
  const { rerender } = render(
    <CrossReferences
      reference={reference}
      onClose={() => {}}
      live={live(send)}
    />,
  );
  fireEvent.click(await screen.findByRole("button", { name: /Commentary/ }));
  fireEvent.click(
    screen.getByRole("button", { name: /Creation.*Matthew Henry/ }),
  );
  fireEvent.click(screen.getByRole("button", { name: "Back to Genesis 1:2" }));
  await act(async () =>
    pending.resolve(
      reply({
        resource: {
          id: "henry",
          title: "Old",
          content: "Stale text",
          links: [],
        },
      }),
    ),
  );
  expect(screen.queryByText("Stale text")).toBeNull();
  rerender(
    <CrossReferences
      reference={reference}
      onClose={() => {}}
      live={{ ...live(send), state: { ...state, content_id: "two" } }}
    />,
  );
  await screen.findByText(/No linked articles/);
  expect(screen.queryByRole("region", { name: "Commentary" })).toBeNull();
  expect(send).toHaveBeenCalledTimes(3);
});

it("ignores old context replies after changing verse or closing", async () => {
  const pending = deferred();
  const send = vi
    .fn()
    .mockReturnValueOnce(pending.promise)
    .mockResolvedValue(reply({ resources: [], cross_references: [] }));
  const { rerender } = render(
    <CrossReferences
      reference={reference}
      onClose={() => {}}
      live={live(send)}
    />,
  );
  rerender(
    <CrossReferences
      reference={{ id: 1001004, label: "Genesis 1:4" }}
      onClose={() => {}}
      live={live(send)}
    />,
  );
  await screen.findByText(/No linked articles/);
  await act(async () => pending.resolve(reply(context)));
  expect(screen.queryByRole("region", { name: "Commentary" })).toBeNull();
  rerender(
    <CrossReferences reference={null} onClose={() => {}} live={live(send)} />,
  );
  expect(screen.queryByRole("dialog")).toBeNull();
});

it("supports retrying context and article failures", async () => {
  const send = vi
    .fn()
    .mockRejectedValueOnce(Error("Context failed"))
    .mockResolvedValueOnce(reply(context))
    .mockRejectedValueOnce(Error("Article failed"))
    .mockResolvedValueOnce(
      reply({
        resource: { id: "henry", content: "Recovered article", links: [] },
      }),
    );
  render(
    <CrossReferences
      reference={reference}
      onClose={() => {}}
      live={live(send)}
    />,
  );
  await screen.findByRole("alert");
  fireEvent.click(screen.getByRole("button", { name: "Retry" }));
  fireEvent.click(await screen.findByRole("button", { name: /Commentary/ }));
  fireEvent.click(
    screen.getByRole("button", { name: /Creation.*Matthew Henry/ }),
  );
  await waitFor(() =>
    expect(screen.getByRole("alert").textContent).toContain("Article failed"),
  );
  fireEvent.click(screen.getByRole("button", { name: "Retry" }));
  await screen.findByText("Recovered article");
});

it("bounds each expanded group, preserves its page through drill-in, and resets on reopening", async () => {
  const resources = Array.from({ length: 9 }, (_, i) => ({
    ...commentary,
    id: `entry-${i}`,
    title: `Entry ${i + 1}`,
  }));
  const cross_references = Array.from({ length: 14 }, (_, i) => ({
    ...target,
    start_ref: 1001003 + i,
    end_ref: 1001003 + i,
    label: `Genesis 1:${i + 3}`,
  }));
  const send = vi.fn(
    async (request: { command: string; params: Record<string, unknown> }) =>
      reply(
        request.command === "study.context"
          ? { resources, cross_references }
          : {
              resource: {
                id: request.params.id,
                title: "Selected entry",
                content: "Selected full text",
                links: [],
              },
            },
      ),
  );
  const { rerender } = render(
    <CrossReferences
      reference={reference}
      onClose={() => {}}
      live={live(send)}
    />,
  );
  const refs = await screen.findByRole("region", { name: "Cross-references" });
  expect(refs.querySelectorAll(".verse-reference")).toHaveLength(6);
  expect(screen.queryByRole("button", { name: /Entry 1/ })).toBeNull();
  fireEvent.click(
    screen.getByRole("button", { name: "Next cross-references page" }),
  );
  expect(screen.getByText("7–12 of 14")).toBeTruthy();
  expect(screen.getByRole("button", { name: "Genesis 1:9" })).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: /Commentary/ }));
  expect(screen.queryByRole("button", { name: "Genesis 1:9" })).toBeNull();
  expect(screen.getAllByRole("button", { name: /Entry \d/ })).toHaveLength(4);
  fireEvent.click(screen.getByRole("button", { name: "Next commentary page" }));
  fireEvent.click(screen.getByRole("button", { name: /Entry 5/ }));
  await screen.findByText("Selected full text");
  fireEvent.click(screen.getByRole("button", { name: "Back to Genesis 1:2" }));
  expect(screen.getByText("5–8 of 9")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: /Cross-references/ }));
  expect(screen.getByText("7–12 of 14")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: /Cross-references/ }));
  expect(screen.queryByRole("button", { name: "Genesis 1:9" })).toBeNull();
  expect(send).toHaveBeenCalledTimes(2);
  rerender(
    <CrossReferences reference={null} onClose={() => {}} live={live(send)} />,
  );
  rerender(
    <CrossReferences
      reference={reference}
      onClose={() => {}}
      live={live(send)}
    />,
  );
  await screen.findByText("1–6 of 14");
  expect(
    screen
      .getByRole("button", { name: "Previous cross-references page" })
      .hasAttribute("disabled"),
  ).toBe(true);
  fireEvent.click(
    screen.getByRole("button", { name: "Next cross-references page" }),
  );
  fireEvent.click(
    screen.getByRole("button", { name: "Next cross-references page" }),
  );
  expect(screen.getByText("13–14 of 14")).toBeTruthy();
  expect(
    screen
      .getByRole("button", { name: "Next cross-references page" })
      .hasAttribute("disabled"),
  ).toBe(true);
});

it("reuses verse contexts across dismissal and revisits but invalidates on edition changes", async () => {
  const send = vi.fn().mockResolvedValue(reply(context));
  const { rerender } = render(
    <CrossReferences
      reference={reference}
      onClose={() => {}}
      live={live(send)}
    />,
  );
  await screen.findByRole("button", { name: "Genesis 1:3" });
  rerender(
    <CrossReferences reference={null} onClose={() => {}} live={live(send)} />,
  );
  rerender(
    <CrossReferences
      reference={reference}
      onClose={() => {}}
      live={live(send)}
    />,
  );
  await screen.findByRole("button", { name: "Genesis 1:3" });
  expect(send).toHaveBeenCalledTimes(1);
  rerender(
    <CrossReferences
      reference={{ id: 1001004, label: "Genesis 1:4" }}
      onClose={() => {}}
      live={live(send)}
    />,
  );
  await waitFor(() => expect(send).toHaveBeenCalledTimes(2));
  rerender(
    <CrossReferences
      reference={reference}
      onClose={() => {}}
      live={live(send)}
    />,
  );
  await screen.findByRole("button", { name: "Genesis 1:3" });
  expect(send).toHaveBeenCalledTimes(2);
  rerender(
    <CrossReferences
      reference={reference}
      onClose={() => {}}
      live={{
        ...live(send),
        state: { ...state, session: { ...state.session, translation: "web" } },
      }}
    />,
  );
  await waitFor(() => expect(send).toHaveBeenCalledTimes(3));
});

it("deduplicates an in-flight context when the same verse is quickly reopened", async () => {
  const pending = deferred();
  const send = vi.fn(() => pending.promise);
  const { rerender } = render(
    <CrossReferences
      reference={reference}
      onClose={() => {}}
      live={live(send)}
    />,
  );
  rerender(
    <CrossReferences reference={null} onClose={() => {}} live={live(send)} />,
  );
  rerender(
    <CrossReferences
      reference={reference}
      onClose={() => {}}
      live={live(send)}
    />,
  );
  expect(send).toHaveBeenCalledTimes(1);
  await act(async () => pending.resolve(reply(context)));
  await screen.findByRole("button", { name: "Genesis 1:3" });
});
