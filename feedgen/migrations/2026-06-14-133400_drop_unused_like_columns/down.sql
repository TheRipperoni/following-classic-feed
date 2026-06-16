-- Restores the columns dropped by the up migration.
ALTER TABLE public.like ADD COLUMN cid VARCHAR NOT NULL DEFAULT '';
ALTER TABLE public.like ADD COLUMN "subjectCid" VARCHAR NOT NULL DEFAULT '';
ALTER TABLE public.like ADD COLUMN "createdAt" VARCHAR NOT NULL DEFAULT '';
ALTER TABLE public.like ADD COLUMN prev VARCHAR;
ALTER TABLE public.like ADD COLUMN sequence BIGINT;
