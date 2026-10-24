# Runbook: install

1. Install Docker (with compose) and Tailscale on the host. Join the tailnet.
2. Clone the repository, or copy `deploy/compose.example.yaml` and `deploy/.env.example`.
3. `cp deploy/.env.example deploy/.env` and fill in every value (strong `POSTGRES_PASSWORD`, `BACKUP_DIR` on the NAS, `FA_PUBLIC_BASE_URL`).
4. Generate the master key and **store a copy offline**:
   ```sh
   head -c 32 /dev/urandom > fa_master_key && chmod 400 fa_master_key
   ```
5. Start: `docker compose --env-file deploy/.env -f deploy/compose.example.yaml up -d`
6. Wait until `docker compose ... ps` shows `api` healthy. The api runs the migrations.
7. Expose: `tailscale serve --bg --https=443 http://127.0.0.1:8080` and `tailscale serve --bg --https=8443 http://127.0.0.1:8090`.
8. Seed: `fa-server config import <household.toml>`, `fa-server user add ...`, `fa-server device pair ...` (inside the api container; see `deploy/README.md`).
9. Verify `/readyz`, log in, and confirm the first backup appears in `${BACKUP_DIR}` (trigger one with `docker compose ... exec backup /backup.sh`).
