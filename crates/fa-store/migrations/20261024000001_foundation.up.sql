-- ADR: how RLS works with pooling and pre-household lookups.
--
-- * Every request/worker path runs inside a transaction opened by fa-store.
--   `tx_for(h)` sets `app.household_id` with set_config(..., is_local = true), so the
--   setting dies with the transaction and cannot leak across pooled connections.
-- * The pool may connect as a superuser or table owner (both bypass RLS, FORCE only
--   covers non-superuser owners). Therefore every fa-store transaction also runs
--   `SET LOCAL ROLE fa_app`, a NOLOGIN role without BYPASSRLS and without ownership.
--   RLS is then always enforced, whatever the login role is.
-- * Lookups that happen before the household is known (login by username, device
--   token, session id, workers) use `tx_system()`, which sets `app.system = 'on'`.
--   The policies accept `app.system = 'on'` as an alternative to the household match.
--   Chosen over SECURITY DEFINER functions (which only work if the owner bypasses RLS)
--   and over a second BYPASSRLS login role (a second credential to manage). The
--   trade-off: any code that can run arbitrary SQL can flip the flag, but such code
--   could equally use the owner role; the guard is that only fa-store's `tx_system`
--   sets it, and the repository functions for those paths return minimal rows.
-- * System-wide tables (job, schedule, system_status) have no RLS.

-- CREATE ROLE is cluster-wide, but advisory locks are per-database, so migrators running
-- concurrently in different databases of one cluster (e.g. parallel #[sqlx::test] DBs) can race.
-- Tolerate a concurrent creator instead of relying on a lock.
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'fa_app') THEN
        BEGIN
            CREATE ROLE fa_app NOLOGIN NOSUPERUSER NOBYPASSRLS;
        EXCEPTION WHEN duplicate_object OR unique_violation THEN
            NULL;
        END;
    END IF;
    BEGIN
        EXECUTE format('GRANT fa_app TO %I', current_user);
    EXCEPTION WHEN unique_violation OR SQLSTATE 'XX000' THEN
        NULL; -- concurrent identical grant ("tuple concurrently updated")
    END;
END
$$;
GRANT USAGE ON SCHEMA public TO fa_app;

CREATE FUNCTION app_household() RETURNS uuid
LANGUAGE sql STABLE AS
$$ SELECT nullif(current_setting('app.household_id', true), '')::uuid $$;

CREATE FUNCTION app_is_system() RETURNS boolean
LANGUAGE sql STABLE AS
$$ SELECT coalesce(current_setting('app.system', true), '') = 'on' $$;

CREATE FUNCTION set_updated_at() RETURNS trigger
LANGUAGE plpgsql AS
$$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END
$$;

CREATE TABLE system_status (
    key        text PRIMARY KEY,
    value      jsonb NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT now()
);
GRANT SELECT, INSERT, UPDATE, DELETE ON system_status TO fa_app;
