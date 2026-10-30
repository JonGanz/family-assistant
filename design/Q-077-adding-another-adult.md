---
id: Q-077
title: How is a second adult added and set up?
status: open
batch: 20
area: security
created: 2026-10-30
resolved:
decisions: []
---
# Q-077: How is a second adult added and set up?

## Context
D-006 says an adult cannot change another adult's settings. The rule is enforced for every edit of an adult member (`ManageMember` allows an adult only on children and themself), for creating an adult's login (the same check) and for setting an adult's PIN (only the adult themself).

So today an adult can add a second adult as a member, but cannot give them a login or a PIN, edit them, or reset their password. The new adult has no way to sign in, so they cannot set anything up themselves either. The first adult exists only because the setup wizard creates them. In the Members page, "Create" for an adult saves the member and then fails on the login step.

(Unrelated and already fixed: the form asked an adult for a grade.)

## Question
How should a second adult be onboarded, while keeping the spirit of D-006 (adults do not edit each other's personal settings, sources or credentials)?

## Options
- **A. Initial login by the adding adult (recommended).** Any adult may create the login and set an initial password for another adult who has no login yet, and may edit their display name and colour until they have signed in once. After that, only that adult changes anything of theirs (as D-006). The new adult is asked to change the password at first sign-in. Smallest change, and it closes the gap without letting anyone take over an existing adult.
- **B. Invite link.** The adding adult creates the member and gets a one-time link (or code) to give them; the new adult opens it and chooses their own username, password and PIN. The adding adult never sees a password. More work (token table, expiry, a page).
- **C. A household owner.** One adult is the owner and may manage the other adults (create, reset password, remove). A change to D-006 itself.
- **D. Configurable, default A.** A household setting chooses between A, B and C, with the default A.

## Answer
<!-- answer below this line -->
Option A.
