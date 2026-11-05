---
id: D-097
title: M4 runs the UI revamp and the AI slice as two parallel tracks
status: accepted
date: 2026-11-04
source_questions: []
supersedes: 
superseded_by:
affects: [arch/Overall.md, README.md]
---

# D-097: M4 runs the UI revamp and the AI slice as two parallel tracks

## Context
Direct user instruction (2026-11-04, no question): run the UI revamp and the AI support in parallel, planned so sub-agents can work them cleanly, with the AI UI left to the end so both land together, and with UI proposals reviewed while they are worked on.

## Decision
- **M4 has two tracks:** track A is the UI revamp (D-095, D-096); track B is the AI slice (D-093, D-094), backend first. They share no paths until M4's last wave, where the AI settings screens are built on the new components.
- Track A runs with **review checkpoints:** a design proposal (tokens, components, navigation, one sample page) is reviewed by the user before views migrate, and each migration wave ends with screenshots for review.

## Consequences
- The M4 plan carries a waves table with both tracks; a track-A stop for review does not block track B.

## Alternatives considered
- Two separate milestones run one after the other: slower, and the AI screens would be built twice.
