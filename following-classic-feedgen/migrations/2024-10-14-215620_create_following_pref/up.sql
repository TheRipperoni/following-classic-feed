CREATE TABLE IF NOT EXISTS public.following_preference
(
    author           character varying NOT NULL,
    did              character varying NOT NULL,
    show_reposts     boolean NOT NULL,
    show_quote_posts boolean NOT NULL
);

ALTER TABLE ONLY public.following_preference
    ADD CONSTRAINT following_preference_pkey PRIMARY KEY (author);