-- Preserve every decision's evidence IDs and package receipt after later status changes.
ALTER TABLE fontes_cloud.source_review_events ADD COLUMN details jsonb NOT NULL DEFAULT '{}'::jsonb;
CREATE FUNCTION fontes_cloud.capture_source_review_decision() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 SELECT jsonb_build_object('status',status,'revision',revision,'source_sha256',source_sha256,'approval',approval,'handoff',handoff)
 INTO NEW.details FROM fontes_cloud.source_reviews WHERE id=NEW.review_id;
 RETURN NEW;
END;
$$;
CREATE TRIGGER capture_source_review_decision BEFORE INSERT ON fontes_cloud.source_review_events FOR EACH ROW EXECUTE FUNCTION fontes_cloud.capture_source_review_decision();
