-- Your SQL goes here
CREATE TABLE IF NOT EXISTS public.seen_post
(
    did character varying NOT NULL,
    uri character varying NOT NULL
);

ALTER TABLE ONLY public.seen_post
    ADD CONSTRAINT seen_post_pkey PRIMARY KEY (did);

CREATE TABLE IF NOT EXISTS public.fetched_post
(
    did    character varying NOT NULL,
    uri    character varying NOT NULL
);

ALTER TABLE ONLY public.fetched_post
    ADD CONSTRAINT post_seen_pkey PRIMARY KEY (did);