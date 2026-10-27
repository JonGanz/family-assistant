# Runbook: restore from backup

A restore needs two things: a database dump (from `BACKUP_DIR`, or the `fa-backups` volume) and the **master key**
(the base64 line the setup wizard showed, or **Settings, System, Master key backup** on a running server). The
database password is not part of it: a fresh install makes its own.

Rehearse this once after setup and after any major upgrade, without touching production:

```sh
scripts/restore-rehearsal.sh <dump> [master-key-file]
```

It restores the dump into a throwaway Postgres, starts the app against it with the same master key, checks
`/readyz` and checks the member count. `scripts/compose-smoke.sh` produces a sample dump to rehearse against.

## Restore on a running or fresh install
Run the `docker compose` commands in the folder that holds `compose.yaml`.

1. **Fresh machine only:** `docker compose up -d` first. The `init` service creates a new master key and database
   password, and the app creates empty tables. Do not run the setup wizard.
2. Stop everything that uses the database:
   ```sh
   docker compose stop app backup
   ```
3. Recreate the database. (On a *new Postgres cluster*, not a new compose install, the dump has no cluster roles:
   create `fa_app` first, with
   `psql -U fa -d postgres -c 'CREATE ROLE fa_app NOLOGIN NOSUPERUSER NOBYPASSRLS' -c 'GRANT fa_app TO fa'`.
   A fresh compose install already has it.)
   ```sh
   docker compose exec -T postgres psql -U fa -d postgres -c 'DROP DATABASE IF EXISTS fa WITH (FORCE)' -c 'CREATE DATABASE fa'
   ```
4. Restore the dump. The files are named `*.sql.gz` but are `pg_dump` custom format (`-Fc`), so use `pg_restore`.
   From the `backup` container (start it first with `docker compose start backup`; it is stopped by step 2):
   ```sh
   docker compose start backup
   docker compose exec -T backup cat /backups/last/fa-latest.sql.gz | docker compose exec -T postgres pg_restore -U fa -d fa --no-owner
   ```
   Pick another file from `daily/`, `weekly/` or `monthly/` if you need an older state. A dump you copied
   from another machine: `docker compose exec -T postgres pg_restore -U fa -d fa --no-owner < fa-20261027.sql.gz`.
5. **Put the master key back.** Paste your base64 line in place of `<the line>`. The key file is read-only for
   the app and owned by uid 10001, so write it through the `init` service's volume:
   ```sh
   docker compose run --rm --no-deps --user 0:0 -e KEY='<the line>' --entrypoint sh init -c \
     'umask 277; printf %s "$KEY" | base64 -d > /run/fa-secrets/master_key; chown 10001:10001 /run/fa-secrets/master_key; ls -l /run/fa-secrets'
   ```
   `master_key` must be 32 bytes. If you have lost the key, skip this step: the data restores, but each stored
   secret (calendar links and passwords, mail passwords, Google sign-ins) is unreadable and has to be added again.
6. Start the app and check it:
   ```sh
   docker compose up -d
   curl -fsS http://127.0.0.1:8080/readyz && echo
   ```
   Log in and check recent data. **Settings, System, Master key backup** shows the key the app is now using.
   (If you built with the build overlay, add `-f compose.yaml -f compose.build.yaml` to every command.)
