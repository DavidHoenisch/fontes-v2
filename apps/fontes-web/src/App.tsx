import { spotlightVerse } from "./verseArrival";
import { LibraryResourceCard, resourceIdentity } from "./components/LibraryResourceCard";
import { ResourceCitations, type Citation } from "./components/ResourceCitations";
import { PassageAttachment } from "./components/PassageAttachment";
import { NoteContext, noteDate } from "./components/NoteContext";
import { NativeInsertChoices } from "./components/NativeInsertChoices";
import { CrossReferences } from "./components/CrossReferences";
import {
  readSelection,
  nativeNotes,
  nativeStyle,
  storedStyle,
  selectionFor,
  refId,
  label,
  type Live,
  type NativeBlock,
} from "./native";
import {
  markColors,
  markKinds,
  studySymbols,
  markCSS,
  defaultMark,
  type MarkStyle,
} from "./components/HighlightStyles";
import {
  ResponsiveContainer,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  Tooltip,
} from "recharts";
import previewOccurrences from "./data/occurrences.json";
import { useEffect, useRef, useState } from "react";
import { Button, Card, Chip, Input, Tabs, TextArea } from "@heroui/react";
import {
  ArrowDown,
  ArrowLeft,
  ArrowRight,
  ArrowUp,
  BookOpen,
  Bookmark,
  Check,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  Library,
  MessageSquare,
  MoreHorizontal,
  NotebookPen,
  Plus,
  Search,
  Settings2,
  BookOpenText,
  Sun,
  Moon,
  Trash2,
  Layers,
  Quote,
  Type,
  ExternalLink,
  Pin,
  X,
} from "lucide-react";
import { Sheet } from "./components/Sheet";
import {
  books,
  passages,
  loadNotes,
  persistNotes,
  samples,
  type Note,
  type Block,
} from "./data/preview";
type Page = "Read" | "Study" | "Notes" | "Library";
const nav = [
  { page: "Read", icon: BookOpen },
  { page: "Study", icon: BookOpenText },
  { page: "Notes", icon: NotebookPen },
  { page: "Library", icon: Library },
] as const;
const makeId = () => crypto.randomUUID();
export default function App({ live }: { live?: Live }) {
  const [page, setPage] = useState<Page>("Read"),
    [phone, setPhone] = useState(true),
    [dark, setDark] = useState(
      live
        ? [
            "midnight_mint",
            "ink_saffron",
            "charcoal_raspberry",
            "aubergine_apricot",
          ].includes(live.state.session.preferences.theme)
        : false,
    ),
    [accent, setAccent] = useState(
      live?.state.session.preferences.theme === "terracotta_linen"
        ? "clay"
        : live?.state.session.preferences.theme === "sea_glass"
          ? "sea"
          : "default",
    );
  const [passageIndex, setPassageIndex] = useState(0),
    [notes, setNotes] = useState<Note[]>(() =>
      live ? nativeNotes(live.state) : loadNotes(),
    ),
    [saved, setSaved] = useState(false);
  const [dialog, setDialog] = useState<
      | "licenses"
      | "highlight"
      | "passages"
      | "settings"
      | "search"
      | "insert"
      | "source"
      | "delete"
      | "discard"
      | "delete-confirm"
      | null
    >(null),
    [cover, setCover] = useState<number | null>(null),
    [query, setQuery] = useState(""),
    [filter, setFilter] = useState("notes"),
    [studyTab, setStudyTab] = useState("meaning"),
    [flipped, setFlipped] = useState(false);
  const [draft, setDraft] = useState<Note | null>(null),
    [noteModal, setNoteModal] = useState<Note[] | null>(null),
    [source, setSource] = useState<Block | null>(null),
    [selection, setSelection] = useState<{
      quote: string;
      anchor: string;
      endRef?: number;
      nativeSelection?: import("./native").NativeSelection;
    } | null>(null),
    [highlights, setHighlights] = useState<Record<string, string>>({}),
    [notice, setNotice] = useState(""),
    [deleteId, setDeleteId] = useState("");
  const [markStyle, setMarkStyle] = useState<MarkStyle>(defaultMark);
  const [highlightStyles, setHighlightStyles] = useState<
    Record<string, MarkStyle>
  >({});
  const [crossRef, setCrossRef] = useState<{
    id: number;
    label: string;
  } | null>(null);
  const [notices, setNotices] = useState("");
  const [contextWords, setContextWords] = useState<
    { surface: string; strongs: string | null; position: number }[]
  >([]);
  const [contextResources, setContextResources] = useState<
    { id: string; title: string; excerpt: string; kind: string; basis: string; author: string | null }[]
  >([]);
  const [studyMode, setStudyMode] = useState("words");
  const [contextLoading, setContextLoading] = useState(false);
  const [articleLinks, setArticleLinks] = useState<{id: string; links: Citation[]} | null>(null);
  const [studyRef, setStudyRef] = useState("");
  const [catalog, setCatalog] = useState<
    { id: string; kind: string; count: number; author: string | null }[]
  >([]);
  const [resourcePage, setResourcePage] = useState<{
    resources: { id: string; title: string; excerpt: string }[];
    has_more: boolean;
    offset: number;
  } | null>(null);
  const [parentSource, setParentSource] = useState<string | null>(null);
  const [searchResults, setSearchResults] = useState<
    { id: string; ref_id: number | null; title: string; excerpt: string }[]
  >([]);
  const [searchMore, setSearchMore] = useState(false);
  const [searchOffset, setSearchOffset] = useState(0);
  const [wordData, setWordData] = useState<{
    code: string;
    headwords: {
      text: string;
      transliteration?: string;
      resource_id: string;
    }[];
    definitions: { id: string; title: string; excerpt: string }[];
  } | null>(null);
  const [liveOccurrences, setLiveOccurrences] = useState<{
    total: number;
    books: { id: number; name: string; verses: number }[];
    verses: { ref_id: number; label: string; preview: string | null }[];
    has_more: boolean;
    offset: number;
  } | null>(null);
  const [wordQuote, setWordQuote] = useState("");
  const [definition, setDefinition] = useState("");
  const [definitionTitle, setDefinitionTitle] = useState("");
  const [definitionId, setDefinitionId] = useState("");
  const definitionRequest = useRef(0);
  const [liveClips, setLiveClips] = useState<Block[]>([]);
  const occurrences = live
    ? liveOccurrences || { total: 0, books: [] }
    : previewOccurrences;
  const [libraryResource, setLibraryResource] = useState<string | null>(null);
  const [pickerBook, setPickerBook] = useState<number | null>(null);
  const [attachmentPassage, setAttachmentPassage] = useState(false);
  const [pickerChapter, setPickerChapter] = useState<number | null>(null);
  const [arrival, setArrival] = useState<number | null>(null);
  const clearSpotlight = useRef<(() => void) | null>(null);
  useEffect(() => () => clearSpotlight.current?.(), []);
  const [pickerQuery, setPickerQuery] = useState("");
  const [leaveTarget, setLeaveTarget] = useState<Page>("Notes");
  const scrollRef = useRef<HTMLDivElement>(null),
    hold = useRef<ReturnType<typeof setTimeout> | null>(null),
    holdOrigin = useRef({ x: 0, y: 0 }),
    pressed = useRef(false);
  const p = live
      ? {
          book: Math.floor(live.state.session.selected_ref / 1000000),
          chapter: Math.floor(
            (live.state.session.selected_ref % 1000000) / 1000,
          ),
          section: "Scripture",
          verses: live.state.passage.map((v) => v.text),
        }
      : passages[passageIndex],
    book = books.find((b) => b.id === p.book)!,
    currentRef = live
      ? label(live.state, live.state.session.selected_ref)
      : `${book.name} ${p.chapter}:3`;
  const run = (command: string, params: Record<string, unknown>) =>
    live!.send({ command, params });
  useEffect(() => {
    if (live) {
      setNotes(nativeNotes(live.state));
      setSaved(
        live.state.saved.some(
          (s) =>
            s.kind === "favorite" &&
            s.ref_id === live.state.session.selected_ref,
        ),
      );
    }
  }, [live?.state]);
  useEffect(() => {
    if (!live || page !== "Study") return;
    let active = true;
    setContextLoading(true); setContextResources([]); setContextWords([]);
    void run("study.context", { ref_id: live.state.session.selected_ref })
      .then(r => { if (active) {
        setContextWords(r.data.tokens as typeof contextWords);
        setContextResources(r.data.resources as typeof contextResources);
      } }).catch(e => { if (active) showNotice(String(e)); })
      .finally(() => { if (active) setContextLoading(false); });
    return () => { active = false; };
  }, [page, live?.state.session.selected_ref, live?.state.content_id]);
  useEffect(() => {
    if (live && page === "Library")
      void run("library.catalog", { source_id: null, offset: 0 })
        .then((r) => setCatalog(r.data.sources as typeof catalog))
        .catch((e) => showNotice(String(e)));
  }, [page]);
  async function searchLibrary(offset = 0) {
    try {
      const r = await run("search.query", { query, limit: 30, offset });
      setSearchResults(r.data.hits as typeof searchResults);
      setSearchMore(Boolean(r.data.has_more));
      setSearchOffset(offset);
    } catch (e) {
      showNotice(String(e));
    }
  }
  async function openArticle(id: string) {
    try {
      const r = await run("study.resource", { id });
      const a = r.data.resource as {
        id: string;
        title: string;
        content: string;
        source: string;
        author: string | null;
        links: Citation[];
      };
      setArticleLinks({id: a.id, links: a.links});
      setSource({
        id: a.id,
        kind: "resource",
        text: a.content,
        ref: a.title,
        source: a.source,
        native: {
          type: "resource",
          resource_id: a.id,
          quote: a.content,
          title: a.title,
          source: a.source,
          author: a.author,
          content_id: r.state.content_id,
        },
      });
      setDialog("source");
    } catch (e) {
      showNotice(String(e));
    }
  }
  async function openSource(id: string, offset = 0) {
    try {
      const r = await run("library.catalog", { source_id: id, offset });
      setParentSource(id);
      setResourcePage(r.data as typeof resourcePage);
      setQuery("");
    } catch (e) {
      showNotice(String(e));
    }
  }
  useEffect(() => {
    if (!live || page !== "Read" || dialog) return;
    let timer: ReturnType<typeof setTimeout>;
    const changed = () => {
      clearTimeout(timer);
      timer = setTimeout(() => {
        const selected = readSelection(live.state);
        if (selected) setSelection(selected);
      }, 160);
    };
    document.addEventListener("selectionchange", changed);
    return () => {
      clearTimeout(timer);
      document.removeEventListener("selectionchange", changed);
    };
  }, [live?.state.passage, page, dialog]);
  useEffect(() => {
    document.documentElement.classList.toggle("dark", dark);
    document.documentElement.dataset.accent = accent;
  }, [dark, accent]);
  useEffect(() => {
    if (live) return;
    try {
      persistNotes(notes);
    } catch {
      setNotice(
        "Browser storage is full. Keep this page open to retain your changes.",
      );
    }
  }, [notes]);
  useEffect(() => {
    if (notice) {
      const t = setTimeout(() => setNotice(""), 3600);
      return () => clearTimeout(t);
    }
  }, [notice]);
  useEffect(() => {
    scrollRef.current?.scrollTo(0, 0);
  }, [page, passageIndex, draft?.id]);
  useEffect(() => {
    if (arrival === null || page !== "Read" || dialog !== null) return;
    const frame = requestAnimationFrame(() => {
      const verse = scrollRef.current?.querySelector<HTMLElement>(`[data-verse-refs~="${arrival}"]`);
      if (!verse) return;
      verse.scrollIntoView({ block: "center", behavior: "instant" });
      clearSpotlight.current?.();
      clearSpotlight.current = spotlightVerse(verse);
      setArrival(null);
    });
    return () => cancelAnimationFrame(frame);
  }, [arrival, page, dialog, live?.state.session.selected_ref, passageIndex]);
  const showNotice = (text: string) => setNotice(text);
  const switchPage = (next: Page) => {
    if (draft) {
      setLeaveTarget(next);
      setDialog("discard");
      return;
    }
    setDraft(null);
    setQuery("");
    setPage(next);
    setSelection(null);
  };
  function newNote(
    quote = selection?.quote || "",
    anchor = selection?.anchor || currentRef,
  ) {
    setDraft({
      id: makeId(),
      title: "",
      nativeSelection:
        selection?.quote === quote ? selection.nativeSelection : undefined,
      endRef: selection?.quote === quote ? selection.endRef : undefined,
      anchor,
      quote,
      blocks: [{ id: makeId(), kind: "thought", text: "" }],
      pinned: false,
      updated: "Just now",
    });
    setSelection(null);
    setNoteModal(null);
    setPage("Notes");
  }
  async function saveNote() {
    if (!draft) return;
    if (!draft.blocks.some((b) => b.text.trim())) {
      showNotice("Add a thought or collected piece first.");
      return;
    }
    if (live) {
      try {
        const old = live.state.notes.find((n) => n.annotation_id === draft.id);
        const blocks: NativeBlock[] = draft.blocks
          .filter((b) => b.text.trim())
          .map((b) => b.native || { type: "thought", text: b.text });
        await run("note.save", {
          id: live.state.annotations.some((a) => a.id === draft.id)
            ? draft.id
            : null,
          start_ref: refId(live.state, draft.anchor),
          end_ref: draft.endRef || refId(live.state, draft.anchor),
          document: {
            title: draft.title,
            pinned: draft.pinned,
            blocks,
            selection:
              old?.document.selection ||
              draft.nativeSelection ||
              selectionFor(live.state, draft.anchor, draft.quote),
          },
        });
      } catch (e) {
        showNotice(String(e));
        return;
      }
    } else
      setNotes((list) => [draft, ...list.filter((n) => n.id !== draft.id)]);
    setDraft(null);
    showNotice("A thought worth returning to. Note saved.");
  }
  const updateBlock = (id: string, text: string) =>
    setDraft((d) =>
      d
        ? {
            ...d,
            blocks: d.blocks.map((b) =>
              b.id === id ? { ...b, text, native: undefined } : b,
            ),
          }
        : null,
    );
  const insertBlock = (block: Block) => {
    setDraft((d) =>
      d ? { ...d, blocks: [...d.blocks, { ...block, id: makeId() }] } : null,
    );
    setDialog(null);
  };
  const [occurrenceBook, setOccurrenceBook] = useState<number | null>(null);
  async function pageOccurrences(bookId: number | null, offset = 0) {
    if (!live || !wordData) return;
    try {
      const r = await run("study.occurrences", {
        code: wordData.code,
        book_id: bookId,
        offset,
        limit: 8,
      });
      const o = r.data.occurrences as {
        total: number;
        books: { book_id: number; label: string; verses: number }[];
        verses: { ref_id: number; label: string; preview: string | null }[];
        has_more: boolean;
        offset: number;
      };
      setOccurrenceBook(bookId);
      setLiveOccurrences({
        ...o,
        books: o.books.map((b) => ({
          id: b.book_id,
          name: b.label,
          verses: b.verses,
        })),
      });
    } catch (e) {
      showNotice(String(e));
    }
  }
  async function chooseDefinition(id: string) {
    const generation = ++definitionRequest.current;
    setDefinitionId(id); setDefinition(""); setDefinitionTitle("Loading dictionary…");
    try {
      const detail = await run("study.resource", {id});
      if (generation !== definitionRequest.current) return;
      const resource = detail.data.resource as {id: string; content: string; title: string; author: string | null; source: string};
      setDefinition(resource.content);
      setDefinitionTitle(resourceIdentity(resource.source, "dictionary").title);
      setLiveClips([{id: resource.id, kind: "word", text: resource.content, source: resource.source, ref: resource.title,
        native:{type:"resource",resource_id:resource.id,quote:resource.content,title:resource.title,author:resource.author,source:resource.source,content_id:detail.state.content_id}}]);
    } catch(e) {if (generation === definitionRequest.current) showNotice(String(e));}
  }
  async function openWord(code: string, quote: string, anchor = currentRef) {
    if (!live) return;
    try {
      if (anchor !== currentRef)
        await run("reading.select", { ref_id: refId(live.state, anchor) });
      setStudyRef(anchor);
      setWordQuote(quote);
      setPage("Study");
      setStudyMode("words");
      setStudyTab("meaning");
      setFlipped(false);
      definitionRequest.current++; setDefinitionTitle(""); setDefinitionId(""); setLiveClips([]);
      setDefinition("");
      const r = await run("study.word", { code });
      const data = r.data as unknown as NonNullable<typeof wordData>;
      setWordData(data);
      const initialDefinition = data.definitions.find(d => d.id.startsWith("resource:strongs-")) || data.definitions[0];
      if (initialDefinition) await chooseDefinition(initialDefinition.id);
      const o = await run("study.occurrences", {
        code,
        book_id: null,
        offset: 0,
        limit: 8,
      });
      const result = o.data.occurrences as {
        total: number;
        books: { book_id: number; label: string; verses: number }[];
        verses: { ref_id: number; label: string; preview: string | null }[];
        has_more: boolean;
        offset: number;
      };
      setLiveOccurrences({
        ...result,
        books: result.books.map((b) => ({
          id: b.book_id,
          name: b.label,
          verses: b.verses,
        })),
      });
    } catch (e) {
      showNotice(String(e));
    }
  }
  async function pinNote(id: string) {
    if (!live) return;
    const rich = live.state.notes.find((n) => n.annotation_id === id),
      a = live.state.annotations.find((a) => a.id === id);
    if (!a) return;
    try {
      await run("note.save", {
        id,
        start_ref: a.start_ref,
        end_ref: a.end_ref,
        document: rich
          ? { ...rich.document, pinned: !rich.document.pinned }
          : {
              title: "Note",
              pinned: true,
              selection: null,
              blocks: [{ type: "thought", text: a.content || "" }],
            },
      });
    } catch (e) {
      showNotice(String(e));
    }
  }
  function nativeInsert() {
    setDialog("insert");
  }
  async function applyHighlight(remove = false) {
    if (!live || !selection) return;
    try {
      const ref = refId(live.state, selection.anchor);
      const sel =
        selection.nativeSelection ||
        selectionFor(live.state, selection.anchor, selection.quote);
      const existing = live.state.highlights.find((h) =>
        h.selection?.spans.some((s) =>
          sel?.spans.some(
            (n) =>
              s.verse_id === n.verse_id &&
              s.start === n.start &&
              s.end === n.end,
          ),
        ),
      );
      if (remove) {
        if (existing)
          await run("annotation.delete", { id: existing.annotation_id });
      } else
        await run("highlight.save", {
          id: existing?.annotation_id || null,
          start_ref: ref,
          end_ref: selection.endRef || ref,
          style: storedStyle(markStyle),
          selection: sel,
        });
      setDialog(null);
      setSelection(null);
      window.getSelection()?.removeAllRanges();
    } catch (e) {
      showNotice(String(e));
    }
  }
  function renderVerse(text: string, ref: string) {
    if (live) {
      const v = live.state.passage.find((v) =>
        v.refs.includes(refId(live.state, ref)),
      );
      if (!v) return text;
      const chars = Array.from(text),
        links = [...v.word_links, ...v.strongs_links];
      const marks = live.state.highlights.flatMap((h) => {
        const annotation = live.state.annotations.find(
          (a) => a.id === h.annotation_id,
        );
        if (!annotation) return [];
        if (h.selection?.translation_id === live.state.session.translation)
          return h.selection.spans
            .filter((s) => s.verse_id === v.id)
            .map((span) => ({ ...span, style: nativeStyle(h.style) }));
        if (
          !h.selection &&
          v.refs.some(
            (r) => r >= annotation.start_ref && r <= annotation.end_ref,
          )
        )
          return [{ start: 0, end: chars.length, style: nativeStyle(h.style) }];
        return [];
      });
      const stops = [
        ...new Set([
          0,
          chars.length,
          ...links.flatMap((l) => [l.start, l.end]),
          ...marks.flatMap((m) => [m.start, m.end]),
        ]),
      ].sort((a, b) => a - b);
      return stops.slice(0, -1).map((start, i) => {
        const end = stops[i + 1],
          word = chars.slice(start, end).join(""),
          link = links.find((l) => l.start <= start && l.end >= end),
          mark = marks.find((m) => m.start <= start && m.end >= end);
        return (
          <span key={start} style={mark ? markCSS(mark.style) : undefined}>
            {link ? (
              <span
                className="study-word"
                role="button"
                tabIndex={0}
                aria-label={`Study ${word}`}
                onClick={() => {
                  if (!pressed.current && !window.getSelection()?.toString())
                    void openWord(link.strongs, word, ref);
                }}
                onKeyDown={(e) => {
                  if (e.key === "Enter") void openWord(link.strongs, word, ref);
                }}
              >
                {word}
              </span>
            ) : (
              word
            )}
          </span>
        );
      });
    }
    const quote = highlights[ref],
      at = quote ? text.indexOf(quote) : -1;
    const parts =
      at >= 0
        ? [
            { text: text.slice(0, at), mark: false },
            { text: quote, mark: true },
            { text: text.slice(at + quote.length), mark: false },
          ]
        : [{ text, mark: false }];
    return parts.map((part, i) => (
      <span
        key={i}
        style={
          part.mark ? markCSS(highlightStyles[ref] || defaultMark) : undefined
        }
      >
        {part.text.split(/(LORD)/).map((word, j) =>
          word === "LORD" ? (
            <span
              key={j}
              role="button"
              tabIndex={0}
              className="study-word"
              aria-label="Study LORD"
              onClick={() => {
                if (!pressed.current && !window.getSelection()?.toString()) {
                  setStudyTab("meaning");
                  setPage("Study");
                }
              }}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  setStudyTab("meaning");
                  setPage("Study");
                }
              }}
            >
              {word}
            </span>
          ) : (
            word
          ),
        )}
      </span>
    ));
  }
  function selectPassage(index: number) {
    if (live) {
      const dest = passages[index];
      void run("reading.navigate", {
        reference: `${books.find((b) => b.id === dest.book)!.name} ${dest.chapter}:1`,
      }).catch((e) => showNotice(String(e)));
    } else setPassageIndex(index);
    setPage("Read");
    setCover(null);
    setDialog(null);
    setSelection(null);
  }
  function captureSelection(e: React.MouseEvent<HTMLElement>, ref: string) {
    if (live) {
      const selected = readSelection(live.state);
      if (selected) setSelection(selected);
      return;
    }
    const sel = window.getSelection();
    const quote = sel?.toString().trim();
    if (
      quote &&
      e.currentTarget.contains(sel?.anchorNode || null) &&
      e.currentTarget.contains(sel?.focusNode || null)
    )
      setSelection({ quote, anchor: ref });
  }
  const attached = (ref: string) => notes.filter((n) => n.anchor === ref);
  function startHold(e: React.PointerEvent<HTMLElement>, ref: string) {
    if (e.button !== 0) return;
    pressed.current = false;
    holdOrigin.current = { x: e.clientX, y: e.clientY };
    const element = e.currentTarget;
    if (hold.current) clearTimeout(hold.current);
    hold.current = setTimeout(() => {
      pressed.current = true;
      const selected = window.getSelection()?.toString().trim();
      if (selected) {
        setSelection({ quote: selected, anchor: ref });
        return;
      }
      const target = element.querySelector("[data-verse-text]");
      const node = target
        ? document.createTreeWalker(target, NodeFilter.SHOW_TEXT).nextNode()
        : null;
      if (node) {
        const range = document.createRange();
        range.setStart(node, 0);
        range.setEnd(
          node,
          Math.min(
            node.textContent?.indexOf(" ") && node.textContent.indexOf(" ") > 0
              ? node.textContent.indexOf(" ")
              : node.textContent?.length || 1,
            node.textContent?.length || 1,
          ),
        );
        const sel = window.getSelection();
        sel?.removeAllRanges();
        sel?.addRange(range);
        setSelection({ quote: range.toString(), anchor: ref });
      }
    }, 550);
  }
  function endHold() {
    if (hold.current) clearTimeout(hold.current);
  }
  const closeDraft = () => {
    setLeaveTarget("Notes");
    setDialog("discard");
  };
  const visibleNotes = notes
    .filter((n) =>
      `${n.title} ${n.anchor} ${n.blocks.map((b) => b.text).join(" ")}`
        .toLowerCase()
        .includes(query.toLowerCase()),
    )
    .sort((a, b) => Number(b.pinned) - Number(a.pinned));
  function Clip({ block }: { block: Block }) {
    return (
      <Card
        className={`clipping ${block.kind === "word" ? "word-clipping" : ""}`}
      >
        <Card.Header>
          <div className="eyebrow flex items-center gap-2">
            {block.kind === "passage" ? (
              <BookOpen size={14} />
            ) : block.kind === "word" ? (
              <Type size={14} />
            ) : (
              <Quote size={14} />
            )}{" "}
            {block.kind === "passage"
              ? "Scripture"
              : block.kind === "word"
                ? "Word study"
                : "From the library"}
          </div>
          <Card.Title>{block.ref}</Card.Title>
        </Card.Header>
        <Card.Content>
          <p
            className={
              block.kind === "passage" ? "scripture excerpt" : "clip-text"
            }
          >
            {block.text}
          </p>
        </Card.Content>
        <Card.Footer>
          <Button
            variant="ghost"
            size="sm"
            className="source-button"
            onPress={() => {
              setSource(block);
              setDialog("source");
            }}
          >
            {block.source}
            <ExternalLink size={14} />
          </Button>
        </Card.Footer>
      </Card>
    );
  }
  function NoteContent({ note }: { note: Note }) {
    return (
      <div className="note-content">
        <div className="anchor-quote">
          <Quote size={16} />
          <span>{note.quote || note.anchor}</span>
        </div>
        {note.blocks.map((b) =>
          b.kind === "thought" ? (
            <p className="thought" key={b.id}>
              {b.text}
            </p>
          ) : (
            <Clip key={b.id} block={b} />
          ),
        )}
      </div>
    );
  }
  return (
    <>
      {!live && (
        <div className="preview-bar">
          <span>
            <span className="preview-dot" /> FONTES / NEW CHAPTER{" "}
            <span className="preview-meta">
              React + HeroUI 3 · Sample library
            </span>
          </span>
          <div className="flex items-center gap-2">
            <Button
              size="sm"
              variant="tertiary"
              onPress={() => setPhone(!phone)}
            >
              {phone ? "Widen view" : "Phone view"}
            </Button>
            <Button
              size="sm"
              variant="ghost"
              isIconOnly
              aria-label="Toggle dark mode"
              onPress={() => setDark(!dark)}
            >
              {dark ? <Sun size={16} /> : <Moon size={16} />}
            </Button>
          </div>
        </div>
      )}
      <div
        className={`stage ${live ? "native-stage" : phone ? "phone-stage" : ""}`}
      >
        <div
          className={`fontes-app ${live ? "native-app" : phone ? "compact" : ""}`}
        >
          <aside className="rail">
            <div className="brand">
              fontes<span>A little closer to the Word.</span>
            </div>
            <div className="rail-nav">
              {nav.map(({ page: next, icon: Icon }) => (
                <Button
                  key={next}
                  variant={page === next ? "secondary" : "ghost"}
                  onPress={() => switchPage(next)}
                >
                  <Icon size={20} />
                  {next}
                </Button>
              ))}
            </div>
            <Card className="rail-card">
              <Card.Content>
                <BookOpen size={22} />
                <p>
                  Make room
                  <br />
                  <span>for the words.</span>
                </p>
                <small>Your library. Your pace.</small>
              </Card.Content>
            </Card>
            <Button variant="ghost" onPress={() => setDialog("settings")}>
              <Settings2 size={18} />
              Appearance
            </Button>
          </aside>
          <div className="app-column">
            <header className="app-header">
              <Button
                variant="tertiary"
                size="sm"
                onPress={() => {
                  setPickerBook(null);
                  setPickerChapter(null);
                  setPickerQuery("");
                  setDialog("passages");
                }}
              >
                <BookOpen size={16} />
                {book.name} {p.chapter}
                <ChevronDown size={14} />
              </Button>
              <div className="header-end">
                <Button
                  isIconOnly
                  variant="ghost"
                  aria-label="Search library"
                  onPress={() => {
                    setQuery("");
                    setDialog("search");
                  }}
                >
                  <Search size={20} />
                </Button>
                <Button
                  isIconOnly
                  variant="ghost"
                  aria-label="Appearance"
                  onPress={() => setDialog("settings")}
                >
                  <Settings2 size={20} />
                </Button>
              </div>
            </header>
            <div className="app-scroll" ref={scrollRef}>
              {page === "Read" && (
                <section className="reader page-enter">
                  <div className="chapter-heading">
                    <div>
                      <div className="eyebrow">{p.section}</div>
                      <button
                        className="chapter-title"
                        onClick={() => setCover(book.id)}
                      >
                        {book.name}
                      </button>
                      <div className="chapter-meta">
                        Chapter {p.chapter}
                        <span>·</span>King James Version
                      </div>
                    </div>
                    <button
                      className="chapter-art"
                      aria-label={`About ${book.name}`}
                      onClick={() => setCover(book.id)}
                    >
                      <img src={book.art} alt="" />
                    </button>
                  </div>
                  <div className="reading-tools">
                    <Chip size="sm" variant="soft">
                      {live
                        ? live.state.translations.find(
                            (t) => t.id === live.state.session.translation,
                          )?.abbr
                        : "KJV"}
                    </Chip>
                    <Button
                      size="sm"
                      variant="ghost"
                      onPress={() => setCover(book.id)}
                    >
                      About this book
                      <ArrowRight size={14} />
                    </Button>
                    <Button
                      isIconOnly
                      size="sm"
                      variant={saved ? "secondary" : "ghost"}
                      aria-label={
                        saved ? "Remove bookmark" : "Bookmark chapter"
                      }
                      onPress={() => {
                        if (live)
                          void run("reference.favorite", {
                            ref_id: live.state.session.selected_ref,
                            enabled: !saved,
                          }).catch((e) => showNotice(String(e)));
                        else setSaved(!saved);
                      }}
                    >
                      <Bookmark
                        size={18}
                        fill={saved ? "currentColor" : "none"}
                      />
                    </Button>
                  </div>
                  <div
                    className="verse-list"
                    style={
                      live
                        ? ({
                            "--reader-size": `${live.state.session.preferences.text_size}px`,
                            "--reader-line":
                              live.state.session.preferences.line_spacing,
                          } as React.CSSProperties)
                        : undefined
                    }
                  >
                    {p.verses.map((verse, i) => {
                      const ref = live
                        ? label(live.state, live.state.passage[i].refs[0])
                        : `${book.name} ${p.chapter}:${i + 1}`;
                      const matches = attached(ref);
                      return (
                        <div className="verse-row" key={ref} data-verse-refs={live ? live.state.passage[i].refs.join(" ") : `${p.book * 1000000 + p.chapter * 1000 + i + 1}`}>
                          <button
                            className="verse-number"
                            aria-label={`Cross-references for ${ref}`}
                            onClick={() =>
                              setCrossRef({
                                id: live
                                  ? refId(live.state, ref)
                                  : p.book * 1000000 + p.chapter * 1000 + i + 1,
                                label: ref,
                              })
                            }
                          >
                            {live
                              ? live.state.passage[i].refs[0] % 1000
                              : i + 1}
                          </button>
                          <p
                            className="scripture verse"
                            data-reference={ref}
                            onPointerDown={(e) => {
                              if (!live) startHold(e, ref);
                            }}
                            onPointerMove={(e) => {
                              if (
                                Math.hypot(
                                  e.clientX - holdOrigin.current.x,
                                  e.clientY - holdOrigin.current.y,
                                ) > 8
                              )
                                endHold();
                            }}
                            onPointerUp={endHold}
                            onPointerCancel={endHold}
                            onMouseUp={(e) => captureSelection(e, ref)}
                            onContextMenu={(e) => {
                              endHold();
                              e.preventDefault();
                              setSelection({
                                quote:
                                  window.getSelection()?.toString().trim() ||
                                  verse,
                                anchor: ref,
                              });
                            }}
                          >
                            <span data-verse-text>
                              {renderVerse(verse, ref)}
                            </span>
                            {matches.length > 0 && (
                              <button
                                className="inline-marker"
                                aria-label={`Open ${matches.length} notes on ${ref}`}
                                onClick={() => setNoteModal(matches)}
                              >
                                <MessageSquare size={16} />
                                {matches.length > 1 && (
                                  <span>{matches.length}</span>
                                )}
                              </button>
                            )}
                          </p>
                        </div>
                      );
                    })}
                  </div>
                  <div className="chapter-end">
                    <span className="ornament">✧</span>
                    <p>Pause. Notice. Return.</p>
                    <div className="flex justify-center gap-2">
                      <Button
                        variant="tertiary"
                        onPress={() =>
                          live
                            ? void run("reading.chapter", { delta: -1 }).catch(
                                (e) => showNotice(String(e)),
                              )
                            : selectPassage(
                                (passageIndex + passages.length - 1) %
                                  passages.length,
                              )
                        }
                      >
                        <ChevronLeft size={16} />
                        Previous passage
                      </Button>
                      <Button
                        variant="tertiary"
                        onPress={() =>
                          live
                            ? void run("reading.chapter", { delta: 1 }).catch(
                                (e) => showNotice(String(e)),
                              )
                            : selectPassage(
                                (passageIndex + 1) % passages.length,
                              )
                        }
                      >
                        Next passage
                        <ChevronRight size={16} />
                      </Button>
                    </div>
                  </div>
                </section>
              )}
              {page === "Study" && (
                <section className="page-pad page-enter">
                  {live && <div className="study-sections" role="group" aria-label="Study resources">
                    {[ ["words", "Words"], ["commentary", "Commentary"], ["topic", "Topics"], ["dictionary", "Dictionaries"] ].map(([key, name]) =>
                      <Button key={key} variant={studyMode === key ? "primary" : "tertiary"} aria-pressed={studyMode === key} onPress={() => setStudyMode(key)}>{name}</Button>)}
                  </div>}
                  {(!live || studyMode === "words") && <>
                  <div className="page-heading">
                    <div>
                      <div className="eyebrow">CLOSER TO THE WORD</div>
                      <h1>Word study</h1>
                    </div>
                    <Chip variant="soft" color="accent">
                      {live ? wordData?.code || "Select a word" : "H3068"}
                    </Chip>
                  </div>
                  <p className="muted intro-line">
                    {live
                      ? `From “${wordQuote}” · ${studyRef || currentRef}`
                      : "From “LORD” · Proverbs 16:3"}
                  </p>
                  {live && (
                    <>
                      <details open={!wordData} className="study-word-chooser">
                        <summary>Words in {currentRef}</summary>
                        <div className="flex flex-wrap gap-2 mb-5">
                          {contextWords
                            .filter(
                              (w, i, a) =>
                                w.strongs &&
                                a.findIndex(
                                  (n) =>
                                    n.position === w.position &&
                                    n.strongs === w.strongs &&
                                    n.surface === w.surface,
                                ) === i,
                            )
                            .map((w, i) => (
                              <Button
                                key={i}
                                variant="tertiary"
                                size="sm"
                                onPress={() =>
                                  void openWord(w.strongs!, w.surface)
                                }
                              >
                                {w.surface}
                                <small>{w.strongs}</small>
                              </Button>
                            ))}
                          {contextWords.length === 0 && (
                            <p className="muted small">
                              No source-word analyses installed for this verse.
                              Select a dictionary-linked word in the passage or
                              browse a study resource.
                            </p>
                          )}
                        </div>
                      </details>
                    </>
                  )}
                  <Tabs
                    selectedKey={studyTab}
                    onSelectionChange={(k) => setStudyTab(String(k))}
                  >
                    <Tabs.ListContainer>
                      <Tabs.List aria-label="Word study">
                        <Tabs.Tab id="meaning">
                          Meaning
                          <Tabs.Indicator />
                        </Tabs.Tab>
                        <Tabs.Tab id="occurrences">
                          Occurrences
                          <Tabs.Indicator />
                        </Tabs.Tab>
                      </Tabs.List>
                    </Tabs.ListContainer>
                    <Tabs.Panel id="meaning">
                      {live && (wordData?.definitions.length || 0) > 1 && <div className="dictionary-choices">
                        <div className="eyebrow">Dictionary source</div>
                        <div className="flex flex-wrap gap-2">{wordData!.definitions.map(d => <Button key={d.id} size="sm" variant={definitionId === d.id ? "primary" : "tertiary"} aria-pressed={definitionId === d.id} onPress={() => void chooseDefinition(d.id)}>{d.title}</Button>)}</div>
                      </div>}

                      <button
                        className={`word-card ${flipped ? "flipped" : ""}`}
                        onClick={() => setFlipped(!flipped)}
                        aria-label={
                          flipped ? "Show Hebrew word" : "Show definition"
                        }
                      >
                        <span className="word-card-rotor">
                          <span
                            className="word-face word-front"
                            aria-hidden={flipped}
                          >
                            <span className="eyebrow">
                              {live
                                ? `${wordData?.code.startsWith("G") ? "GREEK" : "HEBREW"} HEADWORD`
                                : "HEBREW HEADWORD"}
                            </span>
                            <span className="hebrew" lang="he" dir="rtl">
                              {live
                                ? wordData?.headwords[0]?.text ||
                                  "Choose a linked word in the passage"
                                : "יְהֹוָה"}
                            </span>
                            <span className="transliteration">
                              {live
                                ? wordData?.headwords[0]?.transliteration
                                : "Yehôvâh"}
                            </span>
                            <span className="word-card-footer">
                              <span>
                                {live
                                  ? `Reading selection: “${wordQuote}”`
                                  : "Reading selection: “LORD”"}
                              </span>
                              <span>↻ Meaning</span>
                            </span>
                          </span>
                          <span
                            className="word-face word-back"
                            aria-hidden={!flipped}
                          >
                            <span className="eyebrow">
                              {live
                                ? `${definitionTitle || "Dictionary"} · ${wordData?.code || ""}`
                                : "STRONG’S HEBREW DICTIONARY · H3068"}
                            </span>
                            <span className="definition-title">
                              {live ? (
                                definition.slice(0, 200) ||
                                "No installed definition for this selection."
                              ) : (
                                <>
                                  The self-Existent
                                  <br />
                                  or Eternal
                                </>
                              )}
                            </span>
                            <span className="word-card-footer">
                              <span>
                                {live
                                  ? definition.length > 200
                                    ? "Full entry below"
                                    : ""
                                  : "Jehovah, Jewish national name of God."}
                              </span>
                              <span>↶ Word</span>
                            </span>
                          </span>
                        </span>
                      </button>
                      {live && definition.length > 200 && (
                        <Card className="mt-5">
                          <Card.Header>
                            <Card.Title>{definitionTitle || "Full dictionary entry"}</Card.Title>
                          </Card.Header>
                          <Card.Content>
                            <p className="resource-body">{definition}</p>
                          </Card.Content>
                        </Card>
                      )}
                      <div className="section-row">
                        <h2>Keep exploring</h2>
                        <BookOpenText size={16} />
                      </div>
                      <Card>
                        <Card.Header>
                          <Card.Title>Bring the meaning with you.</Card.Title>
                          <Card.Description>
                            Keep this definition beside your own observations.
                          </Card.Description>
                        </Card.Header>
                        <Card.Footer>
                          <Button
                            variant="secondary"
                            onPress={() => {
                              if (live) {
                                newNote(wordQuote, currentRef);
                                setDraft((d) =>
                                  d
                                    ? {
                                        ...d,
                                        blocks: [...d.blocks, ...liveClips],
                                      }
                                    : null,
                                );
                                return;
                              }
                              newNote("LORD", "Proverbs 16:3");
                              setDraft((d) =>
                                d
                                  ? {
                                      ...d,
                                      blocks: [
                                        ...d.blocks,
                                        { ...samples[3], id: makeId() },
                                      ],
                                    }
                                  : null,
                              );
                            }}
                          >
                            <Plus size={16} />
                            Collect in a note
                          </Button>
                        </Card.Footer>
                      </Card>
                    </Tabs.Panel>
                    <Tabs.Panel id="occurrences">
                      <Card className="mt-5">
                        <Card.Header>
                          <Card.Title>A word across the books</Card.Title>
                          <Card.Description>
                            {occurrences.total.toLocaleString()} distinct verses
                            · {live ? "Installed library" : "Source snapshot"}
                          </Card.Description>
                        </Card.Header>
                        <Card.Content>
                          <div
                            className="occurrence-chart"
                            role="img"
                            aria-label="Distinct verses by book: top five books. Exact counts listed below."
                          >
                            <ResponsiveContainer width="100%" height={220}>
                              <BarChart
                                data={occurrences.books.slice(0, 5)}
                                layout="vertical"
                                margin={{ left: 0, right: 24 }}
                              >
                                <XAxis type="number" hide />
                                <YAxis
                                  type="category"
                                  dataKey="name"
                                  width={100}
                                  tick={{
                                    fill: "var(--muted-ink)",
                                    fontSize: 12,
                                  }}
                                  axisLine={false}
                                  tickLine={false}
                                />
                                <Tooltip
                                  cursor={false}
                                  contentStyle={{
                                    background: "var(--paper)",
                                    color: "var(--ink)",
                                    borderRadius: 12,
                                  }}
                                />
                                <Bar
                                  dataKey="verses"
                                  name="Distinct verses"
                                  fill="var(--accent)"
                                  radius={[0, 6, 6, 0]}
                                  isAnimationActive={false}
                                />
                              </BarChart>
                            </ResponsiveContainer>
                          </div>
                          <p className="muted small">
                            Top five books · distinct verses
                          </p>
                          {occurrences.books.slice(0, 5).map((b) => (
                            <div className="occurrence-book" key={b.id}>
                              {live ? (
                                <Button
                                  variant="ghost"
                                  size="sm"
                                  onPress={() => void pageOccurrences(b.id)}
                                >
                                  {b.name}
                                  <ChevronRight size={14} />
                                </Button>
                              ) : (
                                <span>{b.name}</span>
                              )}
                              <Chip size="sm" variant="soft">
                                {b.verses.toLocaleString()} verses
                              </Chip>
                            </div>
                          ))}
                        </Card.Content>
                      </Card>
                      {live &&
                        liveOccurrences?.verses.map((v) => (
                          <Card key={v.ref_id} className="mt-3">
                            <Card.Content>
                              <Button
                                variant="ghost"
                                onPress={() => {
                                  void run("reading.select", {
                                    ref_id: v.ref_id,
                                  })
                                    .then(() => setPage("Read"))
                                    .catch((e) => showNotice(String(e)));
                                }}
                              >
                                {v.label}
                                <ArrowRight size={14} />
                              </Button>
                              <p>{v.preview}</p>
                            </Card.Content>
                          </Card>
                        ))}
                      {live && liveOccurrences && (
                        <div className="flex flex-wrap gap-2 mt-4">
                          <Button
                            variant="ghost"
                            isDisabled={occurrenceBook === null}
                            onPress={() => void pageOccurrences(null)}
                          >
                            All books
                          </Button>
                          <Button
                            isDisabled={!liveOccurrences.offset}
                            onPress={() =>
                              void pageOccurrences(
                                occurrenceBook,
                                Math.max(0, liveOccurrences.offset - 8),
                              )
                            }
                          >
                            Previous
                          </Button>
                          <Button
                            isDisabled={!liveOccurrences.has_more}
                            onPress={() =>
                              void pageOccurrences(
                                occurrenceBook,
                                liveOccurrences.offset + 8,
                              )
                            }
                          >
                            Next
                          </Button>
                        </div>
                      )}
                      <p className="muted small mt-4">
                        {live
                          ? "Distinct verses in the installed source text."
                          : "Snapshot counts from the preview Study library. Live queries will use your installed library."}
                      </p>
                    </Tabs.Panel>
                  </Tabs>
                  </>}
                  {live && studyMode !== "words" && <>
                    <div className="eyebrow">{studyMode === "topic" ? "NAVE’S TOPICAL BIBLE" : studyMode === "dictionary" ? "INSTALLED DICTIONARIES" : "INSTALLED COMMENTARIES"}</div>
                    <h1>{studyMode === "topic" ? "Topics" : studyMode === "dictionary" ? "Dictionaries" : "Commentary"}</h1>
                    <p className="muted intro-line">Related to {currentRef}</p>
                    {contextLoading ? <p role="status">Finding related articles…</p> : (() => {
                      const matches = contextResources.filter((r, i, all) => r.kind === studyMode && all.findIndex(a => a.id === r.id && a.kind === studyMode) === i);
                      return matches.length ? matches.map(r => <Card key={r.id} className="mt-4">
                        <Card.Header><div className="eyebrow">{r.author || r.kind}</div><Card.Title>{r.title}</Card.Title></Card.Header>
                        <Card.Content><p>{r.excerpt}</p><p className="muted small mt-4">{r.basis === "citation" ? "Mentions this passage" : r.basis === "applies_to" ? "On this passage" : "Related to this passage"}</p></Card.Content>
                        <Card.Footer><Button variant="secondary" onPress={() => void openArticle(r.id)}>Read article<ArrowRight size={14}/></Button></Card.Footer>
                      </Card>) : <p className="muted">No linked {studyMode === "topic" ? "topics" : studyMode === "dictionary" ? "dictionary articles" : "commentary"} for this verse. Browse the full resource in Library.</p>;
                    })()}
                    {contextResources.length === 100 && <p className="muted small mt-4">Showing the first 100 related articles. More may be available in Library.</p>}
                  </>}

                </section>
              )}
              {page === "Notes" &&
                (draft ? (
                  <section className="page-pad page-enter">
                    <div className="editor-top">
                      <Button variant="ghost" size="sm" onPress={closeDraft}>
                        <ArrowLeft size={16} />
                        Notebook
                      </Button>
                      <Button
                        isIconOnly
                        variant={draft.pinned ? "secondary" : "ghost"}
                        aria-label="Pin note"
                        onPress={() =>
                          setDraft({ ...draft, pinned: !draft.pinned })
                        }
                      >
                        <Pin size={18} />
                      </Button>
                    </div>
                    <div className="eyebrow mt-5">A NOTE ON {draft.anchor}</div>
                    <Input
                      aria-label="Note title"
                      className="note-title-input w-full"
                      placeholder="Give this thought a title"
                      value={draft.title}
                      onChange={(e) =>
                        setDraft({ ...draft, title: e.target.value })
                      }
                    />
                    <div className="anchor-quote">
                      <Quote size={16} />
                      <span>{draft.quote || draft.anchor}</span>
                    </div>
                    <div className="editor-blocks">
                      {draft.blocks.map((b, i) => (
                        <div key={b.id} className="editor-piece">
                          <div className="piece-toolbar">
                            <span>
                              {b.kind === "thought"
                                ? "Your thought"
                                : "Collected piece"}
                            </span>
                            <Button
                              isIconOnly
                              variant="ghost"
                              size="sm"
                              aria-label={`Move piece ${i + 1} up`}
                              isDisabled={i === 0}
                              onPress={() => {
                                const copy = [...draft.blocks];
                                [copy[i - 1], copy[i]] = [copy[i], copy[i - 1]];
                                setDraft({ ...draft, blocks: copy });
                              }}
                            >
                              <ArrowUp size={14} />
                            </Button>
                            <Button
                              isIconOnly
                              variant="ghost"
                              size="sm"
                              aria-label={`Move piece ${i + 1} down`}
                              isDisabled={i === draft.blocks.length - 1}
                              onPress={() => {
                                const copy = [...draft.blocks];
                                [copy[i + 1], copy[i]] = [copy[i], copy[i + 1]];
                                setDraft({ ...draft, blocks: copy });
                              }}
                            >
                              <ArrowDown size={14} />
                            </Button>
                            <Button
                              isIconOnly
                              variant="ghost"
                              size="sm"
                              aria-label={`Remove piece ${i + 1}`}
                              onPress={() =>
                                setDraft({
                                  ...draft,
                                  blocks: draft.blocks.filter(
                                    (x) => x.id !== b.id,
                                  ),
                                })
                              }
                            >
                              <Trash2 size={14} />
                            </Button>
                          </div>
                          {b.kind === "thought" ? (
                            <TextArea
                              aria-label={`Thought ${i + 1}`}
                              placeholder="What are you noticing?"
                              value={b.text}
                              onChange={(e) =>
                                updateBlock(b.id, e.target.value)
                              }
                              rows={4}
                            />
                          ) : (
                            <Clip block={b} />
                          )}
                        </div>
                      ))}
                    </div>
                    <p className="muted small text-center">
                      Your thoughts. Your sources. Kept together.
                    </p>
                  </section>
                ) : (
                  <section className="page-pad page-enter">
                    <div className="page-heading">
                      <div>
                        <div className="eyebrow">A PLACE TO RETURN</div>
                        <h1>Your notebook</h1>
                      </div>
                      <Chip size="sm" variant="soft">
                        {notes.length}
                      </Chip>
                    </div>
                    <p className="muted intro-line">
                      Thoughts to keep. Passages to return to.
                    </p>
                    <Input
                      aria-label="Search notes"
                      placeholder="Find a thought or passage…"
                      value={query}
                      onChange={(e) => setQuery(e.target.value)}
                      className="mb-5 w-full"
                    />
                    <Tabs
                      selectedKey={filter}
                      onSelectionChange={(k) => setFilter(String(k))}
                    >
                      <Tabs.ListContainer>
                        <Tabs.List aria-label="Notebook filters">
                          {["notes", "highlights", "bookmarks"].map((key) => (
                            <Tabs.Tab key={key} id={key}>
                              {key[0].toUpperCase() + key.slice(1)}
                              <Tabs.Indicator />
                            </Tabs.Tab>
                          ))}
                        </Tabs.List>
                      </Tabs.ListContainer>
                      <Tabs.Panel id="notes">
                        <div className="section-row">
                          <span className="muted small">
                            {visibleNotes.length} {visibleNotes.length === 1 ? "note" : "notes"}
                          </span>
                          <span className="muted small">Pinned & recent</span>
                        </div>
                        {visibleNotes.length ? (
                          visibleNotes.map((n) => (
                            <Card key={n.id} className="note-preview">
                              <Card.Header>
                                <div className="card-meta">
                                  <span>{n.anchor}</span>
                                  {n.pinned ? (
                                    <Pin size={14} />
                                  ) : (
                                    <time title={n.updated}>{noteDate(n.updated)}</time>
                                  )}
                                </div>
                                <Card.Title>
                                  <button onClick={() => setNoteModal([n])}>
                                    {n.title || "An observation"}
                                  </button>
                                </Card.Title>
                              </Card.Header>
                              <Card.Content>
                                <button
                                  className="note-open"
                                  onClick={() => setNoteModal([n])}
                                >
                                  <p>
                                    {n.blocks.find((b) => b.kind === "thought")
                                      ?.text || n.quote}
                                  </p>
                                  <NoteContext quote={n.quote} anchor={n.anchor} live={live} />
                                </button>
                              </Card.Content>
                              <Card.Footer>
                                <span className="muted small">
                                  {n.blocks.filter((b) => b.kind !== "thought")
                                    .length > 0
                                    ? `${n.blocks.filter((b) => b.kind !== "thought").length} collected pieces`
                                    : "Personal note"}
                                </span>
                                <Button
                                  isIconOnly
                                  variant="ghost"
                                  size="sm"
                                  aria-label={`More actions for ${n.title}`}
                                  onPress={() => {
                                    setDeleteId(n.id);
                                    setDialog("delete");
                                  }}
                                >
                                  <MoreHorizontal size={18} />
                                </Button>
                              </Card.Footer>
                            </Card>
                          ))
                        ) : (
                          <div className="empty-state">
                            <NotebookPen size={36} />
                            <h2>Let a thought take root.</h2>
                            <p>
                              Keep a question, a prayer, or something you
                              noticed.
                            </p>
                            <Button onPress={() => newNote()}>
                              Write your first note
                            </Button>
                          </div>
                        )}
                      </Tabs.Panel>
                      <Tabs.Panel id="highlights">
                        <div className="pt-5 space-y-3">
                          {live ? (
                            live.state.annotations
                              .filter((a) => a.kind === "highlight")
                              .map((a) => {
                                const h = live.state.highlights.find(
                                  (h) => h.annotation_id === a.id,
                                );
                                return (
                                  <Card key={a.id}>
                                    <Card.Header>
                                      <Card.Title>
                                        {label(live.state, a.start_ref)}
                                      </Card.Title>
                                    </Card.Header>
                                    <Card.Content>
                                      <span
                                        style={markCSS(
                                          h
                                            ? nativeStyle(h.style)
                                            : defaultMark,
                                        )}
                                      >
                                        {h?.selection?.spans
                                          .map((s) => s.quote)
                                          .join(" ") || "Whole verse highlight"}
                                      </span>
                                    </Card.Content>
                                    <Card.Footer>
                                      <Button
                                        variant="ghost"
                                        onPress={() =>
                                          void run("annotation.delete", {
                                            id: a.id,
                                          }).catch((e) => showNotice(String(e)))
                                        }
                                      >
                                        Remove highlight
                                      </Button>
                                    </Card.Footer>
                                  </Card>
                                );
                              })
                          ) : Object.entries(highlights).length ? (
                            Object.entries(highlights).map(([ref, quote]) => (
                              <Card key={ref}>
                                <Card.Header>
                                  <Card.Title>{ref}</Card.Title>
                                </Card.Header>
                                <Card.Content>
                                  <span
                                    style={markCSS(
                                      highlightStyles[ref] || defaultMark,
                                    )}
                                  >
                                    {quote}
                                  </span>
                                </Card.Content>
                                <Card.Footer>
                                  <Button
                                    variant="ghost"
                                    onPress={() =>
                                      setHighlights((h) => {
                                        const copy = { ...h };
                                        delete copy[ref];
                                        return copy;
                                      })
                                    }
                                  >
                                    Remove highlight
                                  </Button>
                                </Card.Footer>
                              </Card>
                            ))
                          ) : (
                            <div className="empty-state">
                              <Layers size={32} />
                              <h2>Keep what stands out.</h2>
                              <p>
                                Select words while reading to highlight them.
                              </p>
                              <Button
                                variant="secondary"
                                onPress={() => switchPage("Read")}
                              >
                                Back to reading
                              </Button>
                            </div>
                          )}
                        </div>
                      </Tabs.Panel>
                      <Tabs.Panel id="bookmarks">
                        <div className="pt-5">
                          {saved ? (
                            <Card>
                              <Card.Header>
                                <Card.Title>
                                  {book.name} {p.chapter}
                                </Card.Title>
                                <Card.Description>
                                  A passage to return to.
                                </Card.Description>
                              </Card.Header>
                              <Card.Footer>
                                <Button
                                  variant="secondary"
                                  onPress={() => switchPage("Read")}
                                >
                                  Read passage
                                  <ArrowRight size={16} />
                                </Button>
                              </Card.Footer>
                            </Card>
                          ) : (
                            <div className="empty-state">
                              <Bookmark size={32} />
                              <h2>A place to return.</h2>
                              <p>Bookmark a chapter from the reader.</p>
                            </div>
                          )}
                        </div>
                      </Tabs.Panel>
                    </Tabs>
                  </section>
                ))}
              {page === "Library" && (
                <section className="page-pad page-enter">
                  {libraryResource && (
                    <Button
                      variant="ghost"
                      size="sm"
                      onPress={() => {
                        setLibraryResource(null);
                        setQuery("");
                      }}
                    >
                      <ArrowLeft size={16} /> All resources
                    </Button>
                  )}
                  <div className="page-heading">
                    <div>
                      <div className="eyebrow">
                        {libraryResource ? "CONTENTS" : "YOUR READING SHELF"}
                      </div>
                      <h1>
                        {live && libraryResource
                          ? live.state.translations.find(
                              (t) => t.id === libraryResource,
                            )?.name
                          : libraryResource === "kjv"
                            ? "King James Version"
                            : libraryResource === "henry"
                              ? "Matthew Henry"
                              : "The library"}
                      </h1>
                    </div>
                  </div>
                  <p className="muted intro-line">
                    {libraryResource === "kjv"
                      ? "Choose a book of the Bible."
                      : libraryResource === "henry"
                        ? "Introductions to the books of the Bible."
                        : "Bibles and study resources, each in its own place."}
                  </p>
                  <Input
                    fullWidth
                    aria-label={
                      libraryResource ? "Find a Bible book" : "Find a resource"
                    }
                    placeholder={
                      libraryResource
                        ? "Find a Bible book…"
                        : "Find a resource…"
                    }
                    value={query}
                    onChange={(e) => setQuery(e.target.value)}
                  />
                  {live ? (
                    parentSource ? (
                      <>
                        <Button
                          variant="ghost"
                          onPress={() => {
                            setParentSource(null);
                            setResourcePage(null);
                          }}
                        >
                          {" "}
                          <ArrowLeft size={16} /> All resources
                        </Button>
                        <h2>{resourceIdentity(parentSource, "resource").title}</h2>
                        {resourcePage?.resources.map((r) => (
                          <Card className="mt-3" key={r.id}>
                            <Card.Header>
                              <Card.Title>{r.title}</Card.Title>
                            </Card.Header>
                            <Card.Content>
                              <p>{r.excerpt}</p>
                            </Card.Content>
                            <Card.Footer>
                              <Button
                                variant="secondary"
                                onPress={() => void openArticle(r.id)}
                              >
                                Read entry
                                <ArrowRight size={14} />
                              </Button>
                            </Card.Footer>
                          </Card>
                        ))}
                        <div className="flex gap-2 mt-4">
                          <Button
                            isDisabled={!resourcePage?.offset}
                            onPress={() =>
                              void openSource(
                                parentSource,
                                Math.max(0, (resourcePage?.offset || 0) - 50),
                              )
                            }
                          >
                            Previous
                          </Button>
                          <Button
                            isDisabled={!resourcePage?.has_more}
                            onPress={() =>
                              void openSource(
                                parentSource,
                                (resourcePage?.offset || 0) + 50,
                              )
                            }
                          >
                            Next
                          </Button>
                        </div>
                      </>
                    ) : !libraryResource ? (
                      <div className="resource-shelf">
                        {live.state.translations
                          .filter((t) =>
                            `${t.name} ${t.abbr}`
                              .toLowerCase()
                              .includes(query.toLowerCase()),
                          )
                          .map((t) => (
                            <LibraryResourceCard key={t.id} title={t.name} type={`Bible · ${t.abbr}`} monogram={t.abbr} cover={t.abbr.toLowerCase() === "web" ? "web" : "default"}
                              description={`${live.state.books.length} books · Installed`}
                              onOpen={() => {
                                void run("reading.translation", {id: t.id}).then(() => {setLibraryResource(t.id); setQuery("");}).catch(e => showNotice(String(e)));
                              }} />
                          ))}
                        {catalog
                          .filter(
                            (c, i, a) =>
                              a.findIndex((n) => n.id === c.id) === i &&
                              `${c.id} ${c.author} ${resourceIdentity(c.id, c.kind, c.author).title} ${resourceIdentity(c.id, c.kind).type}`
                                .toLowerCase()
                                .includes(query.toLowerCase()),
                          )
                          .map((c) => (
                            <LibraryResourceCard key={c.id} {...resourceIdentity(c.id, c.kind, c.author)}
                              description={`${c.count.toLocaleString()} entries · Installed`}
                              onOpen={() => void openSource(c.id)} />
                          ))}
                      </div>
                    ) : (
                      <div className="book-grid">
                        {books
                          .filter((b) =>
                            b.name.toLowerCase().includes(query.toLowerCase()),
                          )
                          .map((b) => (
                            <button
                              className="book-tile"
                              key={b.id}
                              onClick={() => setCover(b.id)}
                            >
                              <img src={b.art} alt="" />
                              <strong>{b.name}</strong>
                              <span>{b.title}</span>
                            </button>
                          ))}
                      </div>
                    )
                  ) : (
                    <>
                      {!libraryResource ? (
                        <div className="resource-shelf">
                          {[
                            {
                              id: "kjv",
                              title: "King James Version",
                              type: "BIBLE · KJV",
                              description: "66 books · Sample passages",
                              monogram: "KJV",
                            },
                            {
                              id: "web",
                              title: "World English Bible",
                              type: "BIBLE · WEB",
                              description: "Not included in this preview",
                              monogram: "WEB",
                            },
                            {
                              id: "henry",
                              title: "Matthew Henry",
                              type: "COMMENTARY",
                              description: "66 book introductions",
                              monogram: "MH",
                            },
                            {
                              id: "strongs",
                              title: "Strong’s Dictionary",
                              type: "DICTIONARY",
                              description: "Sample Hebrew entry · H3068",
                              monogram: "א",
                            },
                          ]
                            .filter((r) =>
                              `${r.title} ${r.type}`
                                .toLowerCase()
                                .includes(query.toLowerCase()),
                            )
                            .map((r) => (
                              <LibraryResourceCard key={r.id} title={r.title} type={r.type} description={r.description} monogram={r.monogram} cover={r.id}
                                disabled={r.id === "web"} onOpen={() => {
                                  setQuery("");
                                  if (r.id === "kjv" || r.id === "henry") setLibraryResource(r.id);
                                  else {setStudyTab("meaning"); switchPage("Study");}
                                }} />
                            ))}
                        </div>
                      ) : (
                        <>
                          <div className="book-grid">
                            {books
                              .filter((b) =>
                                b.name
                                  .toLowerCase()
                                  .includes(query.toLowerCase()),
                              )
                              .map((b) => (
                                <button
                                  key={b.id}
                                  className="book-tile"
                                  onClick={() => setCover(b.id)}
                                >
                                  <img
                                    src={b.art}
                                    alt={`${b.name} symbolic cover`}
                                  />
                                  <strong>{b.name}</strong>
                                  <span>{b.title}</span>
                                </button>
                              ))}
                          </div>
                        </>
                      )}
                    </>
                  )}
                </section>
              )}
            </div>
            {draft && page === "Notes" && (
              <div className="editor-dock">
                <Button
                  variant="secondary"
                  onPress={() =>
                    live ? void nativeInsert() : setDialog("insert")
                  }
                >
                  <Plus size={18} />
                  Add to note
                </Button>
                <Button onPress={saveNote}>
                  <Check size={16} />
                  Save note
                </Button>
              </div>
            )}
            {page === "Notes" && !draft && notes.length > 0 && !noteModal && (
              <Button
                isIconOnly
                className="new-note"
                aria-label="New note"
                onPress={() => newNote()}
              >
                <Plus size={24} />
              </Button>
            )}
            {selection && page === "Read" && (
              <div
                className="selection-menu"
                role="region"
                aria-label="Selection actions"
              >
                <div className="selection-caption">
                  <div>
                    <small>{selection.anchor}</small>
                    <p>“{selection.quote}”</p>
                  </div>
                  <Button
                    isIconOnly
                    variant="ghost"
                    aria-label="Dismiss selection"
                    onPress={() => {
                      setSelection(null);
                      window.getSelection()?.removeAllRanges();
                    }}
                  >
                    <X size={18} />
                  </Button>
                </div>
                <div className="selection-actions">
                  <Button
                    variant="secondary"
                    fullWidth
                    onPress={() => newNote()}
                  >
                    <NotebookPen size={18} /> Note
                  </Button>
                  <Button
                    fullWidth
                    onPress={() => {
                      setMarkStyle(
                        highlightStyles[selection.anchor] || defaultMark,
                      );
                      setDialog("highlight");
                    }}
                  >
                    <Layers size={18} /> Highlight <ChevronRight size={14} />
                  </Button>
                </div>
              </div>
            )}
            <nav className="bottom-nav" aria-label="Main navigation">
              {nav.map(({ page: next, icon: Icon }) => (
                <Button
                  key={next}
                  variant={page === next ? "secondary" : "ghost"}
                  className={page === next ? "active" : ""}
                  onPress={() => {
                    setQuery("");
                    switchPage(next);
                  }}
                >
                  <Icon size={21} />
                  <span>{next}</span>
                </Button>
              ))}
            </nav>
            <div id="overlay-root" />
          </div>
          <aside className="reading-aside">
            <div className="eyebrow">ALONGSIDE YOUR READING</div>
            <img src={book.art} alt="" />
            <h2>{book.title}</h2>
            <p>{book.paragraphs[0].slice(0, 200)}…</p>
            <Button variant="secondary" onPress={() => setCover(book.id)}>
              Meet the book
              <ArrowRight size={16} />
            </Button>
            <hr />
            <div className="eyebrow">FROM YOUR NOTEBOOK</div>
            <h3>The work & the weight</h3>
            <p>
              “Do the work faithfully. Release the need to carry the outcome.”
            </p>
            <Button variant="ghost" onPress={() => switchPage("Notes")}>
              Open notebook
              <ArrowRight size={16} />
            </Button>
          </aside>
        </div>
      </div>
      <CrossReferences
        reference={crossRef}
        onClose={() => setCrossRef(null)}
        live={live}
      />
      <Sheet
        title="Highlight style"
        open={dialog === "highlight"}
        onClose={() => setDialog(null)}
        footer={
          <div className="highlight-footer">
            <Button
              variant="ghost"
              onPress={() => {
                if (live) {
                  void applyHighlight(true);
                  return;
                }
                if (selection)
                  setHighlights((h) => {
                    const copy = { ...h };
                    delete copy[selection.anchor];
                    return copy;
                  });
                setDialog(null);
                setSelection(null);
                window.getSelection()?.removeAllRanges();
              }}
            >
              Remove highlight
            </Button>
            <Button
              onPress={() => {
                if (live) {
                  void applyHighlight();
                  return;
                }
                if (selection) {
                  setHighlights((h) => ({
                    ...h,
                    [selection.anchor]: selection.quote,
                  }));
                  setHighlightStyles((h) => ({
                    ...h,
                    [selection.anchor]: markStyle,
                  }));
                }
                setDialog(null);
                setSelection(null);
                window.getSelection()?.removeAllRanges();
                showNotice("Highlight applied.");
              }}
            >
              <Check size={16} /> Apply
            </Button>
          </div>
        }
      >
        <div className="highlight-preview">
          <small>
            {selection?.anchor} · {markStyle.symbol || markStyle.kind}
          </small>
          <p>
            <span style={markCSS(markStyle)}>
              {selection?.quote || "Commit thy works unto the LORD"}
            </span>
          </p>
        </div>
        <div className="eyebrow mb-3">COLOR</div>
        <div className="color-choices">
          {markColors.map(([name, color]) => (
            <Button
              key={name}
              isIconOnly
              aria-label={name}
              aria-pressed={markStyle.color === color}
              variant="ghost"
              onPress={() => setMarkStyle({ ...markStyle, color })}
            >
              <span style={{ background: color }}>
                {markStyle.color === color && <Check size={16} />}
              </span>
            </Button>
          ))}
        </div>
        <Tabs aria-label="Highlight category" defaultSelectedKey="markup">
          <Tabs.ListContainer>
            <Tabs.List>
              <Tabs.Tab id="markup">
                Markup
                <Tabs.Indicator />
              </Tabs.Tab>
              <Tabs.Tab id="symbols">
                Study symbols
                <Tabs.Indicator />
              </Tabs.Tab>
            </Tabs.List>
          </Tabs.ListContainer>
          <Tabs.Panel id="markup">
            <div className="style-choices">
              {markKinds.map((kind) => (
                <Button
                  key={kind}
                  aria-pressed={markStyle.kind === kind}
                  variant={markStyle.kind === kind ? "secondary" : "ghost"}
                  onPress={() =>
                    setMarkStyle({ ...markStyle, kind, symbol: undefined })
                  }
                >
                  <span style={markCSS({ ...markStyle, kind })}>Ab</span>
                  <span>{kind}</span>
                  {markStyle.kind === kind && <Check size={16} />}
                </Button>
              ))}
            </div>
          </Tabs.Panel>
          <Tabs.Panel id="symbols">
            <div className="style-choices">
              {studySymbols.map((symbol) => (
                <Button
                  key={symbol}
                  aria-pressed={
                    markStyle.kind === "Study symbol" &&
                    markStyle.symbol === symbol
                  }
                  variant={
                    markStyle.kind === "Study symbol" &&
                    markStyle.symbol === symbol
                      ? "secondary"
                      : "ghost"
                  }
                  onPress={() =>
                    setMarkStyle({ ...markStyle, kind: "Study symbol", symbol })
                  }
                >
                  <span
                    style={markCSS({
                      ...markStyle,
                      kind: "Study symbol",
                      symbol,
                    })}
                  >
                    Ab
                  </span>
                  <span>{symbol}</span>
                  {markStyle.kind === "Study symbol" &&
                    markStyle.symbol === symbol && <Check size={16} />}
                </Button>
              ))}
            </div>
          </Tabs.Panel>
        </Tabs>
      </Sheet>
      <Sheet
        title="Sources & licenses"
        open={dialog === "licenses"}
        onClose={() => setDialog(null)}
      >
        <p className="muted mb-4">
          Book introductions retain Matthew Henry / CCEL attribution in About
          this book. Installed resources retain their source and rights
          identifiers.
        </p>
        <pre className="license-notices">{notices}</pre>
      </Sheet>
      <Sheet
        title="Choose a passage"
        open={dialog === "passages"}
        onClose={() => setDialog(null)}
      >
        {pickerBook && pickerChapter ? (
          <div className="passage-picker">
            <Button variant="ghost" onPress={() => setPickerChapter(null)}><ArrowLeft size={16} /> Chapters</Button>
            <h3>{books.find(b => b.id === pickerBook)?.name} {pickerChapter}</h3>
            <p className="muted small">Choose a verse</p>
            <div className="chapter-grid">
              {Array.from({ length: live
                ? live.state.books.find(b => b.id === pickerBook)?.verse_counts[pickerChapter - 1] || 0
                : passages.find(p => p.book === pickerBook && p.chapter === pickerChapter)?.verses.length || 0 }, (_, i) => (
                <Button key={i} aria-label={`Verse ${i + 1}`} variant="tertiary" onPress={() => {
                  const destination = pickerBook * 1000000 + pickerChapter * 1000 + i + 1;
                  const finish = () => {
                    setPage("Read"); setCover(null); setSelection(null); setDialog(null);
                    setArrival(destination);
                  };
                  if (live) void run("reading.navigate", {
                    reference: `${books.find(b => b.id === pickerBook)?.name} ${pickerChapter}:${i + 1}`,
                  }).then(finish).catch(e => showNotice(String(e)));
                  else {
                    setPassageIndex(passages.findIndex(p => p.book === pickerBook && p.chapter === pickerChapter));
                    finish();
                  }
                }}>{i + 1}</Button>
              ))}
            </div>
          </div>
        ) : pickerBook ? (
          <div className="passage-picker">
            <Button variant="ghost" onPress={() => setPickerBook(null)}>
              <ArrowLeft size={16} /> All books
            </Button>
            <h3>{books.find((b) => b.id === pickerBook)?.name}</h3>
            <p className="muted small">
              {live ? "Choose a chapter" : "Chapters available in this preview"}
            </p>
            <div className="chapter-grid">
              {live
                ? Array.from(
                    {
                      length:
                        live.state.books.find((b) => b.id === pickerBook)
                          ?.verse_counts.length || 0,
                    },
                    (_, i) => (
                      <Button
                        key={i}
                        aria-label={`Chapter ${i + 1}`}
                        onPress={() => setPickerChapter(i + 1)}
                      >
                        {i + 1}
                      </Button>
                    ),
                  )
                : passages.map(
                    (entry, i) =>
                      entry.book === pickerBook && (
                        <Button
                          key={`${entry.book}-${entry.chapter}`}
                          aria-label={`Chapter ${entry.chapter}`}
                          variant={i === passageIndex ? "primary" : "tertiary"}
                          onPress={() => setPickerChapter(entry.chapter)}
                        >
                          {entry.chapter}
                          {i === passageIndex && <Check size={14} />}
                        </Button>
                      ),
                  )}
            </div>
          </div>
        ) : (
          <div className="passage-picker">
            <Input
              fullWidth
              aria-label="Find a book"
              placeholder="Find a book…"
              value={pickerQuery}
              onChange={(e) => setPickerQuery(e.target.value)}
            />
            {["Old Testament", "New Testament"].map((label, testament) => (
              <div key={label}>
                <div className="eyebrow picker-heading">{label}</div>
                <div className="picker-books">
                  {books
                    .filter(
                      (b, i) =>
                        (testament === 0 ? i < 39 : i >= 39) &&
                        b.name
                          .toLowerCase()
                          .includes(pickerQuery.toLowerCase()),
                    )
                    .map((b) => (
                      <Button
                        key={b.id}
                        fullWidth
                        variant={b.id === p.book ? "secondary" : "ghost"}
                        onPress={() => { setPickerBook(b.id); setPickerChapter(null); }}
                      >
                        {b.name}
                        <ChevronRight size={16} />
                      </Button>
                    ))}
                </div>
              </div>
            ))}
          </div>
        )}
      </Sheet>
      <Sheet
        title="Make it yours"
        open={dialog === "settings"}
        onClose={() => setDialog(null)}
      >
        <p className="muted mb-5">
          Fontes typography and artwork. HeroUI’s own controls.
        </p>
        {live && (
          <label className="reading-size">
            Text size{" "}
            <input
              aria-label="Text size"
              type="range"
              min="16"
              max="36"
              value={live.state.session.preferences.text_size}
              onChange={(e) => {
                void run("preferences.set", {
                  preferences: {
                    ...live.state.session.preferences,
                    text_size: Number(e.target.value),
                  },
                }).catch((e) => showNotice(String(e)));
              }}
            />
          </label>
        )}
        <Button
          fullWidth
          variant="tertiary"
          className="mb-5"
          onPress={() => {
            void fetch("/THIRD_PARTY_NOTICES.txt")
              .then((r) => r.text())
              .then(setNotices)
              .catch((e) => showNotice(String(e)));
            setDialog("licenses");
          }}
        >
          Sources & licenses
          <ChevronRight size={16} />
        </Button>
        <div className="eyebrow mb-3">ACCENT</div>
        <div className="flex flex-wrap gap-2">
          {[
            ["default", "HeroUI blue"],
            ["clay", "Fontes clay"],
            ["sea", "Sea glass"],
          ].map(([id, label]) => (
            <Button
              key={id}
              variant={accent === id ? "primary" : "tertiary"}
              onPress={() => {
                setAccent(id);
                if (live)
                  void run("preferences.set", {
                    preferences: {
                      ...live.state.session.preferences,
                      theme:
                        id === "clay"
                          ? "terracotta_linen"
                          : id === "sea"
                            ? "sea_glass"
                            : "cobalt_citron",
                    },
                  }).catch((e) => showNotice(String(e)));
              }}
            >
              {label}
            </Button>
          ))}
        </div>
        <div className="eyebrow mt-6 mb-3">APPEARANCE</div>
        <div className="flex gap-2">
          <Button
            variant={dark ? "tertiary" : "secondary"}
            onPress={() => {
              setDark(false);
              if (live)
                void run("preferences.set", {
                  preferences: {
                    ...live.state.session.preferences,
                    theme: accent === "clay" ? "terracotta_linen" : "sea_glass",
                  },
                }).catch((e) => showNotice(String(e)));
            }}
          >
            <Sun size={16} />
            Light
          </Button>
          <Button
            variant={dark ? "secondary" : "tertiary"}
            onPress={() => {
              setDark(true);
              if (live)
                void run("preferences.set", {
                  preferences: {
                    ...live.state.session.preferences,
                    theme: "midnight_mint",
                  },
                }).catch((e) => showNotice(String(e)));
            }}
          >
            <Moon size={16} />
            Dark
          </Button>
        </div>
      </Sheet>
      <Sheet
        title="Explore your library"
        open={dialog === "search"}
        onClose={() => setDialog(null)}
      >
        <Input
          className="w-full"
          aria-label="Search preview library"
          placeholder={
            live
              ? "Search Scripture and study resources…"
              : "Find a book of the Bible…"
          }
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        {live ? (
          <>
            <Button
              className="mt-4"
              isDisabled={!query.trim() || live.busy}
              onPress={() => void searchLibrary()}
            >
              Search
            </Button>
            {searchResults.map((hit) => (
              <Card key={hit.id} className="mt-3">
                <Card.Header>
                  <Card.Title>{hit.title}</Card.Title>
                </Card.Header>
                <Card.Content>
                  <p>{hit.excerpt}</p>
                </Card.Content>
                <Card.Footer>
                  <Button
                    variant="secondary"
                    onPress={() => {
                      if (hit.ref_id)
                        void run("reading.select", { ref_id: hit.ref_id })
                          .then(() => {
                            setDialog(null);
                            setPage("Read");
                          })
                          .catch((e) => showNotice(String(e)));
                      else void openArticle(hit.id);
                    }}
                  >
                    Open
                    <ArrowRight size={14} />
                  </Button>
                </Card.Footer>
              </Card>
            ))}
            <div className="flex gap-2 mt-4">
              <Button
                isDisabled={!searchOffset}
                onPress={() =>
                  void searchLibrary(Math.max(0, searchOffset - 30))
                }
              >
                Previous
              </Button>
              <Button
                isDisabled={!searchMore}
                onPress={() => void searchLibrary(searchOffset + 30)}
              >
                Next
              </Button>
            </div>
          </>
        ) : (
          <>
            <div className="space-y-3 mt-5">
              {query ? (
                books
                  .filter((b) =>
                    b.name.toLowerCase().includes(query.toLowerCase()),
                  )
                  .slice(0, 8)
                  .map((b) => (
                    <Button
                      key={b.id}
                      fullWidth
                      variant="tertiary"
                      onPress={() => {
                        setDialog(null);
                        setCover(b.id);
                      }}
                    >
                      <BookOpen size={16} />
                      {b.name}
                      <ArrowRight size={16} />
                    </Button>
                  ))
              ) : (
                <>
                  <p className="muted small">A few places to begin</p>
                  {["Proverbs", "Psalms", "Genesis"].map((name) => (
                    <Button
                      key={name}
                      variant="tertiary"
                      fullWidth
                      onPress={() => setQuery(name)}
                    >
                      {name}
                      <ArrowRight size={16} />
                    </Button>
                  ))}
                </>
              )}
            </div>
            {query &&
              !books.some((b) =>
                b.name.toLowerCase().includes(query.toLowerCase()),
              ) && (
                <p className="muted mt-5">
                  No matching books in this preview. Try “Genesis” or “Psalms”.
                </p>
              )}
          </>
        )}
      </Sheet>
      <Sheet
        title={cover ? books[cover - 1].name : ""}
        open={cover !== null}
        onClose={() => setCover(null)}
        footer={
          <Button
            fullWidth
            onPress={() => {
              if (live) {
                void run("reading.navigate", {
                  reference: `${books.find((b) => b.id === cover)?.name} 1:1`,
                })
                  .then(() => {
                    setCover(null);
                    setPage("Read");
                  })
                  .catch((e) => showNotice(String(e)));
                return;
              }
              const index = passages.findIndex(
                (p) => p.book === cover && p.chapter === 1,
              );
              if (index >= 0) selectPassage(index);
              else {
                setCover(null);
                setDialog("passages");
              }
            }}
          >
            Start reading
            <ArrowRight size={16} />
          </Button>
        }
      >
        {cover && (
          <>
            <img
              className="cover-art"
              src={books[cover - 1].art}
              alt="Symbolic book cover"
            />
            <h2 className="cover-subtitle">{books[cover - 1].title}</h2>
            <div className="eyebrow mb-4">{books[cover - 1].attribution}</div>
            {books[cover - 1].paragraphs.map((text, i) => (
              <p className="introduction" key={i}>
                {text}
              </p>
            ))}
            <a
              className="source-link"
              href={books[cover - 1].source}
              target="_blank"
              rel="noreferrer"
            >
              Introduction source · CCEL
              <ExternalLink size={14} />
            </a>
          </>
        )}
      </Sheet>
      <Sheet
        title={
          noteModal?.length === 1
            ? noteModal[0].title || "Your note"
            : `${noteModal?.length || 0} notes on these words`
        }
        open={noteModal !== null}
        onClose={() => setNoteModal(null)}
        footer={
          <Button
            fullWidth
            variant="secondary"
            onPress={() =>
              newNote(noteModal?.[0]?.quote, noteModal?.[0]?.anchor)
            }
          >
            <Plus size={16} />
            Add another note
          </Button>
        }
      >
        {noteModal?.map((n) => (
          <div className="modal-note" key={n.id}>
            <div className="card-meta mb-4">
              <span>{n.anchor}</span>
              {noteModal.length > 1 && <strong>{n.title}</strong>}
            </div>
            <NoteContent note={n} />
            <Button
              variant="tertiary"
              className="mt-4"
              onPress={() => {
                setDraft(structuredClone(n));
                setNoteModal(null);
                setPage("Notes");
              }}
            >
              <NotebookPen size={16} />
              Edit note
            </Button>
          </div>
        ))}
      </Sheet>
      <Sheet
        title="Add to your note"
        open={dialog === "insert"}
        onClose={() => setDialog(null)}
      >
        <p className="muted mb-5">
          Your own thoughts, and the pieces that connect them.
        </p>
        {live ? (
          <NativeInsertChoices live={live} onInsert={insertBlock} />
        ) : attachmentPassage ? (
          <PassageAttachment initialRef={p.book * 1000000 + p.chapter * 1000 + 1}
            books={books.filter(b => passages.some(p => p.book === b.id)).map(b => ({id: b.id, name: b.name, chapters: passages.filter(p => p.book === b.id).map(p => ({number: p.chapter, verses: p.verses.length}))}))}
            preview={ref => passages.find(p => p.book === Math.floor(ref / 1000000) && p.chapter === Math.floor(ref % 1000000 / 1000))?.verses[ref % 1000 - 1]}
            onBack={() => setAttachmentPassage(false)}
            onInsert={b => {setAttachmentPassage(false); insertBlock(b);}} />
        ) : (
          <>
            <div className="space-y-3">
              {samples.map((b) => (
                <Button
                  key={b.id}
                  variant="tertiary"
                  fullWidth
                  className="insert-choice"
                  onPress={() => b.kind === "passage" ? setAttachmentPassage(true) : insertBlock(b)}
                >
                  {b.kind === "thought" ? (
                    <NotebookPen />
                  ) : b.kind === "passage" ? (
                    <BookOpen />
                  ) : b.kind === "word" ? (
                    <Type />
                  ) : (
                    <Quote />
                  )}
                  <span>
                    <strong>
                      {b.kind === "thought"
                        ? "Your thought"
                        : b.kind === "passage"
                          ? "Scripture passage"
                          : b.kind === "word"
                            ? "Word definition"
                            : "Resource excerpt"}
                    </strong>
                    <small>
                      {b.kind === "thought"
                        ? "Write between your collected pieces"
                        : b.source}
                    </small>
                  </span>
                  <Plus size={16} />
                </Button>
              ))}
            </div>
          </>
        )}
      </Sheet>
      <Sheet
        title={source?.ref || "Source context"}
        open={dialog === "source"}
        onClose={() => setDialog(null)}
      >
        <div className="eyebrow mb-4">{source?.source}</div>
        <p className={live ? "resource-body" : "scripture excerpt"}>
          {source?.text}
        </p>
        {live && articleLinks && articleLinks.id === source?.native?.resource_id && <ResourceCitations key={articleLinks.id} live={live} links={articleLinks.links} />}
        {live && source && (
          <Button
            className="mt-4"
            onPress={() => {
              const clip = {
                ...source,
                text: source.text.slice(0, 1500),
                native: source.native
                  ? { ...source.native, quote: source.text.slice(0, 1500) }
                  : undefined,
              };
              if (draft) insertBlock(clip);
              else {
                newNote("", currentRef);
                setDraft((d) =>
                  d ? { ...d, blocks: [...d.blocks, clip] } : null,
                );
              }
              setDialog(null);
            }}
          >
            {" "}
            <Plus size={16} /> Collect excerpt in a note
          </Button>
        )}
        <p className="muted small mt-6">
          {live
            ? "From your installed library. Source attribution is retained when collecting an excerpt."
            : "Retained excerpt from the preview library. The Rust adapter will open the installed source here."}
        </p>
      </Sheet>
      <Sheet
        title="Note actions"
        open={dialog === "delete"}
        onClose={() => setDialog(null)}
      >
        <div className="space-y-3">
          <Button
            fullWidth
            variant="tertiary"
            onPress={() => {
              const n = notes.find((n) => n.id === deleteId);
              if (n) setDraft(structuredClone(n));
              setDialog(null);
            }}
          >
            <NotebookPen size={16} />
            Edit note
          </Button>
          <Button
            fullWidth
            variant="tertiary"
            onPress={() => {
              if (live) void pinNote(deleteId);
              else
                setNotes((list) =>
                  list.map((n) =>
                    n.id === deleteId ? { ...n, pinned: !n.pinned } : n,
                  ),
                );
              setDialog(null);
            }}
          >
            <Pin size={16} />
            Pin / unpin
          </Button>
          <Button
            fullWidth
            variant="danger"
            onPress={() => {
              setDialog("delete-confirm");
            }}
          >
            <Trash2 size={16} />
            Delete note
          </Button>
        </div>
      </Sheet>
      <Sheet
        title="Keep your changes?"
        open={dialog === "discard"}
        onClose={() => setDialog(null)}
        footer={
          <>
            <Button variant="tertiary" onPress={() => setDialog(null)}>
              Keep writing
            </Button>
            <Button
              variant="danger"
              onPress={() => {
                setDraft(null);
                setPage(leaveTarget);
                setDialog(null);
              }}
            >
              Discard changes
            </Button>
          </>
        }
      >
        <p className="muted">
          Your unsaved edits will be lost. Any previously saved version of this
          note will be kept.
        </p>
      </Sheet>
      <Sheet
        title="Delete this note?"
        open={dialog === "delete-confirm"}
        onClose={() => setDialog(null)}
        footer={
          <>
            <Button variant="tertiary" onPress={() => setDialog(null)}>
              Keep note
            </Button>
            <Button
              variant="danger"
              onPress={() => {
                if (live) {
                  void run("annotation.delete", { id: deleteId }).catch((e) =>
                    showNotice(String(e)),
                  );
                } else
                  setNotes((list) => list.filter((n) => n.id !== deleteId));
                setDialog(null);
              }}
            >
              Delete note
            </Button>
          </>
        }
      >
        <p className="muted">
          {live
            ? "This removes the saved note and its collected pieces from this device."
            : "This removes the note and its collected pieces from this preview."}
        </p>
      </Sheet>
      {live?.error && (
        <div className="native-error" role="alert">
          {live.error}
        </div>
      )}
      {notice && (
        <div className="notice" role="status">
          <Check size={16} />
          {notice}
        </div>
      )}
    </>
  );
}
