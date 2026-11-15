"use client";
import { useEffect, useState } from "react";
import {
  Button,
  Card,
  Chip,
  Checkbox,
  Input,
  Label,
  Modal,
  TextField,
  TextArea,
  Select,
  ListBox,
  Spinner,
  Tabs,
} from "@heroui/react";
import {
  ArrowLeft,
  ArrowRight,
  Download,
  FileCheck,
  Send,
  RefreshCw,
} from "lucide-react";
import type {
  SourceReviewDetail,
  SourceReviewItem,
} from "@/lib/source-review-contracts";
import type { Actor } from "@/lib/contracts";
async function call<T>(url: string, options?: RequestInit): Promise<T> {
  const r = await fetch(url, { cache: "no-store", ...options });
  const b = await r.json();
  if (!r.ok)
    throw new Error(
      [
        b.error?.message ?? "The source review request failed.",
        ...(b.error?.details?.diagnostics ?? [])
          .slice(0, 3)
          .map((d: { message?: string }) => d.message),
      ].join(" "),
    );
  return b.data;
}
const labels: Record<string, string> = {
  submitted: "Awaiting review",
  changes_requested: "Changes requested",
  approved: "Maintainer approved",
  handoff_ready: "Publication handoff ready",
  withdrawn: "Withdrawn",
  superseded: "Superseded by publisher edits",
};
export function SubmitSourceReview({
  source,
  revision,
  disabled = false,
}: {
  source:
    | {
        kind: "organization";
        organizationId: string;
        resourceId: string;
        organizationRevision: number;
        reviewRevision: number;
      }
    | { kind: "personal"; resourceId: string };
  revision: number;
  disabled?: boolean;
}) {
  const [open, setOpen] = useState(false),
    [consent, setConsent] = useState(false),
    [notes, setNotes] = useState(""),
    [error, setError] = useState(""),
    [busy, setBusy] = useState(false),
    [submitted, setSubmitted] = useState<SourceReviewDetail | null>(null);
  useEffect(() => {
    setSubmitted(null);
  }, [revision]);
  return (
    <>
      <Button
        variant="secondary"
        isDisabled={disabled}
        onPress={() => {
          setError("");
          setOpen(true);
        }}
      >
        <Send size={16} />
        Submit package to Fontes
      </Button>
      {submitted && (
        <p role="status">
          Submitted for maintainer review. Open{" "}
          <a href="/?view=review">Review queue</a> to follow feedback. Review
          ID: {submitted.item.id}
        </p>
      )}
      <Modal isOpen={open} onOpenChange={(v) => !busy && setOpen(v)}>
        <Modal.Backdrop>
          <Modal.Container size="lg">
            <Modal.Dialog>
              <Modal.CloseTrigger isDisabled={busy} />
              <Modal.Header>
                <Modal.Heading>Submit for public Fontes review</Modal.Heading>
              </Modal.Header>
              <Modal.Body>
                <div className="resource-form">
                  <p>
                    This sends a frozen copy of the approved source package,
                    metadata and rights declarations to Fontes maintainers for
                    public-free publication review. Your organization’s other
                    drafts stay private.
                  </p>
                  <p className="muted">
                    Submission creates a review record. Publication still
                    requires independent rights, publisher, content and
                    reference review, followed by the trusted release workflow.
                    Keep private agreements out of the source package.
                  </p>
                  {error && (
                    <p className="form-error" role="alert">
                      {error}
                    </p>
                  )}
                  <TextField
                    value={notes}
                    onChange={setNotes}
                    isDisabled={busy}
                  >
                    <Label>Release notes / changes for reviewers</Label>
                    <TextArea rows={4} minLength={10} maxLength={4000} />
                  </TextField>
                  <Checkbox
                    isSelected={consent}
                    onChange={setConsent}
                    isDisabled={busy}
                  >
                    <Checkbox.Content>
                      <Checkbox.Control>
                        <Checkbox.Indicator />
                      </Checkbox.Control>
                      <Label>
                        I authorize sharing this exact package with Fontes
                        maintainers for public-free publication consideration.
                      </Label>
                    </Checkbox.Content>
                  </Checkbox>
                </div>
              </Modal.Body>
              <Modal.Footer>
                <Button
                  variant="secondary"
                  isDisabled={busy}
                  onPress={() => setOpen(false)}
                >
                  Cancel
                </Button>
                <Button
                  isPending={busy}
                  isDisabled={!consent || notes.trim().length < 10}
                  onPress={() => {
                    setBusy(true);
                    setError("");
                    void call<SourceReviewDetail>("/api/v1/source-reviews", {
                      method: "POST",
                      headers: { "Content-Type": "application/json" },
                      body: JSON.stringify({
                        source,
                        resourceRevision: revision,
                        consent,
                        releaseNotes: notes,
                      }),
                    })
                      .then((r) => {
                        setSubmitted(r);
                        setOpen(false);
                      })
                      .catch((e) => setError(e.message))
                      .finally(() => setBusy(false));
                  }}
                >
                  Submit approved package
                </Button>
              </Modal.Footer>
            </Modal.Dialog>
          </Modal.Container>
        </Modal.Backdrop>
      </Modal>
    </>
  );
}
type PreviewPage = {
  sourceAssertions?: string[];
  id: string;
  title: string;
  blocks: {
    id: string;
    kind: string;
    text: string;
    sourceAssertions?: string[];
  }[];
  offset: number;
  total: number;
  nextOffset: number | null;
  previousOffset: number | null;
};
export function SourceReviewQueue({ actor }: { actor: Actor }) {
  const [items, setItems] = useState<SourceReviewItem[]>([]),
    [detail, setDetail] = useState<SourceReviewDetail | null>(null),
    [history, setHistory] = useState(false),
    [error, setError] = useState(""),
    [busy, setBusy] = useState(false),
    [loading, setLoading] = useState(true);
  const [location, setLocation] = useState<{
      document: string;
      offset: number;
    } | null>(null),
    [page, setPage] = useState<PreviewPage | null>(null),
    [pageLoading, setPageLoading] = useState(false);
  const [note, setNote] = useState(""),
    [rights, setRights] = useState(""),
    [publisher, setPublisher] = useState(""),
    [checks, setChecks] = useState<Record<string, boolean>>({});
  async function load() {
    setLoading(true);
    setError("");
    try {
      const r = await call<{ items: SourceReviewItem[] }>(
        `/api/v1/source-reviews?history=${history}`,
      );
      setItems(r.items);
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setLoading(false);
    }
  }
  useEffect(() => {
    void load();
  }, [actor.id, history]);
  async function open(id: string) {
    setError("");
    setBusy(true);
    try {
      const r = await call<SourceReviewDetail>(`/api/v1/source-reviews/${id}`);
      setDetail(r);
      setLocation({ document: r.preview.documents[0].id, offset: 0 });
      setChecks({});
      setNote("");
      setRights("");
      setPublisher("");
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }
  useEffect(() => {
    if (!detail || !location) return;
    const abort = new AbortController();
    setPageLoading(true);
    setPage(null);
    if (["withdrawn", "superseded"].includes(detail.item.status)) {
      setPageLoading(false);
      return;
    }
    void call<PreviewPage>(
      `/api/v1/source-reviews/${detail.item.id}/document?${new URLSearchParams({ document: location.document, offset: String(location.offset) })}`,
      { signal: abort.signal },
    )
      .then((p) => !abort.signal.aborted && setPage(p))
      .catch((e) => !abort.signal.aborted && setError(e.message))
      .finally(() => !abort.signal.aborted && setPageLoading(false));
    return () => abort.abort();
  }, [detail, location]);
  async function act(command: Record<string, unknown>) {
    if (!detail) return;
    setBusy(true);
    setError("");
    try {
      const r = await call<SourceReviewDetail>(
        `/api/v1/source-reviews/${detail.item.id}`,
        {
          method: "PATCH",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            revision: detail.item.revision,
            sourceSha256: detail.item.source_sha256,
            command,
          }),
        },
      );
      setDetail(r);
      setNote("");
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }
  const item = detail?.item,
    inactive = !!item && ["superseded", "withdrawn"].includes(item.status),
    base = item ? `/api/v1/source-reviews/${item.id}` : "";
  const checklist = {
    publisher: "Publisher identity and namespace authority verified",
    textRights:
      "Text rights, notices and public-free distribution scope reviewed",
    artworkRights: "Cover and embedded artwork rights reviewed",
    content:
      "Package content, reading order, coverage and conversion fidelity reviewed",
    references:
      "Scripture targets, assertions and concept/link quality reviewed",
  };
  const fields = detail?.preview.fields ?? {};
  const fieldLabels: Record<string, string> = {
    author: "Author / contributors",
    publisher: "Publisher",
    sourceVersification: "Source versification",
    coverageStart: "Coverage start",
    coverageEnd: "Coverage end",
    licenseId: "Text license",
    rightsHolder: "Text rights holder",
    copyright: "Copyright",
    attribution: "Text attribution",
    coverAlt: "Cover description",
    coverLicenseId: "Artwork license",
    coverRightsHolder: "Artwork rights holder",
    coverAttribution: "Artwork attribution",
  };
  return (
    <section className="epub-review-workspace">
      <div className="org-toolbar">
        <div>
          <p className="eyebrow">SOURCE PACKAGE REVIEW</p>
          <h2>
            {detail ? item?.title : "From approved draft to trusted release"}
          </h2>
        </div>
        <Button
          variant="secondary"
          isDisabled={busy}
          onPress={() => (detail ? void open(detail.item.id) : void load())}
        >
          <RefreshCw size={16} />
          Refresh {detail ? "review" : "queue"}
        </Button>
      </div>
      {error && (
        <p className="form-error" role="alert">
          {error}
        </p>
      )}
      {!detail ? (
        <>
          <p className="muted">
            Publisher-submitted package snapshots. Private organization drafts
            appear only after explicit submission. Metadata-only reviews are in
            the other queue.
          </p>
          <Tabs
            selectedKey={history ? "history" : "active"}
            onSelectionChange={(key) => setHistory(key === "history")}
          >
            <Tabs.List aria-label="Source review history">
              <Tabs.Tab id="active">
                Active reviews
                <Tabs.Indicator />
              </Tabs.Tab>
              <Tabs.Tab id="history">
                Withdrawn & superseded
                <Tabs.Indicator />
              </Tabs.Tab>
            </Tabs.List>
          </Tabs>
          {loading ? (
            <Spinner />
          ) : items.length === 0 ? (
            <Card>
              <Card.Content>
                <FileCheck size={28} />
                <h3>No {history ? "archived" : "submitted"} packages yet</h3>
                <p>
                  Approve and generate a package in the publisher workspace,
                  then choose “Submit package to Fontes”.
                </p>
              </Card.Content>
            </Card>
          ) : (
            items.map((r) => (
              <Card key={r.id}>
                <Card.Content>
                  <div className="org-toolbar">
                    <div>
                      <h3>{r.title}</h3>
                      <p className="muted">
                        {r.resource_id} · v{r.version} ·{" "}
                        {new Date(r.created_at).toLocaleDateString()}
                      </p>
                      <Chip variant="secondary">{labels[r.status]}</Chip>
                    </div>
                    <Button isDisabled={busy} onPress={() => void open(r.id)}>
                      Open package review
                      <ArrowRight size={16} />
                    </Button>
                  </div>
                </Card.Content>
              </Card>
            ))
          )}
          <p className="muted">
            Up to 100 {history ? "recent archived" : "active"} accessible
            reviews. Public publication and app installation follow separate
            release gates.
          </p>
        </>
      ) : (
        <>
          <Button
            variant="ghost"
            isDisabled={busy}
            onPress={() => {
              setDetail(null);
              setPage(null);
              void load();
            }}
          >
            <ArrowLeft size={16} />
            Back to source queue
          </Button>
          <Card>
            <Card.Header>
              <Card.Title>Review the frozen package</Card.Title>
              <Chip variant="secondary">{labels[item!.status]}</Chip>
            </Card.Header>
            <Card.Content>
              <p>{detail.preview.description}</p>
              <p className="muted">
                {item!.resource_id} · v{item!.version} ·{" "}
                {detail.preview.language} · {detail.preview.kind}
              </p>
              <p>Release notes: {detail.preview.releaseNotes}</p>
              <p className="muted">
                Submitted by {item!.submitted_by} · Publisher approval by{" "}
                {detail.preview.publisherApproval.actor} at{" "}
                {new Date(detail.preview.publisherApproval.at).toLocaleString()}
              </p>
              <p className="source-review-hash">
                Canonical source SHA-256: <code>{item!.source_sha256}</code>
              </p>
              {item!.source_sha256 !== item!.upstream_sha256 && (
                <p className="muted">
                  The submitted ZIP was normalized by the trusted packer without
                  changing its file payloads. Publisher ZIP SHA-256:{" "}
                  <code>{item!.upstream_sha256}</code>
                </p>
              )}
              <p className="muted">
                Strict source validation and two matching canonical builds
                passed at submission. This preview flattens presentation;
                inspect the exact source ZIP for full markup and advanced
                assertions.
              </p>
              {!inactive && (
                <div className="org-toolbar">
                  <a className="epub-download" href={`${base}/package`}>
                    <Download size={16} />
                    Download reviewed source ZIP
                  </a>
                  {item!.status === "handoff_ready" && (
                    <a className="epub-download" href={`${base}/handoff`}>
                      <Download size={16} />
                      Download publication handoff
                    </a>
                  )}
                </div>
              )}
              {item!.approval && (
                <p>
                  Maintainer approval: {item!.approval.actor} ·{" "}
                  {new Date(item!.approval.at).toLocaleString()} · rights review{" "}
                  {item!.approval.rightsReviewId} · publisher review{" "}
                  {item!.approval.publisherReviewId}
                </p>
              )}
              {item!.status === "handoff_ready" && (
                <p className="epub-fidelity">
                  The unsigned handoff is ready for the trusted source-release
                  workflow. It has not been published, signed or added to an app
                  catalog.
                </p>
              )}
            </Card.Content>
          </Card>
          <Card>
            <Card.Header>
              <Card.Title>Rights, artwork & package inventory</Card.Title>
            </Card.Header>
            <Card.Content>
              <dl className="source-review-rights">
                {[
                  "author",
                  "publisher",
                  "sourceVersification",
                  "coverageStart",
                  "coverageEnd",
                  "licenseId",
                  "rightsHolder",
                  "copyright",
                  "attribution",
                  "coverAlt",
                  "coverLicenseId",
                  "coverRightsHolder",
                  "coverAttribution",
                ].map((k) => (
                  <div key={k}>
                    <dt>{fieldLabels[k] ?? k}</dt>
                    <dd>{String(fields[k] ?? "—")}</dd>
                  </div>
                ))}
              </dl>
              <p>
                Contributors:{" "}
                {Array.isArray(fields.contributors)
                  ? fields.contributors.join("; ")
                  : String(fields.author ?? "")}
              </p>
              <p>
                Declared coverage:{" "}
                {Array.isArray(fields.coverageRanges)
                  ? fields.coverageRanges.join("; ")
                  : String(fields.coverageStart ?? "")}
              </p>
              <details>
                <summary>License / permission text</summary>
                <p className="epub-original">
                  {String(fields.licenseText ?? "")}
                </p>
                <pre>{JSON.stringify(fields.permissions, null, 2)}</pre>
              </details>
              <details>
                <summary>
                  {detail.preview.files.length} hashed package files
                </summary>
                <ul>
                  {detail.preview.files.map((f) => (
                    <li key={f.path}>
                      {f.path} · {f.bytes.toLocaleString()} bytes ·{" "}
                      <code>{f.sha256.slice(0, 16)}…</code>
                      {!inactive && (
                        <>
                          {" "}
                          ·{" "}
                          <a
                            href={`${base}/file?${new URLSearchParams({ path: f.path })}`}
                          >
                            Download
                          </a>
                        </>
                      )}
                    </li>
                  ))}
                </ul>
              </details>
              {!inactive && (
                <div className="source-review-media">
                  {detail.preview.files
                    .filter((f) => /\.(png|jpe?g)$/.test(f.path))
                    .map((f) => (
                      <figure key={f.path}>
                        <img
                          src={`${base}/file?${new URLSearchParams({ path: f.path })}`}
                          alt={`Package artwork: ${f.path}`}
                        />
                        <figcaption>{f.path}</figcaption>
                      </figure>
                    ))}
                </div>
              )}
            </Card.Content>
          </Card>
          {!inactive && (
            <Card>
              <Card.Header>
                <Card.Title>Content & supplied assertions</Card.Title>
              </Card.Header>
              <Card.Content>
                <Select
                  aria-label="Reviewed package section"
                  selectedKey={location?.document ?? null}
                  onSelectionChange={(k) =>
                    k && setLocation({ document: String(k), offset: 0 })
                  }
                >
                  <Label>Section</Label>
                  <Select.Trigger>
                    <Select.Value />
                  </Select.Trigger>
                  <Select.Popover>
                    <ListBox>
                      {detail.preview.documents.map((d) => (
                        <ListBox.Item id={d.id} key={d.id} textValue={d.title}>
                          {d.title}
                          <ListBox.ItemIndicator />
                        </ListBox.Item>
                      ))}
                    </ListBox>
                  </Select.Popover>
                </Select>
                <div className="org-toolbar epub-pages">
                  <Button
                    variant="secondary"
                    isDisabled={
                      !page || page.previousOffset === null || pageLoading
                    }
                    onPress={() =>
                      page &&
                      setLocation({
                        document: page.id,
                        offset: page.previousOffset!,
                      })
                    }
                  >
                    Previous page
                  </Button>
                  <span className="muted">
                    {page
                      ? `Blocks ${page.offset + 1}–${Math.min(page.offset + 40, page.total)} of ${page.total}`
                      : "Loading…"}
                  </span>
                  <Button
                    variant="secondary"
                    isDisabled={
                      !page || page.nextOffset === null || pageLoading
                    }
                    onPress={() =>
                      page &&
                      setLocation({
                        document: page.id,
                        offset: page.nextOffset!,
                      })
                    }
                  >
                    Next page
                  </Button>
                </div>
                {pageLoading ? (
                  <Spinner />
                ) : (
                  page && (
                    <article className="epub-reading">
                      {page.sourceAssertions?.length ? (
                        <div>
                          <h4>Section assertions</h4>
                          <ul>
                            {page.sourceAssertions.map((a, i) => (
                              <li key={i}>{a}</li>
                            ))}
                          </ul>
                        </div>
                      ) : null}
                      {page.blocks.map((b) => (
                        <section key={b.id} className="epub-block">
                          <p className="epub-block-label">
                            {b.kind} · {b.id}
                          </p>
                          {b.kind === "heading" ? (
                            <h4>{b.text}</h4>
                          ) : (
                            <p>{b.text}</p>
                          )}
                          {b.sourceAssertions?.length ? (
                            <ul className="epub-candidates">
                              {b.sourceAssertions.map((a, i) => (
                                <li key={i}>{a}</li>
                              ))}
                            </ul>
                          ) : null}
                        </section>
                      ))}
                    </article>
                  )
                )}
              </Card.Content>
            </Card>
          )}
          {!inactive && (
            <Card>
              <Card.Header>
                <Card.Title>Maintainer decision</Card.Title>
              </Card.Header>
              <Card.Content>
                {error && (
                  <p className="form-error" role="alert">
                    {error}
                  </p>
                )}
                <div className="resource-form">
                  {!item!.independent && (
                    <p className="muted">
                      {actor.role === "reviewer"
                        ? "You submitted, created or approved this publisher draft. Another maintainer must make its editorial decision."
                        : "Publishers can follow feedback, comment or withdraw. Independent Fontes maintainers make editorial decisions."}
                    </p>
                  )}
                  <TextField value={note} onChange={setNote} isDisabled={busy}>
                    <Label>Feedback / decision notes</Label>
                    <TextArea rows={3} maxLength={4000} />
                  </TextField>
                  {item!.independent && item!.status === "submitted" && (
                    <>
                      {Object.entries(checklist).map(([k, label]) => (
                        <Checkbox
                          key={k}
                          isSelected={!!checks[k]}
                          isDisabled={busy}
                          onChange={(v) => setChecks((c) => ({ ...c, [k]: v }))}
                        >
                          <Checkbox.Content>
                            <Checkbox.Control>
                              <Checkbox.Indicator />
                            </Checkbox.Control>
                            <Label>{label}</Label>
                          </Checkbox.Content>
                        </Checkbox>
                      ))}
                      <p className="muted">
                        Use non-secret review IDs referencing actual identity
                        and rights evidence. Private agreements stay outside
                        release files.
                      </p>
                      <TextField
                        value={rights}
                        onChange={setRights}
                        isDisabled={busy}
                      >
                        <Label>Rights review ID</Label>
                        <Input maxLength={240} />
                      </TextField>
                      <TextField
                        value={publisher}
                        onChange={setPublisher}
                        isDisabled={busy}
                      >
                        <Label>Publisher / namespace review ID</Label>
                        <Input maxLength={240} />
                      </TextField>
                    </>
                  )}
                  <div className="org-toolbar">
                    <Button
                      variant="secondary"
                      isDisabled={busy || note.trim().length < 5}
                      onPress={() => void act({ action: "comment", note })}
                    >
                      Save feedback
                    </Button>
                    {item!.independent &&
                      ["submitted", "approved", "handoff_ready"].includes(
                        item!.status,
                      ) && (
                        <Button
                          variant="secondary"
                          isDisabled={busy || note.trim().length < 5}
                          onPress={() =>
                            void act({ action: "request_changes", note })
                          }
                        >
                          Request changes
                        </Button>
                      )}
                    {item!.independent && item!.status === "submitted" && (
                      <Button
                        isPending={busy}
                        isDisabled={
                          Object.keys(checklist).some((k) => !checks[k]) ||
                          note.trim().length < 5 ||
                          rights.trim().length < 3 ||
                          publisher.trim().length < 3
                        }
                        onPress={() =>
                          void act({
                            action: "approve",
                            note,
                            rightsReviewId: rights,
                            publisherReviewId: publisher,
                            checks,
                          })
                        }
                      >
                        Approve exact source package
                      </Button>
                    )}
                    {item!.independent && item!.status === "approved" && (
                      <Button
                        isPending={busy}
                        onPress={() => void act({ action: "handoff" })}
                      >
                        Prepare publication handoff
                      </Button>
                    )}
                    {item!.canWithdraw && (
                      <Button
                        variant="ghost"
                        isDisabled={busy || note.trim().length < 5}
                        onPress={() => void act({ action: "withdraw", note })}
                      >
                        Withdraw submission
                      </Button>
                    )}
                  </div>
                </div>
              </Card.Content>
            </Card>
          )}
          <Card>
            <Card.Header>
              <Card.Title>Review history</Card.Title>
            </Card.Header>
            <Card.Content>
              {detail.events.map((e) => (
                <div key={e.id} className="source-review-event">
                  <strong>{e.action.replaceAll("_", " ")}</strong>
                  <p>{e.note}</p>
                  <span className="muted">
                    {e.actor_id ?? "System"} ·{" "}
                    {new Date(e.created_at).toLocaleString()}
                  </span>
                </div>
              ))}
            </Card.Content>
          </Card>
        </>
      )}
    </section>
  );
}
