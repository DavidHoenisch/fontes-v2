-- Local evaluation artifacts are not source releases or membership download grants.
CREATE TABLE fontes_cloud.organization_epub_preparations (
  resource_id uuid PRIMARY KEY REFERENCES fontes_cloud.organization_resources(id),
  revision integer NOT NULL CHECK (revision > 0),
  manifest_sha256 text NOT NULL CHECK (manifest_sha256 ~ '^[a-f0-9]{64}$'),
  source_sha256 text NOT NULL CHECK (source_sha256 ~ '^[a-f0-9]{64}$'),
  summary jsonb NOT NULL CHECK (octet_length(summary::text) <= 262144),
  actor_id text NOT NULL REFERENCES fontes_cloud.accounts(id),
  updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE fontes_cloud.organization_epub_history (
  resource_id uuid NOT NULL REFERENCES fontes_cloud.organization_resources(id),
  revision integer NOT NULL,
  manifest_sha256 text NOT NULL,
  source_sha256 text NOT NULL,
  summary jsonb NOT NULL,
  actor_id text NOT NULL REFERENCES fontes_cloud.accounts(id),
  created_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY(resource_id, revision)
);
CREATE FUNCTION fontes_cloud.organization_import_epub(p_actor text,p_org uuid,p_resource uuid,p_org_revision integer,p_resource_revision integer,p_manifest text,p_source text,p_summary jsonb)
RETURNS jsonb LANGUAGE plpgsql AS $$
DECLARE o fontes_cloud.organizations%ROWTYPE; r fontes_cloud.organization_resources%ROWTYPE; member_role text; next_revision integer;
BEGIN
  SELECT * INTO o FROM fontes_cloud.organizations WHERE id=p_org FOR UPDATE;
  IF NOT FOUND THEN RETURN jsonb_build_object('error','not_found'); END IF;
  SELECT role INTO member_role FROM fontes_cloud.organization_members WHERE organization_id=p_org AND account_id=p_actor;
  IF NOT FOUND THEN RETURN jsonb_build_object('error','not_found'); END IF;
  IF member_role NOT IN ('owner','admin','publisher') THEN RETURN jsonb_build_object('error','forbidden'); END IF;
  SELECT * INTO r FROM fontes_cloud.organization_resources WHERE id=p_resource AND organization_id=p_org FOR UPDATE;
  IF NOT FOUND THEN RETURN jsonb_build_object('error','not_found'); END IF;
  IF o.revision<>p_org_revision OR r.revision<>p_resource_revision THEN RETURN jsonb_build_object('error','revision_conflict'); END IF;
  SELECT coalesce(max(revision),0)+1 INTO next_revision FROM fontes_cloud.organization_epub_history WHERE resource_id=p_resource;
  INSERT INTO fontes_cloud.organization_epub_preparations(resource_id,revision,manifest_sha256,source_sha256,summary,actor_id)
    VALUES(p_resource,next_revision,p_manifest,p_source,p_summary,p_actor)
    ON CONFLICT(resource_id) DO UPDATE SET revision=EXCLUDED.revision,manifest_sha256=EXCLUDED.manifest_sha256,source_sha256=EXCLUDED.source_sha256,summary=EXCLUDED.summary,actor_id=EXCLUDED.actor_id,updated_at=now();
  INSERT INTO fontes_cloud.organization_epub_history(resource_id,revision,manifest_sha256,source_sha256,summary,actor_id)
    VALUES(p_resource,next_revision,p_manifest,p_source,p_summary,p_actor);
  UPDATE fontes_cloud.organization_resources SET revision=revision+1,updated_at=now() WHERE id=p_resource;
  UPDATE fontes_cloud.organizations SET revision=revision+1,updated_at=now() WHERE id=p_org;
  INSERT INTO fontes_cloud.organization_events(organization_id,actor_id,action,target_id) VALUES(p_org,p_actor,'resource_edited',p_resource::text);
  RETURN jsonb_build_object('revision',next_revision);
END;
$$;
