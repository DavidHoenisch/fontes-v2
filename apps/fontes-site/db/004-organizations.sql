-- Organization workflow is separate from personal drafts and existing source-release contracts.
CREATE TABLE fontes_cloud.organizations (
  id uuid PRIMARY KEY,
  name text NOT NULL CHECK (length(name) BETWEEN 2 AND 100),
  description text NOT NULL DEFAULT '' CHECK (length(description) <= 2000),
  namespace text NOT NULL UNIQUE,
  offline_policy_preference text NOT NULL CHECK (offline_policy_preference IN ('retain_downloads', 'renewable_lease')),
  policy_revision integer NOT NULL DEFAULT 1 CHECK (policy_revision > 0),
  revision integer NOT NULL DEFAULT 1 CHECK (revision > 0),
  is_demo boolean NOT NULL DEFAULT false,
  created_by text NOT NULL REFERENCES fontes_cloud.accounts(id),
  created_at timestamptz NOT NULL DEFAULT now(), updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE fontes_cloud.organization_members (
  organization_id uuid NOT NULL REFERENCES fontes_cloud.organizations(id),
  account_id text NOT NULL REFERENCES fontes_cloud.accounts(id),
  role text NOT NULL CHECK (role IN ('owner', 'admin', 'publisher', 'member')),
  joined_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (organization_id, account_id)
);
CREATE INDEX organization_members_account ON fontes_cloud.organization_members(account_id);
CREATE TABLE fontes_cloud.organization_invitations (
  id uuid PRIMARY KEY,
  organization_id uuid NOT NULL REFERENCES fontes_cloud.organizations(id),
  email text NOT NULL CHECK (email = lower(email) AND length(email) <= 254),
  role text NOT NULL CHECK (role IN ('admin', 'publisher', 'member')),
  token_hash text NOT NULL UNIQUE CHECK (length(token_hash) = 64),
  status text NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'accepted', 'revoked')),
  invited_by text NOT NULL REFERENCES fontes_cloud.accounts(id),
  accepted_by text REFERENCES fontes_cloud.accounts(id),
  expires_at timestamptz NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX organization_pending_invite ON fontes_cloud.organization_invitations(organization_id, email) WHERE status = 'pending';
CREATE TABLE fontes_cloud.organization_policy_revisions (
  organization_id uuid NOT NULL REFERENCES fontes_cloud.organizations(id),
  revision integer NOT NULL CHECK (revision > 0),
  preference text NOT NULL CHECK (preference IN ('retain_downloads', 'renewable_lease')),
  actor_id text NOT NULL REFERENCES fontes_cloud.accounts(id), created_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (organization_id, revision)
);
CREATE TABLE fontes_cloud.organization_events (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  organization_id uuid NOT NULL REFERENCES fontes_cloud.organizations(id),
  actor_id text NOT NULL REFERENCES fontes_cloud.accounts(id),
  action text NOT NULL CHECK (action IN ('created','settings_updated','invited','invite_renewed','invite_revoked','invite_accepted','role_changed','member_removed','member_left','resource_created','resource_edited')),
  target_id text,
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX organization_events_recent ON fontes_cloud.organization_events(organization_id, created_at DESC);
CREATE TABLE fontes_cloud.organization_resources (
  id uuid PRIMARY KEY,
  organization_id uuid NOT NULL REFERENCES fontes_cloud.organizations(id),
  title text NOT NULL CHECK (length(title) BETWEEN 2 AND 180),
  description text NOT NULL CHECK (length(description) BETWEEN 10 AND 4000),
  kind text NOT NULL CHECK (kind IN ('translation','commentary','lexicon','word-study','study-guide','devotional','book')),
  language text NOT NULL, version text NOT NULL,
  status text NOT NULL DEFAULT 'draft' CHECK (status = 'draft'),
  revision integer NOT NULL DEFAULT 1 CHECK (revision > 0),
  created_by text NOT NULL REFERENCES fontes_cloud.accounts(id),
  created_at timestamptz NOT NULL DEFAULT now(), updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX organization_resources_scope ON fontes_cloud.organization_resources(organization_id, updated_at DESC);

-- All organization writes serialize on the organization row. Role checks and
-- last-owner checks occur under that lock, including invitation consumption.
-- Default SECURITY INVOKER: this function grants no database/publication privilege.
CREATE FUNCTION fontes_cloud.organization_command(p_actor text, p_org uuid, p_action text, p jsonb)
RETURNS jsonb LANGUAGE plpgsql AS $$
DECLARE
  o fontes_cloud.organizations%ROWTYPE;
  invite fontes_cloud.organization_invitations%ROWTYPE;
  actor_email text;
  actor_demo boolean;
  actor_role text;
  target_role text;
  event_action text;
  target text;
  rid uuid;
BEGIN
  IF p_action IN ('create','accept') THEN
    SELECT lower(email), is_demo INTO actor_email, actor_demo FROM fontes_cloud.accounts WHERE id = p_actor FOR UPDATE;
    IF NOT FOUND THEN RETURN jsonb_build_object('error','not_found'); END IF;
    IF (SELECT count(*) FROM fontes_cloud.organization_members WHERE account_id = p_actor) >= 20 THEN
      RETURN jsonb_build_object('error','membership_limit');
    END IF;
  END IF;
  IF p_action = 'create' THEN
    IF (SELECT count(*) FROM fontes_cloud.organizations WHERE created_by = p_actor AND created_at > now() - interval '1 day') >= 5 THEN
      RETURN jsonb_build_object('error','rate_limited');
    END IF;
    INSERT INTO fontes_cloud.organizations(id,name,description,namespace,offline_policy_preference,created_by,is_demo)
      VALUES(p_org,p->>'name',p->>'description','org.fontes.o' || replace(p_org::text,'-',''),p->>'offline_policy_preference',p_actor,actor_demo);
    INSERT INTO fontes_cloud.organization_members VALUES(p_org,p_actor,'owner',now());
    INSERT INTO fontes_cloud.organization_policy_revisions VALUES(p_org,1,p->>'offline_policy_preference',p_actor,now());
    INSERT INTO fontes_cloud.organization_events(organization_id,actor_id,action) VALUES(p_org,p_actor,'created');
    RETURN jsonb_build_object('id',p_org);
  END IF;
  IF p_action = 'accept' THEN
    SELECT organization_id INTO p_org FROM fontes_cloud.organization_invitations WHERE token_hash = p->>'token_hash' AND email = actor_email;
    IF NOT FOUND THEN RETURN jsonb_build_object('error','not_found'); END IF;
  END IF;
  SELECT * INTO o FROM fontes_cloud.organizations WHERE id = p_org FOR UPDATE;
  IF NOT FOUND THEN RETURN jsonb_build_object('error','not_found'); END IF;
  IF p_action = 'accept' THEN
    SELECT * INTO invite FROM fontes_cloud.organization_invitations WHERE token_hash = p->>'token_hash' AND email = actor_email FOR UPDATE;
    IF invite.status <> 'pending' OR invite.expires_at <= now() THEN RETURN jsonb_build_object('error','invite_inactive'); END IF;
    IF EXISTS(SELECT 1 FROM fontes_cloud.organization_members WHERE organization_id = p_org AND account_id = p_actor) THEN RETURN jsonb_build_object('error','already_member'); END IF;
    IF (SELECT count(*) FROM fontes_cloud.organization_members WHERE organization_id = p_org) >= 100 THEN RETURN jsonb_build_object('error','member_limit'); END IF;
    INSERT INTO fontes_cloud.organization_members VALUES(p_org,p_actor,invite.role,now());
    UPDATE fontes_cloud.organization_invitations SET status = 'accepted', accepted_by = p_actor WHERE id = invite.id;
    event_action := 'invite_accepted'; target := invite.id::text;
  ELSE
    SELECT role INTO actor_role FROM fontes_cloud.organization_members WHERE organization_id = p_org AND account_id = p_actor;
    IF NOT FOUND THEN RETURN jsonb_build_object('error','not_found'); END IF;
    IF o.revision <> (p->>'revision')::integer THEN RETURN jsonb_build_object('error','revision_conflict'); END IF;
    IF p_action IN ('resource_create','resource_update') THEN
      IF actor_role NOT IN ('owner','admin','publisher') THEN RETURN jsonb_build_object('error','forbidden'); END IF;
    ELSIF p_action <> 'leave' AND actor_role NOT IN ('owner','admin') THEN RETURN jsonb_build_object('error','forbidden');
    END IF;
    IF p_action = 'settings' THEN
      UPDATE fontes_cloud.organizations SET name=p->>'name', description=p->>'description', offline_policy_preference=p->>'offline_policy_preference',
        policy_revision=policy_revision + CASE WHEN offline_policy_preference <> p->>'offline_policy_preference' THEN 1 ELSE 0 END WHERE id=p_org;
      IF o.offline_policy_preference <> p->>'offline_policy_preference' THEN
        INSERT INTO fontes_cloud.organization_policy_revisions VALUES(p_org,o.policy_revision+1,p->>'offline_policy_preference',p_actor,now());
      END IF;
      event_action := 'settings_updated';
    ELSIF p_action IN ('invite','renew') THEN
      IF (SELECT count(*) FROM fontes_cloud.organization_events WHERE organization_id=p_org AND action IN ('invited','invite_renewed') AND created_at > now() - interval '1 hour') >= 20 THEN RETURN jsonb_build_object('error','rate_limited'); END IF;
      IF p_action = 'renew' THEN
        SELECT * INTO invite FROM fontes_cloud.organization_invitations WHERE organization_id=p_org AND id=(p->>'target_id')::uuid;
        IF NOT FOUND THEN RETURN jsonb_build_object('error','not_found'); END IF;
        IF invite.status <> 'pending' THEN RETURN jsonb_build_object('error','invite_inactive'); END IF;
        target_role := invite.role;
      ELSE
        target_role := p->>'role';
      END IF;
      IF actor_role='admin' AND target_role='admin' THEN RETURN jsonb_build_object('error','forbidden'); END IF;
      IF p_action='invite' THEN
        IF EXISTS(SELECT 1 FROM fontes_cloud.organization_members m JOIN fontes_cloud.accounts a ON a.id=m.account_id WHERE m.organization_id=p_org AND lower(a.email)=p->>'email') THEN RETURN jsonb_build_object('error','already_member'); END IF;
        IF EXISTS(SELECT 1 FROM fontes_cloud.organization_invitations WHERE organization_id=p_org AND email=p->>'email' AND status='pending') THEN RETURN jsonb_build_object('error','invite_exists'); END IF;
        IF (SELECT count(*) FROM fontes_cloud.organization_invitations WHERE organization_id=p_org) >= 100 THEN RETURN jsonb_build_object('error','invitation_limit'); END IF;
        rid := (p->>'id')::uuid;
        INSERT INTO fontes_cloud.organization_invitations(id,organization_id,email,role,token_hash,invited_by,expires_at)
          VALUES(rid,p_org,p->>'email',target_role,p->>'token_hash',p_actor,now()+interval '7 days');
        event_action := 'invited';
      ELSE
        rid := invite.id;
        UPDATE fontes_cloud.organization_invitations SET token_hash=p->>'token_hash',expires_at=now()+interval '7 days',invited_by=p_actor WHERE id=rid;
        event_action := 'invite_renewed';
      END IF;
      target := rid::text;
    ELSIF p_action='revoke' THEN
      SELECT * INTO invite FROM fontes_cloud.organization_invitations WHERE organization_id=p_org AND id=(p->>'target_id')::uuid;
      IF NOT FOUND THEN RETURN jsonb_build_object('error','not_found'); END IF;
      IF invite.status <> 'pending' THEN RETURN jsonb_build_object('error','invite_inactive'); END IF;
      IF actor_role='admin' AND invite.role='admin' THEN RETURN jsonb_build_object('error','forbidden'); END IF;
      UPDATE fontes_cloud.organization_invitations SET status='revoked' WHERE id=invite.id;
      event_action := 'invite_revoked'; target := invite.id::text;
    ELSIF p_action IN ('member','remove','leave') THEN
      target := CASE WHEN p_action='leave' THEN p_actor ELSE p->>'target_id' END;
      SELECT role INTO target_role FROM fontes_cloud.organization_members WHERE organization_id=p_org AND account_id=target;
      IF NOT FOUND THEN RETURN jsonb_build_object('error','not_found'); END IF;
      IF p_action <> 'leave' AND actor_role='admin' AND (target_role IN ('owner','admin') OR (p_action='member' AND p->>'role' IN ('owner','admin'))) THEN RETURN jsonb_build_object('error','forbidden'); END IF;
      IF target_role='owner' AND (p_action<>'member' OR p->>'role'<>'owner') AND (SELECT count(*) FROM fontes_cloud.organization_members WHERE organization_id=p_org AND role='owner') <= 1 THEN RETURN jsonb_build_object('error','last_owner'); END IF;
      IF p_action='member' THEN
        UPDATE fontes_cloud.organization_members SET role=p->>'role' WHERE organization_id=p_org AND account_id=target;
        event_action := 'role_changed';
      ELSE
        DELETE FROM fontes_cloud.organization_members WHERE organization_id=p_org AND account_id=target;
        event_action := CASE WHEN p_action='leave' THEN 'member_left' ELSE 'member_removed' END;
      END IF;
    ELSIF p_action IN ('resource_create','resource_update') THEN
      IF p_action='resource_create' THEN
        IF (SELECT count(*) FROM fontes_cloud.organization_resources WHERE organization_id=p_org) >= 100 THEN RETURN jsonb_build_object('error','resource_limit'); END IF;
        rid := (p->>'id')::uuid;
        INSERT INTO fontes_cloud.organization_resources(id,organization_id,title,description,kind,language,version,created_by)
          VALUES(rid,p_org,p->>'title',p->>'description',p->>'kind',p->>'language',p->>'version',p_actor);
        event_action := 'resource_created';
      ELSE
        rid := (p->>'target_id')::uuid;
        UPDATE fontes_cloud.organization_resources SET title=p->>'title',description=p->>'description',kind=p->>'kind',language=p->>'language',version=p->>'version',revision=revision+1,updated_at=now()
          WHERE id=rid AND organization_id=p_org AND revision=(p->>'resource_revision')::integer;
        IF NOT FOUND THEN RETURN jsonb_build_object('error','revision_conflict'); END IF;
        event_action := 'resource_edited';
      END IF;
      target := rid::text;
    ELSE
      RETURN jsonb_build_object('error','invalid_action');
    END IF;
  END IF;
  UPDATE fontes_cloud.organizations SET revision=revision+1, updated_at=now() WHERE id=p_org;
  INSERT INTO fontes_cloud.organization_events(organization_id,actor_id,action,target_id) VALUES(p_org,p_actor,event_action,target);
  RETURN jsonb_build_object('id',p_org,'target_id',target);
END;
$$;
