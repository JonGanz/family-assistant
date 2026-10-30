---
id: D-088
title: An adult can set up another adult until that adult claims their account
status: accepted
date: 2026-10-30
source_questions: [Q-077]
supersedes: 
superseded_by:
affects: [arch/Domain.md, arch/Frontend.md]
---

# D-088: An adult can set up another adult until that adult claims their account

## Context
D-006 says an adult cannot change another adult's settings. Enforced literally, it stopped a second adult from ever getting a login: an adult could add another adult as a member but could not create their login, edit them, reset their password or set their PIN, and the new adult could not sign in to do it themselves.

## Decision
User answer to Q-077: option A.
- An adult is **unclaimed** while they have no account, or their account still has the password another adult set (`user_account.initial_password`). While a member is unclaimed, any adult may edit their profile (name, colour, avatar, aliases), create their login with an initial password, and reset that password.
- The new adult is **made to change the password at first sign-in**. Changing it clears `initial_password`: the adult is then **claimed**, and D-006 applies in full (no other adult edits them, resets them or sees their credentials). Sources and other settings were never covered by the window.
- **PINs stay self-only**: the new adult sets their own PIN after signing in.
- The create form asks an adult for no grade (grade is for children).

## Consequences
- Until a second adult signs in and changes the password, the adding adult holds a known password for them. That is the accepted cost of the smallest option; the invite link (option B) avoids it.
- D-006's rule for **claimed** adults is unchanged. This only narrows when it starts to apply.
- A claimed adult shown to another adult is read-only in the Members page, with an explanation.
- Migration `20261030000001_initial_password` adds `user_account.initial_password boolean NOT NULL DEFAULT false`.
- The server enforces the forced change: while the flag is true a user session may only call `GET /me`, logout and its own password change; everything else is 403 with problem code `password_change_required`. Login and `/me` carry `must_change_password`; `MemberDto.initial_password` lets the client tell an unclaimed adult from a claimed one.
- The CLI's account commands (system actor) are unchanged and do not set the flag.

## Alternatives considered
- B, an invite link the new adult opens to choose their own credentials: no one else knows a password, but it needs a token table, expiry and a page. Not chosen.
- C, a household owner who manages the other adults: changes D-006 itself. Not chosen.
- D, a setting that picks between them: not asked for.
