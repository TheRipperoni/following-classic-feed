-- Drops unused columns from the `like` table.
-- These columns (cid, subjectCid, createdAt, prev, sequence) are written during
-- event processing but never read by any query — only uri, author, subjectUri,
-- and indexedAt are actually used at query time.
ALTER TABLE public.like DROP COLUMN IF EXISTS cid;
ALTER TABLE public.like DROP COLUMN IF EXISTS "subjectCid";
ALTER TABLE public.like DROP COLUMN IF EXISTS "createdAt";
ALTER TABLE public.like DROP COLUMN IF EXISTS prev;
ALTER TABLE public.like DROP COLUMN IF EXISTS sequence;
