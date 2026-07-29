---
gsd_state_version: '1.0'
status: in_progress
progress:
  total_phases: 6
  completed_phases: 4
  total_plans: 4
  completed_plans: 4
  percent: 67
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-07-28)

**Core value:** An engineer with no programming knowledge can configure a system topology, run an
optimization solve, and see dispatch/sizing results on screen — without notebooks or a fragile
local Python environment.
**Current focus:** Phase 5 — Tariff Formula Validation & Port

## Current Position

Phase: 5 of 6 (Tariff Formula Validation & Port) — IN PROGRESS, not complete
Plan: 1 of ~2 in current phase (framework plan done; the "port validated formula" plan is blocked)
Status: Blocked on external input (domain/finance tariff formula), not on Claude
Last activity: 2026-07-28 — Phase 5's scope grew from "port one Spain formula" to "build a
multi-jurisdiction tariff framework" (projects are international; location determines
regulation/prices; clients/projects in the same country share setup). Built and verified against
the live engine: a shared `Market` entity (server), a pluggable `TariffModel` framework in
`engine/tariffs/` with Spain/El Salvador as explicit `TariffPending` stubs, a stateless
`/tariffs/{key}/compute` endpoint, and `engine_client.rs` wired to attempt the real model and fall
back to the provisional flat tariff on any failure — confirmed the exact fallback sequence in
engine logs (501 pending -> solve proceeds anyway -> 200). FIN-04/05 done; FIN-01/02/03 (an actual
validated formula) remain open, blocked on Benja/domain-finance input, not a technical gap.
PROJECT.md, REQUIREMENTS.md, ROADMAP.md updated to reflect partial completion honestly.

Progress: [██████░░░░] 67% (4 of 6 phases fully done; Phase 5 partially done)

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
- Phase 5 (2026-07-28, partial): multi-jurisdiction tariff architecture resolved — shared `Market`
  entity + pluggable `TariffModel` registry, confirmed via research that El Salvador and Nicaragua
  don't share a unified MER price so each is its own `Market`. The tariff formula itself (which
  this decision framework exists to receive) is still open — see Blockers/Concerns.
- 2 decisions remain open (tariff formula for Spain/El Salvador, deployment target) — tracked in
  ROADMAP.md "Open Decisions Tracker" and PROJECT.md Key Decisions

### Pending Todos

None yet.

### Blockers/Concerns

- Tariff formula (`get_index_tariff` family) is unvalidated in the old prototype — original code
  has its own "check bracket"/"ask for the shift" comments. Must not reach a commercial proposal
  before Phase 5 closes it out; Phase 4's KPIs must ship visibly marked provisional in the
  meantime.
- `SessionManager` (engine, Phase 1) is single-process, in-memory only — doesn't survive an
  `engine/` restart and doesn't scale across multiple worker processes/replicas. Acceptable for v1,
  but flag if Phase 6's deployment target needs multiple `engine/` replicas.
- Resolved in Phase 4: a saved `ProjectData` storage efficiency of 0.0 is now treated as "unset,
  use 1.0" in `engine_client.rs` — the Phase 3 form limitation (blank number fields round-trip as
  "0") is handled, not just noted.
- Phase 5 needs to replace the provisional flat tariff `engine_client.rs` injects for the "cost"
  objective (arbitrary -0.05/0.20 per kWh) with real validated formulas — don't extend it, replace
  it outright once Phase 5's review lands.
- LCOS (Levelized Cost of Storage) still isn't computed anywhere — it needs project economics
  (capex, opex, discount rate) not yet collected, plus Phase 5's validated tariff/finance model.
  Not a regression, just unbuilt; scope it into Phase 5 or flag as its own follow-up if it turns
  out to need more than Phase 5's stated scope.
- **Phase 5 is genuinely blocked, not stalled by inaction**: `engine/tariffs/spain.py` and
  `el_salvador.py` both unconditionally raise `TariffPending` — Benja was asked directly whether
  the old prototype's active or commented-out bracket formula was correct and chose neither,
  saying Balore will design fresh formulas and that the framework needed to support multiple
  countries first. That framework now exists; the next concrete thing this phase needs is Benja
  (or whoever he delegates to) supplying an actual formula for at least one country.
- No market-price (spot price) input UI exists — nobody knows what shape each country's formula
  will need until it's provided. `engine_client.rs` sends an all-zero placeholder to
  `/tariffs/{key}/compute` today; harmless only because every model currently ignores its input
  and raises `TariffPending` unconditionally. Don't mistake this plumbing for "input-complete" —
  real formulas will need real input collection built alongside them.

## Deferred Items

Items acknowledged and carried forward from previous milestone close:

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| Scope | Automatic report/proposal generation (PDF/HTML) | Deferred to v2 | Initial roadmap (2026-07-28) |
| Scope | Rolling-horizon dispatch solving | Deferred to v2 | Initial roadmap (2026-07-28) |

## Session Continuity

Last session: 2026-07-28
Stopped at: Phase 5's framework (FIN-04/05) built and verified against the live engine, about to be
committed. **Phase 5 is not complete** — FIN-01/02/03 need an actual validated tariff formula for
at least Spain or El Salvador, which is a domain/finance decision, not a technical one Claude can
make. Next concrete step: get that formula from Benja (or whoever he delegates to), including
clarifying what raw inputs it needs (e.g. does it need a spot-price series? if so from where?),
then implement it in `engine/tariffs/{spain,el_salvador}.py` replacing the `TariffPending` stub,
add a worked-example test, and only then remove the dashboard's "provisional" flag. Until that
lands, Phase 6 (deployment) could reasonably be worked in parallel if Benja prefers, since it
doesn't depend on Phase 5 being finished, only on Phases 1-4's flow being solid (which it is).
Resume file: None
