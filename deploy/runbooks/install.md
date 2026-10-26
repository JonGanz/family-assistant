# Runbook: install (checklist)

The full guide, with explanations, is [`deploy/README.md`](../README.md). Each step below links to its section.

- [ ] Docker with compose and Tailscale installed; the server is on the tailnet ([2](../README.md#2-install-docker-and-tailscale)).
- [ ] MagicDNS and HTTPS certificates are on. Run `sudo tailscale serve --bg --https=443 http://127.0.0.1:8080` and `sudo tailscale serve --bg --https=8443 http://127.0.0.1:8090`, and note the address ([3](../README.md#3-give-the-server-an-https-address)).
- [ ] `git clone`, then `cd family-assistant/deploy && cp .env.example .env`. Set `POSTGRES_PASSWORD`, `FA_PUBLIC_BASE_URL` and `NTFY_BASE_URL` ([4](../README.md#4-get-the-code-and-configure-it)).
- [ ] Create the master key and store a copy offline ([4](../README.md#4-get-the-code-and-configure-it)):
  ```sh
  mkdir -p secrets && chmod 700 secrets
  head -c 32 /dev/urandom > secrets/fa_master_key && chmod 444 secrets/fa_master_key
  ```
- [ ] Run `docker compose up -d --build`. `docker compose ps` should show `api` healthy, and `curl -fsS http://127.0.0.1:8080/readyz` should print `ready` ([5](../README.md#5-start-it)).
- [ ] Import `household.toml`, add a user per person, and set an adult PIN ([6](../README.md#6-describe-your-household-and-create-logins)).
- [ ] Phones: Tailscale and ntfy, then **Send test** ([7.1](../README.md#71-phones-and-notifications)).
- [ ] Calendars, mail, organizations and the wall screen ([7.2–7.5](../README.md#7-connect-phones-calendars-mail-and-the-wall-screen)).
- [ ] Run `docker compose exec backup /backup.sh` and check that a dump appears in `BACKUP_DIR` ([8](../README.md#backups)). Rehearse a restore once ([restore.md](restore.md)).
