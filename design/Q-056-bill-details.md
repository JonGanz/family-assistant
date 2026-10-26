---
id: Q-056
title: Bills: who pays, autopay, and detecting payment
status: open
batch: 16
area: ingest
created: 2026-10-25
resolved:
decisions: []
---

# Q-056: Bills: who pays, autopay, and detecting payment

## Question
D-009: a bill is itemised and gets a "Pay <payee>" task. Three details are open:
1. **Assignee:** who gets the pay task: a fixed adult per biller, the owners of the mailbox it arrived in, or any adult (pool)?
2. **Autopay:** many bills are paid automatically. A pay task for those is noise, but knowing the amount and date is still useful.
3. **Paid detection:** billers often send "payment received" emails. These could mark the bill paid and close the task.

**What should the defaults be?**

## Options
- A. Pool of adults for every bill. No autopay handling. Paid only when the task is ticked off.
- B. **Configurable per biller org, default: the mailbox owners (adults), an autopay flag that records the bill and creates a reminder-only task (no due-date pressure, auto-completed on the due date), and payment-confirmation emails from the same biller with a matching amount mark the bill paid** (recommended).
- C. Like B, but autopay bills create no task at all (only listed on the Bills page).
- D. Something else (describe).

## Answer
<!-- answer below this line -->

