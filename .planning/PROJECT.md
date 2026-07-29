# Balore OptiStor

## What This Is

A hosted, multi-user web application for modeling, sizing, and optimizing the dispatch of BESS
(battery energy storage) and PV (solar) installations. Used directly by engineers (no programming
knowledge required) at Balore Advance Engineering and by their external partners, to produce
technical/economic proposals for client RFQs — e.g. industrial backup power, solar+storage
self-consumption projects.

This is a from-scratch, product-grade successor to an internal Jupyter-notebook prototype. It is a
clean-slate rebuild: do not reference the predecessor company or its old branding anywhere in this
project's docs, UI, or generated content.

## Core Value

An engineer with no programming knowledge can configure a system topology, run an optimization
solve, and see dispatch/sizing results on screen — without notebooks, without a fragile local
Python environment, and without needing a programmer's help.

## Business Context

- **Customer**: Internal Balore Advance Engineering engineers and their external partners, who use
  the tool to produce technical/economic proposals for industrial clients responding to RFQs
  (e.g. cement plants).
- **Revenue model**: Internal enablement tool — it doesn't generate revenue directly, but the
  proposals it produces feed EPC / leasing / PPA contracts that do.
- **Success metric**: An engineer can go from RFQ input data (consumption, production, tariffs,
  specs) to on-screen dispatch/sizing results without programmer involvement.
- **Strategy notes**: None yet.

## Requirements

Full detail with IDs lives in `.planning/REQUIREMENTS.md`. Summary below.

### Validated

<!-- Shipped and confirmed working, pre-roadmap. -->

- ✓ Two-service architecture scaffolded and communicating: `server/` (Rust/Axum) exposes
  `GET /health` and `GET /api/engine/health` (proxies to the Python engine); `engine/`
  (Python 3.11 + FastAPI) exposes `GET /health`. Both verified running (`:8000` / `:8001`) and
  reaching each other — pre-roadmap (2026-07-28)
- ✓ Core GEKKO-based optimization engine
  (`engine/src/optistor_engine/optimization/{base,components,systems,utils}.py`) ported from
  `F:\batopt\src\batopt\`, reviewed and debugged (not copy-pasted) — pre-roadmap (2026-07-28)
- ✓ `engine/tests/test_smoke.py` proves the ported engine solves: builds a consumer + PV producer +
  battery + grid system, runs a cost-minimizing 6-hour horizon, checks energy balance. Passes —
  pre-roadmap (2026-07-28)
- ✓ **Phase 1 — Engine API & Session Isolation** (2026-07-28): `engine/` now exposes a full REST
  API (`POST /sessions`, `.../time`, `.../consumption`, `.../production`, `.../storage`,
  `.../grid`, `.../solve`, `DELETE /sessions/{id}`) built on an in-memory `SessionManager`
  (UUID-keyed, per-session `asyncio.Lock`, blocking `solve()` offloaded to a thread pool) — resolves
  the concurrency/session-isolation decision. Solve responses drop the GEKKO row-0 cold-start value
  and include basic KPIs (consumption, grid import/export, self-consumption %, cost when
  applicable). Covered by `engine/tests/test_api.py` (full flow, 404 on unknown session, and an
  explicit two-session isolation check). ENG-01 through ENG-05 done.
- ✓ **Phase 2 — Server Foundations (Auth & Persistence)** (2026-07-28): `server/` verifies
  Supabase-issued ES256 JWTs (dedicated `OptiStor` Supabase project, `fyqulandxyicawmvquxg`) via
  JWKS, reads `role`/`org_id` from `app_metadata` for multi-tenancy, and persists
  organizations/projects in SQLite via `sqlx`. `POST /organizations` (internal-only),
  `POST /projects`, `GET /projects`, `GET /projects/{id}` all org-scoped for partners. Verified
  end-to-end against the live Supabase project with real internal- and partner-role JWTs (partner
  correctly restricted, 404-not-403 on cross-org access by id, projects survive a server restart),
  plus 3 checked-in unit tests. AUTH-01 through AUTH-03 and PROJ-01 through PROJ-03 done — except
  the login-page UI itself, deferred into Phase 3 by design (no frontend framework chosen yet).
- ✓ **Phase 3 — Configurar (Topology & Data Input UI)** (2026-07-28): frontend decision made —
  HTMX + Askama over Leptos/WASM (forms-and-validation-heavy phase, not worth a second build
  toolchain). Login page calls Supabase's REST auth API directly and sets a session cookie;
  `AuthUser` now accepts either that cookie or the `Authorization` header. New `/app/...` routes
  (`server/src/ui.rs`) for project list/create and the Configurar page (topology checkboxes,
  7-way `ConnectionConfig` matrix, time horizon, consumption/production profiles via textarea,
  storage and grid specs). `server/src/config.rs`'s `ProjectData` mirrors the engine's Pydantic
  schemas field-for-field so Phase 4 can forward it without translation; `missing()` powers a live
  validation panel that updates via an HTMX partial swap. Verified in a real browser end-to-end
  (login through the live Supabase project, create org/project, fill and save the full form,
  validation flips to "complete", persists across a hard reload), plus 5 unit tests. CONF-01
  through CONF-05 done. Known follow-up for Phase 4: blank number fields round-trip as "0" after a
  save, so treat a 0.0 storage efficiency as "unset" rather than literal when wiring into the
  engine.
- ✓ **Phase 4 — Simular & Dashboard (Solve & Results UI)** (2026-07-28): charting library decided
  — Apache ECharts over Plotly.js, chosen explicitly for visual polish (gradient fills, smooth
  animated curves) to get closer to PVSyst-caliber report aesthetics; free, no licensing cost.
  Engine extended with `battery_soh_pct` (scalar KPI) and a `storage_soc_pct` time series in the
  solve response. New `server/src/engine_client.rs` drives a full engine session lifecycle from a
  project's stored config (applying the Phase 3 fix: blank efficiency treated as "unset, use 1.0")
  and injects a clearly-labeled provisional flat tariff so the "cost" objective has something to
  optimize before Phase 5. `projects.data` now stores `ProjectRecord{config, last_solve}` so solve
  results persist like config does. New `POST /app/projects/{id}/solve` aggregates ~9 raw
  per-connection flows into 6 chartable series and returns an HTMX-swappable dashboard fragment
  with KPI cards + two ECharts charts (energy flows, battery SoC). Verified in a real browser
  against the live Supabase project and engine: solved a fully-configured project, confirmed real
  KPI numbers, both charts rendering as actual ECharts canvas instances with no console errors,
  and persistence across a hard reload. SIML-01/02 and DASH-01/02/03 done. LCOS itself isn't
  computed yet (needs Phase 5's finance model) and full multi-cycle SoH decline needs v2's
  rolling-horizon solving (ENG-06) — both noted as scope, not gaps.

### Active

<!-- Current v1 scope. Full descriptions and acceptance detail in REQUIREMENTS.md. -->

**Phase 5 — Tariff Formula Validation & Port** (in progress — scope grew 2026-07-28 from "port one
Spain formula" to "multi-jurisdiction tariff framework"; framework done, formulas still pending)
- [ ] FIN-01: Tariff formula reviewed/designed for at least Spain and El Salvador by domain/finance
  expertise — **blocked**: Benja chose to design fresh per-country formulas rather than resolve the
  old Spain bracket ambiguity, but hasn't provided either country's formula yet
- [ ] FIN-02: Validated tariff calculation ported into `engine/tariffs/` with a test — blocked on
  FIN-01
- [ ] FIN-03: Dashboard KPIs switched from provisional to validated tariff logic — blocked on
  FIN-01/02
- [x] FIN-04: Shared `Market` entity (country + tariff model key), reusable across
  organizations/projects (2026-07-28)
- [x] FIN-05: Pluggable `TariffModel` framework in `engine/` with Spain/El Salvador as explicit
  `TariffPending` stubs, verified end-to-end against the live engine (2026-07-28)

**Phase 6 — Deployment & Go-Live**
- [ ] DEPLOY-01: Hosting/deployment target decided
- [ ] DEPLOY-02: Full stack deployed and reachable at a real URL
- [ ] DEPLOY-03: Full Configurar→Simular→Dashboard flow verified end-to-end in production

### Out of Scope

- **Full Rust rewrite of the optimization core** — GEKKO's NLP/collocation solving (orthogonal
  collocation, automatic differentiation, IPOPT backend) has no Rust equivalent; a from-scratch
  LP/QP rewrite (technically feasible via `good_lp`/HiGHS or Clarabel, since the model is mostly
  linear/quadratic) was judged too risky for v1 given the numbers feed client-facing commercial
  proposals.
- **Tauri desktop app** — the product must be a hosted, multi-user webapp partners can reach
  remotely, not a local desktop tool.
- **100%-Python web stack (e.g. Solara)** — the team was already burned once by Python environment
  fragility (a `pdm` venv broke mid-project on the old prototype); Python is deliberately confined
  to the `engine/` microservice only, kept isolated from the always-on partner-facing service.
- **Rolling-horizon dispatch solving** — v1 ships single-shot solves only. Rolling-horizon (via
  `shift_model()`) is deferred to v2 — see REQUIREMENTS.md v2 section.
- **Automatic proposal/report generation (PDF/HTML)** — explicitly deferred to v2 by the client
  (Benja Ballesteros). The old `batopt/src/report/` and `src/reporting/` modules (Jinja templates,
  mkdocs/zensical site generation, `numpy-financial` NPV/IRR calcs) are a useful reference for that
  phase when it comes, but are not pulled into v1.
- **Referencing the predecessor company or its branding** — this is a clean-slate rebuild;
  `F:\batopt` is a separate repo and must never be described as "ours" or reused for branding.

## Context

- **Predecessor prototype**: `F:\batopt` (a different repo — do not touch or reference as "ours").
  A collection of Jupyter notebooks plus a `src/batopt` Python package built on GEKKO, modeling
  power systems as a graph of components (Consumer, Producer/PV, Grid, Storage/Battery) connected
  by power flows, with pluggable optimization objectives (minimize cost, maximize self-consumption,
  minimize peak power, track a reference). It worked, but was unusable by non-programmers
  (notebook-driven, no UI), tied to the old company's branding, and dependent on a fragile local
  Python env (`pdm` venv broke mid-project).
- **Real RFQs reviewed as use-case validation** (not committed contracts):
  - **El Salvador (Holcim)**: BESS sized for ~1h backup autonomy for critical kiln loads at two
    cement plants; RTE/degradation/availability guarantees required; EPC or leasing model. This is
    fundamentally a backup-autonomy sizing problem, not hourly dispatch optimization — though the
    battery degradation/SoH model is reusable across both use cases.
  - **Nicaragua (Nagarote, Holcim)**: 4.8 MWp PV + 28 MWh / 2 MW BESS sized to cover ~85% of a
    cement plant's annual demand; PPA (10-25yr) or EPC pricing; needs annual dispatch simulation
    plus LCOS/NPV-based pricing. This is a strong direct fit for the hourly dispatch optimizer and
    is the primary validation case for Phases 1-5.
- **GEKKO cold-start quirk** (documented, must be handled in Phase 1): row 0 of a fresh
  (non-rolling) solve is the *fixed initial condition* for the power-flow connections and defaults
  to 0 — it only becomes a real optimized value once a rolling-horizon `shift_model()` call has
  carried a solved value into it on a 2nd+ solve. Any single-shot solve endpoint must either drop
  row 0 or seed it with a sensible initial guess, or results will silently look wrong.
- **Tariff formulas deliberately not yet ported**: `batopt/utils.py`'s `get_index_tariff`,
  `get_index_tariff_simp`, and `adjust_index_tariff` — the original code has its own unresolved
  comments ("check bracket", "ask for the shift") indicating the formula itself was never fully
  validated. This needs a domain/finance review before it's trusted in a commercial proposal — see
  Phase 5, tracked separately from general engine porting (Phase 1).

## Constraints

- **Tech stack**: `server/` = Rust (Axum: axum, tokio, reqwest, serde, serde_json, tower-http).
  `engine/` = Python 3.11, plain venv at `engine/.venv` (deliberately not `pdm`), FastAPI
  (`optistor_engine.main`). Locked in 2026-07-28 — see Key Decisions below, don't relitigate.
- **Service boundary**: `server/` calls `engine/` over internal HTTP; `engine/` does the actual
  GEKKO solve and stays headless (no charting inside the microservice).
- **Team**: Solo/small team — B. Ballesteros, M. Ballesteros, Balore Eng. — with Claude as
  implementer. No enterprise process overhead.
- **Branding**: Clean-slate rebuild. Never reference the predecessor company or its branding in
  generated docs, UI copy, or proposals.
- **Commercial accuracy**: Dispatch numbers feed client-facing commercial proposals. Tariff
  calculations and battery degradation/SoH modeling are accuracy-sensitive and must not reach a
  real proposal unvalidated (see Phase 5).

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Two-service architecture: Rust `server/` + Python `engine/` over internal HTTP | GEKKO's dynamic/NLP solving (orthogonal collocation, autodiff, IPOPT) has no Rust equivalent; a from-scratch LP/QP rewrite was judged too risky for v1 given commercial-proposal stakes; the always-on partner-facing service needed to avoid the Python env fragility that broke the prototype | ✓ Good (locked 2026-07-28) |
| Rejected: full Rust rewrite of the optimization core | Reimplementing GEKKO's solving capability from scratch was judged too risky for v1 | ✓ Good (rejected) |
| Rejected: Tauri desktop app | Product must be a hosted, multi-user webapp, not a desktop tool | ✓ Good (rejected) |
| Rejected: 100%-Python web stack (e.g. Solara) | Avoids repeating the pdm-venv fragility class of failure in an always-on partner-facing service | ✓ Good (rejected) |
| Concurrency/session isolation model for engine solves | In-memory `SessionManager`, UUID-keyed, one GEKKO `Generic` instance per session, async registry lock + per-session lock, blocking `solve()` offloaded to a thread pool. Single-process only — doesn't survive restarts or scale across replicas (fine for v1; revisit at Phase 6 if needed) | ✓ Good (resolved 2026-07-28, Phase 1) |
| Authentication mechanism: Supabase Auth (dedicated `OptiStor` project, `fyqulandxyicawmvquxg`), `server/` verifies ES256 JWTs via JWKS | Already had a Supabase account; avoids self-hosting password storage/reset/invite flows | ✓ Good (resolved 2026-07-28, Phase 2) |
| Client/project data model & persistence: SQLite via `sqlx` | Simplicity for current scale; flat files rejected outright since they can't support listing/querying a user's own projects (PROJ-03) | ✓ Good (resolved 2026-07-28, Phase 2) |
| Multi-tenancy / partner permission model: `role`/`org_id` as Supabase `app_metadata` claims, enforced by `server/` | Standard org-scoping pattern; verified with real internal- and partner-role JWTs, including that cross-org access by id 404s rather than 403s (no existence leak) | ✓ Good (resolved 2026-07-28, Phase 2) |
| Frontend framework: HTMX + server-rendered Askama templates | Phase 3 is forms + validation feedback, not rich client interactivity; avoids a second (WASM) build toolchain alongside the existing Rust one | ✓ Good (resolved 2026-07-28, Phase 3) |
| Charting library: Apache ECharts | Explicitly chosen for visual polish (gradient fills, smooth animated curves) over Plotly's more utilitarian defaults, to get closer to PVSyst-caliber report aesthetics the client asked for; free/open-source, no licensing cost | ✓ Good (resolved 2026-07-28, Phase 4) |
| Multi-jurisdiction tariff architecture: shared `Market` entity + pluggable `TariffModel` registry in `engine/` | Projects are international; a project's location determines applicable regulation/prices, and clients/projects in the same country shouldn't duplicate that setup. Confirmed via research that El Salvador and Nicaragua don't share a unified regional price despite both trading on the MER, so each country is its own `Market` | ✓ Good (resolved 2026-07-28, Phase 5) |
| Tariff formula validity — Spain and El Salvador (scope grew from just `get_index_tariff`) | Needs domain/finance expert review or fresh design; Benja opted to design fresh per-country formulas rather than resolve the old Spain bracket ambiguity directly, but hasn't provided either yet | — **Pending** (Phase 5, blocks FIN-01..03) |
| Deployment/hosting target (cloud VM, PaaS, self-hosted) | Not yet chosen | — Pending (Phase 6) |

---
*Last updated: 2026-07-28 — Phase 5 (Tariff Formula Validation & Port) in progress: multi-
jurisdiction framework (FIN-04/05) built and verified; FIN-01..03 (an actual validated formula)
still blocked on domain/finance input*
