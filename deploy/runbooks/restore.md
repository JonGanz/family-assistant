# Runbook: restore from backup

Rehearse this once during M0 and after any major upgrade, without touching production:

```sh
scripts/restore-rehearsal.sh <dump> [master-key-file]
```

It restores the dump into a throwaway Postgres, starts the api against it with the same master key, checks `/readyz` and checks the member count. `scripts/compose-smoke.sh` produces a sample dump (`/tmp/fa-smoke-out/fa.dump`) to rehearse against.

Note: the dump does not contain cluster roles. Migration 0001 creates `fa_app` (NOLOGIN) and grants it to the login user, so on a **fresh Postgres cluster** create it before restoring, or `pg_restore` fails with `role "fa_app" does not exist`:

```sh
docker compose ... exec postgres psql -U fa -d fa_restore -c 'CREATE ROLE fa_app NOLOGIN NOSUPERUSER NOBYPASSRLS' -c 'GRANT fa_app TO fa'
```

Steps for a real restore:

1. Stop the app: `docker compose --env-file deploy/.env -f deploy/compose.example.yaml stop api worker`
2. Pick a dump in `${BACKUP_DIR}` (`last/`, `daily/`, `weekly/` or `monthly/`). The files are named `*.sql.gz` but are pg_dump custom format (`-Fc`, `POSTGRES_EXTRA_OPTS`), so use `pg_restore`. On-demand backup: `docker compose ... exec backup /backup.sh`.
3. Create a fresh database (the `fa_app` role already exists if you restore into the same cluster; otherwise create it first, see above):
   ```sh
   docker compose ... exec postgres psql -U fa -d postgres -c 'DROP DATABASE IF EXISTS fa_restore' -c 'CREATE DATABASE fa_restore'
   ```
4. Restore into it (custom format):
   ```sh
   docker compose ... exec -T postgres pg_restore -U fa -d fa_restore --no-owner < <dump>
   ```
   For plain `.sql.gz`: `gunzip -c <dump> | docker compose ... exec -T postgres psql -U fa -d fa_restore`.
5. Swap it in: drop or rename `fa`, then rename `fa_restore` to `fa` (no connections may be open).
6. Put the master key back at `FA_MASTER_KEY_FILE` (from your offline copy). If it is lost, secrets are unrecoverable. Re-authorise each source account.
7. Start: `docker compose ... start api worker`.
8. Check `GET /readyz` returns OK, then log in and check recent data.
