---
id: Q-062
title: How big is M4: minimal AI slice or the full registry design?
status: resolved
batch: 17
area: tech
created: 2026-10-26
resolved: 2026-10-26
decisions: [D-067]
---

# Q-062: How big is M4: minimal AI slice or the full registry design?

## Question
arch/Overall.md plans M4 with a backend registry and profile suggester (`fa-backends`), `fa-sysprobe`, `fa-gpu-agent` with Wake-on-LAN and gaming detection, `fa-eval` (calibration, replay), kNN over pgvector, two System One sizes and Ollama. The review judged this platform-scale for one household and recommended a minimal slice first. D-026 and D-029 (hardware-adaptive, configurable pipelines) still hold; the question is sequencing.

**How much of the M4 design should ship first?**

## Options
- A. **Minimal slice** (recommended): one System One backend (CPU, core node) plus one Ollama model on the GPU node behind the existing `fa-pipeline` stages, aimed at the decision points with the most inbox rejections measured in the shakedown. Registry, profile suggester, sysprobe, gaming detection and the full eval suite follow in M4.1 once a second hardware profile actually exists.
- B. The full M4 as designed in arch/Overall.md.
- C. Minimal slice, but include `fa-gpu-agent` wake/yield from the start because the GPU PC is shared with gaming (D-020).
- D. Something else (describe).

## Answer
<!-- answer below this line -->
A.
