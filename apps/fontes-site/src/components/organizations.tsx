"use client";
import { OrganizationEpubReview } from "./organization-epub-review";
import { useCallback, useEffect, useState, type ReactNode } from "react";
import {
  Button,
  Card,
  Chip,
  Input,
  Label,
  ListBox,
  Modal,
  Select,
  Spinner,
  Tabs,
  TextArea,
  TextField,
} from "@heroui/react";
import {
  ArrowLeft,
  ArrowRight,
  Copy,
  Plus,
  Users,
  LockKeyhole,
  Mail,
  BookOpen,
  LogOut,
  RefreshCw,
} from "lucide-react";
import type { Actor } from "@/lib/contracts";
import { kinds } from "@/lib/contracts";
import {
  organizationRoles,
  type Organization,
  type OrganizationDetail,
  type InvitationPreview,
} from "@/lib/organization-contracts";
async function call<T>(
  path: string,
  method = "GET",
  body?: unknown,
): Promise<T> {
  const response = await fetch(`/api/v1/${path}`, {
    method,
    cache: "no-store",
    headers: { "Content-Type": "application/json" },
    ...(body === undefined ? {} : { body: JSON.stringify(body) }),
  });
  const result = await response.json();
  if (!response.ok)
    throw Object.assign(
      new Error(result.error?.message ?? "The request failed."),
      { code: result.error?.code },
    );
  return result.data;
}
const title = (s: string) =>
  s === "retain_downloads"
    ? "Keep downloaded copies"
    : s === "renewable_lease"
      ? "Renewable offline lease"
      : s
          .replaceAll("_", " ")
          .replaceAll("-", " ")
          .replace(/^./, (c) => c.toUpperCase());
const date = (s: string) =>
  new Date(s).toLocaleDateString(undefined, { month: "short", day: "numeric" });
function Choice({
  label,
  value,
  onChange,
  options,
  disabled = false,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  options: readonly string[];
  disabled?: boolean;
}) {
  return (
    <Select
      aria-label={label}
      selectedKey={value}
      onSelectionChange={(key) => key && onChange(String(key))}
      isDisabled={disabled}
    >
      <Label>{label}</Label>
      <Select.Trigger>
        <Select.Value />
      </Select.Trigger>
      <Select.Popover>
        <ListBox>
          {options.map((v) => (
            <ListBox.Item key={v} id={v} textValue={title(v)}>
              {title(v)}
              <ListBox.ItemIndicator />
            </ListBox.Item>
          ))}
        </ListBox>
      </Select.Popover>
    </Select>
  );
}
function Dialog({
  title: heading,
  onClose,
  children,
  busy = false,
}: {
  title: string;
  onClose: () => void;
  children: ReactNode;
  busy?: boolean;
}) {
  return (
    <Modal isOpen onOpenChange={(open) => !open && !busy && onClose()}>
      <Modal.Backdrop>
        <Modal.Container size="lg">
          <Modal.Dialog>
            <Modal.CloseTrigger isDisabled={busy} />
            <Modal.Header>
              <Modal.Heading>{heading}</Modal.Heading>
            </Modal.Header>
            <Modal.Body>{children}</Modal.Body>
          </Modal.Dialog>
        </Modal.Container>
      </Modal.Backdrop>
    </Modal>
  );
}
const policyLabels = {
  retain_downloads: "Keep downloaded copies",
  renewable_lease: "Renewable offline lease",
};
function PolicyField({
  value,
  disabled,
}: {
  value: string;
  disabled: boolean;
}) {
  const [policy, setPolicy] = useState(value);
  return (
    <>
      <input type="hidden" name="policy" value={policy} />
      <Choice
        label="Planned download policy"
        value={policy}
        onChange={setPolicy}
        options={["retain_downloads", "renewable_lease"]}
        disabled={disabled}
      />
      <p className="muted">
        {policyLabels[policy as keyof typeof policyLabels]}
      </p>
    </>
  );
}
export function Organizations({ actor }: { actor: Actor }) {
  const [reviewResource, setReviewResource] = useState<{
    id: string;
    title: string;
  } | null>(null);
  const [items, setItems] = useState<Organization[]>([]),
    [detail, setDetail] = useState<OrganizationDetail | null>(null),
    [selected, setSelected] = useState<string | null>(null),
    [tab, setTab] = useState("resources");
  const [loading, setLoading] = useState(true),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [notice, setNotice] = useState("");
  const [modal, setModal] = useState<
      "create" | "invite" | "resource" | "join" | null
    >(null),
    [editResource, setEditResource] = useState<
      OrganizationDetail["resources"][number] | null
    >(null);
  const [inviteRole, setInviteRole] = useState("member"),
    [resourceKind, setResourceKind] = useState("study-guide"),
    [link, setLink] = useState("");
  const [token, setToken] = useState(""),
    [preview, setPreview] = useState<InvitationPreview | null>(null),
    [roleTarget, setRoleTarget] = useState<
      OrganizationDetail["members"][number] | null
    >(null),
    [newRole, setNewRole] = useState("member");
  const [confirm, setConfirm] = useState<{
    kind: "remove" | "leave";
    account_id?: string;
    name: string;
  } | null>(null);
  const load = useCallback(async () => {
    setLoading(true);
    try {
      const list = await call<{ items: Organization[] }>("organizations");
      setItems(list.items);
      if (selected) {
        try {
          setDetail(
            await call<OrganizationDetail>(`organizations/${selected}`),
          );
        } catch (e) {
          setDetail(null);
          setSelected(null);
          throw e;
        }
      } else setDetail(null);
    } finally {
      setLoading(false);
    }
  }, [selected, actor.id]);
  useEffect(() => {
    setError("");
    void load().catch((e) => setError(e.message));
  }, [load]);
  useEffect(() => {
    setSelected(null);
    setDetail(null);
    setLink("");
    setPreview(null);
  }, [actor.id]);
  useEffect(() => {
    const hash = new URLSearchParams(window.location.hash.slice(1));
    const invitation = hash.get("invite");
    if (invitation) {
      setToken(invitation);
      setModal("join");
      window.history.replaceState(
        null,
        "",
        window.location.pathname + window.location.search,
      );
    }
  }, []);
  async function work(fn: () => Promise<void>) {
    setBusy(true);
    setError("");
    setNotice("");
    try {
      await fn();
    } catch (e) {
      const failure = e as Error & { code?: string };
      setError(failure.message);
      if (failure.code === "revision_conflict") {
        try {
          await load();
          setError(
            `${failure.message} The current workspace is loaded; review your entries and save again.`,
          );
        } catch (refreshError) {
          setError((refreshError as Error).message);
        }
      }
    } finally {
      setBusy(false);
    }
  }
  const otherPersona = actor.role === "reviewer" ? "publisher" : "reviewer";
  const org = detail?.organization;
  const revision = org?.revision;
  const path = `organizations/${selected}`;
  const close = () => {
    setModal(null);
    setError("");
    setLink("");
    setPreview(null);
    setToken("");
  };
  const saveLink = (result: { token: string }) => {
    setLink(
      `${window.location.origin}/?view=organizations#invite=${result.token}`,
    );
    setNotice(
      "Invitation link created. Copy it and share it with the intended recipient. No email was sent.",
    );
  };
  const tokenFrom = (value: string) => {
    try {
      return (
        new URLSearchParams(new URL(value).hash.slice(1)).get("invite") ??
        value.trim()
      );
    } catch {
      return value.trim();
    }
  };
  if (reviewResource && selected)
    return (
      <OrganizationEpubReview
        organizationId={selected}
        resource={reviewResource}
        onBack={() => {
          setReviewResource(null);
          void work(load);
        }}
      />
    );
  return (
    <section className="organizations-workspace">
      <div className="org-toolbar">
        {selected ? (
          <Button
            variant="ghost"
            onPress={() => {
              setSelected(null);
              setDetail(null);
              setLink("");
              setNotice("");
            }}
          >
            <ArrowLeft size={16} />
            All organizations
          </Button>
        ) : (
          <p className="muted">Your memberships and publishing teams.</p>
        )}
        <div className="org-actions">
          <Button
            variant="ghost"
            isDisabled={busy || loading}
            onPress={() => void work(load)}
          >
            <RefreshCw size={16} />
            Refresh
          </Button>
          <Button
            variant="secondary"
            onPress={() => {
              setError("");
              setModal("join");
            }}
          >
            Open invitation
          </Button>
          <Button
            onPress={() => {
              setError("");
              setModal("create");
            }}
          >
            <Plus size={16} />
            Create organization
          </Button>
        </div>
      </div>
      {error && (
        <p className="form-error" role="alert">
          {error}
          <Button variant="ghost" size="sm" onPress={() => void work(load)}>
            Refresh
          </Button>
        </p>
      )}
      {notice && (
        <p className="org-notice" role="status">
          {notice}
        </p>
      )}
      {loading ? (
        <div className="org-loading">
          <Spinner />
          <span>Loading organizations…</span>
        </div>
      ) : !detail ? (
        <>
          {items.length ? (
            <div className="org-grid">
              {items.map((o) => (
                <Card key={o.id}>
                  <Card.Header>
                    <div className="org-avatar">
                      <Users size={22} />
                    </div>
                    <Card.Title>{o.name}</Card.Title>
                  </Card.Header>
                  <Card.Content>
                    <Chip size="sm" variant="secondary">
                      {title(o.role)}
                    </Chip>
                    {o.is_demo && (
                      <Chip size="sm" variant="soft">
                        Development
                      </Chip>
                    )}
                    <p className="muted">
                      {o.description ||
                        "An organization workspace for your team."}
                    </p>
                  </Card.Content>
                  <Card.Footer>
                    <Button
                      variant="secondary"
                      onPress={() => {
                        setSelected(o.id);
                        setTab("resources");
                        setLink("");
                      }}
                    >
                      Open workspace
                      <ArrowRight size={16} />
                    </Button>
                  </Card.Footer>
                </Card>
              ))}
            </div>
          ) : (
            <Card>
              <Card.Content>
                <div className="org-empty">
                  <Users size={32} />
                  <h2>Start a shared workspace.</h2>
                  <p>
                    Create an organization for your church, school, seminary or
                    publishing team. Invite collaborators and prepare private
                    resource metadata together.
                  </p>
                  <Button onPress={() => setModal("create")}>
                    <Plus size={16} />
                    Create your first organization
                  </Button>
                </div>
              </Card.Content>
            </Card>
          )}
          <p className="muted org-boundary">
            <LockKeyhole size={16} />
            Organization-only publication and app downloads are still in
            development. Membership and draft access work now.
          </p>
        </>
      ) : (
        <>
          <div className="org-heading">
            <div>
              <p className="eyebrow">ORGANIZATION WORKSPACE</p>
              <h2>{org!.name}</h2>
              <p className="muted">{org!.description}</p>
            </div>
            <Chip variant="secondary">{title(org!.role)}</Chip>
          </div>
          <Tabs
            selectedKey={tab}
            onSelectionChange={(key) => {
              setTab(String(key));
              setLink("");
              setError("");
            }}
            variant="secondary"
          >
            <Tabs.List aria-label="Organization sections">
              {["resources", "members", "settings", "activity"].map((key) => (
                <Tabs.Tab key={key} id={key}>
                  {title(key)}
                  <Tabs.Indicator />
                </Tabs.Tab>
              ))}
            </Tabs.List>
          </Tabs>
          {tab === "resources" && (
            <Card>
              <Card.Header>
                <div>
                  <Card.Title>Private resource drafts</Card.Title>
                  <p className="muted">
                    Owned by {org!.name}. Visible to owners, admins and
                    publishers.
                  </p>
                </div>
                {detail.capabilities.prepare_metadata && (
                  <Button
                    onPress={() => {
                      setEditResource(null);
                      setResourceKind("study-guide");
                      setModal("resource");
                    }}
                  >
                    <Plus size={16} />
                    New organization resource
                  </Button>
                )}
              </Card.Header>
              <Card.Content>
                {!detail.capabilities.prepare_metadata ? (
                  <p className="muted">
                    Your member role will have access to released resources when
                    private delivery is available. Unpublished drafts are
                    visible only to the publishing team.
                  </p>
                ) : detail.resources.length ? (
                  <div className="org-rows">
                    {detail.resources.map((r) => (
                      <div className="org-row" key={r.id}>
                        <BookOpen size={20} />
                        <div className="org-row-copy">
                          <strong>{r.title}</strong>
                          <p>
                            {title(r.kind)} · v{r.version} · {r.language}
                          </p>
                        </div>
                        <Chip size="sm" variant="secondary">
                          Draft
                        </Chip>
                        <Button
                          variant="secondary"
                          onPress={() =>
                            setReviewResource({ id: r.id, title: r.title })
                          }
                        >
                          {r.has_preparation
                            ? "Read source draft"
                            : "Prepare EPUB"}
                        </Button>
                        <Button
                          variant="ghost"
                          onPress={() => {
                            setEditResource(r);
                            setResourceKind(r.kind);
                            setModal("resource");
                          }}
                        >
                          Edit
                        </Button>
                      </div>
                    ))}
                  </div>
                ) : (
                  <div className="org-empty">
                    <BookOpen size={28} />
                    <h3>Give your next resource a home.</h3>
                    <p>
                      Start with metadata for a resource owned by this
                      organization.
                    </p>
                  </div>
                )}
                <p className="muted org-boundary">
                  Local EPUB corrections, publisher approval and validated
                  source export are available to the publishing team. Private
                  publication and member downloads remain planned. These drafts
                  stay out of the personal library and Fontes review queue.
                </p>
              </Card.Content>
            </Card>
          )}
          {tab === "members" && (
            <div className="org-stack">
              <Card>
                <Card.Header>
                  <div>
                    <Card.Title>People in your organization</Card.Title>
                    <p className="muted">
                      Owners manage ownership. Admins manage publishers and
                      members. Publishers prepare drafts; members receive future
                      releases.
                    </p>
                  </div>
                  {detail.capabilities.manage && (
                    <Button
                      onPress={() => {
                        setInviteRole("member");
                        setLink("");
                        setModal("invite");
                      }}
                    >
                      <Mail size={16} />
                      Invite member
                    </Button>
                  )}
                </Card.Header>
                <Card.Content>
                  <div className="org-rows">
                    {detail.members.map((m) => (
                      <div className="org-row" key={m.account_id}>
                        <div className="org-avatar">{m.name.slice(0, 1)}</div>
                        <div className="org-row-copy">
                          <strong>
                            {m.name}
                            {m.account_id === actor.id ? " (you)" : ""}
                          </strong>
                          {m.email && <p>{m.email}</p>}
                        </div>
                        <Chip size="sm" variant="secondary">
                          {title(m.role)}
                        </Chip>
                        {detail.capabilities.manage &&
                          (org!.role === "owner" ||
                            !["owner", "admin"].includes(m.role)) && (
                            <>
                              <Button
                                size="sm"
                                variant="ghost"
                                onPress={() => {
                                  setRoleTarget(m);
                                  setNewRole(m.role);
                                }}
                              >
                                Change role
                              </Button>
                              {m.account_id !== actor.id && (
                                <Button
                                  size="sm"
                                  variant="ghost"
                                  onPress={() =>
                                    setConfirm({
                                      kind: "remove",
                                      account_id: m.account_id,
                                      name: m.name,
                                    })
                                  }
                                >
                                  Remove
                                </Button>
                              )}
                            </>
                          )}
                      </div>
                    ))}
                  </div>
                </Card.Content>
                <Card.Footer>
                  <Button
                    variant="ghost"
                    onPress={() =>
                      setConfirm({ kind: "leave", name: org!.name })
                    }
                  >
                    <LogOut size={16} />
                    Leave organization
                  </Button>
                </Card.Footer>
              </Card>
              {detail.capabilities.manage && (
                <Card>
                  <Card.Header>
                    <Card.Title>Invitations</Card.Title>
                  </Card.Header>
                  <Card.Content>
                    {link && (
                      <div className="org-link">
                        <TextField value={link} isReadOnly>
                          <Label>New invitation link</Label>
                          <Input />
                        </TextField>
                        <Button
                          variant="secondary"
                          onPress={() =>
                            void work(async () => {
                              await navigator.clipboard.writeText(link);
                              setNotice("Invitation link copied.");
                            })
                          }
                        >
                          <Copy size={16} />
                          Copy link
                        </Button>
                      </div>
                    )}
                    {detail.invitations.length ? (
                      <div className="org-rows">
                        {detail.invitations.map((i) => (
                          <div className="org-row" key={i.id}>
                            <div className="org-row-copy">
                              <strong>{i.email}</strong>
                              <p>
                                {title(i.role)} · expires {date(i.expires_at)}
                              </p>
                            </div>
                            <Chip size="sm" variant="secondary">
                              {title(i.status)}
                            </Chip>
                            {["pending", "expired"].includes(i.status) &&
                              (org!.role === "owner" || i.role !== "admin") && (
                                <>
                                  <Button
                                    size="sm"
                                    variant="ghost"
                                    isDisabled={busy}
                                    onPress={() =>
                                      void work(async () => {
                                        const result = await call<{
                                          token: string;
                                        }>(
                                          `${path}/invitations/${i.id}`,
                                          "POST",
                                          { revision },
                                        );
                                        saveLink(result);
                                        await load();
                                      })
                                    }
                                  >
                                    <RefreshCw size={14} />
                                    Renew link
                                  </Button>
                                  <Button
                                    size="sm"
                                    variant="ghost"
                                    isDisabled={busy}
                                    onPress={() =>
                                      void work(async () => {
                                        await call(
                                          `${path}/invitations/${i.id}`,
                                          "DELETE",
                                          { revision },
                                        );
                                        setLink("");
                                        await load();
                                        setNotice("Invitation revoked.");
                                      })
                                    }
                                  >
                                    Revoke
                                  </Button>
                                </>
                              )}
                          </div>
                        ))}
                      </div>
                    ) : (
                      <p className="muted">No invitations yet.</p>
                    )}
                    <p className="muted org-boundary">
                      Links expire after seven days and require the matching
                      verified email. Renewing a link invalidates the old one.
                      Email delivery is not configured.
                    </p>
                  </Card.Content>
                </Card>
              )}
            </div>
          )}
          {tab === "settings" && (
            <Card>
              <Card.Header>
                <Card.Title>
                  Organization details & access preferences
                </Card.Title>
              </Card.Header>
              <Card.Content>
                <form
                  key={org!.revision}
                  className="resource-form"
                  onSubmit={(e) => {
                    e.preventDefault();
                    const f = new FormData(e.currentTarget);
                    void work(async () => {
                      await call(path, "PATCH", {
                        name: f.get("name"),
                        description: f.get("description"),
                        offline_policy_preference: f.get("policy"),
                        revision,
                      });
                      await load();
                      setNotice("Organization settings saved.");
                    });
                  }}
                >
                  <TextField
                    name="name"
                    defaultValue={org!.name}
                    isRequired
                    isReadOnly={!detail.capabilities.manage}
                  >
                    <Label>Organization name</Label>
                    <Input maxLength={100} />
                  </TextField>
                  <TextField
                    name="description"
                    defaultValue={org!.description}
                    isReadOnly={!detail.capabilities.manage}
                  >
                    <Label>About your organization</Label>
                    <TextArea maxLength={2000} />
                  </TextField>
                  <PolicyField
                    value={org!.offline_policy_preference}
                    disabled={!detail.capabilities.manage}
                  />
                  <p className="muted">
                    This is a recorded preference for future releases, not an
                    active license. Kept copies remain readable after membership
                    ends. A renewable lease would allow offline reading until
                    its disclosed expiration; durations and device enforcement
                    are not implemented. Neither option recalls bytes
                    immediately from an offline device.
                  </p>
                  <p className="muted">
                    Policy revision {org!.policy_revision} · Workspace revision{" "}
                    {revision}
                  </p>
                  <TextField value={org!.namespace} isReadOnly>
                    <Label>Stable organization namespace (unverified)</Label>
                    <Input />
                  </TextField>
                  {detail.capabilities.manage && (
                    <Button type="submit" isPending={busy}>
                      Save settings
                    </Button>
                  )}
                </form>
              </Card.Content>
            </Card>
          )}
          {tab === "activity" && (
            <Card>
              <Card.Header>
                <Card.Title>Organization activity</Card.Title>
              </Card.Header>
              <Card.Content>
                <div className="org-rows">
                  {detail.activity.map((e) => (
                    <div key={e.id} className="org-row">
                      <div className="org-row-copy">
                        <strong>{title(e.action)}</strong>
                        <p>{e.actor_name}</p>
                      </div>
                      <span className="muted">{date(e.created_at)}</span>
                    </div>
                  ))}
                </div>
                <p className="muted">Latest 30 organization actions.</p>
              </Card.Content>
            </Card>
          )}
        </>
      )}
      {modal && (
        <Dialog
          title={
            modal === "create"
              ? "Create an organization"
              : modal === "invite"
                ? "Invite a collaborator"
                : modal === "join"
                  ? "Open your invitation"
                  : editResource
                    ? "Edit organization draft"
                    : "New organization resource"
          }
          busy={busy}
          onClose={close}
        >
          {error && (
            <p className="form-error" role="alert">
              {error}
            </p>
          )}
          {modal === "create" && (
            <form
              className="resource-form"
              onSubmit={(e) => {
                e.preventDefault();
                const f = new FormData(e.currentTarget);
                void work(async () => {
                  const created = await call<OrganizationDetail>(
                    "organizations",
                    "POST",
                    { name: f.get("name"), description: f.get("description") },
                  );
                  setModal(null);
                  setSelected(created.organization.id);
                  setDetail(created);
                  setTab("members");
                  setNotice("Organization created. You are its first owner.");
                });
              }}
            >
              <TextField name="name" isRequired>
                <Label>Organization name</Label>
                <Input
                  placeholder="Your church, school or publishing team"
                  maxLength={100}
                />
              </TextField>
              <TextField name="description">
                <Label>About your organization</Label>
                <TextArea maxLength={2000} />
              </TextField>
              <p className="muted">
                An organization workspace is private to its members. You can
                invite collaborators after creating it. This does not verify
                publisher identity or enable resource delivery.
              </p>
              <div className="org-dialog-actions">
                <Button variant="ghost" onPress={close}>
                  Cancel
                </Button>
                <Button type="submit" isPending={busy}>
                  Create organization
                </Button>
              </div>
            </form>
          )}
          {modal === "invite" && (
            <form
              className="resource-form"
              onSubmit={(e) => {
                e.preventDefault();
                const f = new FormData(e.currentTarget);
                void work(async () => {
                  const result = await call<{ token: string }>(
                    `${path}/invitations`,
                    "POST",
                    { email: f.get("email"), role: inviteRole, revision },
                  );
                  saveLink(result);
                  await load();
                });
              }}
            >
              {link ? (
                <>
                  <TextField value={link} isReadOnly>
                    <Label>Invitation link — shown only now</Label>
                    <Input />
                  </TextField>
                  <p className="muted">
                    Share this link with the intended recipient. It is bound to
                    their verified email. No email has been sent.
                  </p>
                  <Button
                    variant="secondary"
                    onPress={() =>
                      void work(async () => {
                        await navigator.clipboard.writeText(link);
                        setNotice("Invitation link copied.");
                      })
                    }
                  >
                    <Copy size={16} />
                    Copy invitation link
                  </Button>
                  <Button onPress={close}>Done</Button>
                </>
              ) : (
                <>
                  <TextField name="email" type="email" isRequired>
                    <Label>Recipient email</Label>
                    <Input
                      placeholder={
                        actor.mode === "development"
                          ? `${otherPersona}@example.invalid`
                          : "colleague@example.org"
                      }
                    />
                  </TextField>
                  <Choice
                    label="Organization role"
                    value={inviteRole}
                    onChange={setInviteRole}
                    options={
                      org?.role === "owner"
                        ? ["admin", "publisher", "member"]
                        : ["publisher", "member"]
                    }
                  />
                  <p className="muted">
                    Create a single-use link valid for seven days. Only the
                    matching verified account can accept. Owners are assigned
                    from existing members.
                  </p>
                  {actor.mode === "development" && (
                    <p className="muted">
                      Try {otherPersona}@example.invalid, then switch to{" "}
                      {title(otherPersona)} in Workspace settings and open the
                      copied invitation.
                    </p>
                  )}
                  <div className="org-dialog-actions">
                    <Button variant="ghost" onPress={close}>
                      Cancel
                    </Button>
                    <Button type="submit" isPending={busy}>
                      Create invitation link
                    </Button>
                  </div>
                </>
              )}
            </form>
          )}
          {modal === "join" && (
            <div className="resource-form">
              <p className="muted">
                Signed in as {actor.name} ({actor.email}). You must use the
                verified email named by the invitation.
              </p>
              {!preview ? (
                <>
                  <TextField value={token} onChange={setToken}>
                    <Label>Invitation link or code</Label>
                    <Input />
                  </TextField>
                  <Button
                    isPending={busy}
                    isDisabled={!token.trim()}
                    onPress={() =>
                      void work(async () =>
                        setPreview(
                          await call<InvitationPreview>(
                            "invitations/preview",
                            "POST",
                            { token: tokenFrom(token) },
                          ),
                        ),
                      )
                    }
                  >
                    Review invitation
                  </Button>
                </>
              ) : (
                <>
                  <h3>{preview.organization_name}</h3>
                  <p>
                    {preview.inviter_name} invited you as{" "}
                    <strong>{title(preview.role)}</strong>. The invitation
                    expires {date(preview.expires_at)}.
                  </p>
                  <p className="muted">
                    Accepting joins this organization. Your personal drafts and
                    local study notes stay separate.
                  </p>
                  <Button
                    isPending={busy}
                    onPress={() =>
                      void work(async () => {
                        const joined = await call<OrganizationDetail>(
                          "invitations/accept",
                          "POST",
                          { token: tokenFrom(token) },
                        );
                        close();
                        setSelected(joined.organization.id);
                        setDetail(joined);
                        setTab("resources");
                        setNotice("You joined the organization.");
                      })
                    }
                  >
                    Accept invitation
                  </Button>
                </>
              )}
            </div>
          )}
          {modal === "resource" && (
            <form
              className="resource-form"
              onSubmit={(e) => {
                e.preventDefault();
                const f = new FormData(e.currentTarget);
                void work(async () => {
                  await call(
                    `${path}/resources${editResource ? `/${editResource.id}` : ""}`,
                    editResource ? "PATCH" : "POST",
                    {
                      title: f.get("title"),
                      description: f.get("description"),
                      kind: resourceKind,
                      language: f.get("language"),
                      version: f.get("version"),
                      revision,
                      ...(editResource
                        ? { resource_revision: editResource.revision }
                        : {}),
                    },
                  );
                  close();
                  await load();
                  setNotice("Private organization draft saved.");
                });
              }}
            >
              <TextField
                name="title"
                isRequired
                defaultValue={editResource?.title ?? ""}
              >
                <Label>Resource title</Label>
                <Input minLength={2} maxLength={180} />
              </TextField>
              <TextField
                name="description"
                isRequired
                defaultValue={editResource?.description ?? ""}
              >
                <Label>Description</Label>
                <TextArea minLength={10} maxLength={4000} />
              </TextField>
              <Choice
                label="Resource type"
                value={resourceKind}
                onChange={setResourceKind}
                options={kinds}
              />
              <div className="form-row">
                <TextField
                  name="language"
                  isRequired
                  defaultValue={editResource?.language ?? "en"}
                >
                  <Label>Language code</Label>
                  <Input maxLength={40} />
                </TextField>
                <TextField
                  name="version"
                  isRequired
                  defaultValue={editResource?.version ?? "1.0.0"}
                >
                  <Label>Version</Label>
                  <Input pattern="[0-9]+\.[0-9]+\.[0-9]+" />
                </TextField>
              </div>
              <p className="muted">
                This saves organization-owned metadata. Source preparation and
                publishing are a later step.
              </p>
              <div className="org-dialog-actions">
                <Button variant="ghost" onPress={close}>
                  Cancel
                </Button>
                <Button type="submit" isPending={busy}>
                  Save private draft
                </Button>
              </div>
            </form>
          )}
        </Dialog>
      )}
      {roleTarget && (
        <Dialog
          title={`Change ${roleTarget.name}’s role`}
          busy={busy}
          onClose={() => setRoleTarget(null)}
        >
          <div className="resource-form">
            <Choice
              label="New organization role"
              value={newRole}
              onChange={setNewRole}
              options={
                org?.role === "owner"
                  ? organizationRoles
                  : ["publisher", "member"]
              }
            />
            <p className="muted">
              This changes organization access immediately. A Fontes reviewer
              role does not grant organization permissions.
            </p>
            {error && (
              <p className="form-error" role="alert">
                {error}
              </p>
            )}
            <div className="org-dialog-actions">
              <Button variant="ghost" onPress={() => setRoleTarget(null)}>
                Cancel
              </Button>
              <Button
                isPending={busy}
                onPress={() =>
                  void work(async () => {
                    await call(
                      `${path}/members/${encodeURIComponent(roleTarget.account_id)}`,
                      "PATCH",
                      { role: newRole, revision },
                    );
                    setRoleTarget(null);
                    await load();
                    setNotice("Organization role updated.");
                  })
                }
              >
                Save role
              </Button>
            </div>
          </div>
        </Dialog>
      )}
      {confirm && (
        <Dialog
          title={
            confirm.kind === "leave"
              ? `Leave ${confirm.name}?`
              : `Remove ${confirm.name}?`
          }
          busy={busy}
          onClose={() => setConfirm(null)}
        >
          <div className="resource-form">
            <p>
              Online access ends immediately. This action does not erase
              personal notes or recall previously downloaded files. The last
              owner must assign another owner first.
            </p>
            {error && (
              <p className="form-error" role="alert">
                {error}
              </p>
            )}
            <div className="org-dialog-actions">
              <Button variant="ghost" onPress={() => setConfirm(null)}>
                Cancel
              </Button>
              <Button
                variant="danger"
                isPending={busy}
                onPress={() =>
                  void work(async () => {
                    await call(
                      confirm.kind === "leave"
                        ? `${path}/leave`
                        : `${path}/members/${encodeURIComponent(confirm.account_id!)}`,
                      confirm.kind === "leave" ? "POST" : "DELETE",
                      { revision },
                    );
                    const leaving = confirm.kind === "leave";
                    setConfirm(null);
                    if (leaving) {
                      setSelected(null);
                      setDetail(null);
                      setItems(
                        (await call<{ items: Organization[] }>("organizations"))
                          .items,
                      );
                    } else await load();
                    setNotice(
                      leaving
                        ? "You left the organization."
                        : "Member removed.",
                    );
                  })
                }
              >
                {confirm.kind === "leave"
                  ? "Leave organization"
                  : "Remove member"}
              </Button>
            </div>
          </div>
        </Dialog>
      )}
    </section>
  );
}
