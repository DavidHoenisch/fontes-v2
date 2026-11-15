"use client";
import { useEffect, useRef, useState, type ReactNode } from "react";
import {
  Button,
  Card,
  Chip,
  Input,
  Label,
  ListBox,
  Select,
  Spinner,
} from "@heroui/react";
import {
  ArrowLeft,
  ArrowRight,
  FileText,
  Upload,
  ShieldCheck,
} from "lucide-react";
import {
  PublisherReviewActions,
  PublisherBlockEditor,
} from "./publisher-review-controls";
import type { ReviewReply, ReviewPage } from "@/lib/epub-review-contracts";
async function call<T>(path: string, options?: RequestInit) {
  const r = await fetch(path, { cache: "no-store", ...options });
  const body = await r.json();
  if (!r.ok)
    throw new Error(
      [
        body.error?.message ?? "The review request failed.",
        ...(body.error?.details?.diagnostics ?? [])
          .slice(0, 3)
          .map((d: { message?: string; code?: string }) => d.message ?? d.code),
      ].join(" "),
    );
  return body.data as T;
}
export function OrganizationEpubReview({
  organizationId,
  resource,
  onBack,
}: {
  organizationId: string;
  resource: { id: string; title: string };
  onBack: () => void;
}) {
  const base = `/api/v1/organizations/${organizationId}/resources/${resource.id}/preparation`;
  const [reply, setReply] = useState<ReviewReply | null>(null),
    [page, setPage] = useState<ReviewPage | null>(null),
    [error, setError] = useState(""),
    [busy, setBusy] = useState(false),
    [loading, setLoading] = useState(false),
    [showReport, setShowReport] = useState(false),
    [refresh, setRefresh] = useState(0);
  const [location, setLocation] = useState<{
      documentId: string;
      offset: number;
      blockId?: string;
    } | null>(null),
    [previous, setPrevious] = useState<typeof location>(null);
  const file = useRef<HTMLInputElement>(null),
    article = useRef<HTMLElement>(null);
  useEffect(() => {
    let active = true;
    void call<ReviewReply>(base)
      .then((r) => {
        if (active) {
          setReply(r);
          if (r.manifest)
            setLocation({ documentId: r.manifest.documents[0].id, offset: 0 });
        }
      })
      .catch((e) => active && setError(e.message));
    return () => {
      active = false;
    };
  }, [base]);
  useEffect(() => {
    if (!location) return;
    const controller = new AbortController();
    setLoading(true);
    setError("");
    const query = new URLSearchParams({
      document: location.documentId,
      offset: String(location.offset),
      ...(location.blockId ? { block: location.blockId } : {}),
    });
    void call<ReviewPage>(`${base}/document?${query}`, {
      signal: controller.signal,
    })
      .then((p) => {
        if (!controller.signal.aborted) setPage(p);
      })
      .catch((e) => {
        if (!controller.signal.aborted) setError(e.message);
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });
    return () => controller.abort();
  }, [base, location, refresh]);
  useEffect(() => {
    if (page?.focusBlockId) {
      article.current
        ?.querySelector(`[id="${page.focusBlockId}"]`)
        ?.scrollIntoView({ block: "center", behavior: "smooth" });
    }
  }, [page]);
  async function command(command: Record<string, unknown>) {
    if (!reply) return false;
    setBusy(true);
    setError("");
    try {
      const r = await call<ReviewReply>(base, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          resourceRevision: reply.resourceRevision,
          organizationRevision: reply.organizationRevision,
          reviewRevision: reply.reviewRevision,
          command,
        }),
      });
      setReply(r);
      setRefresh((v) => v + 1);
      return true;
    } catch (e) {
      setError(e instanceof Error ? e.message : "Unable to save review.");
      return false;
    } finally {
      setBusy(false);
    }
  }
  async function cover(file: File) {
    if (!reply) return;
    setBusy(true);
    setError("");
    try {
      setReply(
        await call<ReviewReply>(`${base}/cover`, {
          method: "POST",
          headers: {
            "Content-Type": file.type,
            "X-Fontes-Revision": String(reply.resourceRevision),
            "X-Fontes-Organization-Revision": String(
              reply.organizationRevision,
            ),
            "X-Fontes-Review-Revision": String(reply.reviewRevision),
          },
          body: file,
        }),
      );
    } catch (e) {
      setError(e instanceof Error ? e.message : "Unable to upload cover.");
    } finally {
      setBusy(false);
    }
  }
  const manifest = reply?.manifest;
  const jump = (target: { documentId: string; blockId?: string }) => {
    if (page) setPrevious({ documentId: page.id, offset: page.offset });
    setLocation({ ...target, offset: 0 });
  };
  const renderSegments = (block: ReviewPage["blocks"][number]) =>
    block.segments.map((segment, i) => {
      if (segment.lineBreak) return <br key={i} />;
      let content: ReactNode = segment.text;
      for (const mark of segment.marks) {
        content =
          mark === "emphasis" ? (
            <em>{content}</em>
          ) : mark === "strong" ? (
            <strong>{content}</strong>
          ) : (
            <sup>{content}</sup>
          );
      }
      return segment.target && segment.text.trim() ? (
        <a
          key={i}
          href={`#${segment.target.blockId ?? segment.target.documentId}`}
          onClick={(e) => {
            e.preventDefault();
            jump(segment.target!);
          }}
        >
          {content}
        </a>
      ) : (
        <span
          key={i}
          title={
            segment.unresolved
              ? `Unresolved source link: ${segment.unresolved}`
              : undefined
          }
        >
          {content}
        </span>
      );
    });
  return (
    <section className="epub-review-workspace">
      <div className="org-toolbar">
        <Button variant="ghost" onPress={onBack} isDisabled={busy}>
          <ArrowLeft size={16} />
          Back to organization
        </Button>
        <Button
          variant="ghost"
          isDisabled={busy}
          onPress={() => {
            setError("");
            void call<ReviewReply>(base)
              .then((r) => {
                setReply(r);
                setRefresh((v) => v + 1);
              })
              .catch((e) => setError(e.message));
          }}
        >
          Reload saved review
        </Button>
        <Chip variant="secondary">
          {reply?.approved ? "Publisher approved" : "Publisher review"}
        </Chip>
      </div>
      <div>
        <p className="eyebrow">ORGANIZATION SOURCE REVIEW</p>
        <h2>{manifest?.title ?? resource.title}</h2>
        {manifest && (
          <p className="muted">
            {manifest.authors.join(", ")}
            {manifest.contributors.length
              ? ` · Contributor/translator: ${manifest.contributors.join(", ")}`
              : ""}{" "}
            · {manifest.publisher}
          </p>
        )}
      </div>
      {error && (
        <p className="form-error" role="alert">
          {error}
        </p>
      )}
      <Card>
        <Card.Content>
          <div className="org-toolbar">
            <div>
              <h3>
                {manifest
                  ? "Source retained for review"
                  : "Bring a real book into your draft"}
              </h3>
              <p className="muted">
                EPUB bytes and reading text stay in private local storage.
                Postgres keeps inventory and audit metadata. Only the
                organization’s publishing team can open this draft.
              </p>
            </div>
            <Button isPending={busy} onPress={() => file.current?.click()}>
              <Upload size={16} />
              {manifest ? "Replace EPUB" : "Upload EPUB for evaluation"}
            </Button>
          </div>
          <input
            ref={file}
            type="file"
            className="sr-only"
            accept=".epub,application/epub+zip"
            aria-label="Choose EPUB for organization evaluation"
            onChange={(e) => {
              const chosen = e.target.files?.[0];
              e.target.value = "";
              if (!chosen || !reply) return;
              setBusy(true);
              setError("");
              void call<ReviewReply>(base, {
                method: "POST",
                headers: {
                  "Content-Type": "application/epub+zip",
                  "X-Fontes-Revision": String(reply.resourceRevision),
                  "X-Fontes-Organization-Revision": String(
                    reply.organizationRevision,
                  ),
                },
                body: chosen,
              })
                .then((r) => {
                  setReply(r);
                  setPrevious(null);
                  setLocation({
                    documentId: r.manifest!.documents[0].id,
                    offset: 0,
                  });
                })
                .catch(async (e) => {
                  setError(e.message);
                  try {
                    const current = await call<ReviewReply>(base);
                    setReply(current);
                  } catch {}
                })
                .finally(() => setBusy(false));
            }}
          />
          {!reply && !error && <Spinner />}
          {manifest && (
            <>
              <div className="epub-metrics">
                {[
                  ["Sections", manifest.statistics.documents],
                  ["Text / image blocks", manifest.statistics.blocks],
                  ["Footnotes", manifest.statistics.footnotes],
                  ["Source links", manifest.statistics.resolvedLinks],
                  [
                    "Scripture candidates",
                    manifest.statistics.scriptureCandidates,
                  ],
                ].map(([label, value]) => (
                  <div key={label}>
                    <strong>{Number(value).toLocaleString()}</strong>
                    <span>{label}</span>
                  </div>
                ))}
              </div>
              <p className="epub-fidelity">
                <ShieldCheck size={17} />
                The original extraction matched all non-whitespace source text
                in reading order. This check does not establish visual fidelity
                or reference correctness. Saved corrections are separate from
                this original-extraction check.
              </p>
              <Button
                variant="secondary"
                onPress={() => setShowReport(!showReport)}
              >
                <FileText size={16} />
                {showReport
                  ? "Hide conversion report"
                  : "Conversion report & rights notice"}
              </Button>
              {showReport && (
                <div className="epub-report">
                  <h3>Conversion report</h3>
                  <ul>
                    {manifest.warnings.map((w) => (
                      <li key={w}>{w}</li>
                    ))}
                  </ul>
                  <h3>Source rights statement</h3>
                  <p>
                    {manifest.declaredRights ||
                      "No package rights statement supplied."}
                  </p>
                  {manifest.publicationNotice && (
                    <div className="epub-notice">
                      {manifest.publicationNotice}
                    </div>
                  )}
                  <p className="muted">
                    Rights review is required. Publisher preparation approval
                    and source generation do not authorize member delivery, AI
                    processing or public publication.
                  </p>
                  <p className="muted">
                    Source SHA-256: <code>{manifest.sourceSha256}</code>
                  </p>
                </div>
              )}
            </>
          )}
        </Card.Content>
      </Card>
      {reply && (
        <PublisherReviewActions
          reply={reply}
          busy={busy}
          base={base}
          onCommand={command}
          onCover={cover}
          error={error}
        />
      )}
      {manifest && (
        <Card>
          <Card.Header>
            <Card.Title>Read the converted draft</Card.Title>
          </Card.Header>
          <Card.Content>
            <div className="org-toolbar">
              <Button
                variant="secondary"
                isDisabled={busy || loading || !reply?.draft?.pending}
                onPress={() => {
                  if (!page) return;
                  setLoading(true);
                  setError("");
                  const query = new URLSearchParams({
                    document: page.id,
                    pending: "next",
                    ...(page.focusBlockId ? { after: page.focusBlockId } : {}),
                  });
                  void call<ReviewPage>(`${base}/document?${query}`)
                    .then((p) =>
                      setLocation({
                        documentId: p.id,
                        offset: p.offset,
                        blockId: p.focusBlockId ?? undefined,
                      }),
                    )
                    .catch((e) => setError(e.message))
                    .finally(() => setLoading(false));
                }}
              >
                Next unreviewed reference
              </Button>
              <span className="muted">
                {reply?.draft?.pending.toLocaleString()} references need a
                decision
              </span>
            </div>
            <Select
              aria-label="Book section"
              selectedKey={location?.documentId ?? null}
              onSelectionChange={(key) =>
                key && setLocation({ documentId: String(key), offset: 0 })
              }
            >
              <Label>Book / chapter / section</Label>
              <Select.Trigger>
                <Select.Value />
              </Select.Trigger>
              <Select.Popover>
                <ListBox>
                  {manifest.documents.map((d, i) => (
                    <ListBox.Item
                      id={d.id}
                      key={d.id}
                      textValue={`${i + 1}. ${d.title}`}
                    >
                      {i + 1}. {d.title}
                      <ListBox.ItemIndicator />
                    </ListBox.Item>
                  ))}
                </ListBox>
              </Select.Popover>
            </Select>
            <div className="org-toolbar epub-pages">
              <Button
                variant="secondary"
                isDisabled={loading || !page || page.previousOffset === null}
                onPress={() =>
                  page &&
                  setLocation({
                    documentId: page.id,
                    offset: page.previousOffset!,
                  })
                }
              >
                <ArrowLeft size={16} />
                Previous page
              </Button>
              <span className="muted">
                {page
                  ? `Blocks ${page.offset + 1}–${Math.min(page.offset + 40, page.total)} of ${page.total}`
                  : "Loading section…"}
              </span>
              <Button
                variant="secondary"
                isDisabled={loading || !page || page.nextOffset === null}
                onPress={() =>
                  page &&
                  setLocation({ documentId: page.id, offset: page.nextOffset! })
                }
              >
                Next page
                <ArrowRight size={16} />
              </Button>
            </div>
            {previous && (
              <Button
                variant="ghost"
                onPress={() => {
                  setLocation(previous);
                  setPrevious(null);
                }}
              >
                <ArrowLeft size={14} />
                Return to previous location
              </Button>
            )}
            {loading ? (
              <div className="org-loading">
                <Spinner />
                Loading this section…
              </div>
            ) : (
              page && (
                <article className="epub-reading" ref={article}>
                  <h3>{page.title}</h3>
                  {page.blocks.map((block) => (
                    <section
                      key={block.id}
                      id={block.id}
                      className={`epub-block ${block.kind === "note" ? "epub-footnote" : ""} ${block.poetry ? "epub-poetry" : ""} ${page.focusBlockId === block.id ? "epub-focused" : ""}`}
                    >
                      {block.kind === "image" ? (
                        <img
                          className="epub-source-image"
                          src={`${base}/assets/${block.image}`}
                          alt={block.alt ?? "Source image"}
                        />
                      ) : block.kind === "heading" ? (
                        <h4>{renderSegments(block)}</h4>
                      ) : (
                        <p>{renderSegments(block)}</p>
                      )}
                      {block.kind === "note" && (
                        <span className="epub-block-label">
                          Source footnote
                        </span>
                      )}
                      <PublisherBlockEditor
                        block={block}
                        documentId={page.id}
                        busy={busy}
                        onCommand={command}
                        error={error}
                      />
                      {block.references.length > 0 && (
                        <details className="epub-candidates">
                          <summary>
                            {block.references.length} Scripture candidate
                            {block.references.length === 1 ? "" : "s"}
                          </summary>
                          <ul>
                            {block.references.map((r) => (
                              <li key={r.id}>
                                “{r.evidence}” → {r.start}
                                {r.end ? `–${r.end}` : ""} · {r.decision} ·{" "}
                                {r.method === "literal-reference-v1"
                                  ? "detector suggestion"
                                  : "publisher correction"}
                              </li>
                            ))}
                          </ul>
                        </details>
                      )}
                    </section>
                  ))}
                </article>
              )
            )}
          </Card.Content>
        </Card>
      )}
    </section>
  );
}
