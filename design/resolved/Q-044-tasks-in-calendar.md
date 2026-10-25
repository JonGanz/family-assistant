---
id: Q-044
title: Do tasks appear in the calendar and agenda
status: resolved
batch: 14
area: frontend
created: 2026-10-24
resolved: 2026-10-25
decisions: [D-046]
---

# Q-044: Do tasks appear in the calendar and agenda

## Question
Homework due dates, bill-pay tasks (M3), and dated chores have a due time. The dashboard already has `agenda` and `week` panels for events (Frontend.md).

**Should tasks show up next to events?**

## Options
- A. No. Tasks have their own panels (Now, per-member lists, routines).
- B. Yes, tasks with a due date appear in the agenda, week view, and Calendar page as a distinct style (checkbox, member colour), and can be checked off there.
- C. **Configurable per list, default B for homework, A for chores and routines** (recommended): each list has `show_in_calendar: true|false`.
- D. Something else (describe).

## Answer
<!-- answer below this line -->
Option C. Some homework tasks are more than just complete the homework, but knowing that it's a certain day in the calendar, for example, school presentations. There's work that comes before that, but we always want to show it in the calendar even if the work is done. Other things like reading do just needs to be signed. So once it's done, we don't care if it's in the calendar. Chores also would be something we need to see in the calendar. We just want to see that on the task list. The agenda view should be saved for big items -- events for the day and larger items that are happening at school or work.
