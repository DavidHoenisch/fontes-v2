import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { Button, Card } from "@heroui/react";
import App from "./App";
import { type Note, type Block, books } from "./data/preview";
import {
  defaultMark,
  markColors,
  type MarkStyle,
} from "./components/HighlightStyles";
export const native = isTauri();
export type NativeBlock = {
  type: string;
  text?: string;
  ref_id?: number;
  translation_id?: string;
  quote?: string;
  resource_id?: string;
  title?: string;
  author?: string | null;
  source?: string;
  content_id?: string;
};
export type NativeSelection = {
  translation_id: string;
  spans: { verse_id: string; start: number; end: number; quote: string }[];
};
export type Workspace = {
  content_id: string;
  has_study: boolean;
  profile: string;
  session: {
    selected_ref: number;
    translation: string;
    preferences: {
      theme: string;
      text_size: number;
      line_spacing: number;
      verse_numbers: boolean;
      scripture_serif: boolean;
    };
  };
  books: { id: number; name: string; osis: string; verse_counts: number[] }[];
  translations: { id: string; abbr: string; name: string }[];
  passage: {
    id: string;
    refs: number[];
    text: string;
    strongs_links: { start: number; end: number; strongs: string }[];
    word_links: { start: number; end: number; strongs: string }[];
  }[];
  annotations: {
    id: string;
    kind: string;
    start_ref: number;
    end_ref: number;
    content: string | null;
    updated_at: string;
    color: string | null;
  }[];
  notes: {
    annotation_id: string;
    document: {
      title: string;
      pinned: boolean;
      blocks: NativeBlock[];
      selection: NativeSelection | null;
    };
  }[];
  highlights: {
    annotation_id: string;
    style: { kind: string; color: string; symbol: string | null };
    selection: NativeSelection | null;
  }[];
  saved: { ref_id: number; label: string; kind: string }[];
};
export type Response = { state: Workspace; data: Record<string, unknown> };
export type Command = { command: string; params: Record<string, unknown> };
export type Live = {
  state: Workspace;
  send: (command: Command) => Promise<Response>;
  error: string;
  busy: boolean;
};
export function label(w: Workspace, ref: number) {
  return `${w.books.find((b) => b.id === Math.floor(ref / 1000000))?.name || ""} ${Math.floor((ref % 1000000) / 1000)}:${ref % 1000}`;
}
export function refId(w: Workspace, text: string) {
  const match = text.match(/^(.*) (\d+):(\d+)$/);
  if (!match) throw Error("Choose a verse reference.");
  const b = w.books.find((b) => b.name === match[1]);
  if (!b) throw Error("Unknown book.");
  return b.id * 1000000 + Number(match[2]) * 1000 + Number(match[3]);
}
export function selectionFor(
  w: Workspace,
  anchor: string,
  quote: string,
): NativeSelection | null {
  if (!quote) return null;
  const ref = refId(w, anchor),
    v = w.passage.find((v) => v.refs.includes(ref));
  if (!v)
    throw Error("Open the selected passage before creating this annotation.");
  const at = v.text.indexOf(quote);
  if (at < 0) throw Error("Selection no longer matches this edition.");
  const start = Array.from(v.text.slice(0, at)).length;
  return {
    translation_id: w.session.translation,
    spans: [
      { verse_id: v.id, start, end: start + Array.from(quote).length, quote },
    ],
  };
}
export function nativeNotes(w: Workspace): Note[] {
  return w.annotations
    .filter((a) => a.kind === "note")
    .map((a) => {
      const doc = w.notes.find((n) => n.annotation_id === a.id)?.document;
      return {
        id: a.id,
        endRef: a.end_ref,
        title: doc?.title || "Note",
        pinned: doc?.pinned || false,
        anchor: label(w, a.start_ref),
        quote: doc?.selection?.spans.map((s) => s.quote).join(" ") || "",
        updated: a.updated_at,
        blocks: doc?.blocks.map(
          (b, i) =>
            ({
              id: `${a.id}-${i}`,
              kind:
                b.type === "thought"
                  ? "thought"
                  : b.type === "passage"
                    ? "passage"
                    : "resource",
              text: b.text || b.quote || "",
              source: b.source || b.translation_id,
              ref: b.ref_id ? label(w, b.ref_id) : b.title,
              native: b,
            }) as Block,
        ) || [{ id: a.id, kind: "thought", text: a.content || "" }],
      };
    });
}
export function nativeStyle(
  s: Workspace["highlights"][number]["style"],
): MarkStyle {
  const kinds: Record<string, string> = {
    foreground: "Text color",
    symbol: "Study symbol",
  };
  return {
    kind:
      kinds[s.kind] ||
      s.kind
        .split("_")
        .map((v, i) => (i === 0 ? v[0].toUpperCase() + v.slice(1) : v))
        .join(" "),
    color:
      markColors.find((c) => c[0].toLowerCase() === s.color)?.[1] ||
      defaultMark.color,
    symbol: s.symbol || undefined,
  };
}
export function storedStyle(s: MarkStyle) {
  return {
    kind:
      s.kind === "Text color"
        ? "foreground"
        : s.kind === "Study symbol"
          ? "symbol"
          : s.kind.toLowerCase().replaceAll(" ", "_"),
    color:
      markColors.find((c) => c[1] === s.color)?.[0].toLowerCase() || "yellow",
    symbol: s.symbol || null,
  };
}
export function NativeApp() {
  const [state, setState] = useState<Workspace | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [phase, setPhase] = useState<
    "checking" | "missing" | "failed" | "installing" | "ready"
  >("checking");
  async function open() {
    setPhase("checking");
    setError("");
    try {
      const r = await invoke<Response>("library_open");
      setState(r.state);
      setPhase("ready");
    } catch (e) {
      const failure = e as { kind?: string; message?: string };
      setError(failure?.message || String(e));
      setPhase(failure?.kind === "missing" ? "missing" : "failed");
    }
  }
  useEffect(() => {
    void open();
  }, []);
  useEffect(() => {
    if (phase !== "checking")
      void getCurrentWindow()
        .show()
        .catch(() => {});
  }, [phase]);
  async function send(command: Command) {
    setBusy(true);
    try {
      const r = await invoke<Response>("engine_dispatch", { command });
      setState(r.state);
      setError("");
      return r;
    } catch (e) {
      setError(String(e));
      throw e;
    } finally {
      setBusy(false);
    }
  }
  if (state) return <App live={{ state, send, error, busy }} />;
  // A normal cold start never renders the installer or database-loading copy.
  if (phase === "checking")
    return <main className="native-startup" aria-hidden="true" />;
  const installing = phase === "installing";
  const setup = phase === "missing" || installing;
  return (
    <main className="native-setup">
      <Card>
        <Card.Header>
          <Card.Title>
            {setup
              ? "Make room for the Word."
              : "We couldn’t open your library."}
          </Card.Title>
          <Card.Description>
            {setup
              ? "Install the free resources once. Read and study offline, whenever you return."
              : "Your saved notes are kept. Retry opening the library."}
          </Card.Description>
        </Card.Header>
        <Card.Content>
          {setup ? (
            <>
              <p>
                Bibles, dictionaries, Matthew Henry, cross-references, and the
                rest of the included study resources—all together.
              </p>
              {installing && <p role="status">Installing free resources…</p>}
            </>
          ) : (
            <p role="alert">{error}</p>
          )}
        </Card.Content>
        <Card.Footer>
          {setup ? (
            <Button
              isDisabled={installing}
              onPress={() => {
                void (async () => {
                  setPhase("installing");
                  setError("");
                  try {
                    await invoke("library_install");
                    await open();
                  } catch (e) {
                    setError(String(e));
                    setPhase("failed");
                  }
                })();
              }}
            >
              Install free resources
            </Button>
          ) : (
            <Button onPress={() => void open()}>Try again</Button>
          )}
        </Card.Footer>
      </Card>
    </main>
  );
}

export function readSelection(w: Workspace) {
  const sel = window.getSelection();
  if (!sel || sel.isCollapsed || !sel.rangeCount) return null;
  const range = sel.getRangeAt(0);
  const spans: NativeSelection["spans"] = [];
  const refs: number[] = [];
  for (const el of document.querySelectorAll<HTMLElement>(
    "[data-reference] [data-verse-text]",
  )) {
    if (!range.intersectsNode(el)) continue;
    const ref = refId(
      w,
      el.closest<HTMLElement>("[data-reference]")!.dataset.reference!,
    );
    const verse = w.passage.find((v) => v.refs.includes(ref));
    if (!verse) continue;
    function offset(node: Node, pos: number) {
      const prefix = document.createRange();
      prefix.selectNodeContents(el);
      prefix.setEnd(node, pos);
      return Array.from(prefix.toString()).length;
    }
    const start = el.contains(range.startContainer)
      ? offset(range.startContainer, range.startOffset)
      : 0;
    const end = el.contains(range.endContainer)
      ? offset(range.endContainer, range.endOffset)
      : Array.from(verse.text).length;
    if (end > start) {
      spans.push({
        verse_id: verse.id,
        start,
        end,
        quote: Array.from(verse.text).slice(start, end).join(""),
      });
      refs.push(...verse.refs);
    }
  }
  if (!spans.length) return null;
  return {
    quote: spans.map((s) => s.quote).join(" "),
    anchor: label(w, Math.min(...refs)),
    endRef: Math.max(...refs),
    nativeSelection: { translation_id: w.session.translation, spans },
  };
}
