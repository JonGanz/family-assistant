---
id: D-017
title: Weather: configurable location, deterministic threshold rules, LLM-suggested advisories
status: accepted
date: 2026-10-23
source_questions: [Q-016]
supersedes:
superseded_by:
affects: [arch/Overall.md]
---

# D-017: Weather: configurable location, deterministic threshold rules, LLM-suggested advisories

## Context
The location must be configurable. Location-appropriate warnings are wanted as an LLM enhancement.

## Decision
- `household.location` holds lat/long and a timezone. Open-Meteo is polled for current conditions, the hourly forecast, and the daily forecast, and snapshots are stored.
- **Deterministic `WeatherRule`s** take the form 'condition over a window → alert and/or task'. Examples: min temp ≤ 0 °C tonight, or rain probability ≥ 70%. They are configurable, and some ship as defaults.
- **LLM enhancement:** a job runs per location each season, or when the forecast changes a lot. It produces (a) short advisories for the dashboard and (b) **suggested new WeatherRules** that suit the local climate. An adult reviews the suggested rules before they become active.

## Consequences
- Weather feeds into task priority (D-013) and into seasonal task triggers.

## Alternatives considered
LLM-only warnings. Rejected because the system would do nothing useful when the GPU node is off.
