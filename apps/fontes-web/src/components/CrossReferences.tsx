import { useEffect, useId, useRef, useState } from "react";
import { Button } from "@heroui/react";
import {
  ArrowLeft,
  ChevronLeft,
  ChevronRight,
  ChevronDown,
  BookOpen,
  BookOpenText,
  BookA,
  Tags,
  GitBranch,
} from "lucide-react";
import { Sheet } from "./Sheet";
import type { Live } from "../native";
import { createLookupCache } from "../lookupCache";
import { ResourceCitations, type Citation } from "./ResourceCitations";
type Target = {
  start_ref: number;
  end_ref: number;
  label: string;
  preview: string | null;
  verses?: Verse[];
};
type Resource = {
  id: string;
  title: string;
  author: string | null;
  kind: string;
  basis: string;
  excerpt: string;
};
type Detail = {
  id: string;
  title: string;
  author: string | null;
  source: string;
  content: string;
  links: Citation[];
};
const kindLabel = (kind: string) =>
  ({
    commentary: "Commentary",
    dictionary: "Dictionaries",
    topic: "Topics",
    cross_reference: "Cross-reference articles",
  })[kind] || kind.replaceAll("_", " ");
const kindIcon = (kind: string) =>
  ({
    __crossrefs: GitBranch,
    commentary: BookOpenText,
    dictionary: BookA,
    topic: Tags,
    cross_reference: GitBranch,
  })[kind] || BookOpen;
const basisLabel = (basis: string) =>
  ({
    applies_to: "Covers this verse",
    citation: "Cites this verse",
    cites: "Cites this verse",
    cross_reference: "Cross-reference to this verse",
    quotation: "Quotes this verse",
    allusion: "Alludes to this verse",
  })[basis] || basis.replaceAll("_", " ");
type Verse = { ref_id: number; label: string; text: string };
export function CrossReferences({
  reference,
  onClose,
  live,
}: {
  reference: { id: number; label: string } | null;
  onClose: () => void;
  live?: Live;
}) {
  const [targets, setTargets] = useState<Target[]>([]),
    [active, setActive] = useState<Target | null>(null),
    [verses, setVerses] = useState<Verse[]>([]),
    [loading, setLoading] = useState(false),
    [error, setError] = useState(""),
    [more, setMore] = useState(false);
  const [resources, setResources] = useState<Resource[]>([]);
  const [article, setArticle] = useState<Resource | null>(null);
  const [detail, setDetail] = useState<Detail | null>(null);
  const [retry, setRetry] = useState(0);
  const [expanded, setExpanded] = useState<string | null>("__crossrefs");
  const [pages, setPages] = useState<Record<string, number>>({});
  const [contexts] = useState(() => createLookupCache());
  const sectionId = useId();
  const request = useRef(0);
  useEffect(() => {
    contexts.clear();
  }, [live?.state.content_id, live?.state.session.translation]);
  useEffect(() => {
    const generation = ++request.current;
    setArticle(null);
    setDetail(null);
    setResources([]);
    setActive(null);
    setVerses([]);
    setError("");
    setTargets([]);
    setMore(false);
    setExpanded("__crossrefs");
    setPages({});
    if (!reference) return;
    setLoading(true);
    void (async () => {
      try {
        let refs: Target[];
        if (live) {
          const data = await contexts.load(
            JSON.stringify([
              live.state.content_id,
              live.state.session.translation,
              reference.id,
            ]),
            async () =>
              (
                await live.send({
                  command: "study.context",
                  params: { ref_id: reference.id },
                })
              ).data,
          );
          refs = (data.cross_references || []) as Target[];
          if (generation === request.current)
            setResources((data.resources || []) as Resource[]);
        } else {
          const data = (await import("../data/cross-references.json"))
            .default as Record<string, Target[]>;
          refs = data[String(reference.id)] || [];
        }
        if (generation === request.current) setTargets(refs);
      } catch (e) {
        if (generation === request.current) setError(String(e));
      } finally {
        if (generation === request.current) setLoading(false);
      }
    })();
    return () => {
      request.current++;
    };
  }, [
    reference?.id,
    live?.state.content_id,
    live?.state.session.translation,
    retry,
  ]);
  async function preview(target: Target) {
    const generation = ++request.current;
    setActive(target);
    setLoading(true);
    setError("");
    setVerses([]);
    setMore(false);
    try {
      if (live) {
        const r = await live.send({
          command: "reading.preview",
          params: { start_ref: target.start_ref, end_ref: target.end_ref },
        });
        if (generation === request.current) {
          setVerses(r.data.verses as Verse[]);
          setMore(Boolean(r.data.has_more));
        }
      } else if (generation === request.current) setVerses(target.verses || []);
    } catch (e) {
      if (generation === request.current) setError(String(e));
    } finally {
      if (generation === request.current) setLoading(false);
    }
  }
  async function openResource(resource: Resource) {
    if (!live) return;
    const generation = ++request.current;
    setArticle(resource);
    setDetail(null);
    setLoading(true);
    setError("");
    try {
      const response = await live.send({
        command: "study.resource",
        params: { id: resource.id },
      });
      if (generation === request.current)
        setDetail(response.data.resource as Detail);
    } catch (e) {
      if (generation === request.current) setError(String(e));
    } finally {
      if (generation === request.current) setLoading(false);
    }
  }
  // A single article can match through several separately supplied link types.
  const unique = resources.filter(
    (r, i) => resources.findIndex((other) => other.id === r.id) === i,
  );
  const groups = [...new Set(unique.map((r) => r.kind))];
  return (
    <Sheet
      title={
        article?.title || active?.label || reference?.label || "Verse resources"
      }
      open={reference !== null}
      onClose={onClose}
      className="verse-resources-sheet"
      eyebrow="Verse resources"
      description={
        !active && !article ? (
          <>
            Linked to this verse ·{" "}
            {live
              ? live.state.translations.find(
                  (t) => t.id === live.state.session.translation,
                )?.abbr
              : "KJV"}
          </>
        ) : undefined
      }
    >
      {active || article ? (
        <Button
          variant="ghost"
          size="sm"
          className="mb-4"
          onPress={() => {
            request.current++;
            setActive(null);
            setArticle(null);
            setDetail(null);
            setLoading(false);
            setError("");
          }}
        >
          <ArrowLeft size={16} /> Back to {reference?.label}
        </Button>
      ) : null}
      {loading && (
        <p role="status" className="muted">
          Loading{" "}
          {article ? "resource" : active ? "passage" : "verse resources"}…
        </p>
      )}
      {error && (
        <div>
          <p role="alert">{error}</p>
          <Button
            variant="ghost"
            onPress={() =>
              article
                ? void openResource(article)
                : active
                  ? void preview(active)
                  : setRetry((n) => n + 1)
            }
          >
            Retry
          </Button>
        </div>
      )}
      {!loading &&
        !error &&
        (article ? (
          detail && (
            <article>
              <p className="muted small">
                {[detail.source, detail.author].filter(Boolean).join(" · ")}
              </p>
              <p className="resource-body">{detail.content}</p>
              {live && (
                <ResourceCitations
                  key={detail.id}
                  live={live}
                  links={detail.links || []}
                />
              )}
            </article>
          )
        ) : active ? (
          <div className="crossref-passage">
            {verses.length ? (
              verses.map((v) => (
                <div key={v.ref_id}>
                  <small className="muted">{v.label}</small>
                  <p className="scripture">{v.text}</p>
                </div>
              ))
            ) : (
              <p className="muted">
                This passage is not available in the active translation.
              </p>
            )}
            {more && (
              <p className="muted small">
                Showing the first 100 verse segments in this range.
              </p>
            )}
          </div>
        ) : (
          <div className="verse-resource-groups">
            {["__crossrefs", ...groups].map((kind) => {
              const isReferences = kind === "__crossrefs";
              const title = isReferences ? "Cross-references" : kindLabel(kind);
              const entries = unique.filter((r) => r.kind === kind);
              const count = isReferences ? targets.length : entries.length;
              const size = isReferences ? 6 : 4;
              const page = pages[kind] || 0;
              const offset = page * size;
              const open = expanded === kind;
              const id = `${sectionId}-${kind}`;
              const Icon = kindIcon(kind);
              return (
                <section
                  className="verse-resource-group"
                  key={kind}
                  aria-label={title}
                >
                  <h3>
                    <Button
                      variant="ghost"
                      fullWidth
                      className="verse-resource-toggle"
                      id={`${id}-toggle`}
                      aria-expanded={open}
                      aria-controls={id}
                      onPress={() => setExpanded(open ? null : kind)}
                    >
                      <Icon size={18} aria-hidden="true" />
                      <span className="verse-resource-label">{title}</span>
                      <span className="verse-resource-count">{count}</span>
                      <ChevronDown
                        size={18}
                        className={open ? "expanded" : undefined}
                      />
                    </Button>
                  </h3>
                  <div
                    id={id}
                    hidden={!open}
                    className="verse-resource-group-body"
                    aria-labelledby={`${id}-toggle`}
                  >
                    {isReferences ? (
                      <>
                        <p className="muted small mb-3">
                          Treasury of Scripture Knowledge
                        </p>
                        <div className="verse-reference-grid">
                          {targets.slice(offset, offset + size).map((t) => (
                            <Button
                              key={`${t.start_ref}-${t.end_ref}`}
                              variant="tertiary"
                              className="verse-reference"
                              onPress={() => void preview(t)}
                            >
                              {t.label}
                            </Button>
                          ))}
                        </div>
                        {!count && (
                          <p className="muted small">
                            {live
                              ? "No cross-references are supplied for this verse in your installed library."
                              : "No TSK sample for this verse in the browser preview. Try Proverbs 16:1–4."}
                          </p>
                        )}
                        {targets.length === 100 && (
                          <p className="muted small mt-3">
                            Showing the first 100 cross-reference ranges.
                          </p>
                        )}
                      </>
                    ) : (
                      <div className="crossref-list">
                        {entries
                          .slice(offset, offset + size)
                          .map((resource) => (
                            <Button
                              key={resource.id}
                              variant="ghost"
                              fullWidth
                              onPress={() => void openResource(resource)}
                            >
                              <span>
                                <strong>{resource.title}</strong>
                                <small>
                                  {[
                                    resource.author,
                                    ...new Set(
                                      resources
                                        .filter((r) => r.id === resource.id)
                                        .map((r) => basisLabel(r.basis)),
                                    ),
                                  ]
                                    .filter(Boolean)
                                    .join(" · ")}
                                </small>
                              </span>
                              <ChevronRight size={18} />
                            </Button>
                          ))}
                      </div>
                    )}
                    {count > size && (
                      <nav
                        className="verse-resource-pager"
                        aria-label={`${title} pages`}
                      >
                        <Button
                          variant="ghost"
                          size="sm"
                          aria-label={`Previous ${title.toLowerCase()} page`}
                          isDisabled={page === 0}
                          onPress={() =>
                            setPages((p) => ({ ...p, [kind]: page - 1 }))
                          }
                        >
                          <ChevronLeft size={16} /> Previous
                        </Button>
                        <span className="muted small" aria-live="polite">
                          {offset + 1}–{Math.min(offset + size, count)} of{" "}
                          {count}
                        </span>
                        <Button
                          variant="ghost"
                          size="sm"
                          aria-label={`Next ${title.toLowerCase()} page`}
                          isDisabled={offset + size >= count}
                          onPress={() =>
                            setPages((p) => ({ ...p, [kind]: page + 1 }))
                          }
                        >
                          Next <ChevronRight size={16} />
                        </Button>
                      </nav>
                    )}
                  </div>
                </section>
              );
            })}
            {resources.length === 100 && (
              <p className="muted small mt-3">
                Showing the first 100 article matches. More may be available in
                Library.
              </p>
            )}
            {!live && (
              <p className="muted small mt-3">
                Other linked resources are available from your installed library
                in the app.
              </p>
            )}
            {live && !resources.length && (
              <p className="muted small mt-3">
                No linked articles are supplied for this verse in your installed
                library.
              </p>
            )}
          </div>
        ))}
    </Sheet>
  );
}
