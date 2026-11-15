ALTER TABLE fontes_cloud.resources ADD COLUMN IF NOT EXISTS authoring jsonb;
ALTER TABLE fontes_cloud.resources ADD COLUMN IF NOT EXISTS authoring_approval jsonb;
