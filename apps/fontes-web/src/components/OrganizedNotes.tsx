import { createPortal } from "react-dom";
import { useTimedNotice } from "./useTimedNotice";
import { ThemedPicker } from "./ThemedPicker";
import { useMemo, useState } from "react";
import { Button, Input } from "@heroui/react";
import {
  BookOpen,
  ChevronRight,
  Folder,
  Inbox,
  Pin,
  Search,
  Tag,
  X,
  Check,
  Settings2,
  MoreHorizontal,
} from "lucide-react";
import { Sheet } from "./Sheet";
import { noteDate } from "./NoteContext";
import { books, type Note } from "../data/preview";
import type { Live } from "../native";
import "./organized-notes.css";
export type Organization = {
  folders: { id: string; name: string }[];
  entries: { note_id: string; folder_id: string | null; tags: string[] }[];
};
const empty: Organization = { folders: [], entries: [] };
export function noteLocation(n: Note) {
  const m = n.anchor.match(/^(.*) (\d+):(\d+)/);
  return { book: m?.[1] || "", chapter: m?.[2] || "", verse: m?.[3] || "" };
}
export function filterOrganizedNotes(
  notes: Note[],
  org: Organization,
  folder: string,
  query: string,
  tags: string[],
  book: string,
  chapter: string,
  verse: string,
  pinned: boolean,
  sort: string,
) {
  const metadata = new Map(org.entries.map((e) => [e.note_id, e]));
  const words = query.trim().toLocaleLowerCase().split(/\s+/).filter(Boolean);
  return notes
    .filter((n) => {
      const e = metadata.get(n.id);
      const loc = noteLocation(n);
      const text = [
        n.title,
        n.anchor,
        n.quote,
        ...n.blocks.map((b) => `${b.text} ${b.ref || ""} ${b.source || ""}`),
        ...(e?.tags || []),
      ]
        .join(" ")
        .toLocaleLowerCase();
      return (
        (folder === "all" ||
          (folder === "inbox" ? !e?.folder_id : e?.folder_id === folder)) &&
        words.every((w) => text.includes(w)) &&
        tags.every((t) => e?.tags.includes(t)) &&
        (!book || loc.book === book) &&
        (!chapter || loc.chapter === chapter) &&
        (!verse || loc.verse === verse) &&
        (!pinned || n.pinned)
      );
    })
    .sort((a, b) => {
      if (sort === "title") return a.title.localeCompare(b.title);
      if (sort === "scripture") {
        const al = noteLocation(a),
          bl = noteLocation(b);
        return (
          books.findIndex((b) => b.name === al.book) -
            books.findIndex((b) => b.name === bl.book) ||
          Number(al.chapter) - Number(bl.chapter) ||
          Number(al.verse) - Number(bl.verse)
        );
      }
      return (
        Number(b.pinned) - Number(a.pinned) ||
        (sort === "oldest" ? 1 : -1) *
          ((Date.parse(a.updated) || 0) - (Date.parse(b.updated) || 0)) ||
        a.id.localeCompare(b.id)
      );
    });
}
export function OrganizedNotes({
  notes,
  live,
  onOpen,
  onNew,
  onMore,
}: {
  notes: Note[];
  live?: Live;
  onOpen: (n: Note) => void;
  onNew: () => void;
  onMore: (n: Note) => void;
}) {
  const [preview, setPreview] = useState<Organization>(() => {
    try {
      return (
        JSON.parse(
          localStorage.getItem("fontes-note-organization-preview") || "null",
        ) || empty
      );
    } catch {
      return empty;
    }
  });
  const org = live?.state.note_organization || preview;
  const [actionNote, setActionNote] = useState<Note | null>(null);
  const [folder, setFolder] = useState("all"),
    [query, setQuery] = useState(""),
    [tags, setTags] = useState<string[]>([]),
    [book, setBook] = useState(""),
    [chapter, setChapter] = useState(""),
    [verse, setVerse] = useState(""),
    [pinned, setPinned] = useState(false),
    [sort, setSort] = useState("recent"),
    [limit, setLimit] = useState(40);
  const [sheet, setSheet] = useState<
      "folders" | "filters" | "organize" | "view" | "actions" | null
    >(null),
    [selected, setSelected] = useState<string[]>([]),
    [selecting, setSelecting] = useState(false),
    [name, setName] = useState(""),
    [editFolder, setEditFolder] = useState<string | null>(null),
    [removeFolderId, setRemoveFolderId] = useState<string | null>(null),
    [target, setTarget] = useState(""),
    [tagText, setTagText] = useState(""),
    [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  const [notice,setNotice]=useTimedNotice();
  const visible = useMemo(
    () =>
      filterOrganizedNotes(
        notes,
        org,
        folder,
        query,
        tags,
        book,
        chapter,
        verse,
        pinned,
        sort,
      ),
    [notes, org, folder, query, tags, book, chapter, verse, pinned, sort],
  );
  const allTags = [...new Set(org.entries.flatMap((e) => e.tags))].sort();
  const scoped = filterOrganizedNotes(
    notes,
    org,
    folder,
    "",
    tags,
    "",
    "",
    "",
    pinned,
    sort,
  );
  const bookNames = books
    .filter((b) => scoped.some((n) => noteLocation(n).book === b.name))
    .map((b) => b.name);
  const chapters = [
    ...new Set(
      scoped
        .filter((n) => noteLocation(n).book === book)
        .map((n) => noteLocation(n).chapter),
    ),
  ].sort((a, b) => +a - +b);
  const verses = [
    ...new Set(
      scoped
        .filter(
          (n) =>
            noteLocation(n).book === book &&
            noteLocation(n).chapter === chapter,
        )
        .map((n) => noteLocation(n).verse),
    ),
  ].sort((a, b) => +a - +b);
  const active = !!(query || tags.length || book || pinned || folder !== "all");
  function changeView() {
    setLimit(40);
    setSelected([]);
  }
  function updatePreview(next: Organization) {
    localStorage.setItem(
      "fontes-note-organization-preview",
      JSON.stringify(next),
    );
    setPreview(next);
  }
  async function mutate(action: () => Promise<void>) {
    setBusy(true);
    setError("");
    try {
      await action();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  async function saveFolder() {
    await mutate(async () => {
      if (!name.trim()) throw Error("Give your folder a name.");
      if (live)
        await live.send({
          command: "note.folder.save",
          params: { id: editFolder, name },
        });
      else {
        if (
          org.folders.some(
            (f) =>
              f.name.toLowerCase() === name.trim().toLowerCase() &&
              f.id !== editFolder,
          )
        )
          throw Error("That folder name is already in use.");
        updatePreview({
          ...org,
          folders: [
            ...org.folders.filter((f) => f.id !== editFolder),
            { id: editFolder || crypto.randomUUID(), name: name.trim() },
          ],
        });
      }
      setName("");
      setEditFolder(null);
      setNotice("Folder saved");
    });
  }
  async function removeFolder(id: string) {
    await mutate(async () => {
      if (live)
        await live.send({ command: "note.folder.delete", params: { id } });
      else
        updatePreview({
          folders: org.folders.filter((f) => f.id !== id),
          entries: org.entries.map((e) =>
            e.folder_id === id ? { ...e, folder_id: null } : e,
          ),
        });
      if (folder === id) setFolder("inbox");
      setRemoveFolderId(null);
      setNotice("Folder removed · notes kept in Inbox");
    });
  }
  function organize(ids: string[]) {
    setSelected(ids);
    setTarget("");
    setTagText("");
    setError("");
    setSheet("organize");
  }
  async function applyOrganization() {
    await mutate(async () => {
      const newTags = tagText
        .split(",")
        .map((t) => t.trim().replace(/^#/, "").toLowerCase())
        .filter(Boolean);
      const changes = selected.map((id) => {
        const e = org.entries.find((e) => e.note_id === id);
        return {
          note_id: id,
          folder_id: target
            ? target === "inbox"
              ? null
              : target
            : e?.folder_id || null,
          tags: [...new Set([...(e?.tags || []), ...newTags])],
        };
      });
      if (changes.length > 200)
        throw Error("Select up to 200 notes at a time.");
      if (live)
        await live.send({ command: "note.organize", params: { changes } });
      else
        updatePreview({
          ...org,
          entries: [
            ...org.entries.filter((e) => !selected.includes(e.note_id)),
            ...changes,
          ],
        });
      setSheet(null);
      setSelected([]);
      setSelecting(false);
      setNotice(
        `${changes.length} ${changes.length === 1 ? "note" : "notes"} organized`,
      );
    });
  }
  async function removeTag(noteId: string, tag: string) {
    await mutate(async () => {
      const e = org.entries.find((e) => e.note_id === noteId)!;
      const change = { ...e, tags: e.tags.filter((t) => t !== tag) };
      if (live)
        await live.send({
          command: "note.organize",
          params: { changes: [change] },
        });
      else
        updatePreview({
          ...org,
          entries: org.entries.map((e) => (e.note_id === noteId ? change : e)),
        });
    });
  }
  return (
    <div className="organized-notes">
      <div className="org-search">
        <Search size={18} />
        <Input
          aria-label="Search all note content"
          placeholder="Search your notes"
          value={query}
          onChange={(e) => {
            setQuery(e.target.value);
            changeView();
          }}
        />
        {query && (
          <button aria-label="Clear search" onClick={() => setQuery("")}>
            <X size={16} />
          </button>
        )}
      </div>
      <div className="org-toolbar">
        <button
          className="org-filter"
          onClick={() => {
            setError("");
            setSheet("folders");
          }}
        >
          <Folder size={17} />
          {folder === "all"
            ? "All notes"
            : folder === "inbox"
              ? "Inbox"
              : org.folders.find((f) => f.id === folder)?.name || "All notes"}
          <ChevronRight size={14} />
        </button>
        <button
          aria-label="Filter notes"
          className={`org-filter org-icon ${book || tags.length || pinned ? "active" : ""}`}
          onClick={() => setSheet("filters")}
        >
          <Settings2 size={17} />
          {tags.length + Number(!!book) + Number(pinned) > 0 && (
            <span>{tags.length + Number(!!book) + Number(pinned)}</span>
          )}
        </button>
        <button
          className="org-icon"
          aria-label="View options"
          onClick={() => setSheet("view")}
        >
          <MoreHorizontal size={20} />
        </button>
      </div>
      {(book || tags.length > 0 || pinned) && (
        <div className="org-chips">
          {book && (
            <button
              onClick={() => {
                setBook("");
                setChapter("");
                setVerse("");
              }}
            >
              <BookOpen size={13} />
              {book}
              {chapter && ` ${chapter}`}
              {verse && `:${verse}`}
              <X size={12} />
            </button>
          )}
          {tags.map((t) => (
            <button
              key={t}
              onClick={() => setTags(tags.filter((x) => x !== t))}
            >
              #{t}
              <X size={12} />
            </button>
          ))}
          {pinned && (
            <button onClick={() => setPinned(false)}>
              <Pin size={12} />
              Pinned
              <X size={12} />
            </button>
          )}
        </div>
      )}
      <div className="org-result-row">
        <span>
          {visible.length} {visible.length === 1 ? "note" : "notes"}{" "}
          <span className="org-sort-caption">
            ·{" "}
            {sort === "recent"
              ? "Pinned & recent"
              : sort === "oldest"
                ? "Oldest first"
                : sort === "title"
                  ? "Title A–Z"
                  : "Scripture order"}
          </span>
        </span>
        {selecting && (
          <button
            onClick={() => {
              setSelecting(false);
              setSelected([]);
            }}
          >
            Cancel selection
          </button>
        )}
      </div>
      {notice && createPortal(
        <div className="org-notice org-toast" role="status" aria-live="polite">
          {notice}
          <button aria-label="Dismiss notice" onClick={() => setNotice("")}>
            <X size={14} />
          </button>
        </div>, document.getElementById("overlay-root") || document.body
      )}
      {selecting && (
        <div className="org-selection">
          <span>{selected.length} selected</span>
          <Button
            size="sm"
            isDisabled={!selected.length}
            onPress={() => organize(selected)}
          >
            Move / tag
          </Button>
        </div>
      )}
      <div className="org-list">
        {visible.slice(0, limit).map((n) => {
          const meta = org.entries.find((e) => e.note_id === n.id);
          return (
            <article className="org-note" key={n.id}>
              {selecting && (
                <input
                  type="checkbox"
                  aria-label={`Select ${n.title || n.anchor}`}
                  checked={selected.includes(n.id)}
                  onChange={() =>
                    setSelected(
                      selected.includes(n.id)
                        ? selected.filter((id) => id !== n.id)
                        : [...selected, n.id],
                    )
                  }
                />
              )}
              <button
                className="org-note-body"
                onClick={() =>
                  selecting
                    ? setSelected(
                        selected.includes(n.id)
                          ? selected.filter((id) => id !== n.id)
                          : [...selected, n.id],
                      )
                    : onOpen(n)
                }
              >
                <span className="org-note-meta">
                  {n.anchor}
                  <span>
                    {n.pinned ? <Pin size={13} /> : noteDate(n.updated)}
                  </span>
                </span>
                <strong>{n.title || "An observation"}</strong>
                <p>
                  {n.blocks.find((b) => b.kind === "thought")?.text || n.quote}
                </p>
                {n.quote && <blockquote>{n.quote}</blockquote>}
              </button>
              <div className="org-note-footer">
                <span>
                  {meta?.folder_id && (
                    <span>
                      <Folder size={12} />{" "}
                      {org.folders.find((f) => f.id === meta.folder_id)?.name}{" "}
                      ·{" "}
                    </span>
                  )}
                  {n.blocks.filter((b) => b.kind !== "thought").length}{" "}
                  collected pieces
                </span>
                <button
                  aria-label={`Actions for ${n.title || n.anchor}`}
                  onClick={() => {
                    setActionNote(n);
                    setSheet("actions");
                  }}
                >
                  <MoreHorizontal size={18} />
                </button>
              </div>
              {!!meta?.tags.length && (
                <div className="org-chips org-note-tags">
                  {meta.tags.map((t) => (
                    <button
                      key={t}
                      onClick={() => {
                        setTags(tags.includes(t) ? tags : [...tags, t]);
                        changeView();
                      }}
                    >
                      #{t}
                    </button>
                  ))}
                </div>
              )}
            </article>
          );
        })}
      </div>
      {visible.length > limit && (
        <Button
          className="w-full mt-4"
          variant="secondary"
          onPress={() => setLimit(limit + 40)}
        >
          Show 40 more · {visible.length - limit} remaining
        </Button>
      )}
      {!visible.length && (
        <div className="empty-state">
          <Inbox size={32} />
          <h2>
            {notes.length
              ? "Room for a different view."
              : "Let a thought take root."}
          </h2>
          <p>
            {notes.length
              ? "Try fewer filters or search all your notes."
              : "Keep a question, a prayer, or something you noticed."}
          </p>
          {active ? (
            <Button
              onPress={() => {
                setFolder("all");
                setQuery("");
                setTags([]);
                setBook("");
                setChapter("");
                setVerse("");
                setPinned(false);
              }}
            >
              Clear filters
            </Button>
          ) : (
            <Button onPress={onNew}>Write your first note</Button>
          )}
        </div>
      )}
      <Sheet
        title="View options"
        open={sheet === "view"}
        onClose={() => setSheet(null)}
      >
        <ThemedPicker
          visibleLabel
          label="Sort notes"
          value={sort}
          onChange={(value) => {
            setSort(value);
            changeView();
          }}
          options={[
            { value: "recent", label: "Pinned & recent" },
            { value: "oldest", label: "Oldest first" },
            { value: "title", label: "Title A–Z" },
            { value: "scripture", label: "Scripture order" },
          ]}
        />
        <Button
          className="w-full mt-4"
          variant="secondary"
          onPress={() => {
            setSelecting(true);
            setSelected([]);
            setSheet(null);
          }}
        >
          Select notes to move or tag
        </Button>
      </Sheet>
      <Sheet
        title={actionNote?.title || "Note actions"}
        open={sheet === "actions"}
        onClose={() => setSheet(null)}
      >
        <div className="org-action-list">
          <Button
            variant="ghost"
            onPress={() => actionNote && organize([actionNote.id])}
          >
            <Folder size={18} />
            Move or manage tags
          </Button>
          <Button
            variant="ghost"
            onPress={() => {
              setSheet(null);
              if (actionNote) onMore(actionNote);
            }}
          >
            <MoreHorizontal size={18} />
            Edit, pin or delete
          </Button>
        </div>
      </Sheet>
      <Sheet
        title="Your folders"
        open={sheet === "folders"}
        onClose={() => setSheet(null)}
      >
        <div className="org-folder-list">
          {[
            { id: "all", name: "All notes" },
            { id: "inbox", name: "Inbox" },
            ...org.folders,
          ].map((f) => (
            <div key={f.id}>
              <button
                onClick={() => {
                  setFolder(f.id);
                  changeView();
                  setSheet(null);
                }}
              >
                {f.id === "inbox" ? <Inbox size={18} /> : <Folder size={18} />}
                <span>{f.name}</span>
                <small>
                  {
                    filterOrganizedNotes(
                      notes,
                      org,
                      f.id,
                      "",
                      [],
                      "",
                      "",
                      "",
                      false,
                      "recent",
                    ).length
                  }
                </small>
                {folder === f.id && <Check size={16} />}
              </button>
              {!["all", "inbox"].includes(f.id) && (
                <>
                  <button
                    aria-label={`Rename ${f.name}`}
                    onClick={() => {
                      setEditFolder(f.id);
                      setName(f.name);
                    }}
                  >
                    Rename
                  </button>
                  <button
                    aria-label={`Delete ${f.name}`}
                    onClick={() => setRemoveFolderId(f.id)}
                  >
                    Delete
                  </button>
                </>
              )}
            </div>
          ))}
        </div>
        {removeFolderId && (
          <div className="org-filter-section">
            <p>
              Remove this folder? Its notes will move to Inbox. Notes and tags
              will be kept.
            </p>
            <Button
              isDisabled={busy}
              onPress={() => void removeFolder(removeFolderId)}
            >
              Remove folder
            </Button>
            <Button variant="ghost" onPress={() => setRemoveFolderId(null)}>
              Keep folder
            </Button>
          </div>
        )}
        <label className="org-label">
          {editFolder ? "Rename folder" : "New folder"}
          <Input
            maxLength={128}
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="e.g. Sermon preparation"
          />
        </label>
        <Button
          isDisabled={busy || !name.trim()}
          onPress={() => void saveFolder()}
        >
          {editFolder ? "Save name" : "Create folder"}
        </Button>
        {editFolder && (
          <Button
            variant="ghost"
            onPress={() => {
              setEditFolder(null);
              setName("");
            }}
          >
            Cancel rename
          </Button>
        )}
        {error && <p role="alert">{error}</p>}
      </Sheet>
      <Sheet
        title="Find your notes"
        open={sheet === "filters"}
        onClose={() => setSheet(null)}
        footer={
          <Button
            onPress={() => {
              changeView();
              setSheet(null);
            }}
          >
            Show {visible.length} notes
          </Button>
        }
      >
        <div className="org-filter-section">
          <h3>
            <BookOpen size={18} />
            By Scripture
          </h3>
          <p className="muted small">
            Notes anchored to Scripture, within your current folder.
          </p>
          <ThemedPicker
            label="Filter by Bible book"
            value={book}
            onChange={(value) => {
              setBook(value);
              setChapter("");
              setVerse("");
            }}
            options={[
              { value: "", label: "All books" },
              ...bookNames.map((b) => ({ value: b, label: b })),
            ]}
          />
          {book && (
            <div className="org-atlas">
              {chapters.map((c) => (
                <button
                  aria-pressed={chapter === c}
                  key={c}
                  onClick={() => {
                    setChapter(chapter === c ? "" : c);
                    setVerse("");
                  }}
                >
                  Chapter {c}
                  <small>
                    {
                      scoped.filter(
                        (n) =>
                          noteLocation(n).book === book &&
                          noteLocation(n).chapter === c,
                      ).length
                    }
                  </small>
                </button>
              ))}
            </div>
          )}
          {chapter && (
            <div className="org-atlas">
              {verses.map((v) => (
                <button
                  aria-pressed={verse === v}
                  key={v}
                  onClick={() => setVerse(verse === v ? "" : v)}
                >
                  {book} {chapter}:{v}
                </button>
              ))}
            </div>
          )}
        </div>
        <div className="org-filter-section">
          <h3>
            <Tag size={18} />
            Tags
          </h3>
          <p className="muted small">
            Choose several to find notes that share every tag.
          </p>
          <div className="org-chips">
            {allTags.map((t) => (
              <button
                key={t}
                aria-pressed={tags.includes(t)}
                onClick={() =>
                  setTags(
                    tags.includes(t)
                      ? tags.filter((x) => x !== t)
                      : [...tags, t],
                  )
                }
              >
                #{t}
              </button>
            ))}
          </div>
          {!allTags.length && (
            <p className="muted small">
              Add tags using the tag icon on a note.
            </p>
          )}
        </div>
        <label className="org-check">
          <input
            type="checkbox"
            checked={pinned}
            onChange={(e) => setPinned(e.target.checked)}
          />
          Pinned notes only
        </label>
        <Button
          variant="ghost"
          onPress={() => {
            setBook("");
            setChapter("");
            setVerse("");
            setTags([]);
            setPinned(false);
          }}
        >
          Reset filters
        </Button>
      </Sheet>
      <Sheet
        title={`Organize ${selected.length === 1 ? "note" : `${selected.length} notes`}`}
        open={sheet === "organize"}
        onClose={() => setSheet(null)}
        footer={
          <Button isDisabled={busy} onPress={() => void applyOrganization()}>
            {busy ? "Saving…" : "Save organization"}
          </Button>
        }
      >
        <div className="org-label">
          <ThemedPicker
            visibleLabel
            label="Move to folder"
            value={target}
            onChange={setTarget}
            options={[
              { value: "", label: "Keep current folder" },
              { value: "inbox", label: "Inbox" },
              ...org.folders.map((f) => ({ value: f.id, label: f.name })),
            ]}
          />
        </div>
        <label className="org-label">
          Add tags
          <Input
            value={tagText}
            maxLength={1300}
            onChange={(e) => setTagText(e.target.value)}
            placeholder="grace, prayer, questions"
          />
        </label>
        <p className="muted small">
          Separate tags with commas. Existing tags are kept.
        </p>
        {selected.length === 1 && (
          <div className="org-chips">
            {org.entries
              .find((e) => e.note_id === selected[0])
              ?.tags.map((t) => (
                <button
                  disabled={busy}
                  key={t}
                  aria-label={`Remove tag ${t}`}
                  onClick={() => void removeTag(selected[0], t)}
                >
                  #{t}
                  <X size={12} />
                </button>
              ))}
          </div>
        )}
        {error && <p role="alert">{error}</p>}
      </Sheet>
    </div>
  );
}
