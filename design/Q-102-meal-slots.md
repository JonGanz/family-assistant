---
id: Q-102
title: "Meal slots: names, order and times"
status: open
batch: 24
area: domain
created: 2026-11-07
resolved:
decisions: []
---

# Q-102: Meal slots: names, order and times

## Context
D-015 says the slots (breakfast, lunch, dinner, snack) are configurable. Domain.md has them as a fixed enum. A time per slot would let the dashboard show "next meal" and order the day.

## Question
How are slots modelled?

## Options
- A. An editable slot list per household in Settings → Meals (name, order, `"HH:MM"` time, shown on/off); defaults Breakfast 07:30, Lunch 12:00, Dinner 18:00, Snack 15:30 hidden (suggested)
- B. The fixed four slots, no times
- C. Fixed slots with editable times

## Answer
<!-- answer below this line -->

