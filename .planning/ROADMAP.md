# Roadmap: Balore OptiStor

## Overview

The engine's optimization core is already ported, debugged, and proven with a passing smoke test —
that work is done. What's not done is everything around it. Phase 1 gives `engine/` a real HTTP
surface and closes its one flagged architectural gap (concurrent-session isolation). Phase 2 gives
`server/` actual users — authentication, permission tiers, and durable project storage — instead of
just a health-check proxy. Phases 3 and 4 deliver the client's own mental model of the product,
literally: **Configurar → Simular → Dashboard** — an engineer builds a topology and enters data
(Phase 3), then triggers a solve and sees the results on screen (Phase 4). Two real product
decisions (frontend framework, charting library) get made and locked in along the way. Phase 5 is a
deliberately separate, explicit gate: the old tariff-calculation formulas are unvalidated, and they
don't get trusted in a commercial proposal until a domain/finance expert has reviewed them — this
is called out on its own so it can never be quietly skipped as "part of porting the engine." Phase 6
takes the whole validated stack from developer machines to a real, reachable, partner-usable hosted
service. Automatic proposal/report generation is intentionally not in this roadmap — it's tracked as
v2 in REQUIREMENTS.md, to be roadmapped as its own milestone later.

## Open Decisions Tracker

Every open decision point from the project brief, in one place, each cross-referenced to the phase
that resolves it. Nothing here should be quietly dropped — if a phase is planned or executed
without addressing its row, that's a gap, not a resolution.

| # | Decision | Resolved in | Status |
|---|----------|-------------|--------|
| 1 | Concurrency/session isolation model for engine solves (in-memory GEKKO `System` per session) | Phase 1 | Pending |
| 2 | Authentication mechanism (internal team vs. external partner tiers) | Phase 2 | Pending |
| 3 | Client/project data model & persistence (flat per-client files vs. real database) | Phase 2 | Pending |
| 4 | Multi-tenancy / partner permission model | Phase 2 | Pending |
| 5 | Frontend framework (Leptos/WASM vs. server-rendered HTMX + Askama) | Phase 3 | Pending |
| 6 | Charting library (Plotly.js vs. ECharts) | Phase 4 | Pending |
| 7 | Tariff formula validity — domain/finance expert review of `get_index_tariff` family | Phase 5 | Pending |
| 8 | Deployment/hosting target (cloud VM, PaaS, self-hosted) | Phase 6 | Pending |

Two additional known items from the brief are deliberately *not* decision points needing resolution
before v1 — they're scope calls already made:
- Automatic report/proposal generation → deferred to v2 (see REQUIREMENTS.md v2 section), not
  planned in this roadmap at all.
- Rolling-horizon dispatch solving → deferred to v2 (single-shot only in v1).

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

- [ ] **Phase 1: Engine API & Session Isolation** - Give `engine/` a full HTTP surface (topology, data, solve, results) that's safe for concurrent users
- [ ] **Phase 2: Server Foundations — Auth & Project Persistence** - Give `server/` real login, permission tiers, and durable client/project storage
- [ ] **Phase 3: Configurar — Topology & Data Input UI** - Engineers build a system topology and enter all input data through a real UI, no code required
- [ ] **Phase 4: Simular & Dashboard — Solve & Results UI** - Engineers trigger a solve and see energy flows, KPIs, and degradation on screen
- [ ] **Phase 5: Tariff Formula Validation & Port** - Replace the unvalidated tariff math with a domain-reviewed, tested version before it's trusted in commercial numbers
- [ ] **Phase 6: Deployment & Go-Live** - Take the full stack from localhost to a real, reachable, partner-usable hosted service

## Phase Details

### Phase 1: Engine API & Session Isolation
**Goal**: `engine/` exposes a complete, multi-session-safe REST API — topology configuration,
data/spec input, single-shot solve trigger, and results retrieval — so any client (including the
future dashboard) can drive a full solve end-to-end over HTTP.
**Already done** (pre-roadmap, 2026-07-28): core optimization engine
(`engine/src/optistor_engine/optimization/{base,components,systems,utils}.py`) ported from
`F:\batopt` with 4 real bugs found and fixed (multi-instance name-list generator bug, duplicate
`set_storage_power_ref` method, missing `case _` default in objective dispatch, removed
import-time `plotly` plotting side effect); `engine/tests/test_smoke.py` passes, proving the
engine solves a consumer+PV+battery+grid system and holds energy balance; both services' `/health`
endpoints and the `/api/engine/health` proxy are verified working. This phase adds the HTTP layer
and concurrency handling on top of that already-working core — it does not re-port or re-validate
the optimization math itself.
**Depends on**: Nothing (first phase; builds on already-completed engine core)
**Requirements**: ENG-01, ENG-02, ENG-03, ENG-04, ENG-05
**Decision point**: Concurrency/session isolation model — how simultaneous users each get an
isolated in-memory GEKKO `System` instance is currently undesigned; must be resolved in this
phase, not deferred (see Open Decisions Tracker #1).
**Success Criteria** (what must be TRUE):
  1. An engineer (via API call) can define a topology (which components exist, which connections
     are enabled) for a fresh session
  2. An engineer can submit consumption/production profiles, tariffs, and technical specs
     (power/energy caps, efficiency, cycle max, degradation profile) for that session
  3. An engineer can trigger a single-shot solve and get back energy-flow time series and KPIs,
     with the GEKKO row-0 cold-start quirk handled (not left as a spurious fixed zero)
  4. Two engineers running solves at the same time never see each other's data or corrupt each
     other's in-memory session state
**Plans**: TBD

### Phase 2: Server Foundations — Auth & Project Persistence
**Goal**: `server/` has real multi-user access control and a durable place to store client/project
data, replacing today's health-check-only shell.
**Depends on**: Phase 1 (project/session data shapes are informed by the engine's topology/data
model)
**Requirements**: AUTH-01, AUTH-02, AUTH-03, PROJ-01, PROJ-02, PROJ-03
**Decision points**:
  - Authentication mechanism — none chosen yet; internal Balore team vs. external partners likely
    need different access levels (Open Decisions Tracker #2)
  - Client/project data model & persistence — flat per-client files (like the old prototype's
    `projects/senia/` pattern) vs. a real database, not decided (#3)
  - Multi-tenancy/permission model — not designed (#4)
**Success Criteria** (what must be TRUE):
  1. A user can log in via a login page with credentials appropriate to their access tier
  2. An external partner can only see/access their own client's projects; internal staff have
     appropriately broader access
  3. A project (topology + data + solve results) persists across server restarts and browser
     sessions
  4. A user can create, list, and reopen their own past projects
**Plans**: TBD
**UI hint**: yes

### Phase 3: Configurar — Topology & Data Input UI
**Goal**: Engineers with no programming knowledge can build a system topology and enter all
required input data through a real UI.
**Depends on**: Phase 1 (engine API to call), Phase 2 (project persistence to attach configs to)
**Requirements**: CONF-01, CONF-02, CONF-03, CONF-04, CONF-05
**Decision point**: Frontend framework — Leptos (Rust/WASM, keeps the whole stack in one language)
vs. server-rendered HTMX + Askama (simpler, less new-tech risk, pairs naturally with Axum) — not
chosen; must be resolved before UI work in this phase proceeds (Open Decisions Tracker #5).
**Success Criteria** (what must be TRUE):
  1. Frontend framework decision is made and recorded in PROJECT.md Key Decisions
  2. Engineer can visually add/remove/enable components (consumer, PV producer, battery, grid)
     for a project
  3. Engineer can enable/disable connections between components matching the existing
     `ConnectionConfig` model
  4. Engineer can upload or manually enter consumption profile, production profile, tariff/price
     data, and technical specs (power caps, energy caps, efficiency, cycle max, degradation
     profile)
  5. Incomplete or invalid configurations are flagged to the engineer before a solve is attempted
**Plans**: TBD
**UI hint**: yes

### Phase 4: Simular & Dashboard — Solve & Results UI
**Goal**: Engineers can trigger a solve from the dashboard and see the results — energy flows,
KPIs, degradation — on screen.
**Depends on**: Phase 3
**Requirements**: SIML-01, SIML-02, DASH-01, DASH-02, DASH-03
**Decision point**: Charting library — Plotly.js vs. ECharts — not chosen; must be resolved before
results views are built (Open Decisions Tracker #6).
**Success Criteria** (what must be TRUE):
  1. Charting library decision is made and recorded in PROJECT.md Key Decisions
  2. Engineer can trigger a solve from the dashboard and see its progress/completion/error status
  3. Engineer sees a chart of energy flows over time after a solve completes
  4. Engineer sees KPIs (self-consumption %, cost, LCOS) after a solve completes, visibly marked
     provisional pending tariff validation (Phase 5)
  5. Engineer sees a battery degradation/SoH curve over the modeled horizon
**Plans**: TBD
**UI hint**: yes

### Phase 5: Tariff Formula Validation & Port
**Goal**: Tariff-calculation logic is validated by a domain/finance expert and ported into the
engine as trustworthy, replacing the provisional placeholder wired up in Phase 4. This is
deliberately its own phase — not silently bundled into "porting the engine" — because the original
formulas carry the old code's own unresolved comments ("check bracket", "ask for the shift")
questioning their correctness, and the numbers they produce feed client-facing commercial
proposals.
**Depends on**: Phase 4 (replaces the provisional tariff logic wired into the dashboard's
cost/LCOS KPIs)
**Requirements**: FIN-01, FIN-02, FIN-03
**Decision point**: Not a technical choice — gated on a domain/finance expert review (Open
Decisions Tracker #7). That review can be scheduled and run any time from Phase 2 onward; it's
sequenced last here only because that's when its output gets wired into the dashboard, not because
the review itself must wait.
**Success Criteria** (what must be TRUE):
  1. A domain/finance expert has reviewed and confirmed (or corrected) the `get_index_tariff` /
     `get_index_tariff_simp` / `adjust_index_tariff` formulas from the old prototype
  2. The validated tariff calculation is ported into `engine/` with a test against a
     known-correct worked example
  3. Dashboard cost/LCOS KPIs use the validated tariff logic and the "provisional" flag
     introduced in Phase 4 is removed
**Plans**: TBD

### Phase 6: Deployment & Go-Live
**Goal**: The full stack runs as a real, always-on hosted service partners can reach and log into
— not just localhost dev servers.
**Depends on**: Phases 1-5 (needs the complete, validated Configurar→Simular→Dashboard flow to be
worth deploying)
**Requirements**: DEPLOY-01, DEPLOY-02, DEPLOY-03
**Decision point**: Deployment/hosting target — cloud VM, PaaS, or self-hosted — not chosen (Open
Decisions Tracker #8).
**Success Criteria** (what must be TRUE):
  1. Deployment/hosting target decision is made and recorded in PROJECT.md Key Decisions
  2. `server/` and `engine/` are deployed and reachable at a real, non-localhost URL
  3. The full Configurar→Simular→Dashboard flow works end-to-end against the deployed instance
     for both an internal account and an external-partner account
  4. Both services' `/health` endpoints are monitored in production
**Plans**: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4 → 5 → 6

| Phase | Plans Complete | Status | Completed |
|-------|-----------------|--------|-----------|
| 1. Engine API & Session Isolation | 0/TBD | Not started | - |
| 2. Server Foundations — Auth & Project Persistence | 0/TBD | Not started | - |
| 3. Configurar — Topology & Data Input UI | 0/TBD | Not started | - |
| 4. Simular & Dashboard — Solve & Results UI | 0/TBD | Not started | - |
| 5. Tariff Formula Validation & Port | 0/TBD | Not started | - |
| 6. Deployment & Go-Live | 0/TBD | Not started | - |
