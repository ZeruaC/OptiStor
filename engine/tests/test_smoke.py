"""Smoke test proving the ported optimization core solves end to end.

Builds a small consumer + PV producer + battery + grid system, runs one
cost-minimizing horizon, and checks the solve actually completes and
respects the basic energy balance.
"""

import numpy as np

from optistor_engine.optimization.systems import StorageProducerGridConsumer


def test_cost_optimization_solves():
    n_pred = 6

    m = StorageProducerGridConsumer(
        objective="cost",
        consumption=True,
        production=True,
        storage=True,
        name="smoke",
        remote=False,
    )

    m.set_time(n_pred + 1, 1, n_pred)

    m.set_storage_power_cap({"storage": (50.0, 50.0)})
    m.set_storage_energy_cap_degradation({"storage": {0: 1.0, 8000: 0.7}})
    m.set_storage_eff({"storage": (0.95, 0.95)})
    m.set_storage_energy_cap({"storage": (0.0, 100.0)})
    m.set_storage_cycle_max({"storage": 1.0})
    m._components["storage"].set_energy_init(50.0)

    consumption = np.array([100.0, 120.0, 90.0, 80.0, 110.0, 130.0, 100.0])
    production = np.array([0.0, 10.0, 60.0, 90.0, 40.0, 5.0, 0.0])
    grid_cost_out = np.array([0.20, 0.20, 0.10, 0.10, 0.15, 0.30, 0.30])
    grid_cost_in = -0.9 * grid_cost_out

    m.set_consumption({"consumer": consumption})
    m.set_production({"producer": production})
    m.set_energy_cost({(None, "grid"): grid_cost_in, ("grid", None): grid_cost_out})
    m.set_power_max({(None, "grid"): 200.0, ("grid", None): 200.0})

    m.solve(disp=False)

    assert not m.results.empty

    power_columns = [v.name for v in m._power_connections.values()]
    flows = m.results.loc[:, power_columns]
    assert (flows.to_numpy() >= -1e-6).all(), "power flows must stay non-negative"

    # Row 0 is GEKKO's fixed initial condition for the MV power connections
    # (fixed_initial=True) — on a cold start it has no prior solved value to
    # inherit and stays at its default of 0. It only becomes meaningful once a
    # rolling-horizon shift_model() carries a real value into it, so we check
    # the energy balance from t=1 onward here.
    inflow_columns = [n for n in power_columns if n.endswith("_2_consumer_power")]
    total_in = flows.loc[:, inflow_columns].sum(axis=1)
    np.testing.assert_allclose(total_in.to_numpy()[1:], consumption[1:], atol=1e-2)
