import { ThemedPicker } from "./ThemedPicker";
import { useRef, useState } from "react";
import { Button } from "@heroui/react";
import { ArrowLeft, BookOpen, Plus } from "lucide-react";
import type { Block } from "../data/preview";
import type { Live } from "../native";
type Book = {
  id: number;
  name: string;
  chapters: { number: number; verses: number }[];
};
export function PassageAttachment({
  live,
  books,
  initialRef,
  preview,
  onInsert,
  onBack,
}: {
  live?: Live;
  books: Book[];
  initialRef: number;
  preview?: (ref: number) => string | undefined;
  onInsert: (block: Block) => void;
  onBack: () => void;
}) {
  const [book, setBook] = useState(Math.floor(initialRef / 1000000));
  const [chapter, setChapter] = useState(
    Math.floor((initialRef % 1000000) / 1000),
  );
  const [verse, setVerse] = useState(initialRef % 1000);
  const [text, setText] = useState("");
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  const request = useRef(0);
  const selected = books.find((b) => b.id === book)!;
  const ref = book * 1000000 + chapter * 1000 + verse;
  const label = `${selected.name} ${chapter}:${verse}`;
  const edition = live?.state.session.translation || "KJV";
  const clear = () => {
    request.current++;
    setText("");
    setError("");
    setPending(false);
  };
  async function load() {
    const token = ++request.current;
    setPending(true);
    setError("");
    try {
      const quote = live
        ? (
            (
              await live.send({
                command: "note.passage",
                params: { ref_id: ref },
              })
            ).data.block as { quote: string }
          ).quote
        : preview?.(ref);
      if (token !== request.current) return;
      if (!quote) throw Error("This passage is unavailable in this edition.");
      setText(quote);
    } catch (e) {
      if (token === request.current) setError(String(e));
    } finally {
      if (token === request.current) setPending(false);
    }
  }
  return (
    <div className="passage-attachment">
      <Button variant="ghost" size="sm" onPress={onBack}>
        <ArrowLeft size={16} /> Attachment types
      </Button>
      <h3>Choose a Scripture passage</h3>
      <p className="muted small">
        Preview a verse, then add it to your note. Your reading position stays
        where it is.
      </p>
      <ThemedPicker
        visibleLabel
        label="Book"
        value={String(book)}
        onChange={(value) => {
          const next = books.find((b) => b.id === Number(value))!;
          setBook(next.id);
          setChapter(next.chapters[0].number);
          setVerse(1);
          clear();
        }}
        options={books.map((b) => ({ value: String(b.id), label: b.name }))}
      />
      <div className="passage-attachment-location">
        <ThemedPicker
          visibleLabel
          label="Chapter"
          value={String(chapter)}
          onChange={(value) => {
            setChapter(Number(value));
            setVerse(1);
            clear();
          }}
          options={selected.chapters.map((c) => ({
            value: String(c.number),
            label: String(c.number),
          }))}
        />
        <ThemedPicker
          visibleLabel
          label="Verse"
          value={String(verse)}
          onChange={(value) => {
            setVerse(Number(value));
            clear();
          }}
          options={Array.from(
            {
              length:
                selected.chapters.find((c) => c.number === chapter)?.verses ||
                0,
            },
            (_, i) => ({ value: String(i + 1), label: String(i + 1) }),
          )}
        />
      </div>
      <Button
        fullWidth
        variant="secondary"
        isDisabled={pending}
        onPress={() => void load()}
      >
        <BookOpen size={16} />
        {pending ? "Loading passage…" : "Preview passage"}
      </Button>
      {pending && (
        <p role="status" className="muted small">
          Loading {label}…
        </p>
      )}
      {error && <p role="alert">{error}</p>}
      {text && (
        <div className="attachment-preview">
          <div className="card-meta">
            {label} · {edition}
          </div>
          <p className="scripture">{text}</p>
          <Button
            fullWidth
            onPress={() =>
              onInsert({
                id: crypto.randomUUID(),
                kind: "passage",
                text,
                ref: label,
                source: edition,
                ...(live
                  ? {
                      native: {
                        type: "passage",
                        ref_id: ref,
                        translation_id: edition,
                        quote: text,
                      },
                    }
                  : {}),
              })
            }
          >
            <Plus size={16} />
            Add {label}
          </Button>
        </div>
      )}
    </div>
  );
}
