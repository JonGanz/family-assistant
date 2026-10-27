# Runbook: rotate the master key

Status: planned procedure. The CLI command is **TODO (M4+)** and does not exist yet.

Envelope encryption: each secret has its own DEK, wrapped by the master key (AES-256-GCM) and tagged with `key_version`.
The key lives in the Docker volume `fa-secrets` as `master_key` (32 bytes, owned by uid 10001, mode 0400) and is
created by the `init` service. See [restore.md](restore.md), step 5, for how to write it.

Planned procedure:
1. Back up the database (`docker compose exec backup /backup.sh`) and the current master key (Settings, System,
   Master key backup, stored offline).
2. Generate the new key in a scratch file: `head -c 32 /dev/urandom > master_key.new`.
3. TODO (M4+): run `docker compose exec app fa-server key rotate --new-key <path>` (command name provisional). It
   re-wraps every DEK with the new key and bumps `key_version`. Secret payloads are not re-encrypted.
4. Replace `master_key` in the `fa-secrets` volume with the new key, exactly as in restore step 5, then
   `docker compose restart app`.
5. Verify `/readyz` and that a source account still syncs.
6. Store the new key offline (the wizard's backup page shows it as base64). Keep the old one until the next
   successful backup has been verified.

Until the command exists, the fallback is to re-authorise each account with a new key.
