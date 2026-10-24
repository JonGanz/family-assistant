---
id: Q-022
title: Core node hardware specs
status: open
batch: 3
area: infra
created: 2026-10-23
resolved:
decisions: []
---

# Q-022: Core node hardware specs

## Question
Q-020 approved the optional CPU triage model, but we still need the Ubuntu server's specs to size it: CPU model and cores, RAM, disk, and whether it is on the same LAN as the gaming PC (Wake-on-LAN needs that).

## Answer
<!-- answer below this line -->
Same LAN. Either probe for hardware, or just let it be a feature to turn on or off.
