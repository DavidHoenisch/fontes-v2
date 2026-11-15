CREATE FUNCTION fontes_cloud.lock_source_intake_capacity() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 PERFORM pg_advisory_xact_lock(hashtext('fontes_cloud.source_intake_capacity'));
 IF (SELECT count(*) FROM fontes_cloud.source_reviews WHERE status IN ('submitted','changes_requested','approved','handoff_ready'))>=100 THEN
  RAISE EXCEPTION 'Source review capacity reached' USING ERRCODE='check_violation';
 END IF;
 RETURN NEW;
END;
$$;
CREATE TRIGGER lock_source_intake_capacity BEFORE INSERT ON fontes_cloud.source_reviews FOR EACH ROW EXECUTE FUNCTION fontes_cloud.lock_source_intake_capacity();
