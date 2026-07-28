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

### Active

<!-- Current v1 scope. Full descriptions and acceptance detail in REQUIREMENTS.md. -->

**Phase 1 — Engine API & Session Isolation**
- [ ] ENG-01: Engine exposes a topology-configuration endpoint
- [ ] ENG-02: Engine exposes data/spec-input endpoints
- [ ] ENG-03: Engine exposes a single-shot solve-trigger endpoint
- [ ] ENG-04: Solve endpoint correctly handles the GEKKO row-0 cold-start quirk
- [ ] ENG-05: Concurrent sessions are isolated from each other

**Phase 2 — Server Foundations (Auth & Persistence)**
- [ ] AUTH-01: Authentication mechanism decided
- [ ] AUTH-02: User can log in with tiered access
- [ ] AUTH-03: External partners restricted to their own client/project data
- [ ] PROJ-01: Client/project data model & persistence approach decided
- [ ] PROJ-02: Projects persist across server restarts
- [ ] PROJ-03: User can create/list/reopen their own projects

**Phase 3 — Configurar (Topology & Data Input UI)**
- [ ] CONF-01: Frontend framework decided
- [ ] CONF-02: Engineer can configure components (consumer/PV/battery/grid) visually
- [ ] CONF-03: Engineer can enable/disable connections between components
- [ ] CONF-04: Engineer can enter/upload all input data (profiles, tariffs, specs)
- [ ] CONF-05: Invalid/incomplete configurations flagged before solve

**Phase 4 — Simular & Dashboard (Solve & Results UI)**
- [ ] SIML-01: Engineer can trigger a solve from the UI
- [ ] SIML-02: Charting library decided
- [ ] DASH-01: Energy-flow-over-time chart shown after solve
- [ ] DASH-02: KPIs shown (self-consumption %, cost, LCOS) — provisional-tariff-flagged
- [ ] DASH-03: Battery degradation/SoH curve shown

**Phase 5 — Tariff Formula Validation & Port**
- [ ] FIN-01: Tariff formulas reviewed and validated by a domain/finance expert
- [ ] FIN-02: Validated tariff calculation ported into engine/ with a test
- [ ] FIN-03: Dashboard KPIs switched from provisional to validated tariff logic

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
| Concurrency/session isolation model for engine solves | Not yet designed — real architectural gap | — Pending (Phase 1) |
| Authentication mechanism (internal team vs. external partner tiers) | Not yet chosen | — Pending (Phase 2) |
| Client/project data model & persistence (flat per-client files vs. real database) | Not yet chosen | — Pending (Phase 2) |
| Multi-tenancy / partner permission model | Not yet designed | — Pending (Phase 2) |
| Frontend framework: Leptos (Rust/WASM) vs. server-rendered HTMX + Askama | Not yet chosen | — Pending (Phase 3) |
| Charting library: Plotly.js vs. ECharts | Not yet chosen | — Pending (Phase 4) |
| Tariff formula validity (`get_index_tariff` family) | Needs domain/finance expert review before porting; original code's own comments flag it as unvalidated | — Pending (Phase 5) |
| Deployment/hosting target (cloud VM, PaaS, self-hosted) | Not yet chosen | — Pending (Phase 6) |

---
*Last updated: 2026-07-28 after initial roadmap creation*
