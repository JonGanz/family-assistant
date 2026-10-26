---
id: Q-052
title: Filtering school and daycare mail per child: grade, age, gender?
status: resolved
batch: 16
area: ingest
created: 2026-10-25
resolved: 2026-10-25
decisions: [D-055]
---

# Q-052: Filtering school and daycare mail per child: grade, age, gender?

## Question
PROJECT.md: *school information can be filtered to relevant details based on the **age/grade/gender** of the children*. Members already have `birthdate` and `grade`. There is **no gender field**, and the data model avoids personal attributes that aren't needed.

Gender matters for things like "girls' volleyball tryouts" or "boys' scout meeting". Other filters work the same way: a team name, a house colour, a club, a bus route, a class name ("Room 12", "Ms. Lee's class").

In M3, filtering is deterministic: a section of a newsletter is shown for a child when it names their grade, class, or a matching term (Ingest.md §6.3). Per-child summaries come with the LLM in M4.

**How should children be described for this filtering?**

## Options
- A. Add an optional `gender` field to members and match gendered terms (girls', boys', etc.).
- B. Grade, class and age only. No gender.
- C. **Free-form relevance tags per child, configurable, with no fixed gender field** (recommended): an adult adds tags such as `girls`, `U10 soccer`, `Team Blue`, `Room 12`, `bus 7`. Sections and messages matching a child's tags (or grade or class) are linked to them. This covers gender where it matters and many other cases.
- D. Both A and C.
- E. Something else (describe).

## Answer
<!-- answer below this line -->
Option D.
