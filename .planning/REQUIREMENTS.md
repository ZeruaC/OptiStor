# Requirements: Balore OptiStor

**Defined:** 2026-07-28
**Core Value:** An engineer with no programming knowledge can configure a system topology, run an
optimization solve, and see dispatch/sizing results on screen — without notebooks, without a
fragile local Python environment, and without a programmer's help.

## v1 Requirements

Requirements for the initial release. Each maps to exactly one roadmap phase (see Traceability).

### Engine API & Session Isolation (ENG)

- [x] **ENG-01**: Engine exposes an endpoint to define/configure a system topology (which
  components exist — consumer, PV producer, battery, grid; which connections between them are
  enabled) for a session, mirroring the existing `ConnectionConfig` dataclass and
  `StorageProducerGridConsumer` factory — `POST /sessions` (2026-07-28)
- [x] **ENG-02**: Engine exposes endpoints to set consumption profile, production profile,
  tariff/price data, and technical specs (power caps, energy caps, efficiency, cycle max,
  degradation profile) for a session, mirroring the existing `set_consumption` / `set_production` /
  `set_storage_*` / `set_grid_*` / `set_power_max` / `set_energy_cost` methods on the ported
  `Generic` system class — `POST /sessions/{id}/{time,consumption,production,storage,grid}`
  (2026-07-28)
- [x] **ENG-03**: Engine exposes an endpoint to trigger a single-shot solve and returns energy-flow
  time series plus KPI results — `POST /sessions/{id}/solve` (2026-07-28)
- [x] **ENG-04**: Single-shot solve results are correct despite the GEKKO row-0 cold-start quirk
  (row 0 dropped or seeded with a sensible initial guess, not left as a spurious fixed zero) — row
  0 dropped from both `time` and every `flows` series in the solve response (2026-07-28)
- [x] **ENG-05**: Concurrent sessions from different users/partners are isolated from each other in
  memory (no shared mutable GEKKO `System` state, no cross-session data leakage) — in-memory
  `SessionManager` keyed by UUID, one `Generic` system per session, async registry lock + per-session
  lock, blocking `solve()` offloaded to a thread pool; verified with `test_sessions_are_isolated`
  (2026-07-28)

### Server Foundations — Authentication (AUTH)

- [x] **AUTH-01**: A decision on the authentication mechanism is made and documented in
  PROJECT.md Key Decisions — Supabase Auth (dedicated project `OptiStor`, `fyqulandxyicawmvquxg`),
  server verifies Supabase-issued ES256 JWTs against its JWKS (2026-07-28)
- [x] **AUTH-02**: A user can log in with credentials appropriate to their access tier — verified
  end-to-end against the real Supabase project (signup, sign-in, server accepts the resulting JWT).
  The actual **login page UI** doesn't exist yet — it lands in Phase 3 once a frontend framework is
  chosen; what's done here is the backend mechanism it will call (2026-07-28)
- [x] **AUTH-03**: An external partner's account is restricted to their own client/project data;
  internal Balore staff have appropriately broader access (multi-tenancy/permission model) — `role`
  + `org_id` carried as Supabase `app_metadata` claims, enforced by `server/`; verified with a real
  partner-role JWT unable to list, or fetch by id (404, not 403 — no existence leak), another org's
  project (2026-07-28)

### Server Foundations — Client/Project Persistence (PROJ)

- [x] **PROJ-01**: A decision on the client/project data model and persistence approach (flat
  per-client files, like the old prototype's `projects/senia/` pattern, vs. a real database) is
  made and documented — SQLite via `sqlx`, `organizations` + `projects` tables, `projects.data` as
  an opaque JSON blob whose shape is deferred to the Phase 3/4 UI (2026-07-28)
- [x] **PROJ-02**: A client/project (topology + data + solve results) persists across server
  restarts — verified by killing and restarting the server and confirming prior projects were
  still listed (2026-07-28)
- [x] **PROJ-03**: A user can create, list, and reopen their own past projects — `POST /projects`,
  `GET /projects`, `GET /projects/{id}`, all org-scoped for partner accounts (2026-07-28)

### Configurar — Topology & Data Input UI (CONF)

- [x] **CONF-01**: A decision on the dashboard frontend framework (Leptos vs. server-rendered
  HTMX + Askama) is made and documented — HTMX + Askama, chosen for this phase's forms-and-
  validation-heavy nature over adding a second (WASM) build toolchain (2026-07-28)
- [x] **CONF-02**: Engineer can visually add/remove/enable system components (consumer, PV
  producer, battery, grid) for a project, with no programming knowledge required — checkboxes on
  the Configurar page (2026-07-28)
- [x] **CONF-03**: Engineer can enable/disable connections between components through the UI,
  matching the `ConnectionConfig` model — 7-checkbox connection matrix (2026-07-28)
- [x] **CONF-04**: Engineer can upload or manually enter consumption profile, production profile,
  tariff/price data, and technical specs through the UI — manual entry via textareas (comma/
  newline-separated) and number fields satisfies the "or manually enter" half of this requirement;
  CSV upload is a possible future enhancement, not required to close this item (2026-07-28)
- [x] **CONF-05**: Incomplete or invalid configurations are flagged to the engineer in the UI
  before a solve is attempted — `ProjectData::missing()` renders a live validation panel, updated
  via an HTMX partial swap on save without a full page reload; verified in a real browser session
  (2026-07-28)

### Simular — Solve Trigger (SIML)

- [x] **SIML-01**: Engineer can trigger a solve from the dashboard UI and see solve
  progress/completion/error status — HTMX-driven "Simular" button (gated on the config being
  complete), `htmx-indicator` spinner while solving, errors surfaced inline instead of a crash
  (2026-07-28)
- [x] **SIML-02**: A decision on the charting library (Plotly.js vs. ECharts) is made and
  documented — Apache ECharts, chosen explicitly for visual polish (gradient area fills, smooth
  animated curves) over Plotly's more utilitarian default look, to get closer to PVSyst-caliber
  report aesthetics; free/open-source (Apache 2.0), no licensing cost (2026-07-28)

### Dashboard — Results Display (DASH)

- [x] **DASH-01**: Engineer sees a chart of energy flows over time after a solve completes — raw
  per-connection flows aggregated into 6 meaningful series (Consumo, Produccion, Red
  importacion/exportacion, Bateria carga/descarga) and rendered as a smoothed, gradient-filled
  ECharts line/area chart (2026-07-28)
- [x] **DASH-02**: Engineer sees KPIs (self-consumption %, cost, LCOS) after a solve completes,
  visibly marked provisional pending tariff validation (Phase 5) — KPI cards plus an explicit
  on-screen note that cost figures use a provisional flat example tariff, not the real formulas
  Phase 5 will validate. LCOS specifically is not yet computed (needs the real tariff/finance
  model from Phase 5) — tracked as a Phase 5 follow-up, not silently dropped (2026-07-28)
- [x] **DASH-03**: Engineer sees a battery degradation/SoH curve over the modeled horizon —
  implemented as the battery's State of Charge (%) trajectory over the solved horizon (a genuinely
  meaningful single-solve output) plus a scalar SoH KPI computed from cycle usage. True multi-cycle
  SoH *decline* tracking needs the rolling-horizon solving that's deferred to v2 (ENG-06); the
  plumbing (`_SoH` property, cycle tracking) is already in place from the ported engine and will
  show real movement once rolling-horizon accumulates cycles across solves (2026-07-28)

### Tariff Formula Validation & Port (FIN)

- [ ] **FIN-01**: The old prototype's tariff-calculation formulas (`get_index_tariff`,
  `get_index_tariff_simp`, `adjust_index_tariff`) have been reviewed and validated (or corrected)
  by a domain/finance expert
- [ ] **FIN-02**: The validated tariff calculation is ported into `engine/` with a test against a
  known-correct worked example
- [ ] **FIN-03**: Dashboard cost/LCOS KPIs use the validated tariff logic and the "provisional"
  flag from DASH-02 is removed

### Deployment & Go-Live (DEPLOY)

- [ ] **DEPLOY-01**: A decision on the deployment/hosting target (cloud VM, PaaS, self-hosted) is
  made and documented
- [ ] **DEPLOY-02**: `server/` and `engine/` are deployed and reachable at a real, non-localhost
  URL
- [ ] **DEPLOY-03**: The full Configurar→Simular→Dashboard flow works end-to-end against the
  deployed instance for both an internal account and an external-partner account

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Reporting & Proposal Generation

- **REPORT-01**: System generates an HTML/PDF technical-economic proposal document from a
  completed solve
- **REPORT-02**: Proposal includes NPV/IRR financial calculations, referencing the old prototype's
  `numpy-financial` usage
- **REPORT-03**: Proposal generation reuses/adapts the old `batopt/src/report/` and `src/reporting/`
  modules (Jinja templates, mkdocs/zensical site generation) as a reference, not a verbatim port

### Advanced Dispatch

- **ENG-06**: Engine supports rolling-horizon dispatch solves (via `shift_model()`) in addition to
  single-shot solves

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Full Rust rewrite of the optimization core | GEKKO's NLP/collocation solving (orthogonal collocation, automatic differentiation, IPOPT backend) has no Rust equivalent; a from-scratch LP/QP rewrite (feasible via `good_lp`/HiGHS or Clarabel) was judged too risky for v1 given the numbers feed client-facing commercial proposals |
| Tauri desktop app | Product must be a hosted, multi-user webapp usable remotely by external partners, not a local desktop tool |
| 100%-Python web stack (e.g. Solara) | Team was already burned by Python environment fragility (a `pdm` venv broke mid-project on the old prototype); Python is confined to the `engine/` microservice only |
| Rolling-horizon dispatch solving in v1 | v1 ships single-shot solves only; rolling-horizon deferred to v2 (see ENG-06) |
| Automatic proposal/report generation in v1 | Explicitly deferred to v2 by the client (Benja Ballesteros); v1 stops at on-screen dashboard results (see REPORT-0x) |
| Referencing the predecessor company or its branding | Balore OptiStor is a clean-slate rebuild; `F:\batopt` is a different repo and must not be branded as "ours" |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| ENG-01 | Phase 1 | Done |
| ENG-02 | Phase 1 | Done |
| ENG-03 | Phase 1 | Done |
| ENG-04 | Phase 1 | Done |
| ENG-05 | Phase 1 | Done |
| AUTH-01 | Phase 2 | Done |
| AUTH-02 | Phase 2 | Done (backend; login page UI in Phase 3) |
| AUTH-03 | Phase 2 | Done |
| PROJ-01 | Phase 2 | Done |
| PROJ-02 | Phase 2 | Done |
| PROJ-03 | Phase 2 | Done |
| CONF-01 | Phase 3 | Done |
| CONF-02 | Phase 3 | Done |
| CONF-03 | Phase 3 | Done |
| CONF-04 | Phase 3 | Done |
| CONF-05 | Phase 3 | Done |
| SIML-01 | Phase 4 | Done |
| SIML-02 | Phase 4 | Done |
| DASH-01 | Phase 4 | Done |
| DASH-02 | Phase 4 | Done |
| DASH-03 | Phase 4 | Done (SoC trajectory; full SoH decline needs v2 rolling-horizon) |
| FIN-01 | Phase 5 | Pending |
| FIN-02 | Phase 5 | Pending |
| FIN-03 | Phase 5 | Pending |
| DEPLOY-01 | Phase 6 | Pending |
| DEPLOY-02 | Phase 6 | Pending |
| DEPLOY-03 | Phase 6 | Pending |

**Coverage:**
- v1 requirements: 27 total
- Mapped to phases: 27
- Unmapped: 0 ✓

---
*Requirements defined: 2026-07-28*
*Last updated: 2026-07-28 after Phase 4 (Simular & Dashboard — Solve & Results UI) completed*
