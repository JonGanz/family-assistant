# Runbook: rotate the master key

Status: planned procedure. The CLI command is **TODO (M4+)** and does not exist yet.

Envelope encryption: each secret has its own DEK, wrapped by the master key (AES-256-GCM) and tagged with `key_version`.

Planned procedure:
1. Back up the database and the current master key (offline).
2. Generate the new key: `head -c 32 /dev/urandom > secrets/fa_master_key.new && chmod 444 secrets/fa_master_key.new` (in `deploy/`; the app runs as uid 10001 and must be able to read the file, and the `secrets/` folder is `chmod 700`).
3. TODO (M4+): run `fa-server key rotate --new-key <path>` (command name provisional). It re-wraps every DEK with the new key and bumps `key_version`. Secret payloads are not re-encrypted.
4. Replace the file at `FA_MASTER_KEY_FILE` with the new key, then restart `api` and `worker`.
5. Verify `/readyz` and that a source account still syncs.
6. Store the new key offline. Keep the old one until the next successful backup has been verified.

Until the command exists, the fallback is to re-authorise each account with a new key.
