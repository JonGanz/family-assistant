---
id: Q-065
title: Where does the Members page live, and what do children see there?
status: resolved
batch: 17
area: frontend
created: 2026-10-26
resolved: 2026-10-26
decisions: [D-070]
---

# Q-065: Where does the Members page live, and what do children see there?

## Question
Review finding U-08. `/members` is a top-level page for every signed-in user, but it is a management screen (add, edit, avatar, role). The nav is now split into pages of value and a Settings area; the proposed structure moves Members under Settings (`/settings/members`, adults) with a redirect from `/members`.

**Where should Members go, and what should a child see?**

## Options
- A. **Under Settings for adults; children get a read-only 'Household' card on Today showing names, avatars and today's routines** (recommended).
- B. Under Settings for adults; children have no members page at all.
- C. Keep `/members` top-level, read-only for children, editable for adults.
- D. Something else (describe).

## Answer
<!-- answer below this line -->
Member management should definitely be an Adult-only settings screen, we don't need redirect routes to it. Children can see the list on the dashboard, as that is obviously where the entire group views shared information. Since elements of the application will use the details of the members, they adults are the only ones who can edit it.
