"use client";
import { SubmitSourceReview } from "./source-review";
import { useEffect, useState } from "react";
import {
  Button,
  Card,
  Checkbox,
  Chip,
  Input,
  Label,
  ListBox,
  Modal,
  Select,
  Spinner,
  TextArea,
  TextField,
} from "@heroui/react";
import {
  ArrowLeft,
  ArrowRight,
  BookOpen,
  Check,
  FileArchive,
  FileText,
  Upload,
  WandSparkles,
} from "lucide-react";
import { kinds, type Resource } from "@/lib/contracts";
import {
  emptyIntake,
  permissionKeys,
  type IntakeFields,
  type IntakeReply,
} from "@/lib/intake-contracts";

async function call<T>(url: string, init?: RequestInit): Promise<T> {
  const response = await fetch(url, {
    cache: "no-store",
    ...init,
    headers: { "Content-Type": "application/json", ...init?.headers },
  });
  const body = await response.json();
  if (!response.ok)
    throw new Error(
      body.error?.details?.diagnostics
        ?.map((d: { message: string }) => d.message)
        .join(" ") ||
        body.error?.message ||
        "The request failed.",
    );
  return body.data;
}
const label = (s: string) =>
  s.replaceAll("-", " ").replace(/^./, (s) => s.toUpperCase());
type Props = {
  open: boolean;
  onClose: () => void;
  onSaved: (id: string) => Promise<void>;
  resource?: Resource;
  readOnly?: boolean;
};
export function IntakeWizard({
  open,
  onClose,
  onSaved,
  resource,
  readOnly = false,
}: Props) {
  const [method, setMethod] = useState<"guided" | "prepared">("guided"),
    [step, setStep] = useState(0),
    [id, setId] = useState<string | null>(null),
    [reply, setReply] = useState<IntakeReply | null>(null),
    [fields, setFields] = useState(emptyIntake().fields),
    [error, setError] = useState(""),
    [busy, setBusy] = useState(false),
    [dirty, setDirty] = useState(false);
  const [title, setTitle] = useState(""),
    [description, setDescription] = useState(""),
    [kind, setKind] = useState("book"),
    [language, setLanguage] = useState("en"),
    [version, setVersion] = useState("1.0.0");
  const steps =
    method === "prepared"
      ? ["Choose a path", "Upload package", "Review & approve"]
      : [
          "Choose a path",
          "Resource details",
          "Rights & artwork",
          "Import EPUB",
          "Review & approve",
        ];
  const endpoint = (value = id) =>
    `/api/v1/resources/${encodeURIComponent(value!)}/intake`;
  const adopt = (next: IntakeReply) => {
    setReply(next);
    setFields(next.state.fields);
    setMethod(next.state.method);
    setDirty(false);
  };
  useEffect(() => {
    if (!open) return;
    setError("");
    setReply(null);
    setStep(0);
    setId(resource?.id ?? null);
    setTitle(resource?.title ?? "");
    setDescription(resource?.description ?? "");
    setKind(resource?.kind ?? "book");
    setLanguage(resource?.language ?? "en");
    setVersion(resource?.version ?? "1.0.0");
    setMethod("guided");
    setFields(emptyIntake().fields);
    let active = true;
    if (resource) {
      setBusy(true);
      call<IntakeReply>(endpoint(resource.id))
        .then((next) => {
          if (active) {
            adopt(next);
            setStep(
              next.state.source
                ? next.state.method === "prepared"
                  ? 2
                  : 4
                : 1,
            );
          }
        })
        .catch((e) => active && setError(e.message))
        .finally(() => active && setBusy(false));
    }
    return () => {
      active = false;
    };
  }, [open, resource?.id]);
  const field = (key: keyof IntakeFields, value: unknown) => {
    setDirty(true);
    setFields((old) => ({ ...old, [key]: value }));
  };
  async function save(): Promise<IntakeReply> {
    let currentId = id,
      current = reply;
    if (currentId && current && !dirty) return current;
    if (!currentId) {
      const created = await call<Resource>("/api/v1/resources", {
        method: "POST",
        body: JSON.stringify({
          title: method === "prepared" ? "Prepared source package" : title,
          description:
            method === "prepared"
              ? "A private prepared-source submission awaiting import and publisher review."
              : description,
          kind,
          language,
          version,
        }),
      });
      currentId = created.id;
      setId(currentId);
      current = await call<IntakeReply>(endpoint(currentId));
    } else if (!current) current = await call<IntakeReply>(endpoint(currentId));
    const next = await call<IntakeReply>(endpoint(currentId), {
      method: "PUT",
      body: JSON.stringify({
        revision: current!.revision,
        method,
        fields,
        ...(method === "guided"
          ? { metadata: { title, description, kind, language, version } }
          : {}),
        decisions: (current?.state.proposals ?? []).map((p) => ({
          id: p.id,
          decision: p.decision,
        })),
      }),
    });
    adopt(next);
    return next;
  }
  async function upload(
    file: File | undefined,
    type: "epub" | "prepared" | "cover",
  ) {
    if (!file) return;
    setBusy(true);
    setError("");
    try {
      const current = await save();
      const response = await fetch(endpoint(current.resourceId) + `/${type}`, {
        method: "POST",
        headers: {
          "Content-Type": file.type || "application/octet-stream",
          "X-Fontes-Revision": String(current.revision),
        },
        body: file,
      });
      const body = await response.json();
      if (!response.ok)
        throw new Error(
          body.error?.details?.diagnostics
            ?.map((d: { message: string }) => d.message)
            .join(" ") ||
            body.error?.message ||
            "Upload failed.",
        );
      adopt(body.data);
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }
  const dirtyProposal = (
    proposalId: string,
    decision: "accepted" | "rejected",
  ) => {
    setDirty(true);
    setReply((old) =>
      old
        ? {
            ...old,
            approval: null,
            state: {
              ...old.state,
              proposals: old.state.proposals.map((p) =>
                p.id === proposalId
                  ? {
                      ...p,
                      decision: p.decision === decision ? "pending" : decision,
                    }
                  : p,
              ),
            },
          }
        : old,
    );
  };
  const final = step === steps.length - 1;
  return (
    <Modal
      isOpen={open}
      onOpenChange={(value) => {
        if (!value && !busy) onClose();
      }}
    >
      <Modal.Backdrop>
        <Modal.Container size="lg" placement="center">
          <Modal.Dialog>
            <Modal.CloseTrigger isDisabled={busy} />
            <Modal.Header>
              <p className="eyebrow">RESOURCE PREPARATION</p>
              <Modal.Heading>
                {readOnly ? "Read the publisher’s draft" : steps[step]}
              </Modal.Heading>
            </Modal.Header>
            <Modal.Body>
              <div
                className="intake-progress"
                aria-label="Preparation progress"
              >
                {steps.map((name, index) => (
                  <span
                    key={name}
                    aria-current={index === step ? "step" : undefined}
                  >
                    <Chip
                      size="sm"
                      color={index === step ? "accent" : "default"}
                    >
                      {index + 1}
                    </Chip>
                    <span>{name}</span>
                  </span>
                ))}
              </div>
              {busy && (
                <p className="intake-working" role="status">
                  <Spinner size="sm" />
                  Saving or preparing your source…
                </p>
              )}
              <form
                id="intake-form"
                onSubmit={async (e) => {
                  e.preventDefault();
                  setBusy(true);
                  setError("");
                  try {
                    if (step === 0) {
                      setStep(1);
                      return;
                    }
                    await save();
                    setStep(Math.min(step + 1, steps.length - 1));
                  } catch (e) {
                    setError((e as Error).message);
                  } finally {
                    setBusy(false);
                  }
                }}
              >
                {step === 0 && (
                  <>
                    <p className="muted">
                      Two ways in. The same Fontes resource format out.
                    </p>
                    <div className="intake-paths">
                      <Card>
                        <Card.Content>
                          <WandSparkles size={25} />
                          <h3>Build it with us</h3>
                          <p className="muted">
                            Bring an EPUB. Add your metadata, rights, and cover,
                            then review a readable draft.
                          </p>
                          <Button
                            variant={
                              method === "guided" ? "primary" : "secondary"
                            }
                            onPress={() => {
                              setDirty(true);
                              setMethod("guided");
                            }}
                          >
                            {method === "guided" && <Check size={16} />}Guided
                            preparation
                          </Button>
                        </Card.Content>
                      </Card>
                      <Card>
                        <Card.Content>
                          <FileArchive size={25} />
                          <h3>I have a Fontes package</h3>
                          <p className="muted">
                            Keep your own pipeline. Upload the source ZIP
                            created by the Fontes tools.
                          </p>
                          <Button
                            variant={
                              method === "prepared" ? "primary" : "secondary"
                            }
                            onPress={() => {
                              setDirty(true);
                              setMethod("prepared");
                            }}
                          >
                            {method === "prepared" && <Check size={16} />}
                            Prepared package
                          </Button>
                        </Card.Content>
                      </Card>
                    </div>
                    <p className="muted">
                      Files stay private to the draft. Publication still
                      requires the appropriate review and signing.
                    </p>
                  </>
                )}
                {method === "guided" && step === 1 && (
                  <div className="resource-form">
                    <TextField
                      isRequired
                      minLength={2}
                      maxLength={180}
                      value={title}
                      onChange={(v) => {
                        setDirty(true);
                        setTitle(v);
                      }}
                      isReadOnly={readOnly}
                    >
                      <Label>Resource title</Label>
                      <Input placeholder="The title readers will see" />
                    </TextField>
                    <TextField
                      isRequired
                      minLength={10}
                      maxLength={4000}
                      value={description}
                      onChange={(v) => {
                        setDirty(true);
                        setDescription(v);
                      }}
                      isReadOnly={readOnly}
                    >
                      <Label>Description</Label>
                      <TextArea
                        rows={3}
                        placeholder="Describe the scope and approach"
                      />
                    </TextField>
                    <div className="form-row">
                      <Select
                        value={kind}
                        onChange={(v) => {
                          setDirty(true);
                          setKind(String(v));
                        }}
                        isDisabled={readOnly}
                      >
                        <Label>Resource type</Label>
                        <Select.Trigger>
                          <Select.Value />
                          <Select.Indicator />
                        </Select.Trigger>
                        <Select.Popover>
                          <ListBox>
                            {kinds
                              .filter((k) =>
                                [
                                  "book",
                                  "commentary",
                                  "study-guide",
                                  "devotional",
                                ].includes(k),
                              )
                              .map((k) => (
                                <ListBox.Item
                                  key={k}
                                  id={k}
                                  textValue={label(k)}
                                >
                                  {label(k)}
                                </ListBox.Item>
                              ))}
                          </ListBox>
                        </Select.Popover>
                      </Select>
                      <TextField
                        value={version}
                        onChange={(v) => {
                          setDirty(true);
                          setVersion(v);
                        }}
                        isReadOnly={readOnly}
                        isRequired
                      >
                        <Label>Version</Label>
                        <Input pattern="[0-9]+\.[0-9]+\.[0-9]+" />
                      </TextField>
                    </div>
                    <div className="form-row">
                      <TextField
                        value={fields.author}
                        onChange={(v) => field("author", v)}
                        isRequired
                      >
                        <Label>Author / contributor</Label>
                        <Input maxLength={240} />
                      </TextField>
                      <TextField
                        value={fields.publisher}
                        onChange={(v) => field("publisher", v)}
                        isRequired
                      >
                        <Label>Publisher display name</Label>
                        <Input maxLength={240} />
                      </TextField>
                    </div>
                    <div className="form-row">
                      <TextField
                        value={language}
                        onChange={(v) => {
                          setDirty(true);
                          setLanguage(v);
                        }}
                        isReadOnly={readOnly}
                        isRequired
                      >
                        <Label>Language code</Label>
                        <Input />
                      </TextField>
                      <TextField
                        value={fields.edition}
                        onChange={(v) => field("edition", v)}
                      >
                        <Label>Edition (optional)</Label>
                        <Input maxLength={240} />
                      </TextField>
                    </div>
                    <div className="form-row">
                      <TextField
                        value={fields.publicationDate}
                        onChange={(v) => field("publicationDate", v)}
                      >
                        <Label>Publication date (optional)</Label>
                        <Input placeholder="2026 or 2026-10-17" />
                      </TextField>
                      <TextField
                        value={fields.sourceVersification}
                        onChange={(v) => field("sourceVersification", v)}
                        isRequired
                      >
                        <Label>Source versification</Label>
                        <Input placeholder="KJV" />
                      </TextField>
                    </div>
                    <div className="form-row">
                      <TextField
                        value={fields.coverageStart}
                        onChange={(v) => field("coverageStart", v)}
                        isRequired
                      >
                        <Label>Coverage start</Label>
                        <Input placeholder="Matt.5.1" />
                      </TextField>
                      <TextField
                        value={fields.coverageEnd}
                        onChange={(v) => field("coverageEnd", v)}
                      >
                        <Label>Coverage end (optional)</Label>
                        <Input placeholder="Matt.7.29" />
                      </TextField>
                    </div>
                    <p className="muted">
                      Coverage describes the work’s declared Scripture scope. It
                      is separate from references found inside its paragraphs.
                      Advanced translation/lexicon profiles can use the
                      prepared-package path.
                    </p>
                  </div>
                )}
                {method === "guided" && step === 2 && (
                  <div className="resource-form">
                    <h3>Text rights</h3>
                    <div className="form-row">
                      <TextField
                        value={fields.licenseId}
                        onChange={(v) => field("licenseId", v)}
                        isRequired
                      >
                        <Label>License identifier</Label>
                        <Input
                          placeholder="CC-BY-4.0 or LicenseRef-…"
                          maxLength={240}
                        />
                      </TextField>
                      <TextField
                        value={fields.rightsHolder}
                        onChange={(v) => field("rightsHolder", v)}
                        isRequired
                      >
                        <Label>Text rights holder</Label>
                        <Input maxLength={240} />
                      </TextField>
                    </div>
                    <TextField
                      value={fields.copyright}
                      onChange={(v) => field("copyright", v)}
                      isRequired
                    >
                      <Label>Copyright statement</Label>
                      <TextArea rows={2} />
                    </TextField>
                    <TextField
                      value={fields.attribution}
                      onChange={(v) => field("attribution", v)}
                      isRequired
                    >
                      <Label>Required attribution</Label>
                      <TextArea rows={2} />
                    </TextField>
                    <TextField
                      value={fields.licenseText}
                      onChange={(v) => field("licenseText", v)}
                      isRequired
                    >
                      <Label>License / permission text</Label>
                      <TextArea
                        rows={4}
                        placeholder="The actual terms authorizing distribution and permitted uses"
                      />
                    </TextField>
                    <div className="intake-permissions">
                      {permissionKeys.map((key) => (
                        <Checkbox
                          key={key}
                          isSelected={fields.permissions[key] === "allow"}
                          onChange={(checked) =>
                            field("permissions", {
                              ...fields.permissions,
                              [key]: checked ? "allow" : "deny",
                            })
                          }
                        >
                          <Checkbox.Control>
                            <Checkbox.Indicator />
                          </Checkbox.Control>
                          <Checkbox.Content>
                            <Label>{label(key)}</Label>
                          </Checkbox.Content>
                        </Checkbox>
                      ))}
                    </div>
                    <p className="muted">
                      Permissions begin denied. Checking AI context does not
                      grant consent to send the book to an AI provider.
                    </p>
                    <h3>Cover artwork</h3>
                    <p className="muted">
                      A 1200 × 1800 PNG/JPEG, at most 4 MiB. Artwork rights are
                      separate from text rights.
                    </p>
                    <input
                      aria-label="Upload cover artwork"
                      type="file"
                      accept="image/png,image/jpeg"
                      disabled={busy}
                      onChange={(e) =>
                        void upload(e.target.files?.[0], "cover")
                      }
                    />
                    {reply?.state.cover && id && (
                      <img
                        className="intake-cover"
                        src={endpoint() + "/cover"}
                        alt="Uploaded cover preview"
                      />
                    )}
                    <TextField
                      value={fields.coverAlt}
                      onChange={(v) => field("coverAlt", v)}
                      isRequired
                    >
                      <Label>Cover description / alt text</Label>
                      <Input />
                    </TextField>
                    <div className="form-row">
                      <TextField
                        value={fields.coverLicenseId}
                        onChange={(v) => field("coverLicenseId", v)}
                        isRequired
                      >
                        <Label>Artwork license</Label>
                        <Input maxLength={240} />
                      </TextField>
                      <TextField
                        value={fields.coverRightsHolder}
                        onChange={(v) => field("coverRightsHolder", v)}
                        isRequired
                      >
                        <Label>Artwork rights holder</Label>
                        <Input maxLength={240} />
                      </TextField>
                    </div>
                    <TextField
                      value={fields.coverAttribution}
                      onChange={(v) => field("coverAttribution", v)}
                      isRequired
                    >
                      <Label>Artwork attribution</Label>
                      <Input />
                    </TextField>
                  </div>
                )}
                {((method === "guided" && step === 3) ||
                  (method === "prepared" && step === 1)) && (
                  <div className="resource-form">
                    <Card>
                      <Card.Content>
                        <Upload size={26} />
                        <h3>
                          {method === "guided"
                            ? "Bring your EPUB"
                            : "Upload your prepared source ZIP"}
                        </h3>
                        <p className="muted">
                          {method === "guided"
                            ? "We read the EPUB’s reading order, headings, and paragraphs. This first importer supports unencrypted prose EPUBs; complex tables, inline images, and advanced content need offline preparation."
                            : "Use the existing Fontes authoring tools and upload a .fontes-source.zip. The Rust validator checks the archive, references, artwork, and source schemas; its original bytes are retained."}
                        </p>
                        <input
                          aria-label={
                            method === "guided"
                              ? "Upload EPUB"
                              : "Upload Fontes source ZIP"
                          }
                          type="file"
                          accept={method === "guided" ? ".epub" : ".zip"}
                          disabled={busy}
                          onChange={(e) =>
                            void upload(
                              e.target.files?.[0],
                              method === "guided" ? "epub" : "prepared",
                            )
                          }
                        />
                      </Card.Content>
                    </Card>
                    {reply?.state.source && (
                      <p role="status">
                        <Check size={16} />
                        {reply.state.documents.length} reading-order sections,{" "}
                        {reply.state.documents.reduce(
                          (n, d) => n + d.blocks.length,
                          0,
                        )}{" "}
                        blocks prepared.
                      </p>
                    )}
                    <Card>
                      <Card.Content>
                        <div className="section-heading">
                          <h3>Optional AI enrichment</h3>
                          <Chip size="sm">Not connected</Chip>
                        </div>
                        <p className="muted">
                          A future service can suggest themes, allusions, and
                          links with evidence. It will require explicit
                          processing permission. Nothing is sent to an AI
                          service today.
                        </p>
                        <p className="muted">
                          The current explicit-reference detector offers
                          reviewable candidates; it never rewrites your prose or
                          invents verses.
                        </p>
                      </Card.Content>
                    </Card>
                  </div>
                )}
                {final && (
                  <div className="resource-form">
                    <div className="section-heading">
                      <h3>
                        {reply?.state.documents.length ?? 0} sections to review
                      </h3>
                      <Chip
                        size="sm"
                        color={
                          reply?.approval && !dirty ? "success" : "default"
                        }
                      >
                        {reply?.approval && !dirty
                          ? "Publisher approved"
                          : "Draft"}
                      </Chip>
                    </div>
                    {reply?.state.source?.resourceId && (
                      <p className="muted">
                        Submitted package identity:{" "}
                        {reply.state.source.resourceId}. Publisher authority
                        still requires verification.
                      </p>
                    )}
                    <div className="intake-rights-summary">
                      <strong>
                        {fields.author || "Contributor not supplied"}
                      </strong>
                      <span>
                        {fields.publisher || "Publisher not supplied"}
                      </span>
                      <span>
                        {fields.licenseId || "Text license not supplied"} ·{" "}
                        {fields.rightsHolder || "Rights holder not supplied"}
                      </span>
                    </div>
                    <dl className="intake-rights-summary">
                      <div>
                        <dt>Copyright</dt>
                        <dd>{fields.copyright}</dd>
                      </div>
                      <div>
                        <dt>Text attribution</dt>
                        <dd>{fields.attribution}</dd>
                      </div>
                      <div>
                        <dt>Artwork</dt>
                        <dd>
                          {fields.coverLicenseId} · {fields.coverRightsHolder} ·{" "}
                          {fields.coverAttribution}
                        </dd>
                      </div>
                      <div>
                        <dt>Coverage</dt>
                        <dd>
                          {fields.coverageStart}
                          {fields.coverageEnd ? ` — ${fields.coverageEnd}` : ""}
                        </dd>
                      </div>
                    </dl>
                    <div className="intake-permissions">
                      {permissionKeys.map((key) => (
                        <Chip
                          key={key}
                          size="sm"
                          color={
                            fields.permissions[key] === "allow"
                              ? "success"
                              : "default"
                          }
                        >
                          {label(key)}: {fields.permissions[key]}
                        </Chip>
                      ))}
                    </div>
                    <h3>Conversion report</h3>
                    {reply?.state.warnings.map((warning, i) => (
                      <p className="muted" key={i}>
                        {warning}
                      </p>
                    ))}
                    <div className="intake-reading">
                      {reply?.state.documents.map((document) => (
                        <section key={document.id}>
                          <h3>{document.title}</h3>
                          {document.blocks.map((block) => (
                            <div key={block.id} className="intake-block">
                              {block.kind === "heading" ? (
                                <h4>{block.text}</h4>
                              ) : (
                                <p>{block.text}</p>
                              )}
                              {block.sourceAssertions?.map(
                                (assertion, index) => (
                                  <p key={index} className="muted">
                                    Supplied annotation · {assertion}
                                  </p>
                                ),
                              )}
                              {reply.state.proposals
                                .filter((p) => p.blockId === block.id)
                                .map((proposal) => (
                                  <div
                                    key={proposal.id}
                                    className="intake-proposal"
                                  >
                                    <div>
                                      <Chip size="sm" variant="soft">
                                        Reference candidate
                                      </Chip>
                                      <span>
                                        {proposal.evidence} → {proposal.start}
                                        {proposal.end ? `–${proposal.end}` : ""}
                                      </span>
                                      <small>
                                        Literal reference detector · publisher
                                        review required
                                      </small>
                                    </div>
                                    {!readOnly && (
                                      <div>
                                        <Button
                                          size="sm"
                                          variant={
                                            proposal.decision === "accepted"
                                              ? "primary"
                                              : "secondary"
                                          }
                                          onPress={() =>
                                            dirtyProposal(
                                              proposal.id,
                                              "accepted",
                                            )
                                          }
                                        >
                                          {proposal.decision === "accepted"
                                            ? "Accepted"
                                            : "Accept"}
                                        </Button>
                                        <Button
                                          size="sm"
                                          variant={
                                            proposal.decision === "rejected"
                                              ? "primary"
                                              : "ghost"
                                          }
                                          onPress={() =>
                                            dirtyProposal(
                                              proposal.id,
                                              "rejected",
                                            )
                                          }
                                        >
                                          {proposal.decision === "rejected"
                                            ? "Rejected"
                                            : "Reject"}
                                        </Button>
                                      </div>
                                    )}
                                  </div>
                                ))}
                            </div>
                          ))}
                        </section>
                      ))}
                    </div>
                    {!readOnly && (
                      <>
                        <Checkbox
                          isSelected={fields.rightsAttested}
                          onChange={(v) => field("rightsAttested", v)}
                        >
                          <Checkbox.Control>
                            <Checkbox.Indicator />
                          </Checkbox.Control>
                          <Checkbox.Content>
                            <Label>
                              I am authorized to submit this text and its
                              artwork.
                            </Label>
                          </Checkbox.Content>
                        </Checkbox>
                        <Checkbox
                          isSelected={fields.conversionAcknowledged}
                          onChange={(v) => field("conversionAcknowledged", v)}
                        >
                          <Checkbox.Control>
                            <Checkbox.Indicator />
                          </Checkbox.Control>
                          <Checkbox.Content>
                            <Label>
                              I reviewed the rendered sections, references, and
                              conversion report against the original.
                            </Label>
                          </Checkbox.Content>
                        </Checkbox>
                        <p className="muted">
                          Your approval confirms this exact preparation draft.
                          Fontes editorial/rights review, signing, and app
                          delivery are separate.
                        </p>
                        <Button
                          isPending={busy}
                          isDisabled={!reply?.state.source}
                          onPress={async () => {
                            setBusy(true);
                            setError("");
                            try {
                              const saved = await save();
                              const approved = await call<IntakeReply>(
                                endpoint(saved.resourceId) + "/approve",
                                {
                                  method: "POST",
                                  body: JSON.stringify({
                                    revision: saved.revision,
                                  }),
                                },
                              );
                              adopt(approved);
                            } catch (e) {
                              setError((e as Error).message);
                            } finally {
                              setBusy(false);
                            }
                          }}
                        >
                          <Check size={17} />
                          Approve this preparation
                        </Button>
                        {reply?.approval && !dirty && (
                          <Button
                            variant="secondary"
                            onPress={() =>
                              window.location.assign(endpoint() + "/package")
                            }
                          >
                            <FileArchive size={17} />
                            Download source package
                          </Button>
                        )}
                      </>
                    )}
                  </div>
                )}
                {error && (
                  <p role="alert" className="form-error">
                    {error}
                  </p>
                )}
              </form>
            </Modal.Body>
            <Modal.Footer>
              {step > 0 && !readOnly && (
                <Button
                  variant="ghost"
                  isDisabled={busy}
                  onPress={() => setStep(step - 1)}
                >
                  <ArrowLeft size={16} />
                  Back
                </Button>
              )}
              <Button
                variant="ghost"
                isDisabled={busy}
                onPress={async () => {
                  if (id && !readOnly) {
                    setBusy(true);
                    try {
                      await save();
                      await onSaved(id);
                    } catch (e) {
                      setError((e as Error).message);
                    } finally {
                      setBusy(false);
                    }
                  } else onClose();
                }}
              >
                {readOnly ? "Close" : "Save & close"}
              </Button>
              {!final && !readOnly && (
                <Button
                  type="submit"
                  form="intake-form"
                  isDisabled={
                    busy ||
                    ((step === 3 || (method === "prepared" && step === 1)) &&
                      !reply?.state.source)
                  }
                >
                  Continue
                  <ArrowRight size={16} />
                </Button>
              )}
              {final && reply?.approval && !dirty && !readOnly && (
                <Button
                  isDisabled={busy}
                  onPress={() => id && void onSaved(id)}
                >
                  Done
                  <Check size={16} />
                </Button>
              )}
            </Modal.Footer>
          </Modal.Dialog>
        </Modal.Container>
      </Modal.Backdrop>
    </Modal>
  );
}

export function ResourcePreparation({
  resource,
  owner,
  onUpdate,
}: {
  resource: Resource;
  owner: boolean;
  onUpdate: () => Promise<void>;
}) {
  const [open, setOpen] = useState(false);
  const [sourceReady, setSourceReady] = useState(false);
  useEffect(() => {
    let active = true;
    setSourceReady(false);
    if (owner && resource.has_authoring)
      void fetch(`/api/v1/resources/${resource.id}/intake`, {
        cache: "no-store",
      })
        .then((r) => (r.ok ? r.json() : null))
        .then((r) => {
          if (active)
            setSourceReady(!!r?.data?.approval && r.data.checks.length === 0);
        })
        .catch(() => {});
    return () => {
      active = false;
    };
  }, [owner, resource.id, resource.revision, resource.has_authoring]);
  const editable =
    owner && ["draft", "changes_requested"].includes(resource.status);
  return (
    <>
      <div className="source-empty">
        <FileText size={26} />
        <div>
          <strong>
            {resource.has_authoring
              ? "Your preparation draft"
              : "Two paths to a Fontes resource"}
          </strong>
          <p>
            Upload an offline-prepared package, or use the guided EPUB workflow.
            Review the readable draft before approving exact source preparation.
          </p>
          {(editable || resource.has_authoring) && (
            <Button variant="secondary" onPress={() => setOpen(true)}>
              {editable
                ? resource.has_authoring
                  ? "Continue preparation"
                  : "Prepare source"
                : "Read prepared draft"}
              <ArrowRight size={16} />
            </Button>
          )}
        </div>
      </div>
      {owner && resource.has_authoring && (
        <SubmitSourceReview
          source={{ kind: "personal", resourceId: resource.id }}
          revision={resource.revision}
          disabled={!sourceReady}
        />
      )}
      <IntakeWizard
        open={open}
        resource={resource}
        readOnly={!editable}
        onClose={() => setOpen(false)}
        onSaved={async () => {
          setOpen(false);
          await onUpdate();
        }}
      />
    </>
  );
}
