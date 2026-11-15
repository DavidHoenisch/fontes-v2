-- Additive review snapshots; the immutable imported evaluation remains untouched.
ALTER TABLE fontes_cloud.organization_epub_preparations ADD COLUMN review_sha256 text CHECK(review_sha256 ~ '^[a-f0-9]{64}$');
ALTER TABLE fontes_cloud.organization_epub_preparations ADD COLUMN review_revision integer NOT NULL DEFAULT 0;
CREATE TABLE fontes_cloud.organization_publisher_reviews (
 resource_id uuid NOT NULL REFERENCES fontes_cloud.organization_resources(id),
 revision integer NOT NULL, manifest_sha256 text NOT NULL, snapshot_sha256 text NOT NULL,
 action text NOT NULL CHECK(action IN ('correct','fields','cover','approve','package')),
 actor_id text NOT NULL REFERENCES fontes_cloud.accounts(id), created_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(resource_id,revision)
);
CREATE FUNCTION fontes_cloud.reset_import_review() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.manifest_sha256<>OLD.manifest_sha256 OR NEW.revision<>OLD.revision THEN
  NEW.review_sha256=NULL; NEW.review_revision=OLD.review_revision+1;
 END IF;
 RETURN NEW;
END;
$$;
CREATE TRIGGER reset_import_review BEFORE UPDATE ON fontes_cloud.organization_epub_preparations FOR EACH ROW EXECUTE FUNCTION fontes_cloud.reset_import_review();
CREATE FUNCTION fontes_cloud.organization_save_review(p_actor text,p_org uuid,p_resource uuid,p_org_revision integer,p_resource_revision integer,p_review_revision integer,p_manifest text,p_snapshot text,p_action text)
RETURNS jsonb LANGUAGE plpgsql AS $$
DECLARE o fontes_cloud.organizations%ROWTYPE; r fontes_cloud.organization_resources%ROWTYPE; p fontes_cloud.organization_epub_preparations%ROWTYPE; member_role text;
BEGIN
 SELECT * INTO o FROM fontes_cloud.organizations WHERE id=p_org FOR UPDATE;
 IF NOT FOUND THEN RETURN jsonb_build_object('error','not_found'); END IF;
 SELECT role INTO member_role FROM fontes_cloud.organization_members WHERE organization_id=p_org AND account_id=p_actor;
 IF NOT FOUND THEN RETURN jsonb_build_object('error','not_found'); END IF;
 IF member_role NOT IN ('owner','admin','publisher') THEN RETURN jsonb_build_object('error','forbidden'); END IF;
 SELECT * INTO r FROM fontes_cloud.organization_resources WHERE id=p_resource AND organization_id=p_org FOR UPDATE;
 SELECT * INTO p FROM fontes_cloud.organization_epub_preparations WHERE resource_id=p_resource FOR UPDATE;
 IF r.id IS NULL OR p.resource_id IS NULL THEN RETURN jsonb_build_object('error','not_found'); END IF;
 IF o.revision<>p_org_revision OR r.revision<>p_resource_revision OR p.review_revision<>p_review_revision OR p.manifest_sha256<>p_manifest THEN RETURN jsonb_build_object('error','revision_conflict'); END IF;
 UPDATE fontes_cloud.organization_epub_preparations SET review_sha256=p_snapshot,review_revision=review_revision+1,actor_id=p_actor,updated_at=now() WHERE resource_id=p_resource;
 INSERT INTO fontes_cloud.organization_publisher_reviews(resource_id,revision,manifest_sha256,snapshot_sha256,action,actor_id) VALUES(p_resource,p.review_revision+1,p_manifest,p_snapshot,p_action,p_actor);
 UPDATE fontes_cloud.organization_resources SET revision=revision+1,updated_at=now() WHERE id=p_resource;
 UPDATE fontes_cloud.organizations SET revision=revision+1,updated_at=now() WHERE id=p_org;
 INSERT INTO fontes_cloud.organization_events(organization_id,actor_id,action,target_id) VALUES(p_org,p_actor,'resource_edited',p_resource::text);
 RETURN jsonb_build_object('revision',p.review_revision+1);
END;
$$;
