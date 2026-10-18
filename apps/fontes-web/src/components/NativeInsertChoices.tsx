import { PassageAttachment } from "./PassageAttachment";
import { useEffect, useRef, useState } from "react";
import { Button } from "@heroui/react";
import { BookOpen, NotebookPen, Plus, Quote, Type } from "lucide-react";
import type { Live } from "../native";
import type { Block } from "../data/preview";
type Resource = {
  id: string;
  title: string;
  author: string | null;
  kind: string;
  excerpt: string;
};
// In-memory, bounded and keyed to the exact snapshot, edition and verse.
const contexts = new Map<string, Promise<Resource[]>>();
export function clearInsertContextCache() {
  contexts.clear();
}
function related(live: Live, ref: number) {
  const key = `${live.state.content_id}:${live.state.session.translation}:${ref}`;
  let result = contexts.get(key);
  if (!result) {
    result = live
      .send({ command: "study.context", params: { ref_id: ref } })
      .then((r) => r.data.resources as Resource[])
      .catch((error) => {
        if (contexts.get(key) === result) contexts.delete(key);
        throw error;
      });
    contexts.set(key, result);
    if (contexts.size > 16) contexts.delete(contexts.keys().next().value!);
  }
  return result;
}
export function NativeInsertChoices({
  live,
  onInsert,
}: {
  live: Live;
  onInsert: (block: Block) => void;
}) {
  const ref = live.state.session.selected_ref;
  const [choosingPassage, setChoosingPassage] = useState(false);
  const [resources, setResources] = useState<Resource[] | null>(null),
    [error, setError] = useState(""),
    [pending, setPending] = useState<string | null>(null),
    [retry, setRetry] = useState(0);
  const active = useRef(true);
  useEffect(() => {
    active.current = true;
    let current = true;
    setResources(null);
    setError("");
    void related(live, ref)
      .then((r) => {
        if (current) setResources(r);
      })
      .catch((e) => {
        if (current) setError(String(e));
      });
    return () => {
      current = false;
      active.current = false;
    };
  }, [live.state.content_id, live.state.session.translation, ref, retry]);
  async function choose(resource: Resource) {
    setPending(resource.id);
    setError("");
    try {
      const response = await live.send({
        command: "study.resource",
        params: { id: resource.id },
      });
      if (!active.current) return;
      const detail = response.data.resource as {
        id: string;
        content: string;
        title: string;
        author: string | null;
        source: string;
      };
      const quote = Array.from(detail.content).slice(0, 800).join("");
      if (!quote.trim())
        throw new Error("This entry has no excerpt to collect.");
      onInsert({
        id: crypto.randomUUID(),
        kind: resource.kind === "dictionary" ? "word" : "resource",
        text: quote,
        ref: detail.title,
        source: detail.source,
        native: {
          type: "resource",
          resource_id: detail.id,
          quote,
          title: detail.title,
          author: detail.author,
          source: detail.source,
          content_id: response.state.content_id,
        },
      });
    } catch (e) {
      if (active.current) setError(String(e));
    } finally {
      if (active.current) setPending(null);
    }
  }
  if (choosingPassage) return <PassageAttachment live={live} initialRef={ref}
    books={live.state.books.map(b => ({ id: b.id, name: b.name, chapters: b.verse_counts.map((verses, i) => ({number: i + 1, verses})) }))}
    onInsert={onInsert} onBack={() => setChoosingPassage(false)} />;
  return (
    <div className="space-y-3">
      <Button
        variant="tertiary"
        fullWidth
        className="insert-choice"
        onPress={() =>
          onInsert({ id: crypto.randomUUID(), kind: "thought", text: "" })
        }
      >
        <NotebookPen />
        <span>
          <strong>Your thought</strong>
          <small>Write between your collected pieces</small>
        </span>
        <Plus size={16} />
      </Button>
      <Button variant="tertiary" fullWidth className="insert-choice" onPress={() => setChoosingPassage(true)}>
        <BookOpen/><span><strong>Scripture passage</strong><small>Choose a book, chapter and verse</small></span><Plus size={16}/>
      </Button>
      <div className="eyebrow pt-4">Related resources</div>
      {resources === null && !error && (
        <p className="muted small" role="status">
          Finding related resources…
        </p>
      )}
      {resources?.slice(0, 8).map((resource) => (
        <Button
          key={resource.id}
          variant="tertiary"
          fullWidth
          className="insert-choice"
          isDisabled={pending !== null}
          onPress={() => void choose(resource)}
        >
          {resource.kind === "dictionary" ? <Type /> : <Quote />}
          <span>
            <strong>{resource.title}</strong>
            <small>
              {pending === resource.id
                ? "Adding excerpt…"
                : resource.author || resource.kind}
            </small>
          </span>
          <Plus size={16} />
        </Button>
      ))}
      {resources?.length === 0 && (
        <p className="muted small">No related resources for this verse.</p>
      )}
      {pending && (
        <p role="status" className="muted small">
          Preparing the selected excerpt…
        </p>
      )}
      {error && (
        <div>
          <p role="alert" className="muted small">
            {error}
          </p>
          {resources === null && (
            <Button variant="ghost" onPress={() => setRetry((n) => n + 1)}>
              Retry resources
            </Button>
          )}
        </div>
      )}
    </div>
  );
}
