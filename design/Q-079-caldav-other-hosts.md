---
id: Q-079
title: CalDAV servers that send the client to another host
status: open
batch: 21
area: feeds
created: 2026-11-02
resolved:
decisions: []
---

# Q-079: CalDAV servers that send the client to another host

## Question
Code review 2 (C2-15) found that the CalDAV client follows server-supplied hrefs to any host and sends Basic credentials there. iCloud legitimately redirects to shards such as pNN-caldav.icloud.com. May a CalDAV server send us to a different host?

## Options
- A. Same registrable domain only, with a per-source 'trust other hosts' toggle, default off (suggested default)
- B. Configured origin only, no exceptions
- C. Any host, as today

## Answer
<!-- answer below this line -->

