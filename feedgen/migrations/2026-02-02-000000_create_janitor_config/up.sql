CREATE TABLE IF NOT EXISTS public.janitor_config (
    id SERIAL PRIMARY KEY,
    cron_schedule VARCHAR NOT NULL DEFAULT '0 0 0 * * *',
    retention_days INT NOT NULL DEFAULT 2,
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

INSERT INTO public.janitor_config (cron_schedule, retention_days) VALUES ('0 0 0 * * *', 2);
