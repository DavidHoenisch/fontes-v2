"use client";
import { SubmitSourceReview } from "./source-review";
import { useState, useRef } from "react";
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
} from "@heroui/react";
import {
  Check,
  Download,
  FileCheck,
  Settings2,
  Upload,
  Plus,
  Trash2,
} from "lucide-react";
import type { ReviewReply, ReviewPage } from "@/lib/epub-review-contracts";
import { permissionKeys, type IntakeFields } from "@/lib/intake-contracts";
export const conversionNotice =
  "The source package keeps all text, footnote blocks, images and emphasis. Internal footnote/navigation links, bold and superscript styling become plain text; poetry line breaks remain text whitespace. The original EPUB and rich review stay available here.";
type Command = Record<string, unknown>;
export function PublisherReviewActions({
  reply,
  busy,
  base,
  onCommand,
  onCover,
  error,
}: {
  reply: ReviewReply;
  busy: boolean;
  base: string;
  onCommand: (c: Command) => Promise<boolean>;
  onCover: (file: File) => void;
  error: string;
}) {
  const [open, setOpen] = useState(false),
    [fields, setFields] = useState<IntakeFields | null>(null),
    file = useRef<HTMLInputElement>(null);
  const draft = reply.draft;
  if (!draft) return null;
  const text = (key: keyof IntakeFields, label: string, multiline = false) => (
    <TextField
      key={key}
      value={String(fields?.[key] ?? "")}
      onChange={(v) => setFields((f) => (f ? { ...f, [key]: v } : f))}
    >
      <Label>{label}</Label>
      {multiline ? <TextArea rows={3} /> : <Input />}
    </TextField>
  );
  return (
    <>
      <Card>
        <Card.Header>
          <Card.Title>Publisher review & approval</Card.Title>
          <Chip variant="secondary">
            {reply.approved ? "Approved" : "Review in progress"}
          </Chip>
        </Card.Header>
        <Card.Content>
          <div className="epub-metrics">
            {[
              ["Pending references", draft.pending],
              ["Accepted", draft.accepted],
              ["Rejected", draft.rejected],
              ["Edited blocks", draft.editedBlocks.length],
            ].map(([label, count]) => (
              <div key={label}>
                <strong>{count}</strong>
                <span>{label}</span>
              </div>
            ))}
          </div>
          <p className="muted">
            Review revision {reply.reviewRevision} · Corrections retain the
            original extraction. Text edits reset references in that block. Any
            content, metadata, rights or cover edit requires a new approval.
          </p>
          <p>{conversionNotice}</p>
          <div className="org-toolbar">
            <Button
              variant="secondary"
              isDisabled={busy}
              onPress={() => {
                setFields(structuredClone(draft.fields));
                setOpen(true);
              }}
            >
              <Settings2 size={16} />
              Publication details & rights
            </Button>
            <Button
              variant="secondary"
              isDisabled={busy}
              onPress={() => file.current?.click()}
            >
              <Upload size={16} />
              {draft.cover ? "Replace cover" : "Upload cover"}
            </Button>
            <Button
              isPending={busy}
              isDisabled={reply.checks.length > 0 || reply.approved}
              onPress={() => void onCommand({ action: "approve" })}
            >
              <Check size={16} />
              Approve reviewed draft
            </Button>
            <Button
              variant="secondary"
              isPending={busy}
              isDisabled={!reply.approved}
              onPress={() => void onCommand({ action: "package" })}
            >
              <FileCheck size={16} />
              Generate Fontes source package
            </Button>
            {reply.approved && draft.package && (
              <a className="epub-download" href={`${base}/package`}>
                <Download size={16} />
                Download source ZIP
              </a>
            )}
          </div>
          <input
            className="sr-only"
            type="file"
            ref={file}
            accept="image/png,image/jpeg"
            aria-label="Upload publisher cover"
            onChange={(e) => {
              const f = e.target.files?.[0];
              e.target.value = "";
              if (f) onCover(f);
            }}
          />
          {reply.approved && draft.package && (
            <SubmitSourceReview
              revision={reply.resourceRevision}
              source={{
                kind: "organization",
                organizationId: base.split("/")[4],
                resourceId: reply.resourceId,
                organizationRevision: reply.organizationRevision,
                reviewRevision: reply.reviewRevision,
              }}
              disabled={busy}
            />
          )}
          {reply.checks.length > 0 && (
            <details className="epub-readiness">
              <summary>
                {reply.checks.length}{" "}
                {reply.checks.length === 1 ? "item" : "items"} before approval
              </summary>
              <ul>
                {reply.checks.map((c) => (
                  <li key={c}>{c}</li>
                ))}
              </ul>
            </details>
          )}
          {reply.approved && draft.approval && (
            <p className="epub-fidelity">
              <Check size={16} />
              Approved {new Date(draft.approval.at).toLocaleString()} ·{" "}
              {draft.approval.actor} · snapshot{" "}
              <code>{draft.approval.digest.slice(0, 16)}…</code>
            </p>
          )}
          {draft.approval && !reply.approved && (
            <p className="form-error">
              The saved approval is stale. Review the changed metadata and
              approve this draft again.
            </p>
          )}
          {reply.approved && draft.package && (
            <p className="muted">
              Strict Rust validation passed. Package SHA-256:{" "}
              <code>{draft.package.sha256}</code>
            </p>
          )}
          <p className="muted">
            Local source preparation only. Organization member delivery, signing
            and publication remain separate steps.
          </p>
        </Card.Content>
      </Card>
      <Modal isOpen={open} onOpenChange={(v) => !busy && setOpen(v)}>
        <Modal.Backdrop>
          <Modal.Container size="lg">
            <Modal.Dialog>
              <Modal.CloseTrigger isDisabled={busy} />
              <Modal.Header>
                <Modal.Heading>Publication details & rights</Modal.Heading>
              </Modal.Header>
              <Modal.Body>
                {error && (
                  <p className="form-error" role="alert">
                    {error}
                  </p>
                )}
                {fields && (
                  <div className="resource-form">
                    <p className="muted">
                      Use the actual authorization for this edition and its
                      artwork. Source notices remain in the conversion report.
                      No permission is inferred from the age of the original
                      work.
                    </p>
                    <div className="form-grid">
                      {text("author", "Author / contributor")}
                      {text("publisher", "Publisher")}
                      {text("edition", "Edition")}
                      {text(
                        "publicationDate",
                        "Publication date (YYYY, YYYY-MM or YYYY-MM-DD)",
                      )}
                      {text("sourceVersification", "Source versification")}
                      {text(
                        "coverageStart",
                        "Coverage start (OSIS, e.g. John.3.16)",
                      )}
                      {text("coverageEnd", "Coverage end (optional OSIS)")}
                    </div>
                    <h3>Text rights</h3>
                    <div className="form-grid">
                      {text("licenseId", "Text license ID")}
                      {text("rightsHolder", "Text rights holder")}
                    </div>
                    {text("copyright", "Copyright statement", true)}
                    {text("attribution", "Required attribution", true)}
                    {text("licenseText", "License / authorization text", true)}
                    <div className="intake-permissions">
                      {permissionKeys.map((k) => (
                        <Checkbox
                          key={k}
                          isSelected={fields.permissions[k] === "allow"}
                          onChange={(v) =>
                            setFields({
                              ...fields,
                              permissions: {
                                ...fields.permissions,
                                [k]: v ? "allow" : "deny",
                              },
                            })
                          }
                        >
                          <Checkbox.Control>
                            <Checkbox.Indicator />
                          </Checkbox.Control>
                          <Checkbox.Content>{k}</Checkbox.Content>
                        </Checkbox>
                      ))}
                    </div>
                    <h3>Cover rights</h3>
                    <p className="muted">
                      Upload an upright single-frame 1200 × 1800 PNG or JPEG, at
                      most 4 MiB.
                    </p>
                    {text("coverAlt", "Cover description")}
                    <div className="form-grid">
                      {text("coverLicenseId", "Artwork license ID")}
                      {text("coverRightsHolder", "Artwork rights holder")}
                    </div>
                    {text("coverAttribution", "Artwork attribution")}
                    <Checkbox
                      isSelected={fields.rightsAttested}
                      onChange={(v) =>
                        setFields({ ...fields, rightsAttested: v })
                      }
                    >
                      <Checkbox.Control>
                        <Checkbox.Indicator />
                      </Checkbox.Control>
                      <Checkbox.Content>
                        I am authorized to prepare and distribute this source
                        text and artwork under these terms.
                      </Checkbox.Content>
                    </Checkbox>
                    <Checkbox
                      isSelected={fields.conversionAcknowledged}
                      onChange={(v) =>
                        setFields({ ...fields, conversionAcknowledged: v })
                      }
                    >
                      <Checkbox.Control>
                        <Checkbox.Indicator />
                      </Checkbox.Control>
                      <Checkbox.Content>
                        I reviewed the content, references, source notices and
                        conversion report, including the source-package
                        formatting and navigation limitations.
                      </Checkbox.Content>
                    </Checkbox>
                  </div>
                )}
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
                  onPress={() =>
                    void onCommand({ action: "fields", fields }).then(
                      (ok) => ok && setOpen(false),
                    )
                  }
                >
                  Save details & rights
                </Button>
              </Modal.Footer>
            </Modal.Dialog>
          </Modal.Container>
        </Modal.Backdrop>
      </Modal>
    </>
  );
}
export function PublisherBlockEditor({
  block,
  documentId,
  busy,
  onCommand,
  error,
}: {
  block: ReviewPage["blocks"][number];
  documentId: string;
  busy: boolean;
  onCommand: (c: Command) => Promise<boolean>;
  error: string;
}) {
  const [open, setOpen] = useState(false),
    [texts, setTexts] = useState<string[]>([]),
    [refs, setRefs] = useState(block.references);
  if (block.kind === "image") return null;
  const changed = texts.join("") !== block.text;
  const update = (i: number, key: string, value: string) =>
    setRefs((rs) => rs.map((r, n) => (n === i ? { ...r, [key]: value } : r)));
  return (
    <>
      <Button
        size="sm"
        variant="ghost"
        isDisabled={busy}
        onPress={() => {
          setTexts(block.segments.map((s) => s.text));
          setRefs(structuredClone(block.references));
          setOpen(true);
        }}
      >
        Edit text & references
      </Button>
      <Modal isOpen={open} onOpenChange={(v) => !busy && setOpen(v)}>
        <Modal.Backdrop>
          <Modal.Container size="lg">
            <Modal.Dialog>
              <Modal.CloseTrigger isDisabled={busy} />
              <Modal.Header>
                <Modal.Heading>Correct extracted content</Modal.Heading>
              </Modal.Header>
              <Modal.Body>
                {error && (
                  <p className="form-error" role="alert">
                    {error}
                  </p>
                )}
                <div className="resource-form">
                  <details>
                    <summary>Original extracted text</summary>
                    <p className="epub-original">
                      {block.originalText ?? block.text}
                    </p>
                  </details>
                  <p className="muted">
                    Edit the inline text segments below. Existing emphasis, note
                    links and line breaks stay attached. Saving changed text
                    resets this block’s references; then review them against the
                    corrected text.
                  </p>
                  {block.segments.map((s, i) =>
                    s.lineBreak ? (
                      <p key={i} className="muted">
                        Line break
                      </p>
                    ) : (
                      <TextField
                        key={i}
                        value={texts[i] ?? ""}
                        onChange={(v) =>
                          setTexts((ts) => ts.map((t, n) => (n === i ? v : t)))
                        }
                      >
                        <Label>
                          Text {i + 1}
                          {s.target ? " · linked note / source location" : ""}
                          {s.marks.length ? ` · ${s.marks.join(", ")}` : ""}
                        </Label>
                        <TextArea rows={s.text.length > 250 ? 5 : 2} />
                      </TextField>
                    ),
                  )}
                  <h3>Scripture references</h3>
                  {changed ? (
                    <p role="status">
                      Save the text correction first. References will be
                      redetected as pending and previous decisions retained in
                      revision history.
                    </p>
                  ) : (
                    <>
                      <p className="muted">
                        Each target is checked against the canonical registry.
                        Evidence must match this block exactly. Accepted
                        references become publisher assertions with relation
                        “cites”. Remove an incorrect suggestion or reject it to
                        keep a visible decision.
                      </p>
                      {refs.map((r, i) => (
                        <Card key={r.id}>
                          <Card.Content>
                            <div className="form-grid">
                              <TextField
                                value={r.evidence}
                                onChange={(v) => update(i, "evidence", v)}
                              >
                                <Label>Quoted evidence</Label>
                                <TextArea rows={2} />
                              </TextField>
                              <TextField
                                value={r.start}
                                onChange={(v) => update(i, "start", v)}
                              >
                                <Label>Start OSIS</Label>
                                <Input />
                              </TextField>
                              <TextField
                                value={r.end ?? ""}
                                onChange={(v) =>
                                  setRefs((rs) =>
                                    rs.map((r, n) =>
                                      n === i
                                        ? { ...r, end: v || undefined }
                                        : r,
                                    ),
                                  )
                                }
                              >
                                <Label>End OSIS (optional)</Label>
                                <Input />
                              </TextField>
                            </div>
                            <div className="org-toolbar">
                              <Chip variant="secondary">{r.decision}</Chip>
                              <Button
                                size="sm"
                                variant={
                                  r.decision === "accepted"
                                    ? "primary"
                                    : "secondary"
                                }
                                onPress={() =>
                                  update(i, "decision", "accepted")
                                }
                              >
                                Accept
                              </Button>
                              <Button
                                size="sm"
                                variant={
                                  r.decision === "rejected"
                                    ? "primary"
                                    : "secondary"
                                }
                                onPress={() =>
                                  update(i, "decision", "rejected")
                                }
                              >
                                Reject
                              </Button>
                              <Button
                                size="sm"
                                variant="ghost"
                                onPress={() => update(i, "decision", "pending")}
                              >
                                Reset
                              </Button>
                              <Button
                                size="sm"
                                variant="ghost"
                                aria-label={`Remove reference ${i + 1}`}
                                onPress={() =>
                                  setRefs((rs) => rs.filter((_, n) => n !== i))
                                }
                              >
                                <Trash2 size={15} />
                              </Button>
                            </div>
                          </Card.Content>
                        </Card>
                      ))}
                      <Button
                        variant="secondary"
                        onPress={() =>
                          setRefs((rs) => [
                            ...rs,
                            {
                              id: `publisher-${crypto.randomUUID()}`,
                              blockId: block.id,
                              evidence: "",
                              start: "",
                              method: "publisher-correction-v1",
                              decision: "pending",
                            },
                          ])
                        }
                      >
                        <Plus size={16} />
                        Add Scripture reference
                      </Button>
                    </>
                  )}
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
                  onPress={() =>
                    void onCommand({
                      action: "correct",
                      documentId,
                      blockId: block.id,
                      segmentTexts: texts,
                      references: refs,
                    }).then((ok) => ok && setOpen(false))
                  }
                >
                  Save correction
                </Button>
              </Modal.Footer>
            </Modal.Dialog>
          </Modal.Container>
        </Modal.Backdrop>
      </Modal>
    </>
  );
}
