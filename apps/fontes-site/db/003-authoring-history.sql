CREATE TABLE IF NOT EXISTS fontes_cloud.authoring_history (
  resource_id text NOT NULL REFERENCES fontes_cloud.resources(id),
  revision integer NOT NULL,
  actor_id text NOT NULL REFERENCES fontes_cloud.accounts(id),
  snapshot jsonb NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (resource_id, revision)
);
