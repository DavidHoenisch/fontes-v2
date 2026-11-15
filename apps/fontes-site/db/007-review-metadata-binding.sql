-- A metadata edit and later reversion must not revive a former approval.
ALTER TABLE fontes_cloud.organization_resources ADD COLUMN metadata_revision integer NOT NULL DEFAULT 1;
CREATE FUNCTION fontes_cloud.bump_resource_metadata_revision() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 NEW.metadata_revision=OLD.metadata_revision+1;
 RETURN NEW;
END;
$$;
CREATE TRIGGER bump_resource_metadata_revision BEFORE UPDATE OF title,description,kind,language,version ON fontes_cloud.organization_resources FOR EACH ROW EXECUTE FUNCTION fontes_cloud.bump_resource_metadata_revision();
