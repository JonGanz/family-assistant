---
id: R-001
title: System One models (systemonemodels.org)
date: 2026-10-23
method: Sonnet 5.5 sub-agent (medium effort), web fetch of systemonemodels.org and its model pages
verified: directory pages only. The HF and GitHub repos, the request/response JSON, quantized builds, and CPU latency (except one vendor claim) were NOT verified
informs: [D-024, Q-024, Q-025, Q-026, Q-027]
---

# R-001: System One models

Prompted by the user's correction that "System 1" in Q-023 meant **System One models** (https://systemonemodels.org/). The first reading, in D-022, took it as the generic Kahneman System 1 idea.

## What they are
- They return **typed answers with probabilities** instead of generated text ([guide](https://systemonemodels.org/guides/what-is-a-system-one-model/)).
  - **Decision models:**
    - `Choice`: up to 255 options, with a probability for each.
    - `Score`: an ordered scale of 2 to 10 levels.
    - `Noul`: yes/no, as a single probability.
  - **Extraction models:** return typed values (dates, numbers, spans).
- The term was coined by TypeSafe AI (Jev, hosted, 2026-09-15). The directory is independent of any vendor.
- Labels are supplied **for each request**, so it works zero-shot with no retraining. These models can't generate text, so they are meant to be paired with an LLM.
- The directory lists 34 models: 33 decision and 1 extraction. 12 are open source, and 19 have weights available.

## Candidates that fit our hardware (8 GB 3070 / CPU core node)
| Model | Size | License | Notes |
|---|---|---|---|
| GLiNER2.5-Decide (Fastino) | 340M (HF says ~486M) | Apache-2.0 | Python `gliner2`. Vendor claims **~167 ms on CPU**. Low independent score (JDI full 10.14) |
| Strands-Decider 2B | 1.9B | Apache-2.0 (v21 training data has commercial restrictions) | GPU, Mac, or CPU. 4k context. Has a `strands-decider serve` server |
| Kev (0.8B / 4B) | 0.8B, 4B | Apache-2.0 | LoRA plus a pointer head, so **not Ollama-loadable**. CUDA, ROCm, or MLX. Serves one request at a time. Trained on ≤384 state tokens. CPU not documented |
| pplx-decider-flash | 4.5B | Apache-2.0 | ~9 GiB, too big for the 3070 unless quantized, and no quantized build is documented |

Too big or not suitable: pplx-decider v1.2 (26B), GLiDE 27B (community license), Kev-27B, elephant-1 (9.4B, one CUDA GPU, possibly fits only if quantized), the CC-BY-NC models, and the hosted-only ones (Jev, Clairic, and others).

## Runtime & API
- **Nothing found for GGUF, Ollama, llama.cpp, ONNX, or Rust.** The models are Python-based, so a self-hosted one runs as a **Python sidecar service**.
- The de facto API is `POST /v1/systemone`, from Jev. Kev and Strands copy it. Others use `/v1/decisions` (Perplexity, Kai). Calling it from Rust is plain reqwest + JSON. The exact body was not verified.
- Probability outputs aren't part of Ollama's API.

## Fit for the email use case
- ✅ Category: `Choice` over the household's configured categories. Time-sensitive: `Noul`. Urgency: `Score`. Labels are configured at runtime, which matches D-004.
- ✅ The output already includes probabilities, so it plugs straight into confidence thresholds.
- ⚠️ **Single-label only.** Results can shift when options are reordered or reworded. Small models are weaker, so ask one question per request.
- ⚠️ **No learning from corrections** without fine-tuning, and fine-tuning is barely documented.
- ❌ **No local extraction** (events, amounts, dates). The only extraction model is Clairic, which is hosted-only and in preview. A generative LLM (System 2) is still needed for that.
- ⚠️ **Maturity:** everything is under 6 weeks old. Benchmarks are mostly vendor-run and calibration isn't perfect, so **test on our own emails**.

## Implications (my assessment)
1. The concept fits D-002 and D-022's tiering well. A decision model is a better System 1 than a small generative LLM: it is faster, gives calibrated-ish probabilities, and has no JSON-format failures.
2. Because the area is young, the backend must be **pluggable**: a `DecisionBackend` trait speaking the `/v1/systemone` dialect, with model choice made by an **evaluation harness** on a labelled sample of real household email.
3. The embedding kNN over user corrections is still worth having, because System One models don't learn from corrections.
4. A Python sidecar is a third-party runtime in a container. That is different from writing our own code in Python (D-003), but it needs the user's OK (Q-024).
5. The same primitive could serve other decisions too: member linking, calendar routing, task priority, weather severity (Q-026).
