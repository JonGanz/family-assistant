# src/gen

Generated, do not edit by hand.

- `openapi.json`: from `cargo run -q -p fa-server -- gen-openapi --out apps/web/src/gen/openapi.json`.
- `api.d.ts`: `pnpm run gen` (openapi-typescript) from `openapi.json`. Request/response shapes come from here (see `src/api/client.ts`).
- `types/*.ts`: ts-rs output from `fa-domain`; used only for ID and enum types the OpenAPI schema leaves as plain strings.
