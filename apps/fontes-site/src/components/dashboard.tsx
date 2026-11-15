"use client";

import { Organizations } from "./organizations";
import { SourceReviewQueue } from "./source-review";
import { IntakeWizard, ResourcePreparation } from "./intake-wizard";

import { useCallback, useEffect, useState } from "react";
import Image from "next/image";
import {
  Avatar,
  Button,
  Card,
  Chip,
  Input,
  Label,
  ListBox,
  Modal,
  Select,
  Separator,
  Spinner,
  Table,
  Tabs,
  TextArea,
  TextField,
} from "@heroui/react";
import {
  ArrowDownToLine,
  ArrowLeft,
  ArrowRight,
  BookOpen,
  Check,
  CheckCheck,
  ChevronDown,
  CircleHelp,
  Clock3,
  Code2,
  FileText,
  FolderOpen,
  LayoutDashboard,
  Leaf,
  Library,
  ListChecks,
  LockKeyhole,
  Menu,
  Plus,
  RefreshCw,
  Search,
  Settings2,
  ShieldCheck,
  Sparkles,
  Users,
  X,
} from "lucide-react";
import {
  kinds,
  type Activity,
  type Resource,
  type ResourceStatus,
  type Workspace,
} from "@/lib/contracts";

type Page =
  | "overview"
  | "resources"
  | "review"
  | "organizations"
  | "activity"
  | "api"
  | "settings";
const statusNames: Record<ResourceStatus, string> = {
  draft: "Draft",
  submitted: "In review",
  changes_requested: "Changes requested",
  metadata_approved: "Metadata approved",
};
const kindName = (s: string) =>
  s.replaceAll("-", " ").replace(/^./, (c) => c.toUpperCase());
const formatDate = (s: string) =>
  new Date(s).toLocaleDateString(undefined, { month: "short", day: "numeric" });
const actionName: Record<string, string> = {
  created: "Draft created",
  edited: "Draft updated",
  submit: "Submitted for review",
  request_changes: "Changes requested",
  approve_metadata: "Metadata approved",
};
async function request<T>(url: string, init?: RequestInit): Promise<T> {
  const response = await fetch(url, {
    ...init,
    headers: { "Content-Type": "application/json", ...init?.headers },
    cache: "no-store",
  });
  const result = await response.json();
  if (!response.ok) {
    const error = new Error(result.error?.message || "The request failed.");
    Object.assign(error, { code: result.error?.code });
    throw error;
  }
  return result.data;
}
const navigation = [
  { id: "overview", title: "Overview", icon: LayoutDashboard },
  { id: "resources", title: "My resources", icon: Library },
  { id: "review", title: "Review queue", icon: ListChecks },
  { id: "organizations", title: "Organizations", icon: Users },
  { id: "activity", title: "Activity", icon: Clock3 },
] as const;

function Status({ status }: { status: ResourceStatus }) {
  return (
    <Chip
      size="sm"
      variant="soft"
      color={
        status === "submitted"
          ? "accent"
          : status === "changes_requested"
            ? "warning"
            : status === "metadata_approved"
              ? "success"
              : "default"
      }
    >
      <Chip.Label>{statusNames[status]}</Chip.Label>
    </Chip>
  );
}
function Cover({
  resource,
  large = false,
}: {
  resource: Resource;
  large?: boolean;
}) {
  return (
    <div
      aria-hidden="true"
      className={`book-cover cover-${resource.kind} ${large ? "large-cover" : ""}`}
    >
      <span className="cover-rule" />
      <BookOpen size={large ? 32 : 17} />
      <span className="cover-title">
        {large
          ? resource.title
          : resource.title.split(" ").slice(0, 2).join(" ")}
      </span>
      <span className="cover-author">
        FONTES · {resource.language.toUpperCase()}
      </span>
    </div>
  );
}
export function Dashboard() {
  const [reviewMode, setReviewMode] = useState("packages");
  const [data, setData] = useState<Workspace | null>(null),
    [page, setPage] = useState<Page>("overview"),
    [error, setError] = useState(""),
    [unauthorized, setUnauthorized] = useState(false);
  const [loading, setLoading] = useState(true),
    [newOpen, setNewOpen] = useState(false),
    [selectedId, setSelectedId] = useState<string | null>(null),
    [query, setQuery] = useState(""),
    [filter, setFilter] = useState("all"),
    [mobileOpen, setMobileOpen] = useState(false),
    [notice, setNotice] = useState("");
  const load = useCallback(async () => {
    try {
      const state = await request<Workspace>("/api/v1/workspace");
      setData(state);
      setUnauthorized(false);
      setError("");
    } catch (e) {
      setError((e as Error).message);
      setUnauthorized((e as { code?: string }).code === "unauthenticated");
    } finally {
      setLoading(false);
    }
  }, []);
  useEffect(() => {
    void load();
  }, [load]);
  useEffect(() => {
    const channel = new BroadcastChannel("fontes-workspace-session");
    const refresh = () => {
      void load();
    };
    const onVisible = () => {
      if (document.visibilityState === "visible") refresh();
    };
    channel.onmessage = refresh;
    window.addEventListener("focus", refresh);
    document.addEventListener("visibilitychange", onVisible);
    return () => {
      channel.close();
      window.removeEventListener("focus", refresh);
      document.removeEventListener("visibilitychange", onVisible);
    };
  }, [load]);
  useEffect(() => {
    const id = new URLSearchParams(window.location.search).get("view");
    if (
      [
        "overview",
        "resources",
        "review",
        "organizations",
        "activity",
        "api",
        "settings",
      ].includes(id || "")
    )
      setPage(id as Page);
  }, []);
  const navigate = (next: Page) => {
    setPage(next);
    setSelectedId(null);
    setMobileOpen(false);
    setQuery("");
    setFilter("all");
    window.history.replaceState(null, "", `?view=${next}`);
  };
  const selected = data?.resources.find((r) => r.id === selectedId);
  const resources = data?.resources ?? [];
  const pending = resources.filter((r) => r.status === "submitted").length;
  const changes = resources.filter(
    (r) => r.status === "changes_requested",
  ).length;
  const displayed = resources.filter(
    (r) =>
      (page !== "review" || r.status === "submitted") &&
      (filter === "all" || r.status === filter) &&
      `${r.title} ${r.kind}`.toLowerCase().includes(query.toLowerCase()),
  );
  const update = async () => {
    await load();
  };

  return (
    <div className="workspace-shell">
      <a href="#main" className="skip-link">
        Skip to content
      </a>
      {mobileOpen && (
        <button
          className="sidebar-scrim"
          aria-label="Close navigation"
          onClick={() => setMobileOpen(false)}
        />
      )}
      <aside className={`sidebar ${mobileOpen ? "is-open" : ""}`}>
        <a className="brand" href="/">
          <Image
            className="brand-logo"
            src="/brand/logo-horizontal.svg"
            alt="Fontes"
            width={539}
            height={256}
            priority
          />
          <span className="brand-subtitle">PUBLISHER WORKSPACE</span>
        </a>
        <div className="workspace-identity">
          <span className="workspace-initial">
            {data?.actor.name.slice(0, 1) ?? "F"}
          </span>
          <div>
            <strong>
              {data?.actor.role === "reviewer"
                ? "Fontes editorial"
                : "Personal workspace"}
            </strong>
            <span>
              {data?.actor.role === "reviewer"
                ? "Maintainer"
                : "Independent publisher"}
            </span>
          </div>
        </div>
        <p className="nav-caption">WORKSPACE</p>
        <nav aria-label="Main navigation">
          {navigation.map((item) => (
            <Button
              key={item.id}
              variant={page === item.id ? "secondary" : "ghost"}
              className="nav-item"
              onPress={() => navigate(item.id)}
            >
              <item.icon size={18} strokeWidth={1.6} />
              <span>{item.title}</span>
              {item.id === "review" && pending > 0 && (
                <span className="nav-count">{pending}</span>
              )}
            </Button>
          ))}
        </nav>
        <Separator className="sidebar-separator" />
        <p className="nav-caption">TOOLS & SUPPORT</p>
        <nav aria-label="Tools">
          <Button
            variant={page === "api" ? "secondary" : "ghost"}
            className="nav-item"
            onPress={() => navigate("api")}
          >
            <Code2 size={18} />
            <span>API explorer</span>
          </Button>
          <Button
            variant={page === "settings" ? "secondary" : "ghost"}
            className="nav-item"
            onPress={() => navigate("settings")}
          >
            <Settings2 size={18} />
            <span>Workspace settings</span>
          </Button>
        </nav>
        <div className="sidebar-bottom">
          <div className="offline-note">
            <Leaf size={19} />
            <div>
              <strong>Made for offline study.</strong>
              <p>Your work helps a library grow beyond a connection.</p>
            </div>
          </div>
          <Separator />
          <div className="profile">
            <Avatar size="sm">
              <Avatar.Fallback>
                {data?.actor.name
                  .split(" ")
                  .map((s) => s[0])
                  .join("") || "F"}
              </Avatar.Fallback>
            </Avatar>
            <div>
              <strong>{data?.actor.name ?? "Fontes workspace"}</strong>
              <span>
                {data?.actor.mode === "development"
                  ? "Development persona"
                  : "Publishing account"}
              </span>
            </div>
            {data?.actor.mode === "neon" && (
              <Button
                variant="ghost"
                size="sm"
                onPress={async () => {
                  const { authClient } = await import("@/lib/auth-client");
                  await authClient.signOut();
                  window.location.assign("/auth");
                }}
              >
                Sign out
              </Button>
            )}
          </div>
        </div>
      </aside>
      <div className="main-shell">
        <header className="topbar">
          <div className="breadcrumb">
            <Button
              variant="ghost"
              isIconOnly
              aria-label="Open navigation"
              className="mobile-menu"
              onPress={() => setMobileOpen(true)}
            >
              <Menu size={20} />
            </Button>
            <span>Workspace</span>
            <span className="breadcrumb-slash">/</span>
            <strong>
              {navigation.find((n) => n.id === page)?.title ??
                (page === "api" ? "API explorer" : "Settings")}
            </strong>
          </div>
          <div className="topbar-tools">
            {data?.actor.mode === "development" && (
              <span className="dev-badge">
                <span />
                Local development
              </span>
            )}
            <Button
              variant="ghost"
              isIconOnly
              aria-label="Workspace help"
              onPress={() => navigate("settings")}
            >
              <CircleHelp size={19} />
            </Button>
          </div>
        </header>
        <main id="main" className="main-content">
          {notice && (
            <div className="notice" role="status">
              <Check size={17} />
              {notice}
              <Button
                variant="ghost"
                isIconOnly
                size="sm"
                aria-label="Dismiss message"
                onPress={() => setNotice("")}
              >
                <X size={16} />
              </Button>
            </div>
          )}
          {loading ? (
            <div className="loading-state">
              <Spinner />
              <p>Opening your workspace…</p>
            </div>
          ) : error ? (
            <Card className="connection-error">
              <Card.Content>
                <h1>
                  {unauthorized
                    ? "Your next chapter starts here."
                    : "We couldn’t open your workspace."}
                </h1>
                <p>{error}</p>
                {unauthorized ? (
                  <a href="/auth" className="text-link">
                    Sign in <ArrowRight size={16} />
                  </a>
                ) : (
                  <Button
                    onPress={() => {
                      setLoading(true);
                      void load();
                    }}
                  >
                    Try again
                  </Button>
                )}
              </Card.Content>
            </Card>
          ) : (
            data && (
              <>
                {selected ? (
                  <ResourceDetail
                    resource={selected}
                    actor={data.actor}
                    activity={data.activity.filter(
                      (e) => e.resource_id === selected.id,
                    )}
                    onBack={() => setSelectedId(null)}
                    onUpdate={update}
                  />
                ) : (
                  <>
                    <div className="page-heading">
                      <div>
                        <p className="eyebrow">
                          {page === "review"
                            ? "FONTES EDITORIAL"
                            : "A PLACE FOR YOUR WORK"}
                        </p>
                        <h1>
                          {page === "overview"
                            ? `Welcome back, ${data.actor.name.split(" ")[0]}.`
                            : page === "resources"
                              ? "Your growing library."
                              : page === "review"
                                ? "Thoughtful work, carefully reviewed."
                                : page === "organizations"
                                  ? "Good work belongs together."
                                  : page === "activity"
                                    ? "Every step, remembered."
                                    : page === "api"
                                      ? "A shared language."
                                      : "Your workspace, connected."}
                        </h1>
                        <p className="page-description">
                          {page === "overview"
                            ? "Prepare, refine, and share resources for deeper study."
                            : page === "resources"
                              ? "From a first draft to a place in the Fontes library."
                              : page === "review"
                                ? "Review generated source packages and give publishers a clear path toward publication."
                                : page === "organizations"
                                  ? "Manage your team, invitations and private resource drafts."
                                  : page === "activity"
                                    ? "The latest changes across resources you can access."
                                    : page === "api"
                                      ? "Explore the working API that connects this dashboard to its data."
                                      : "Manage your publishing identity and development environment."}
                        </p>
                      </div>
                      {["overview", "resources"].includes(page) && (
                        <Button
                          className="primary-button"
                          onPress={() => setNewOpen(true)}
                        >
                          <Plus size={17} />
                          New resource
                        </Button>
                      )}
                    </div>
                    {page === "overview" && (
                      <>
                        <section className="welcome-banner">
                          <div className="banner-copy">
                            <span className="banner-kicker">
                              <span className="tiny-rule" />
                              THE FONTES LIBRARY
                            </span>
                            <h2>
                              Knowledge worth sharing.
                              <br />
                              Study that stays with you.
                            </h2>
                            <p>
                              Bring your words to a library built for careful
                              reading, meaningful connections, and a life
                              offline.
                            </p>
                            <Button
                              className="banner-button"
                              onPress={() => setNewOpen(true)}
                            >
                              Start a resource <ArrowRight size={16} />
                            </Button>
                          </div>
                          <div className="banner-art" aria-hidden="true">
                            <div className="art-circle" />
                            <div className="art-book back-book">
                              <span>THE PSALMS</span>
                              <Leaf size={36} />
                              <small>PRAYER & POETRY</small>
                            </div>
                            <div className="art-book front-book">
                              <img src="/art/romans.svg" alt="" />
                              <span>
                                Reading
                                <br />
                                Romans
                              </span>
                              <small>A STUDY TOGETHER</small>
                            </div>
                          </div>
                        </section>
                        <section
                          className="summary-strip"
                          aria-label="Resource summary"
                        >
                          <div>
                            <span className="metric-icon">
                              <Library size={21} />
                            </span>
                            <div>
                              <span>Resources in workspace</span>
                              <strong>
                                {resources.length.toString().padStart(2, "0")}
                              </strong>
                            </div>
                          </div>
                          <div>
                            <span className="metric-icon">
                              <Clock3 size={21} />
                            </span>
                            <div>
                              <span>Awaiting review</span>
                              <strong>
                                {pending.toString().padStart(2, "0")}
                              </strong>
                            </div>
                          </div>
                          <div>
                            <span className="metric-icon amber">
                              <FileText size={21} />
                            </span>
                            <div>
                              <span>Need your attention</span>
                              <strong>
                                {changes.toString().padStart(2, "0")}
                              </strong>
                            </div>
                          </div>
                        </section>
                        <div className="overview-columns">
                          <section className="resources-section">
                            <div className="section-heading">
                              <h2>
                                Recent resources <span>{resources.length}</span>
                              </h2>
                              <Button
                                variant="ghost"
                                size="sm"
                                onPress={() => navigate("resources")}
                              >
                                View all <ArrowRight size={14} />
                              </Button>
                            </div>
                            <ResourceTable
                              resources={resources.slice(0, 4)}
                              onOpen={(r) => setSelectedId(r.id)}
                              compact
                            />
                            <p className="table-note">
                              Sample records are labeled Demo. Changes are saved
                              to your development database.
                            </p>
                          </section>
                          <aside className="next-step">
                            <p className="eyebrow">THE PUBLISHING PATH</p>
                            <h2>
                              A little care.
                              <br />A lasting resource.
                            </h2>
                            <ol>
                              <li>
                                <span>01</span>
                                <div>
                                  <strong>Shape your resource</strong>
                                  <p>
                                    Add a title, description, and the kind of
                                    work you’re creating.
                                  </p>
                                </div>
                              </li>
                              <li>
                                <span>02</span>
                                <div>
                                  <strong>Start a conversation</strong>
                                  <p>
                                    Submit your metadata and respond to
                                    editorial feedback.
                                  </p>
                                </div>
                              </li>
                              <li className="future-step">
                                <span>03</span>
                                <div>
                                  <strong>Prepare for publication</strong>
                                  <p>
                                    Review the prepared source. Signed app
                                    delivery comes later.
                                  </p>
                                </div>
                              </li>
                            </ol>
                            <Button
                              variant="ghost"
                              size="sm"
                              onPress={() => navigate("settings")}
                            >
                              About this workspace <ArrowRight size={14} />
                            </Button>
                          </aside>
                        </div>
                        <section className="recent-activity">
                          <div className="section-heading">
                            <h2>Latest activity</h2>
                            <Button
                              variant="ghost"
                              size="sm"
                              onPress={() => navigate("activity")}
                            >
                              All activity <ArrowRight size={14} />
                            </Button>
                          </div>
                          <ActivityList
                            activity={data.activity.slice(0, 3)}
                            onOpen={(id) => setSelectedId(id)}
                          />
                        </section>
                      </>
                    )}
                    {page === "review" && (
                      <>
                        <Tabs
                          selectedKey={reviewMode}
                          onSelectionChange={(k) => setReviewMode(String(k))}
                        >
                          <Tabs.List aria-label="Review queue type">
                            <Tabs.Tab id="packages">
                              Source packages
                              <Tabs.Indicator />
                            </Tabs.Tab>
                            <Tabs.Tab id="metadata">
                              Metadata only
                              <Tabs.Indicator />
                            </Tabs.Tab>
                          </Tabs.List>
                        </Tabs>
                        {reviewMode === "packages" && (
                          <SourceReviewQueue
                            key={data.actor.id}
                            actor={data.actor}
                          />
                        )}
                      </>
                    )}
                    {(page === "resources" ||
                      (page === "review" && reviewMode === "metadata")) && (
                      <section className="resource-list-section">
                        <div className="list-toolbar">
                          <Tabs
                            selectedKey={filter}
                            onSelectionChange={(key) => setFilter(String(key))}
                            variant="secondary"
                          >
                            <Tabs.List aria-label="Filter resources">
                              <Tabs.Tab id="all">
                                All resources
                                <Tabs.Indicator />
                              </Tabs.Tab>
                              <Tabs.Tab id="draft">
                                Drafts
                                <Tabs.Indicator />
                              </Tabs.Tab>
                              <Tabs.Tab id="submitted">
                                In review
                                <Tabs.Indicator />
                              </Tabs.Tab>
                              <Tabs.Tab id="changes_requested">
                                Needs attention
                                <Tabs.Indicator />
                              </Tabs.Tab>
                            </Tabs.List>
                          </Tabs>
                          <TextField
                            aria-label="Search resources"
                            className="resource-search"
                            value={query}
                            onChange={setQuery}
                          >
                            <div className="search-wrap">
                              <Search size={17} />
                              <Input placeholder="Find a resource…" />
                            </div>
                          </TextField>
                        </div>
                        {page === "review" && (
                          <p className="review-note">
                            <ShieldCheck size={16} />
                            {data.actor.role === "reviewer"
                              ? "You’re reviewing metadata only. Technical, rights, and publication checks remain separate."
                              : "Your submitted resources appear here. Switch to the reviewer persona in Settings to try editorial actions."}
                          </p>
                        )}
                        <ResourceTable
                          resources={displayed}
                          onOpen={(r) => setSelectedId(r.id)}
                        />
                        <p className="table-note">
                          Showing {displayed.length} of up to 100 accessible
                          resources. Nothing here is published automatically.
                        </p>
                      </section>
                    )}
                    {page === "activity" && (
                      <section className="activity-page">
                        <ActivityList
                          activity={data.activity}
                          onOpen={(id) => setSelectedId(id)}
                        />
                      </section>
                    )}
                    {page === "organizations" && (
                      <Organizations key={data.actor.id} actor={data.actor} />
                    )}
                    {page === "api" && <ApiExplorer />}
                    {page === "settings" && (
                      <section className="settings-grid">
                        <Card>
                          <Card.Header>
                            <Card.Title>Publishing identity</Card.Title>
                          </Card.Header>
                          <Card.Content>
                            <dl className="settings-list">
                              <div>
                                <dt>Name</dt>
                                <dd>{data.actor.name}</dd>
                              </div>
                              <div>
                                <dt>Email</dt>
                                <dd>{data.actor.email}</dd>
                              </div>
                              <div>
                                <dt>Namespace</dt>
                                <dd>
                                  <code>{data.actor.namespace}</code>
                                </dd>
                              </div>
                              <div>
                                <dt>Role</dt>
                                <dd>{kindName(data.actor.role)}</dd>
                              </div>
                            </dl>
                          </Card.Content>
                        </Card>
                        <Card>
                          <Card.Header>
                            <Card.Title>
                              {data.actor.mode === "development"
                                ? "Development playground"
                                : "Connected services"}
                            </Card.Title>
                          </Card.Header>
                          <Card.Content>
                            <p className="muted">
                              {data.actor.mode === "development"
                                ? "No login needed. Switch between the two synthetic personas to try resource review and organization invitations."
                                : "Your web session is managed by Neon Auth. Publishing roles are assigned by the server."}
                            </p>
                            {data.actor.mode === "development" && (
                              <div className="persona-actions">
                                {(["publisher", "reviewer"] as const).map(
                                  (persona) => (
                                    <Button
                                      key={persona}
                                      variant={
                                        data.actor.role === persona
                                          ? "primary"
                                          : "secondary"
                                      }
                                      onPress={async () => {
                                        try {
                                          await request("/api/v1/dev/persona", {
                                            method: "POST",
                                            body: JSON.stringify({ persona }),
                                          });
                                          await load();
                                          const channel = new BroadcastChannel(
                                            "fontes-workspace-session",
                                          );
                                          channel.postMessage(
                                            "persona-changed",
                                          );
                                          channel.close();
                                          setNotice(
                                            `Switched to ${persona} workspace.`,
                                          );
                                        } catch (e) {
                                          setNotice((e as Error).message);
                                        }
                                      }}
                                    >
                                      {persona === "publisher" ? (
                                        <BookOpen size={16} />
                                      ) : (
                                        <ShieldCheck size={16} />
                                      )}
                                      {kindName(persona)}
                                    </Button>
                                  ),
                                )}
                              </div>
                            )}
                            <Separator />
                            <div className="connection-line">
                              <span className="connection-dot" />
                              Postgres connected
                              <span className="muted">
                                Neon development database
                              </span>
                            </div>
                          </Card.Content>
                        </Card>
                        <Card className="scope-card">
                          <Card.Header>
                            <Card.Title>
                              What works in this first scaffold
                            </Card.Title>
                          </Card.Header>
                          <Card.Content>
                            <p>
                              Create and edit resource metadata, submit it for
                              review, request changes, approve metadata, and
                              inspect the API. These actions persist in
                              Postgres.
                            </p>
                            <p>
                              Local source uploads, EPUB preparation, publisher
                              review and source ZIP export are available. Hosted
                              workers, maintainer rights approval, signing and
                              app delivery are the next layer. Metadata approval
                              is not approval to publish. Organization roles,
                              invitations and private metadata drafts work now;
                              private publication, payments and AI remain
                              planned.
                            </p>
                            <p className="muted">
                              Fontes reading, study, and personal notes stay on
                              your device. This dashboard does not connect to
                              the phone app yet.
                            </p>
                          </Card.Content>
                        </Card>
                      </section>
                    )}
                  </>
                )}
                <footer className="page-footer">
                  <span>
                    <BookOpen size={14} />
                    FONTES <span>Thoughtful resources. Lasting study.</span>
                  </span>
                  <span>
                    Publisher workspace <span className="footer-dot">·</span>{" "}
                    Early development
                  </span>
                </footer>
              </>
            )
          )}
        </main>
      </div>
      <IntakeWizard
        open={newOpen}
        onClose={() => setNewOpen(false)}
        onSaved={async (id) => {
          setNewOpen(false);
          await load();
          setSelectedId(id);
          setNotice("Your resource draft is saved.");
        }}
      />
    </div>
  );
}

function ResourceTable({
  resources,
  onOpen,
  compact = false,
}: {
  resources: Resource[];
  onOpen: (r: Resource) => void;
  compact?: boolean;
}) {
  return (
    <Table className="resource-table">
      <Table.ScrollContainer>
        <Table.Content aria-label="Resources">
          <Table.Header>
            <Table.Column isRowHeader>RESOURCE</Table.Column>
            <Table.Column>STATUS</Table.Column>
            {!compact && <Table.Column>TYPE</Table.Column>}
            <Table.Column>UPDATED</Table.Column>
            <Table.Column>
              <span className="sr-only">Open resource</span>
            </Table.Column>
          </Table.Header>
          <Table.Body
            renderEmptyState={() => (
              <div className="empty-state">
                <FolderOpen size={30} />
                <p>No resources here yet.</p>
                <span>Create a draft or try another filter.</span>
              </div>
            )}
          >
            {resources.map((r) => (
              <Table.Row key={r.id} id={r.id}>
                <Table.Cell>
                  <button
                    className="resource-name-cell"
                    onClick={() => onOpen(r)}
                  >
                    <Cover resource={r} />
                    <span>
                      <strong>{r.title}</strong>
                      <small>
                        {compact ? kindName(r.kind) : r.owner_name}{" "}
                        <span className="inline-dot">·</span> v{r.version}
                        {r.is_demo && <span className="demo-label">Demo</span>}
                      </small>
                    </span>
                  </button>
                </Table.Cell>
                <Table.Cell>
                  <Status status={r.status} />
                </Table.Cell>
                {!compact && (
                  <Table.Cell>
                    <span className="resource-kind">{kindName(r.kind)}</span>
                  </Table.Cell>
                )}
                <Table.Cell>
                  <span className="date-cell">{formatDate(r.updated_at)}</span>
                </Table.Cell>
                <Table.Cell>
                  <Button
                    variant="ghost"
                    size="sm"
                    isIconOnly
                    aria-label={`Open ${r.title}`}
                    onPress={() => onOpen(r)}
                  >
                    <ArrowRight size={17} />
                  </Button>
                </Table.Cell>
              </Table.Row>
            ))}
          </Table.Body>
        </Table.Content>
      </Table.ScrollContainer>
    </Table>
  );
}

function ActivityList({
  activity,
  onOpen,
}: {
  activity: Activity[];
  onOpen: (id: string) => void;
}) {
  return (
    <div className="activity-list">
      {activity.length === 0 ? (
        <p className="muted">
          No activity yet. Your first draft will start the story.
        </p>
      ) : (
        activity.map((event) => (
          <div key={event.id} className="activity-item">
            <span
              className={`activity-icon ${event.action === "request_changes" ? "attention" : ""}`}
            >
              {event.action === "approve_metadata" ? (
                <CheckCheck size={17} />
              ) : event.action === "request_changes" ? (
                <FileText size={17} />
              ) : (
                <Clock3 size={17} />
              )}
            </span>
            <div>
              <p>
                <strong>{actionName[event.action] || event.action}</strong>
                <span className="activity-separator">·</span>
                <button onClick={() => onOpen(event.resource_id)}>
                  {event.title}
                </button>
              </p>
              <small>
                {event.actor_name} <span>—</span> {event.note}
              </small>
            </div>
            <time dateTime={event.created_at}>
              {formatDate(event.created_at)}
            </time>
          </div>
        ))
      )}
    </div>
  );
}

function ResourceForm({
  open,
  onClose,
  onSaved,
  resource,
}: {
  open: boolean;
  onClose: () => void;
  onSaved: (id: string) => Promise<void>;
  resource?: Resource;
}) {
  const [kind, setKind] = useState<string>(resource?.kind ?? "study-guide"),
    [error, setError] = useState(""),
    [busy, setBusy] = useState(false);
  useEffect(() => {
    if (open) {
      setKind(resource?.kind ?? "study-guide");
      setError("");
    }
  }, [open, resource]);
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
              <p className="eyebrow">PUBLISH SOMETHING MEANINGFUL</p>
              <Modal.Heading>
                {resource
                  ? "Refine your draft."
                  : "Every resource starts here."}
              </Modal.Heading>
            </Modal.Header>
            <Modal.Body>
              <p className="muted form-intro">
                Start with the details. You can refine these before submitting
                for metadata review.
              </p>
              <form
                id="resource-form"
                className="resource-form"
                onSubmit={async (event) => {
                  event.preventDefault();
                  setBusy(true);
                  setError("");
                  const f = new FormData(event.currentTarget);
                  try {
                    const body = {
                      title: f.get("title"),
                      description: f.get("description"),
                      language: f.get("language"),
                      version: f.get("version"),
                      kind,
                      ...(resource ? { revision: resource.revision } : {}),
                    };
                    const result = await request<Resource>(
                      resource
                        ? `/api/v1/resources/${encodeURIComponent(resource.id)}`
                        : "/api/v1/resources",
                      {
                        method: resource ? "PATCH" : "POST",
                        body: JSON.stringify(body),
                      },
                    );
                    await onSaved(result.id);
                  } catch (e) {
                    setError((e as Error).message);
                  } finally {
                    setBusy(false);
                  }
                }}
              >
                <TextField
                  name="title"
                  isRequired
                  minLength={2}
                  maxLength={180}
                  defaultValue={resource?.title}
                >
                  <Label>Resource title</Label>
                  <Input placeholder="Give your work a name" autoFocus />
                </TextField>
                <Select
                  value={kind}
                  onChange={(value) => value && setKind(String(value))}
                  isRequired
                >
                  <Label>Resource type</Label>
                  <Select.Trigger>
                    <Select.Value />
                    <Select.Indicator />
                  </Select.Trigger>
                  <Select.Popover>
                    <ListBox>
                      {kinds.map((k) => (
                        <ListBox.Item key={k} id={k} textValue={kindName(k)}>
                          {kindName(k)}
                          <ListBox.ItemIndicator />
                        </ListBox.Item>
                      ))}
                    </ListBox>
                  </Select.Popover>
                </Select>
                <TextField
                  name="description"
                  isRequired
                  minLength={10}
                  maxLength={4000}
                  defaultValue={resource?.description}
                >
                  <Label>Description</Label>
                  <TextArea
                    placeholder="What will readers find here? Describe the scope and approach."
                    rows={4}
                  />
                </TextField>
                <div className="form-row">
                  <TextField
                    name="language"
                    isRequired
                    defaultValue={resource?.language ?? "en"}
                  >
                    <Label>Language code</Label>
                    <Input maxLength={40} />
                  </TextField>
                  <TextField
                    name="version"
                    isRequired
                    defaultValue={resource?.version ?? "1.0.0"}
                  >
                    <Label>Version</Label>
                    <Input pattern="[0-9]+\.[0-9]+\.[0-9]+" />
                  </TextField>
                </div>
                {error && (
                  <p role="alert" className="form-error">
                    {error}
                  </p>
                )}
              </form>
            </Modal.Body>
            <Modal.Footer>
              <Button variant="ghost" isDisabled={busy} onPress={onClose}>
                Cancel
              </Button>
              <Button
                className="primary-button"
                type="submit"
                form="resource-form"
                isPending={busy}
              >
                {resource ? "Save changes" : "Create draft"}
                <ArrowRight size={16} />
              </Button>
            </Modal.Footer>
          </Modal.Dialog>
        </Modal.Container>
      </Modal.Backdrop>
    </Modal>
  );
}

function ResourceDetail({
  resource,
  actor,
  activity,
  onBack,
  onUpdate,
}: {
  resource: Resource;
  actor: Workspace["actor"];
  activity: Activity[];
  onBack: () => void;
  onUpdate: () => Promise<void>;
}) {
  const [note, setNote] = useState(""),
    [error, setError] = useState(""),
    [busy, setBusy] = useState(false),
    [edit, setEdit] = useState(false);
  const owner = actor.id === resource.owner_id;
  const canEdit =
    owner && ["draft", "changes_requested"].includes(resource.status);
  const act = async (action: string) => {
    setBusy(true);
    setError("");
    try {
      await request(
        `/api/v1/resources/${encodeURIComponent(resource.id)}/transitions`,
        {
          method: "POST",
          body: JSON.stringify({ action, revision: resource.revision, note }),
        },
      );
      setNote("");
      await onUpdate();
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  };
  return (
    <section className="resource-detail">
      <Button variant="ghost" className="back-button" onPress={onBack}>
        <ArrowLeft size={16} />
        Back to workspace
      </Button>
      <div className="detail-heading">
        <div>
          <p className="eyebrow">
            {kindName(resource.kind).toUpperCase()}{" "}
            {resource.is_demo && "· DEMONSTRATION"}
          </p>
          <h1>{resource.title}</h1>
          <div className="detail-meta">
            <Status status={resource.status} />
            <span>Version {resource.version}</span>
            <span>Updated {formatDate(resource.updated_at)}</span>
          </div>
        </div>
        {canEdit && !resource.has_authoring && (
          <Button variant="secondary" onPress={() => setEdit(true)}>
            Edit draft
          </Button>
        )}
      </div>
      <div className="detail-grid">
        <Card className="detail-main">
          <Card.Content>
            <h2>About this resource</h2>
            <p className="detail-description">{resource.description}</p>
            <dl className="detail-fields">
              <div>
                <dt>Publisher</dt>
                <dd>{resource.owner_name}</dd>
              </div>
              <div>
                <dt>Language</dt>
                <dd>{resource.language.toUpperCase()}</dd>
              </div>
              <div>
                <dt>Resource identity</dt>
                <dd>
                  <code>{resource.id}</code>
                </dd>
              </div>
            </dl>
            <Separator />
            <h2>Source package</h2>
            <ResourcePreparation
              resource={resource}
              owner={owner}
              onUpdate={onUpdate}
            />
            <div className="detail-boundary">
              <ShieldCheck size={18} />
              <p>
                Metadata approval never publishes a resource or adds it to the
                default library.
              </p>
            </div>
          </Card.Content>
        </Card>
        <aside className="detail-sidebar">
          <Cover resource={resource} large />
          <Card className="review-card">
            <Card.Content>
              <h2>
                {resource.status === "submitted"
                  ? "The next step"
                  : statusNames[resource.status]}
              </h2>
              {canEdit ? (
                <>
                  <p className="muted">
                    When the description is ready, send it to the Fontes review
                    queue.
                  </p>
                  <Button
                    className="primary-button"
                    isPending={busy}
                    onPress={() => void act("submit")}
                  >
                    Submit metadata <ArrowRight size={15} />
                  </Button>
                </>
              ) : resource.status === "submitted" &&
                actor.role === "reviewer" &&
                !owner ? (
                <>
                  <TextField value={note} onChange={setNote}>
                    <Label>Editorial note</Label>
                    <TextArea
                      rows={3}
                      placeholder="What should the publisher know?"
                    />
                  </TextField>
                  <Button
                    className="primary-button"
                    isPending={busy}
                    onPress={() => void act("approve_metadata")}
                  >
                    <Check size={16} />
                    Approve metadata
                  </Button>
                  <Button
                    variant="secondary"
                    isDisabled={busy || note.trim().length < 5}
                    onPress={() => void act("request_changes")}
                  >
                    Request changes
                  </Button>
                </>
              ) : (
                <p className="muted">
                  {resource.status === "metadata_approved"
                    ? "The description is ready. Source validation, rights review, and trusted publication are still required."
                    : "Your metadata is with the editorial team. You’ll see their feedback here."}
                </p>
              )}
              {error && (
                <p role="alert" className="form-error">
                  {error}
                </p>
              )}
            </Card.Content>
          </Card>
        </aside>
      </div>
      <section className="detail-history">
        <h2>Resource history</h2>
        <ActivityList activity={activity} onOpen={() => {}} />
      </section>
      <ResourceForm
        open={edit}
        resource={resource}
        onClose={() => setEdit(false)}
        onSaved={async () => {
          setEdit(false);
          await onUpdate();
        }}
      />
    </section>
  );
}

function ApiExplorer() {
  const [path, setPath] = useState("/api/v1/resources"),
    [result, setResult] = useState(""),
    [busy, setBusy] = useState(false);
  const paths = ["/api/v1/resources", "/api/v1/workspace", "/api/v1/openapi"];
  return (
    <section className="api-explorer">
      <Card>
        <Card.Header>
          <div>
            <p className="eyebrow">API v1 · DRAFT CONTRACT</p>
            <Card.Title>Try a live request</Card.Title>
          </div>
          <Chip variant="soft" size="sm">
            Read only
          </Chip>
        </Card.Header>
        <Card.Content>
          <p className="muted">
            These requests use your current workspace permissions. Changes in
            the dashboard use the same API.
          </p>
          <div className="api-request">
            <span className="http-method">GET</span>
            <Select
              aria-label="API endpoint"
              value={path}
              onChange={(value) => {
                setPath(String(value));
                setResult("");
              }}
            >
              <Select.Trigger>
                <Select.Value />
                <Select.Indicator />
              </Select.Trigger>
              <Select.Popover>
                <ListBox>
                  {paths.map((p) => (
                    <ListBox.Item key={p} id={p} textValue={p}>
                      {p}
                    </ListBox.Item>
                  ))}
                </ListBox>
              </Select.Popover>
            </Select>
            <Button
              className="primary-button"
              isPending={busy}
              onPress={async () => {
                setBusy(true);
                try {
                  const response = await fetch(path);
                  setResult(JSON.stringify(await response.json(), null, 2));
                } catch {
                  setResult("The request could not be completed.");
                } finally {
                  setBusy(false);
                }
              }}
            >
              Send request <ArrowRight size={16} />
            </Button>
          </div>
          <pre className="api-response" aria-live="polite">
            {result || "// Send a request to inspect its JSON response."}
          </pre>
        </Card.Content>
      </Card>
      <div className="api-contract-grid">
        <Card>
          <Card.Content>
            <Code2 size={22} />
            <h2>One versioned contract</h2>
            <p>
              Runtime-validated JSON, stable error codes, and optimistic
              revision checks.
            </p>
            <a
              className="text-link"
              href="/api/v1/openapi"
              target="_blank"
              rel="noreferrer"
            >
              Open OpenAPI document <ArrowRight size={14} />
            </a>
          </Card.Content>
        </Card>
        <Card>
          <Card.Content>
            <ShieldCheck size={22} />
            <h2>Permissions at the boundary</h2>
            <p>
              Owners manage drafts. Independent maintainers review submitted
              metadata. Native token access is not established yet.
            </p>
          </Card.Content>
        </Card>
      </div>
    </section>
  );
}
