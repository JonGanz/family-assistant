# Runbook: install (checklist)

The full guide, with explanations, is [`deploy/README.md`](../README.md). Each step links to its section. Run the
commands in the folder that holds `compose.yaml`.

- [ ] Docker with Compose 2.23.1 or newer: `docker compose version` ([2](../README.md#2-install-in-three-steps)).
- [ ] Download `compose.yaml` from the release tag (or clone and use `deploy/`). Nothing needs editing.
- [ ] `docker compose up -d`. `docker compose ps` shows `init` exited (0), the others up, and `app` healthy;
  `curl -fsS http://127.0.0.1:8080/readyz` prints `ready`.
- [ ] Open `http://<server>:8080` and **finish the setup wizard now**, including the master key backup
  (copy it into a password manager). Until the first adult exists, anyone who can reach the page can run it.
- [ ] Optional remote access: `TS_AUTHKEY=... docker compose --profile tailscale up -d`
  ([3](../README.md#3-remote-access-with-tailscale)).
- [ ] Phones: ntfy address in **Settings, System**, the ntfy app, then **Send test**
  ([4.1](../README.md#41-phones-and-notifications)).
- [ ] Calendars, mail, organizations and the wall screen ([4.2 to 4.5](../README.md#4-connect-phones-calendars-mail-and-the-wall-screen)).
- [ ] `docker compose exec backup /backup.sh` and check that a dump appears with
  `docker compose exec backup ls -R /backups` ([5](../README.md#5-backups-and-the-secrets-volume)).
  Rehearse a restore once ([restore.md](restore.md)).
