---
gsd_state_version: '1.0'
status: in_progress
progress:
  total_phases: 6
  completed_phases: 4
  total_plans: 7
  completed_plans: 7
  percent: 67
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-07-28)

**Core value:** An engineer with no programming knowledge can configure a system topology, run an
optimization solve, and see dispatch/sizing results on screen — without notebooks or a fragile
local Python environment.
**Current focus:** Phase 5 (Tariff Formula Validation & Port — formulas done, FIN-03 UI work
remains) and Phase 6 (Deployment & Go-Live — Docker being installed for verification).

## Current Position

Phase: 5 AND 6 of 6, both IN PROGRESS, neither complete
Plan: Phase 5 — framework plan done, formula-validation plan done (FIN-01/02); a third
UI-collection plan (FIN-03) is unstarted, not blocked. Phase 6 — artifact plan done,
verify+provision plan in progress (Docker install underway).
Status: Phase 5 is unblocked now (Benja resolved the open questions, Claude independently
verified) — remaining work is normal unstarted UI work, not an external blocker. Phase 6 is
actively being unblocked (installing Docker Desktop) rather than stalled.
Last activity: 2026-07-28 — Spain and El Salvador tariff formulas validated: a background research
agent's report (copied durably into `.planning/research/tariff_spain_el_salvador.md`) plus Benja's
corrections plus Claude's own independent web-search re-verification of the highest-stakes claims
(Spain's 1.5% "tasa municipal" confirmed real; El Salvador's 13% IVA confirmed to apply to private
AES distributors; CUST/COSTAMM confirmed as real distinct charges). Both formulas ported into
`engine/tariffs/{spain,el_salvador}.py` with hand-computable worked-example tests (13 engine tests
passing). In parallel, installed Docker Desktop via winget for Phase 6 verification — installed
successfully but the daemon needs WSL2/Virtual-Machine-Platform enabled and first-run setup, both
of which need admin elevation and an interactive session Claude's automation doesn't have; Benja is
handling that manually. PROJECT.md, REQUIREMENTS.md, ROADMAP.md updated.

Progress: [██████░░░░] 67% (4 of 6 phases fully done; Phases 5 and 6 both partially done — Phase 5
much closer to done than Phase 6 now)

## Performance Metrics

**Velocity:**
- Total plans completed: 0
- Average duration: -
- Total execution time: 0 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| - | - | - | - |

**Recent Trend:**
- Last 5 plans: none yet
- Trend: N/A

*Updated after each plan completion*

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- Pre-roadmap: two-service architecture locked in — Rust `server/` + Python `engine/` over
  internal HTTP (see PROJECT.md)
- Pre-roadmap: rejected full Rust optimization rewrite, Tauri desktop app, and 100%-Python stack
  (see PROJECT.md)
- Phase 1 (2026-07-28): concurrency/session isolation resolved — in-memory `SessionManager`,
  UUID-keyed, per-session lock, blocking solves offloaded to a thread pool (see PROJECT.md Key
  Decisions and ROADMAP.md Phase 1 detail)
- Phase 2 (2026-07-28): auth mechanism (Supabase Auth, dedicated `OptiStor` project), persistence
  approach (SQLite via `sqlx`), and multi-tenancy model (`role`/`org_id` in Supabase `app_metadata`)
  all resolved and verified against the live Supabase project (see PROJECT.md Key Decisions and
  ROADMAP.md Phase 2 detail)
- Phase 3 (2026-07-28): frontend framework resolved — HTMX + server-rendered Askama templates,
  chosen over Leptos/WASM since this phase is forms + validation, not rich client interactivity
  (see PROJECT.md Key Decisions and ROADMAP.md Phase 3 detail)
- Phase 4 (2026-07-28): charting library resolved — Apache ECharts over Plotly.js, chosen
  explicitly for visual polish to get closer to PVSyst-caliber report aesthetics (see PROJECT.md
  Key Decisions and ROADMAP.md Phase 4 detail)
- Phase 5 (2026-07-28): multi-jurisdiction tariff architecture resolved — shared `Market` entity +
  pluggable `TariffModel` registry, confirmed via research that El Salvador and Nicaragua don't
  share a unified MER price so each is its own `Market`. **Both countries' actual formulas also
  now resolved and validated** — see ROADMAP.md Phase 5 detail for the full verification trail.
- Phase 6 (2026-07-28, partial): deployment target resolved — Fly.io, for persistent SQLite
  volumes, private inter-app networking, built-in TLS, low ops overhead (see PROJECT.md Key
  Decisions and ROADMAP.md Phase 6 detail). Artifacts built, Docker install in progress.
- 0 decisions remain open in the Open Decisions Tracker — all 8 original ones are now resolved.
  Remaining work is execution (Phase 5's FIN-03 UI, Phase 6's verify+deploy), not more decisions.

### Pending Todos

- **Phase 5's real remaining work (FIN-03)**: build Configurar UI fields to collect each market's
  regulated-rate parameters — Spain needs `peaje_energia`/`cargo_energia` (and ideally the client's
  actual contracted tariff band, e.g. 6.3 TD, rather than a single manually-entered number); El
  Salvador needs `distribucion`/`cust`/`costamm`/`comercializacion`. Once collected, wire them
  through `engine_client.rs`'s currently-empty `params` in `try_compute_tariff`, then remove the
  dashboard's "provisional" flag. Also unbuilt, lower priority: a real market-price (spot price)
  input, since `engine_client.rs` currently sends an all-zero placeholder array.
- **Docker setup for Phase 6**: Docker Desktop installed via winget, but starting it needs WSL2/
  Virtual Machine Platform enabled (admin elevation) and first-run setup (interactive) — neither
  of which Claude's automation could complete. Benja is finishing this manually; once the daemon
  is up, resume with `docker compose up --build` and confirm the engine can actually solve inside
  its container (the GEKKO/Fortran risk noted in Blockers/Concerns), then move to real Fly.io
  provisioning.

### Blockers/Concerns

- `SessionManager` (engine, Phase 1) is single-process, in-memory only — doesn't survive an
  `engine/` restart and doesn't scale across multiple worker processes/replicas. Acceptable for v1,
  but flag if Phase 6's deployment target needs multiple `engine/` replicas.
- **Resolved, Phase 5**: Spain and El Salvador tariff formulas validated and ported — see
  `.planning/research/tariff_spain_el_salvador.md` and ROADMAP.md Phase 5 detail. The old
  prototype's unvalidated `get_index_tariff` is fully superseded, not resurrected.
- FIN-03 still needs a Configurar UI extension to collect each market's regulated-rate parameters
  before `engine_client.rs` can stop falling back to the provisional flat tariff — see Pending
  Todos. Not a blocker on anyone external, just unstarted UI work.
- LCOS (Levelized Cost of Storage) still isn't computed anywhere — needs project economics
  (capex, opex, discount rate) not yet collected. Independent of the tariff formulas now being
  done; scope as its own follow-up if it needs more than a quick addition to FIN-03's work.
- Neither validated tariff model has a validated *export* (excess-injection) price — both return
  the raw energy reference price for export, which is a documented simplification, not a
  researched answer (Spain's real "compensacion simplificada de excedentes" mechanism specifically
  was out of scope for this pass).
- Spain's time-of-use period mapping (P1-P6, which calendar hours map to which period) isn't
  implemented — `peaje_energia`/`cargo_energia` are single representative values per call, not
  per-period arrays. Matters once someone wants period-accurate cost modeling, not before.
- **Docker Desktop is installed but not yet running** — winget install succeeded, but starting the
  WSL2 backend needs Windows features enabled (admin elevation) and first-run setup (interactive
  EULA/onboarding), neither of which Claude's automated shell could complete (no elevation, no
  human to click through prompts). Benja is finishing this manually. The single biggest unverified
  risk once it's up: whether GEKKO's bundled Fortran-compiled local solver binary actually runs
  inside the `python:3.11-slim` container — added `libgfortran5` proactively as a likely
  requirement, but this is a guess, not a confirmed fix. Confirm the engine can *solve*, not just
  start, before trusting the Dockerfiles.

## Deferred Items

Items acknowledged and carried forward from previous milestone close:

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| Scope | Automatic report/proposal generation (PDF/HTML) | Deferred to v2 | Initial roadmap (2026-07-28) |
| Scope | Rolling-horizon dispatch solving | Deferred to v2 | Initial roadmap (2026-07-28) |

## Session Continuity

Last session: 2026-07-28
Stopped at: Spain and El Salvador tariff formulas validated, ported, and tested (Phase 5's
FIN-01/02 done); Docker Desktop installed but needs Benja to finish manual first-run setup (WSL2 +
EULA) before Phase 6 verification can resume. About to commit both. Neither phase is fully
complete:
- **Phase 5**: FIN-03 remains — build Configurar UI fields for each market's regulated-rate
  parameters (Spain: peaje/cargo; El Salvador: distribucion/cust/costamm/comercializacion), wire
  them through `engine_client.rs`, then remove the dashboard's "provisional" flag. This is normal
  unstarted work now, not blocked on anyone.
- **Phase 6**: once Benja confirms Docker Desktop is running, resume with
  `docker compose up --build`, confirm both health checks and specifically that the engine can
  *solve* (not just start) inside its container, then move to real Fly.io provisioning.
Resume file: None
