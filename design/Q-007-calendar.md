---
id: Q-007
title: Calendar sources & write-back target
status: open
batch: 1
area: ingest
created: 2026-10-23
resolved:
decisions: []
---

# Q-007: Calendar sources & write-back target

## Question
List the calendars (Google, iCloud/CalDAV, school ICS feeds, sports apps such as TeamSnap). When an event is pulled from an email, where should it go: the member's own Google calendar, one shared family calendar, or a calendar only this system owns? Should it auto-add, or ask an adult to approve first?

## Options
- A: auto-add to a system-owned family calendar
- B: propose → adult approves → add
- C: auto-add above a confidence threshold, otherwise propose

## Answer
<!-- answer below this line -->
Option C. All of those sources of calendars would be valid. The system should not necessarily always send to a specific calendar. That could be configured, either manually or via learned rules. For example, a shared email between parents would likely result in events being created in any of the calendars, depending on LLM results.
