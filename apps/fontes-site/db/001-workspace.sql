CREATE SCHEMA IF NOT EXISTS fontes_cloud;
CREATE TABLE IF NOT EXISTS fontes_cloud.accounts (
  id text PRIMARY KEY,
  name text NOT NULL,
  email text NOT NULL,
  namespace text UNIQUE NOT NULL,
  role text NOT NULL DEFAULT 'publisher' CHECK (role IN ('publisher', 'reviewer')),
  is_demo boolean NOT NULL DEFAULT false,
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS fontes_cloud.resources (
  id text PRIMARY KEY,
  owner_id text NOT NULL REFERENCES fontes_cloud.accounts(id),
  title text NOT NULL CHECK (length(title) BETWEEN 2 AND 180),
  description text NOT NULL CHECK (length(description) BETWEEN 10 AND 4000),
  kind text NOT NULL CHECK (kind IN ('translation', 'commentary', 'lexicon', 'word-study', 'study-guide', 'devotional', 'book')),
  language text NOT NULL DEFAULT 'en',
  version text NOT NULL DEFAULT '1.0.0',
  status text NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'submitted', 'changes_requested', 'metadata_approved')),
  revision integer NOT NULL DEFAULT 1 CHECK (revision > 0),
  is_demo boolean NOT NULL DEFAULT false,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS resources_owner_updated ON fontes_cloud.resources(owner_id, updated_at DESC);
CREATE INDEX IF NOT EXISTS resources_review_updated ON fontes_cloud.resources(status, updated_at DESC);
CREATE TABLE IF NOT EXISTS fontes_cloud.events (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  resource_id text NOT NULL REFERENCES fontes_cloud.resources(id),
  actor_id text NOT NULL REFERENCES fontes_cloud.accounts(id),
  action text NOT NULL CHECK (action IN ('created', 'edited', 'submit', 'request_changes', 'approve_metadata')),
  note text NOT NULL DEFAULT '',
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS events_created ON fontes_cloud.events(created_at DESC);
