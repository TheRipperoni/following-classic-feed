-- Your SQL goes here
alter table public.following_preference
add column followed_replies_only boolean NOT NULL DEFAULT false;