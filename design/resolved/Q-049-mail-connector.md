---
id: Q-049
title: Gmail API only, or also IMAP?
status: resolved
batch: 16
area: ingest
created: 2026-10-25
resolved: 2026-10-25
decisions: [D-052]
---

# Q-049: Gmail API only, or also IMAP?

## Question
D-007 chose the **Gmail API** (OAuth, incremental `history.list` sync, Gmail labels and categories). Both Gmail scopes it needs (`gmail.readonly`, `gmail.modify`) are **restricted** scopes. So the household runs its own OAuth client in Testing mode, and **each connected Google account must re-consent every 7 days**. That is already true for Google calendars in M1, so you'd re-consent once a week per account for both.

The alternative is **IMAP with a Google app password** (needs 2-step verification on the account). It never expires, it also works for non-Gmail providers, and it supports archive and mark-read. The downsides:
- Gmail's own categories are less reliable over IMAP (Promotions and Social aren't IMAP folders; they come only through Gmail's `X-GM-LABELS` extension);
- IMAP needs its own incremental sync (UIDVALIDITY and UIDNEXT, or Gmail's `X-GM-MSGID`);
- Google has been restricting less-secure access, but app passwords still work.

**Which connectors should M3 build?**

## Options
- A. Gmail API only (as D-007). Weekly re-consent per account until the OAuth app is published.
- B. IMAP only, with app passwords. No re-consent, and works with any provider.
- C. **Both, chosen per source, default Gmail API** (recommended if re-consent bothers you): the same pipeline behind both, and IMAP as the fallback for accounts where weekly re-consent is a pain or the provider isn't Gmail.
- D. Gmail API now, IMAP later (a follow-up milestone).
- E. Something else (describe).

## Answer
<!-- answer below this line -->
Option C.
