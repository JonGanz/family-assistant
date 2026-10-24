---
id: D-003
title: Backend in Rust (Axum/Tokio/SQLx); frontend Vue 3 + TypeScript
status: accepted
date: 2026-10-23
source_questions: [Q-002]
supersedes:
superseded_by:
affects: [arch/Overall.md]
---

> **Amended by [D-025](D-025-python-model-sidecars.md):** third-party model servers may run as Python containers. Our code stays Rust.

# D-003: Backend in Rust (Axum/Tokio/SQLx); frontend Vue 3 + TypeScript

## Context
The user doesn't want Python and is fine with TypeScript or Rust. They know JS, want to learn Rust, and prefer Vue to React.

## Decision
- **Backend:** a Rust workspace. Axum handles HTTP and SSE, Tokio is the runtime, SQLx talks to Postgres with compile-time checked queries, Serde handles serialisation, and `utoipa` produces OpenAPI. The API and workers ship as one binary that can run in either mode.
- **Frontend:** Vue 3 with `<script setup>` and TS, Vite, Pinia, Vue Router, and `vite-plugin-pwa`. TS types are generated from the Rust DTOs (`ts-rs` or the OpenAPI spec).
- **TUI (stretch):** Rust `ratatui`, using the same API client crate.

## Consequences
- The language meets the learning goal, and a Rust binary uses little memory on the low-power core node.
- Some ecosystem gaps need custom code. There is no Rust equivalent of `recipe-scrapers`, so we parse JSON-LD and microdata with `scraper`. CalDAV client support is thin, so we use `reqwest` with WebDAV XML via `quick-xml` (or the `libdav` crate). Recurrence uses the `rrule` crate and ICS uses the `icalendar` crate. Gmail and Google Calendar use plain REST via `reqwest` plus the `oauth2` crate.
- Writing Rust will be slower at first. The crates are split into small modules so the user can learn one area at a time.

## Alternatives considered
TypeScript end to end (Nest/Fastify with Vue) would mean faster delivery and a richer Google and CalDAV ecosystem (`googleapis`, `tsdav`). Not chosen because it doesn't meet the learning goal. It stays the fallback if Rust slows progress too much.
