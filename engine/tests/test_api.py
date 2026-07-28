"""End-to-end test of the session API: create -> configure -> solve -> delete.

Covers ENG-01 (topology), ENG-02 (data/spec input), ENG-03/ENG-04 (solve with
row-0 handling and KPIs), and confirms a deleted session is gone (ENG-05).
"""

from fastapi.testclient import TestClient

from optistor_engine.main import app

client = TestClient(app)


def test_full_session_flow():
    resp = client.post("/sessions", json={"objective": "cost"})
    assert resp.status_code == 200, resp.text
    session_id = resp.json()["session_id"]

    resp = client.post(f"/sessions/{session_id}/time", json={"period": 7, "step": 1.0})
    assert resp.status_code == 200, resp.text

    consumption = [100.0, 120.0, 90.0, 80.0, 110.0, 130.0, 100.0]
    production = [0.0, 10.0, 60.0, 90.0, 40.0, 5.0, 0.0]

    resp = client.post(f"/sessions/{session_id}/consumption", json={"values": consumption})
    assert resp.status_code == 200, resp.text
    resp = client.post(f"/sessions/{session_id}/production", json={"values": production})
    assert resp.status_code == 200, resp.text

    resp = client.post(
        f"/sessions/{session_id}/storage",
        json={
            "power_cap": [50.0, 50.0],
            "energy_cap": [0.0, 100.0],
            "efficiency": [0.95, 0.95],
            "cycle_max": 1.0,
            "degradation_profile": {"0": 1.0, "8000": 0.7},
            "energy_init": 50.0,
        },
    )
    assert resp.status_code == 200, resp.text

    grid_cost_out = [0.20, 0.20, 0.10, 0.10, 0.15, 0.30, 0.30]
    grid_cost_in = [-0.9 * c for c in grid_cost_out]

    resp = client.post(
        f"/sessions/{session_id}/grid",
        json={"power_cap": [200.0, 200.0], "energy_cost": [grid_cost_in, grid_cost_out]},
    )
    assert resp.status_code == 200, resp.text

    resp = client.post(f"/sessions/{session_id}/solve")
    assert resp.status_code == 200, resp.text
    result = resp.json()

    # Row 0 (fixed initial condition, see ENG-04) dropped from a 7-point horizon.
    assert len(result["time"]) == 6
    for series in result["flows"].values():
        assert len(series) == 6

    kpis = result["kpis"]
    assert kpis["total_consumption_kwh"] > 0
    assert 0 <= kpis["self_consumption_pct"] <= 100
    assert "total_energy_cost" in kpis

    resp = client.delete(f"/sessions/{session_id}")
    assert resp.status_code == 200

    resp = client.post(f"/sessions/{session_id}/time", json={"period": 7, "step": 1.0})
    assert resp.status_code == 404


def test_unknown_session_returns_404():
    resp = client.post("/sessions/does-not-exist/solve")
    assert resp.status_code == 404


def test_sessions_are_isolated():
    """ENG-05: two concurrently-open sessions must not share GEKKO state."""

    def make_session(consumption: list[float]) -> str:
        session_id = client.post("/sessions", json={"objective": "energy"}).json()["session_id"]
        client.post(f"/sessions/{session_id}/time", json={"period": 4, "step": 1.0})
        client.post(f"/sessions/{session_id}/consumption", json={"values": consumption})
        client.post(f"/sessions/{session_id}/production", json={"values": [0.0] * 4})
        client.post(
            f"/sessions/{session_id}/storage",
            json={
                "power_cap": [10.0, 10.0],
                "energy_cap": [0.0, 20.0],
                "cycle_max": 1.0,
                "energy_init": 10.0,
            },
        )
        client.post(f"/sessions/{session_id}/grid", json={"power_cap": [100.0, 100.0]})
        return session_id

    session_a = make_session([50.0, 50.0, 50.0, 50.0])
    session_b = make_session([90.0, 90.0, 90.0, 90.0])

    result_a = client.post(f"/sessions/{session_a}/solve").json()
    result_b = client.post(f"/sessions/{session_b}/solve").json()

    assert result_a["kpis"]["total_consumption_kwh"] != result_b["kpis"]["total_consumption_kwh"]

    client.delete(f"/sessions/{session_a}")
    client.delete(f"/sessions/{session_b}")
