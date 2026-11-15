-- Explicit public-intake snapshots are separate from private drafts and metadata review.
CREATE TABLE fontes_cloud.source_reviews (
 id uuid PRIMARY KEY, personal_resource_id text REFERENCES fontes_cloud.resources(id),
 organization_resource_id uuid REFERENCES fontes_cloud.organization_resources(id), organization_id uuid REFERENCES fontes_cloud.organizations(id),
 origin_revision integer NOT NULL, submitted_by text NOT NULL REFERENCES fontes_cloud.accounts(id), publisher_actor text NOT NULL REFERENCES fontes_cloud.accounts(id), creator_id text NOT NULL REFERENCES fontes_cloud.accounts(id),
 resource_id text NOT NULL, publisher_id text NOT NULL, version text NOT NULL, title text NOT NULL,
 source_sha256 text NOT NULL CHECK(source_sha256 ~ '^[a-f0-9]{64}$'), upstream_sha256 text NOT NULL CHECK(upstream_sha256 ~ '^[a-f0-9]{64}$'), snapshot_sha256 text NOT NULL CHECK(snapshot_sha256 ~ '^[a-f0-9]{64}$'),
 status text NOT NULL DEFAULT 'submitted' CHECK(status IN ('submitted','changes_requested','approved','handoff_ready','withdrawn','superseded')),
 revision integer NOT NULL DEFAULT 1, approval jsonb, handoff jsonb, created_at timestamptz NOT NULL DEFAULT now(), updated_at timestamptz NOT NULL DEFAULT now(),
 CHECK((personal_resource_id IS NOT NULL AND organization_resource_id IS NULL AND organization_id IS NULL) OR (personal_resource_id IS NULL AND organization_resource_id IS NOT NULL AND organization_id IS NOT NULL))
);
CREATE UNIQUE INDEX source_review_personal_revision ON fontes_cloud.source_reviews(personal_resource_id,origin_revision);
CREATE UNIQUE INDEX source_review_org_revision ON fontes_cloud.source_reviews(organization_resource_id,origin_revision);
CREATE TABLE fontes_cloud.source_review_events (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), review_id uuid NOT NULL REFERENCES fontes_cloud.source_reviews(id), actor_id text REFERENCES fontes_cloud.accounts(id),
 action text NOT NULL, note text NOT NULL DEFAULT '', created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE fontes_cloud.source_handoff_versions (
 resource_id text NOT NULL, version text NOT NULL, source_sha256 text NOT NULL, review_id uuid NOT NULL REFERENCES fontes_cloud.source_reviews(id), PRIMARY KEY(resource_id,version)
);
CREATE FUNCTION fontes_cloud.invalidate_source_reviews() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 WITH changed AS (
 UPDATE fontes_cloud.source_reviews SET status='superseded',approval=NULL,handoff=NULL,revision=revision+1,updated_at=now()
 WHERE ((TG_TABLE_NAME='resources' AND personal_resource_id=NEW.id::text) OR (TG_TABLE_NAME='organization_resources' AND organization_resource_id::text=NEW.id::text))
 AND status IN ('submitted','changes_requested','approved','handoff_ready') RETURNING id
 ) INSERT INTO fontes_cloud.source_review_events(review_id,action,note) SELECT id,'superseded','The publisher draft changed. A new approved package must be submitted.' FROM changed;
 RETURN NEW;
END;
$$;
CREATE TRIGGER invalidate_personal_source_review AFTER UPDATE ON fontes_cloud.resources FOR EACH ROW EXECUTE FUNCTION fontes_cloud.invalidate_source_reviews();
CREATE TRIGGER invalidate_org_source_review AFTER UPDATE ON fontes_cloud.organization_resources FOR EACH ROW EXECUTE FUNCTION fontes_cloud.invalidate_source_reviews();
CREATE FUNCTION fontes_cloud.submit_source_review(p_actor text,p_id uuid,p_personal text,p_org uuid,p_resource uuid,p_revision integer,p_org_revision integer,p_review_revision integer,p_publisher_actor text,p_resource_id text,p_publisher text,p_version text,p_title text,p_source text,p_upstream text,p_snapshot text)
RETURNS jsonb LANGUAGE plpgsql AS $$
DECLARE r fontes_cloud.resources%ROWTYPE; o fontes_cloud.organizations%ROWTYPE; org_r fontes_cloud.organization_resources%ROWTYPE; prep fontes_cloud.organization_epub_preparations%ROWTYPE; role_value text; creator text;
BEGIN
 IF p_personal IS NOT NULL THEN
  SELECT * INTO r FROM fontes_cloud.resources WHERE id=p_personal FOR UPDATE;
  IF NOT FOUND OR r.owner_id<>p_actor THEN RETURN jsonb_build_object('error','not_found'); END IF;
  IF r.revision<>p_revision OR r.authoring_approval IS NULL OR r.authoring_approval->>'actor'<>p_publisher_actor THEN RETURN jsonb_build_object('error','revision_conflict'); END IF;
  creator=r.owner_id;
 ELSE
  SELECT * INTO o FROM fontes_cloud.organizations WHERE id=p_org FOR UPDATE;
  SELECT role INTO role_value FROM fontes_cloud.organization_members WHERE organization_id=p_org AND account_id=p_actor;
  IF role_value IS NULL OR role_value NOT IN ('owner','admin','publisher') THEN RETURN jsonb_build_object('error','not_found'); END IF;
  SELECT * INTO org_r FROM fontes_cloud.organization_resources WHERE id=p_resource AND organization_id=p_org FOR UPDATE;
  SELECT * INTO prep FROM fontes_cloud.organization_epub_preparations WHERE resource_id=p_resource FOR UPDATE;
  IF org_r.id IS NULL OR prep.resource_id IS NULL THEN RETURN jsonb_build_object('error','not_found'); END IF;
  IF org_r.revision<>p_revision OR o.revision<>p_org_revision OR prep.review_revision<>p_review_revision THEN RETURN jsonb_build_object('error','revision_conflict'); END IF;
  creator=org_r.created_by;
 END IF;
 IF EXISTS(SELECT 1 FROM fontes_cloud.source_reviews WHERE (personal_resource_id=p_personal OR organization_resource_id=p_resource) AND origin_revision=p_revision) THEN RETURN jsonb_build_object('error','already_submitted'); END IF;
 IF (SELECT count(*) FROM fontes_cloud.source_reviews WHERE status IN ('submitted','changes_requested','approved','handoff_ready'))>=100 THEN RETURN jsonb_build_object('error','queue_capacity'); END IF;
 INSERT INTO fontes_cloud.source_reviews(id,personal_resource_id,organization_resource_id,organization_id,origin_revision,submitted_by,publisher_actor,creator_id,resource_id,publisher_id,version,title,source_sha256,upstream_sha256,snapshot_sha256)
 VALUES(p_id,p_personal,p_resource,p_org,p_revision,p_actor,p_publisher_actor,creator,p_resource_id,p_publisher,p_version,p_title,p_source,p_upstream,p_snapshot);
 INSERT INTO fontes_cloud.source_review_events(review_id,actor_id,action,note) VALUES(p_id,p_actor,'submitted','Publisher explicitly submitted this frozen package for public-free Fontes review.');
 RETURN jsonb_build_object('id',p_id);
END;
$$;
CREATE FUNCTION fontes_cloud.source_review_command(p_actor text,p_id uuid,p_revision integer,p_hash text,p_action text,p_note text,p_approval jsonb,p_handoff jsonb)
RETURNS jsonb LANGUAGE plpgsql AS $$
DECLARE v fontes_cloud.source_reviews%ROWTYPE; actor_role text; publishing boolean; same_source text;
BEGIN
 SELECT * INTO v FROM fontes_cloud.source_reviews WHERE id=p_id;
 IF NOT FOUND THEN RETURN jsonb_build_object('error','not_found'); END IF;
 -- Lock order matches publisher/membership writes: organization, resource, review.
 IF v.organization_id IS NOT NULL THEN
  PERFORM 1 FROM fontes_cloud.organizations WHERE id=v.organization_id FOR UPDATE;
  PERFORM 1 FROM fontes_cloud.organization_resources WHERE id=v.organization_resource_id FOR UPDATE;
 ELSE PERFORM 1 FROM fontes_cloud.resources WHERE id=v.personal_resource_id FOR UPDATE;
 END IF;
 SELECT * INTO v FROM fontes_cloud.source_reviews WHERE id=p_id FOR UPDATE;
 SELECT role INTO actor_role FROM fontes_cloud.accounts WHERE id=p_actor;
 publishing=EXISTS(SELECT 1 FROM fontes_cloud.resources WHERE id=v.personal_resource_id AND owner_id=p_actor) OR EXISTS(SELECT 1 FROM fontes_cloud.organization_members WHERE organization_id=v.organization_id AND account_id=p_actor AND role IN ('owner','admin','publisher'));
 IF actor_role IS DISTINCT FROM 'reviewer' AND NOT publishing THEN RETURN jsonb_build_object('error','not_found'); END IF;
 IF v.revision<>p_revision OR v.source_sha256<>p_hash THEN RETURN jsonb_build_object('error','revision_conflict'); END IF;
 IF v.status IN ('withdrawn','superseded') THEN RETURN jsonb_build_object('error','review_inactive'); END IF;
 IF p_action='withdraw' THEN
  IF NOT publishing THEN RETURN jsonb_build_object('error','forbidden'); END IF;
  UPDATE fontes_cloud.source_reviews SET status='withdrawn',approval=NULL,handoff=NULL WHERE id=p_id;
 ELSIF p_action='comment' THEN NULL;
 ELSE
  IF actor_role IS DISTINCT FROM 'reviewer' OR p_actor IN (v.submitted_by,v.publisher_actor,v.creator_id) THEN RETURN jsonb_build_object('error','independent_reviewer_required'); END IF;
  IF p_action='request_changes' AND v.status IN ('submitted','approved','handoff_ready') THEN
   UPDATE fontes_cloud.source_reviews SET status='changes_requested',approval=NULL,handoff=NULL WHERE id=p_id;
  ELSIF p_action='approve' AND v.status='submitted' THEN
   UPDATE fontes_cloud.source_reviews SET status='approved',approval=p_approval,handoff=NULL WHERE id=p_id;
  ELSIF p_action='handoff' AND v.status='approved' THEN
   SELECT source_sha256 INTO same_source FROM fontes_cloud.source_handoff_versions WHERE resource_id=v.resource_id AND version=v.version;
   IF same_source IS NOT NULL AND same_source<>v.source_sha256 THEN RETURN jsonb_build_object('error','version_conflict'); END IF;
   INSERT INTO fontes_cloud.source_handoff_versions(resource_id,version,source_sha256,review_id) VALUES(v.resource_id,v.version,v.source_sha256,v.id) ON CONFLICT DO NOTHING;
   SELECT source_sha256 INTO same_source FROM fontes_cloud.source_handoff_versions WHERE resource_id=v.resource_id AND version=v.version;
   IF same_source<>v.source_sha256 THEN RETURN jsonb_build_object('error','version_conflict'); END IF;
   UPDATE fontes_cloud.source_reviews SET status='handoff_ready',handoff=p_handoff WHERE id=p_id;
  ELSE RETURN jsonb_build_object('error','invalid_transition'); END IF;
 END IF;
 UPDATE fontes_cloud.source_reviews SET revision=revision+1,updated_at=now() WHERE id=p_id;
 INSERT INTO fontes_cloud.source_review_events(review_id,actor_id,action,note) VALUES(p_id,p_actor,p_action,p_note);
 RETURN jsonb_build_object('id',p_id);
END;
$$;
