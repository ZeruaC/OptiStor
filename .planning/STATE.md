---
gsd_state_version: '1.0'
status: in_progress
progress:
  total_phases: 6
  completed_phases: 4
  total_plans: 6
  completed_plans: 6
  percent: 67
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-07-28)

**Core value:** An engineer with no programming knowledge can configure a system topology, run an
optimization solve, and see dispatch/sizing results on screen — without notebooks or a fragile
local Python environment.
**Current focus:** Phase 5 (Tariff Formula Validation & Port) and Phase 6 (Deployment & Go-Live),
worked in parallel at Benja's request — Phase 6 doesn't need Phase 5 finished first.

## Current Position

Phase: 5 AND 6 of 6, both IN PROGRESS, neither complete
Plan: Phase 5 — framework plan done, "port validated formula" plan blocked. Phase 6 — artifact
plan done, "verify + provision" plan blocked.
Status: Both blocked on external input/resources, not on Claude — Phase 5 needs a domain/finance
tariff formula; Phase 6 needs Docker (not installed in this environment) and then a real Fly.io
account.
Last activity: 2026-07-28 — Phase 6: decided Fly.io, wrote Dockerfiles/fly.toml/docker-compose for
both services, found and fixed a real bug by inspection (`server/` bound to `127.0.0.1`, invisible
outside a container — now `0.0.0.0`), wrote `DEPLOYMENT.md`. Explicitly could not verify any of it
— no `docker`, no WSL distribution available. In parallel, a background research agent investigated
Spain and El Salvador's actual tariff regulation (CNMC/BOE for Spain, SIGET/UT for El Salvador) —
see Pending Todos, its report needs to be read and relayed to Benja next turn, not yet done.
PROJECT.md, REQUIREMENTS.md, ROADMAP.md updated to reflect both phases' partial completion.

Progress: [██████░░░░] 67% (4 of 6 phases fully done; Phases 5 and 6 both partially done)

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
- Phase 6 (2026-07-28, partial): deployment target resolved — Fly.io, for persistent SQLite
  volumes, private inter-app networking, built-in TLS, low ops overhead (see PROJECT.md Key
  Decisions and ROADMAP.md Phase 6 detail). Artifacts built but unverified.
- 1 decision remains open (tariff formula for Spain/El Salvador) — tracked in ROADMAP.md "Open
  Decisions Tracker" and PROJECT.md Key Decisions

### Pending Todos

- A background research agent's tariff-formula report is sitting unread at
  `C:\Users\Ben\AppData\Local\Temp\claude\F--batopt\81866d14-1e1a-4828-a3bc-fcd78e380ef9\scratchpad\tariff_research.md`
  (a session-scoped scratch path — move/copy anything worth keeping into the repo or a durable
  note before that session's temp directory is gone). Needs to be read and relayed to Benja next
  turn; findings are NOT yet reflected in `engine/tariffs/spain.py` or `el_salvador.py` and must
  not be coded up without his sign-off (per the agent's own instructions and Phase 5's FIN-01
  requirement — domain/finance expert confirmation, not an AI's best guess).

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
- **Docker is not installed in this environment** — no `docker` binary, no WSL distribution. Every
  Phase 6 artifact (`server/Dockerfile`, `engine/Dockerfile`, both `fly.toml`s,
  `docker-compose.yml`) was written and hand-reviewed but never actually built or run. The single
  biggest unverified risk: whether GEKKO's bundled Fortran-compiled local solver binary actually
  runs inside the `python:3.11-slim` container — added `libgfortran5` proactively as a likely
  requirement, but this is a guess, not a confirmed fix. Get Docker access before trusting any of
  it, and specifically confirm the engine can *solve*, not just start.

## Deferred Items

Items acknowledged and carried forward from previous milestone close:

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| Scope | Automatic report/proposal generation (PDF/HTML) | Deferred to v2 | Initial roadmap (2026-07-28) |
| Scope | Rolling-horizon dispatch solving | Deferred to v2 | Initial roadmap (2026-07-28) |

## Session Continuity

Last session: 2026-07-28
Stopped at: Phase 6's deployment artifacts (Fly.io, Dockerfiles, fly.toml, docker-compose,
DEPLOYMENT.md) built and about to be committed, worked in parallel with a background research
agent investigating Spain/El Salvador tariff regulation (per Benja's explicit request to do both
at once). Neither Phase 5 nor Phase 6 is complete:
- **Phase 5**: needs an actual validated tariff formula from Benja (domain/finance decision, not
  technical). The research agent's findings are ready at
  `.../scratchpad/tariff_research.md` (session-scoped path, not yet relayed to Benja or copied
  anywhere durable) — next step is reading that report and presenting it to him for confirmation,
  NOT coding it up unilaterally.
- **Phase 6**: needs Docker access to verify the artifacts actually build/run (specifically
  whether GEKKO solves inside the container), then a real Fly.io account to provision against.
Resume file: None
