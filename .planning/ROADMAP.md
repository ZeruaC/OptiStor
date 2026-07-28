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
| 1 | Concurrency/session isolation model for engine solves (in-memory GEKKO `System` per session) | Phase 1 | **Resolved** — in-memory `SessionManager` keyed by UUID, async registry lock + per-session lock, blocking `solve()` offloaded to a thread pool (see Phase 1 detail) |
| 2 | Authentication mechanism (internal team vs. external partner tiers) | Phase 2 | **Resolved** — Supabase Auth, dedicated `OptiStor` project (`fyqulandxyicawmvquxg`); `server/` verifies ES256 JWTs against Supabase's JWKS |
| 3 | Client/project data model & persistence (flat per-client files vs. real database) | Phase 2 | **Resolved** — SQLite via `sqlx`; `organizations` + `projects` tables |
| 4 | Multi-tenancy / partner permission model | Phase 2 | **Resolved** — `role`/`org_id` as Supabase `app_metadata` claims, enforced server-side; verified with real internal and partner JWTs |
| 5 | Frontend framework (Leptos/WASM vs. server-rendered HTMX + Askama) | Phase 3 | **Resolved** — HTMX + Askama; this phase is forms-and-validation-heavy, not worth a second (WASM) build toolchain |
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

- [x] **Phase 1: Engine API & Session Isolation** - Give `engine/` a full HTTP surface (topology, data, solve, results) that's safe for concurrent users
- [x] **Phase 2: Server Foundations — Auth & Project Persistence** - Give `server/` real login, permission tiers, and durable client/project storage
- [x] **Phase 3: Configurar — Topology & Data Input UI** - Engineers build a system topology and enter all input data through a real UI, no code required
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
  1. ✅ An engineer (via API call) can define a topology (which components exist, which connections
     are enabled) for a fresh session
  2. ✅ An engineer can submit consumption/production profiles, tariffs, and technical specs
     (power/energy caps, efficiency, cycle max, degradation profile) for that session
  3. ✅ An engineer can trigger a single-shot solve and get back energy-flow time series and KPIs,
     with the GEKKO row-0 cold-start quirk handled (not left as a spurious fixed zero)
  4. ✅ Two engineers running solves at the same time never see each other's data or corrupt each
     other's in-memory session state
**Plans**: Implemented directly (no separate PLAN.md — scope was small and well-understood from the
already-ported engine); see commit history for the change set.
**Completed 2026-07-28.** What shipped, in `engine/src/optistor_engine/api/`:
  - `sessions.py` — `SessionManager`: in-memory dict keyed by a UUID `session_id`, async registry
    lock guarding the dict itself, one `asyncio.Lock` per session guarding its GEKKO `Generic`
    instance. This is the resolution of Open Decisions Tracker #1.
  - `schemas.py` — Pydantic request/response models mirroring the existing `Generic`/
    `StorageProducerGridConsumer` method signatures (topology, time, consumption, production,
    storage spec, grid spec, solve result).
  - `kpis.py` — post-solve KPI computation (total consumption, grid import/export, self-consumption
    %, total energy cost when a cost objective with grid costs was configured) read from the
    engine's already-computed cumulative `_energy_connections`, not re-derived from raw tariff math
    (keeps Phase 5's scope clean).
  - `routes.py` — `POST /sessions`, `POST /sessions/{id}/{time,consumption,production,storage,grid}`,
    `POST /sessions/{id}/solve`, `DELETE /sessions/{id}`. The solve route runs GEKKO's blocking
    `solve()` in a thread pool (`anyio.to_thread.run_sync`) so one session's solve can't stall
    another's request handling, and drops row 0 from both the time array and every flow series in
    the response (v1 is single-shot only, so row 0 is always the meaningless cold-start value, per
    ENG-04).
  - Tests: `engine/tests/test_api.py` — full create→configure→solve→delete flow, a 404 check for
    unknown sessions, and `test_sessions_are_isolated` (two sessions with different consumption
    inputs produce different KPI results, proving no shared state). All passing alongside the
    pre-existing `test_smoke.py`.
**Known limitation carried forward (not a gap in this phase's scope, but worth remembering):** the
`SessionManager` is single-process, in-memory only — it doesn't survive an `engine/` restart and
doesn't support horizontal scaling across multiple worker processes/replicas. That's fine for v1
(durable persistence is Phase 2's job, PROJ-02), but revisit if Phase 6's deployment target ends up
needing multiple `engine/` replicas behind a load balancer.

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
  1. ⚠️ A user can log in with credentials appropriate to their access tier — the *mechanism* is done
     (Supabase Auth, verified end-to-end with a real account); the actual **login page** doesn't
     exist yet, since no frontend framework is chosen until Phase 3. Not a gap in this phase's own
     scope, just don't mistake it for a finished login screen.
  2. ✅ An external partner can only see/access their own client's projects; internal staff have
     appropriately broader access
  3. ✅ A project (topology + data + solve results) persists across server restarts and browser
     sessions
  4. ✅ A user can create, list, and reopen their own past projects
**Plans**: Implemented directly (no separate PLAN.md — same rationale as Phase 1).
**Completed 2026-07-28.** What shipped, in `server/src/`:
  - `auth.rs` — `JwtVerifier`: fetches and caches Supabase's JWKS (ES256 EC keys, 10-minute TTL,
    auto-refetch on unknown `kid` in case of key rotation), verifies bearer tokens, and extracts
    `role` (`internal`/`partner`) + `org_id` from the `app_metadata` claims into an `AuthUser` Axum
    extractor. Resolves Open Decisions Tracker #2 and #4.
  - `db.rs` — SQLite via `sqlx` (`migrations/0001_init.sql`: `organizations` + `projects` tables,
    IDs stored as TEXT for easy inspection). Resolves Open Decisions Tracker #3.
  - `projects.rs` — `POST /organizations` (internal-only), `POST /projects`, `GET /projects`,
    `GET /projects/{id}`, all scoped by `org_id` for partner accounts; internal accounts (`org_id`
    claim absent) see everything.
  - **Verified against the live Supabase project, not just designed**: created a real test user,
    toggled its `app_metadata` between `role: internal` and `role: partner` + `org_id` via direct
    SQL (to avoid Supabase's email rate limit blocking a second signup), and confirmed with real
    JWTs that (a) an internal account can create organizations and projects anywhere, (b) a partner
    account listing projects only sees their own org's, (c) a partner fetching another org's
    project by id gets 404 — not 403, so existence isn't leaked, (d) a partner trying to create an
    organization gets 403, and (e) projects survive a full server restart. Test user deleted
    afterward to leave the Supabase project clean.
  - `db.rs` also has 3 checked-in `#[cfg(test)]` unit tests against an in-memory SQLite DB, covering
    the same org-scoping logic deterministically without depending on live Supabase network calls.
  - **Bug found and fixed during implementation**: `jsonwebtoken` v11 requires explicitly selecting
    a crypto backend (`aws_lc_rs` or `rust_crypto`) or it panics on first use — not a code mistake,
    a breaking change in that crate's v11 API. Fixed by enabling the `aws_lc_rs` feature, matching
    the backend `reqwest` already pulls in via `rustls`.
**Known gap, explicitly deferred, not silently dropped:** the actual login page UI. It's out of this
phase's scope by design (no frontend framework chosen yet) and will be built in Phase 3 alongside
the Configurar UI, calling straight into the Supabase Auth REST API this phase verified works.

### Phase 3: Configurar — Topology & Data Input UI
**Goal**: Engineers with no programming knowledge can build a system topology and enter all
required input data through a real UI.
**Depends on**: Phase 1 (engine API to call), Phase 2 (project persistence to attach configs to)
**Requirements**: CONF-01, CONF-02, CONF-03, CONF-04, CONF-05
**Decision point**: Frontend framework — Leptos (Rust/WASM, keeps the whole stack in one language)
vs. server-rendered HTMX + Askama (simpler, less new-tech risk, pairs naturally with Axum) — not
chosen; must be resolved before UI work in this phase proceeds (Open Decisions Tracker #5).
**Success Criteria** (what must be TRUE):
  1. ✅ Frontend framework decision is made and recorded in PROJECT.md Key Decisions
  2. ✅ Engineer can visually add/remove/enable components (consumer, PV producer, battery, grid)
     for a project
  3. ✅ Engineer can enable/disable connections between components matching the existing
     `ConnectionConfig` model
  4. ✅ Engineer can upload or manually enter consumption profile, production profile, tariff/price
     data, and technical specs (power caps, energy caps, efficiency, cycle max, degradation
     profile) — manual entry only in v1; upload is a future enhancement
  5. ✅ Incomplete or invalid configurations are flagged to the engineer before a solve is attempted
**Plans**: Implemented directly (no separate PLAN.md — same rationale as Phases 1-2).
**Completed 2026-07-28.** What shipped, in `server/`:
  - **Frontend decision**: HTMX + Askama over Leptos/WASM — this phase is forms and validation
    feedback, not rich client-side interactivity, so a second build toolchain wasn't worth it.
  - `templates/` — Askama templates (`layout.html`, `login.html`, `projects_list.html`,
    `project_edit.html`, `_validation.html`), `static/htmx.min.js` vendored locally (no runtime CDN
    dependency) and served via `tower_http::services::ServeDir`.
  - `src/auth.rs` extended: `AuthUser` now accepts a session cookie (`optistor_token`) as well as
    the `Authorization` header, since plain browser page navigations can't easily attach custom
    headers the way API/HTMX calls can.
  - `templates/login.html` — calls Supabase's `/auth/v1/token` REST endpoint directly with the
    project's public anon/publishable key (no `@supabase/supabase-js` dependency needed), sets the
    session cookie, redirects to `/app/projects`.
  - `src/config.rs` — `ProjectData` (+ `ConnectionConfigData`, `StorageSpecData`, `GridSpecData`),
    the shape of `projects.data`. Field names mirror the engine's Pydantic schemas
    (`engine/src/optistor_engine/api/schemas.py`) so Phase 4 can forward this blob into the
    engine's session endpoints without translation. `missing()` implements CONF-05; `parse_series`
    turns a textarea of comma/newline-separated numbers into a `Vec<f64>` for manual profile entry.
    5 unit tests.
  - `src/ui.rs` — new `/app/...` routes (`/app/login`, `/app/projects` list+create,
    `/app/projects/{id}` Configurar page, `/app/projects/{id}/config` HTMX partial save endpoint).
    Deliberately separate from the JSON API in `projects.rs`: HTML forms post
    `application/x-www-form-urlencoded`, not JSON, so rather than branching one route on
    Content-Type, the UI got its own routes calling straight into the same `db` functions
    (`projects.rs::scope_for` made `pub` and reused, so org-scoping logic isn't duplicated).
  - **Verified in a real browser** (not just curl): logged in against the live Supabase project,
    created an organization and project through the UI, filled in the full Configurar form
    (topology, connections, time horizon, consumption/production profiles, storage and grid
    specs), saved via the HTMX partial POST, watched the validation panel flip from listing 8
    missing items to "Configuracion completa" without a full page reload, then did a hard page
    reload and confirmed every field and the validation state persisted from SQLite.
  - **Tooling quirk found, not a product bug**: the browser automation tool's synthetic mouse click
    on the submit button didn't trigger the browser's native form-submit-on-click wiring in this
    environment (confirmed via `checkValidity()` — form was valid — and by manually invoking
    `htmx.trigger(form, 'submit')`, which fired the POST correctly and returned 200). A real user
    clicking a real submit button in a real browser doesn't hit this; noted here only so a future
    session doesn't mistake it for a regression.
**Known, deliberate limitation for Phase 4 to handle carefully:** number fields left blank render
as "0" once round-tripped through a save (since HTML number inputs can't cleanly distinguish
"never set" from "explicitly zero" without extra plumbing this phase didn't build). `missing()`
still correctly treats a field as satisfied only once the user actually enters something — optional
fields like storage efficiency default to displaying 0 rather than the engine's own default of 1.0.
When Phase 4 forwards a saved `ProjectData` into the engine's `/storage` endpoint, it should treat a
0.0 efficiency as "unset, use 1.0" rather than passing it through literally.

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
| 1. Engine API & Session Isolation | 1/1 | Done | 2026-07-28 |
| 2. Server Foundations — Auth & Project Persistence | 1/1 | Done | 2026-07-28 |
| 3. Configurar — Topology & Data Input UI | 1/1 | Done | 2026-07-28 |
| 4. Simular & Dashboard — Solve & Results UI | 0/TBD | Not started | - |
| 5. Tariff Formula Validation & Port | 0/TBD | Not started | - |
| 6. Deployment & Go-Live | 0/TBD | Not started | - |
