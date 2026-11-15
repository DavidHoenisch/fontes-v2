import { useEffect, useId, useRef, useState } from "react";
import {
  ArrowLeft,
  ArrowRight,
  BookOpen,
  ChevronLeft,
  ChevronRight,
  History,
  Library,
  Search,
  X,
} from "lucide-react";
import { Sheet } from "./Sheet";
import { ResourceCitations, type Citation } from "./ResourceCitations";
import { books, passages } from "../data/preview";
import type { Live } from "../native";
import "./search-workspace.css";
type Scope = "all" | "scripture" | "resources";
export type SearchHit = {
  id: string;
  ref_id: number | null;
  title: string;
  excerpt: string;
};
type Article = {
  id: string;
  title: string;
  content: string;
  source: string;
  author: string | null;
  links: Citation[];
};
const size = 10;
function excerpt(text: string, query: string) {
  const word = query.trim().split(/\s+/)[0] || "";
  const at = word
    ? text.toLocaleLowerCase().indexOf(word.toLocaleLowerCase())
    : -1;
  return at < 0 ? (
    text
  ) : (
    <>
      {text.slice(0, at)}
      <mark>{text.slice(at, at + word.length)}</mark>
      {text.slice(at + word.length)}
    </>
  );
}
function previewHits(query: string): SearchHit[] {
  const words = query.toLocaleLowerCase().trim().split(/\s+/);
  return passages
    .flatMap((p) =>
      p.verses.map((text, i) => {
        const ref = p.book * 1000000 + p.chapter * 1000 + i + 1;
        return {
          id: `preview:${ref}`,
          ref_id: ref,
          title: `${books.find((b) => b.id === p.book)?.name} ${p.chapter}:${i + 1}`,
          excerpt: text,
        };
      }),
    )
    .filter(
      (h) =>
        h.title.toLowerCase().replaceAll(" ", "") ===
          query.toLowerCase().replaceAll(" ", "") ||
        words.every((w) => h.excerpt.toLowerCase().includes(w)),
    );
}
export function SearchWorkspace({
  open,
  onClose,
  onRead,
  live,
}: {
  open: boolean;
  onClose: () => void;
  onRead: (ref: number) => Promise<void> | void;
  live?: Live;
}) {
  const [query, setQuery] = useState(""),
    [submitted, setSubmitted] = useState(""),
    [scope, setScope] = useState<Scope>("all");
  const [hits, setHits] = useState<SearchHit[]>([]),
    [offset, setOffset] = useState(0),
    [more, setMore] = useState(false);
  const [loading, setLoading] = useState(false),
    [error, setError] = useState(""),
    [recent, setRecent] = useState<string[]>([]);
  const [article, setArticle] = useState<Article | null>(null),
    [opening, setOpening] = useState<string | null>(null);
  const request = useRef(0),
    panelId = useId();
  useEffect(() => {
    request.current++;
    setLoading(false);
    setOpening(null);
    setError("");
    setArticle(null);
    if (open) {
      setQuery("");
      setSubmitted("");
      setHits([]);
      setOffset(0);
      setMore(false);
      setScope("all");
    }
    return () => {
      request.current++;
    };
  }, [open, live?.state.content_id, live?.state.session.translation]);
  async function search(text = query, nextScope = scope, nextOffset = 0) {
    const q = text.trim();
    const generation = ++request.current;
    setArticle(null);
    setOpening(null);
    setError("");
    setSubmitted(q);
    setQuery(text);
    setScope(nextScope);
    setHits([]);
    setMore(false);
    setOffset(nextOffset);
    if (!q) {
      setLoading(false);
      return;
    }
    setLoading(true);
    try {
      let data: { hits: SearchHit[]; has_more: boolean };
      if (live) {
        const response = await live.send({
          command: "search.query",
          params: {
            query: q,
            scope: nextScope,
            limit: size,
            offset: nextOffset,
          },
        });
        data = response.data as typeof data;
      } else {
        const all = nextScope === "resources" ? [] : previewHits(q);
        data = {
          hits: all.slice(nextOffset, nextOffset + size),
          has_more: all.length > nextOffset + size,
        };
      }
      if (generation !== request.current) return;
      setHits(data.hits);
      setMore(data.has_more);
      setRecent((r) => [q, ...r.filter((s) => s !== q)].slice(0, 5));
    } catch (e) {
      if (generation === request.current) setError(String(e));
    } finally {
      if (generation === request.current) setLoading(false);
    }
  }
  function edit(value: string) {
    request.current++;
    setQuery(value);
    setSubmitted("");
    setHits([]);
    setMore(false);
    setLoading(false);
    setOpening(null);
    setError("");
  }
  async function choose(hit: SearchHit) {
    const generation = ++request.current;
    setOpening(hit.id);
    setError("");
    try {
      if (hit.ref_id !== null) {
        await onRead(hit.ref_id);
        if (generation === request.current) onClose();
      } else if (live) {
        const response = await live.send({
          command: "study.resource",
          params: { id: hit.id },
        });
        if (generation === request.current)
          setArticle(response.data.resource as Article);
      }
    } catch (e) {
      if (generation === request.current) setError(String(e));
    } finally {
      if (generation === request.current) setOpening(null);
    }
  }
  return (
    <Sheet
      title={article?.title || "Search your library"}
      eyebrow="Search"
      open={open}
      onClose={onClose}
      className="search-workspace"
      closeLabel="Close search"
      topLine={
        <>
          <span>Fontes</span>
          <span>{live ? "Installed library" : "Sample library"}</span>
        </>
      }
    >
      {article ? (
        <>
          <button
            className="search-text-action"
            onClick={() => setArticle(null)}
          >
            <ArrowLeft size={16} />
            Back to results
          </button>
          <p className="search-source">
            {[article.source, article.author].filter(Boolean).join(" · ")}
          </p>
          <p className="resource-body">{article.content}</p>
          {live && (
            <ResourceCitations
              key={article.id}
              live={live}
              links={article.links || []}
            />
          )}
        </>
      ) : (
        <>
          <form
            className="library-search-form"
            onSubmit={(e) => {
              e.preventDefault();
              void search();
            }}
            role="search"
          >
            <Search size={18} aria-hidden="true" />
            <input
              type="search"
              aria-label="Search Scripture and resources"
              placeholder="Words or a Scripture reference"
              autoComplete="off"
              autoCapitalize="none"
              spellCheck={false}
              enterKeyHint="search"
              value={query}
              onChange={(e) => edit(e.target.value)}
            />
            {query && (
              <button
                type="button"
                className="search-clear"
                aria-label="Clear search"
                onClick={() => edit("")}
              >
                <X size={16} />
              </button>
            )}
            <button
              type="submit"
              className="search-submit"
              aria-label="Search"
              disabled={!query.trim()}
            >
              <ArrowRight size={18} />
            </button>
          </form>
          <div
            className="search-scopes"
            role="tablist"
            aria-label="Search scope"
            onKeyDown={(e) => {
              const choices: Scope[] = ["all", "scripture", "resources"],
                index = choices.indexOf(scope);
              const next =
                e.key === "Home"
                  ? 0
                  : e.key === "End"
                    ? 2
                    : e.key === "ArrowRight"
                      ? (index + 1) % 3
                      : e.key === "ArrowLeft"
                        ? (index + 2) % 3
                        : null;
              if (next === null) return;
              e.preventDefault();
              const target = choices[next];
              (
                e.currentTarget.querySelector(
                  `[data-scope="${target}"]`,
                ) as HTMLButtonElement
              )?.focus();
              if (submitted) void search(submitted, target);
              else setScope(target);
            }}
          >
            {(
              [
                ["all", "All"],
                ["scripture", "Scripture"],
                ["resources", "Resources"],
              ] as const
            ).map(([key, title]) => (
              <button
                key={key}
                role="tab"
                id={`${panelId}-${key}`}
                data-scope={key}
                aria-controls={panelId}
                aria-selected={scope === key}
                tabIndex={scope === key ? 0 : -1}
                onClick={() =>
                  submitted ? void search(submitted, key) : setScope(key)
                }
              >
                {title}
              </button>
            ))}
          </div>
          <div
            id={panelId}
            role="tabpanel"
            aria-labelledby={`${panelId}-${scope}`}
            className="search-results"
          >
            {loading && (
              <p role="status">
                Searching your {live ? "library" : "sample library"}…
              </p>
            )}
            {error && (
              <div>
                <p role="alert">{error}</p>
                <button
                  className="search-text-action"
                  onClick={() => void search(submitted, scope, offset)}
                >
                  Retry search
                </button>
              </div>
            )}
            {!loading &&
              !error &&
              (!submitted ? (
                <>
                  <p className="search-source">
                    {recent.length ? "Recent searches" : "Try a search"}
                  </p>
                  {(recent.length
                    ? recent
                    : ["trust", "grace", "John 3:16"]
                  ).map((q) => (
                    <button
                      className="search-recent"
                      key={q}
                      onClick={() => void search(q)}
                    >
                      <History size={16} />
                      <span>{q}</span>
                      <ArrowRight size={16} />
                    </button>
                  ))}
                  {!live && (
                    <p className="search-source">
                      Browser preview searches sample Scripture only. The
                      installed app searches your complete library.
                    </p>
                  )}
                </>
              ) : (
                <>
                  <p role="status" className="search-source">
                    {hits.length
                      ? `${offset + 1}–${offset + hits.length}${more ? " · more matches available" : ""}`
                      : `No matches for “${submitted}”`}
                  </p>
                  {!hits.length && (
                    <p className="search-source">
                      Try a shorter word, a different scope, or a reference such
                      as John 3:16.
                    </p>
                  )}
                  {hits.map((hit) => (
                    <button
                      className="search-result"
                      key={hit.id}
                      disabled={opening !== null}
                      onClick={() => void choose(hit)}
                    >
                      {hit.ref_id === null ? (
                        <Library size={18} />
                      ) : (
                        <BookOpen size={18} />
                      )}
                      <span className="search-result-copy">
                        <strong>{hit.title}</strong>
                        <small>
                          {hit.ref_id === null
                            ? "Study resource"
                            : live?.state.translations.find(
                                (t) => t.id === live.state.session.translation,
                              )?.name || "Sample Scripture"}
                        </small>
                        <span className="search-excerpt">
                          {excerpt(hit.excerpt, submitted)}
                        </span>
                        {opening === hit.id && (
                          <small role="status">Opening…</small>
                        )}
                      </span>
                      <ChevronRight size={16} />
                    </button>
                  ))}
                  {(offset > 0 || more) && (
                    <nav
                      className="search-pagination"
                      aria-label="Search result pages"
                    >
                      <button
                        disabled={!offset}
                        onClick={() =>
                          void search(
                            submitted,
                            scope,
                            Math.max(0, offset - size),
                          )
                        }
                      >
                        <ChevronLeft size={16} />
                        Previous
                      </button>
                      <span>{Math.floor(offset / size) + 1}</span>
                      <button
                        disabled={!more}
                        onClick={() =>
                          void search(submitted, scope, offset + size)
                        }
                      >
                        Next
                        <ChevronRight size={16} />
                      </button>
                    </nav>
                  )}
                </>
              ))}
          </div>
          <button className="search-return" onClick={onClose}>
            <ArrowLeft size={16} />
            Back to reading
          </button>
        </>
      )}
    </Sheet>
  );
}
