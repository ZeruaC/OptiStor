"""Post-solve KPI computation.

Reads the already-computed cumulative `_energy_connections` (see
`System.solve` in `optimization/base.py`) rather than re-integrating power
flows, and only reports KPIs that are actually meaningful for the topology
that was solved (e.g. no self-consumption % without a producer).

Cost/LCOS-grade financial KPIs are intentionally out of scope here — they
depend on the tariff formulas that Phase 5 (FIN-01..03) has not yet
validated. Anything derived from raw energy costs the caller supplied is
fine to surface; nothing from the old, unvalidated tariff math is used.
"""

from ..optimization.systems import Generic, CONSUMER, PRODUCER, GRID, STORAGE


def _last_energy(system: Generic, source: str | None, sink: str | None) -> float | None:
    conn = (source, sink)
    e_conn = system._energy_connections.get(conn)
    if e_conn is None:
        return None
    return float(system.get_value(e_conn)[-1])


def compute_kpis(system: Generic) -> dict[str, float]:
    kpis: dict[str, float] = {}

    def total_into(component: str) -> float:
        return sum(
            _last_energy(system, so, si) or 0.0
            for (so, si) in system._energy_connections
            if si == component
        )

    def total_out_of(component: str) -> float:
        return sum(
            _last_energy(system, so, si) or 0.0
            for (so, si) in system._energy_connections
            if so == component
        )

    if system._consumers:
        kpis["total_consumption_kwh"] = total_into(CONSUMER)

    kpis["total_grid_import_kwh"] = total_out_of(GRID)
    kpis["total_grid_export_kwh"] = total_into(GRID)

    if system._producers:
        total_production_kwh = total_out_of(PRODUCER)
        kpis["total_production_kwh"] = total_production_kwh
        if total_production_kwh > 0:
            self_used_kwh = (
                (_last_energy(system, PRODUCER, CONSUMER) or 0.0)
                + (_last_energy(system, PRODUCER, STORAGE) or 0.0)
            )
            kpis["self_consumption_pct"] = 100.0 * self_used_kwh / total_production_kwh

    if system._storages:
        storage = system._components[STORAGE]
        kpis["battery_soh_pct"] = 100.0 * storage._SoH

    cost_rate_in = system._energy_cost_rate.get((None, GRID))
    cost_rate_out = system._energy_cost_rate.get((GRID, None))
    if cost_rate_in is not None or cost_rate_out is not None:
        total_cost = 0.0
        for rate in (cost_rate_in, cost_rate_out):
            if rate is not None:
                values = system.get_value(rate)
                total_cost += float(values[1:].sum()) * system._timestep
        kpis["total_energy_cost"] = total_cost

    return kpis


def battery_soc_series(system: Generic) -> list[float] | None:
    """State of charge (%) over the solved horizon (row 0 dropped, see ENG-04).

    This is the single-horizon SoC trajectory, not a multi-cycle degradation
    curve — tracking SoH decline across many cycles needs the rolling-horizon
    solving that's deferred to v2 (ENG-06). `battery_soh_pct` in `compute_kpis`
    is the scalar SoH this solve's cycle usage implies, which is what will
    move once rolling-horizon solves start accumulating cycles.
    """
    if not system._storages:
        return None
    storage = system._components[STORAGE]
    capacity = storage.energy.UPPER
    if not capacity:
        return None
    energy = system.results.loc[:, storage.energy.name].to_numpy()[1:]
    return (100.0 * energy / capacity).tolist()
