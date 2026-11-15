import { neon } from "@neondatabase/serverless";
if (process.env.FONTES_AUTH_MODE !== "development")
  throw new Error(
    "Demo seeding is permitted only with FONTES_AUTH_MODE=development.",
  );
const sql = neon(process.env.DATABASE_URL);
await sql.transaction([
  sql`INSERT INTO fontes_cloud.accounts (id, name, email, namespace, role, is_demo) VALUES
    ('dev-publisher', 'Alex Morgan', 'publisher@example.invalid', 'dev.fontes.alex', 'publisher', true),
    ('dev-reviewer', 'Morgan Ellis', 'reviewer@example.invalid', 'dev.fontes.review', 'reviewer', true)
    ON CONFLICT (id) DO NOTHING`,
  sql`INSERT INTO fontes_cloud.resources (id, owner_id, title, description, kind, status, is_demo, created_at, updated_at) VALUES
    ('dev.fontes.alex.romans', 'dev-publisher', 'Reading Romans Together', 'An original demonstration study guide tracing the movement of Paul’s letter, with discussion prompts and canonical references. Synthetic metadata for dashboard development; no compiled content is included.', 'study-guide', 'submitted', true, now() - interval '7 days', now() - interval '2 hours'),
    ('dev.fontes.alex.psalms', 'dev-publisher', 'A Pattern of Prayer', 'A demonstration devotional exploring the language of the Psalms. This record is synthetic and is not a published resource or a rights-cleared manuscript.', 'devotional', 'draft', true, now() - interval '4 days', now() - interval '1 day'),
    ('dev.fontes.alex.mark', 'dev-publisher', 'The Gospel According to Mark', 'Synthetic commentary metadata for testing editorial feedback. This example needs a fuller coverage statement and separate artwork attribution before a future source review.', 'commentary', 'changes_requested', true, now() - interval '12 days', now() - interval '2 days'),
    ('dev.fontes.alex.words', 'dev-publisher', 'Words of the New Testament', 'A fictional word-study collection used to exercise approved-metadata state. No source archive, lexicon data, publisher verification or signed app payload is represented.', 'word-study', 'metadata_approved', true, now() - interval '18 days', now() - interval '3 days')
    ON CONFLICT (id) DO NOTHING`,
  sql`INSERT INTO fontes_cloud.events (id, resource_id, actor_id, action, note, created_at) VALUES
    ('00000000-0000-4000-8000-000000000001', 'dev.fontes.alex.romans', 'dev-publisher', 'submit', 'Submitted the resource description for metadata review.', now() - interval '2 hours'),
    ('00000000-0000-4000-8000-000000000002', 'dev.fontes.alex.psalms', 'dev-publisher', 'created', 'Started a new devotional draft.', now() - interval '1 day'),
    ('00000000-0000-4000-8000-000000000003', 'dev.fontes.alex.mark', 'dev-reviewer', 'request_changes', 'Please clarify chapter coverage and add separate artwork attribution.', now() - interval '2 days'),
    ('00000000-0000-4000-8000-000000000004', 'dev.fontes.alex.words', 'dev-reviewer', 'approve_metadata', 'Metadata reviewed. Source validation and exact-hash approval are still required before publishing.', now() - interval '3 days')
    ON CONFLICT (id) DO NOTHING`,
]);
console.log(
  "Seeded two development personas and four labeled synthetic resource drafts. Existing drafts were preserved.",
);
