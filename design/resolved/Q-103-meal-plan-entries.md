---
id: Q-103
title: "Meal-plan entries: free text, leftovers, repeating meals"
status: resolved
batch: 24
area: domain
created: 2026-11-07
resolved: 2026-11-08
decisions: [D-112]
---

# Q-103: Meal-plan entries: free text, leftovers, repeating meals

## Context
Not every meal is a recipe ("Leftovers", "Eat out", "Pizza night"). Families also repeat weeks or have fixed nights.

## Question
What can a meal-plan entry be, and how are repeats made?

## Options
- A. A recipe or free text; several entries per slot allowed; "copy a week" to a later week (suggested)
- B. A, plus repeating entries (for example every Friday, an RRULE like task definitions) that the plan fills in up to the horizon
- C. One recipe or free-text entry per slot; no copying

## Answer
<!-- answer below this line -->
We should meal templates. These can contain any number of days and include a recipe, free-text, or nothing, per slot. The user can then configure a template, and choose to apply a template starting for a certain date. That would then fill up the slots in the mealplan. No repeating at this point in time.

This will allow the user to create a "Fall 2026" meal template that might include 5 weeks worth of recipes for dinner. Then the user can apply that template for the first Monday in September and have all the dinner's filled out for the next 5 weeks. They can opt to re-apply that themselves at the end of the 5 weeks, or modify the template first.

Once items are in slots, they are no longer tied to the template. From there they can be shifted around, deleted, swapped with each other, etc. Doing so will not impact the root template.

This system should feel relatively intuitive and allow separate templates to populate different slots. When a template is applied, it should notify the user of any meals that were already selected in slots that would be replaced, so they can opt to preserve some of those (for example, there might be a free-text form that was added "Thanksgiving with cousins" which the user does not want to clobber with a meal template item).
