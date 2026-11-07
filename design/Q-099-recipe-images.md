---
id: Q-099
title: "Recipe images: store a copy or link to the source"
status: open
batch: 24
area: domain
created: 2026-11-07
resolved:
decisions: []
---

# Q-099: Recipe images: store a copy or link to the source

## Context
Imported recipes usually carry an `image_url` on the source site. Showing it by linking makes every phone and the kiosk request the third-party site (a privacy leak and a broken image when the site changes). Storing a copy costs disk space (a resized JPEG/WebP is about 50–150 KB).

## Question
How should recipe images be handled?

## Options
- A. Download a resized copy on import and serve it from the app; adults can also upload a photo for any recipe (suggested)
- B. Link to the source image
- C. No images
- D. Configurable in Settings → Meals (store / link / off), default A

## Answer
<!-- answer below this line -->

