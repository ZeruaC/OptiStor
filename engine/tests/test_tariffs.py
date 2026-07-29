"""Tariff framework: registry lookup, plus a hand-computable worked example
per validated model (Phase 5, FIN-02) proving the formula matches what's
documented in `.planning/research/tariff_spain_el_salvador.md`, not just
that it runs.
"""

import pytest
from fastapi.testclient import TestClient

from optistor_engine.main import app
from optistor_engine.tariffs.registry import get_tariff_model

client = TestClient(app)


def test_registry_has_spain_and_el_salvador():
    assert get_tariff_model("spain").key == "spain"
    assert get_tariff_model("el_salvador").key == "el_salvador"


def test_unknown_market_key_raises():
    with pytest.raises(KeyError):
        get_tariff_model("nicaragua")


def test_spain_worked_example():
    """0.05 EUR/kWh spot, peaje 0.005, cargo 0.003 EUR/kWh — hand-computed:
    (0.05*1.05 + 0.005 + 0.003) * 1.015 * 1.0511 * 1.21
    """
    model = get_tariff_model("spain")
    export_cost, import_cost = model.compute(
        [0.05], peaje_energia=0.005, cargo_energia=0.003
    )

    base = 0.05 * 1.05 + 0.005 + 0.003
    expected_import = base * 1.015 * 1.0511 * 1.21

    assert export_cost == [0.05]
    assert import_cost == pytest.approx([expected_import])


def test_spain_requires_peaje_and_cargo():
    """Regulatory band values vary per client/contract — must not be
    silently defaulted."""
    model = get_tariff_model("spain")
    with pytest.raises(TypeError):
        model.compute([0.05])


def test_el_salvador_worked_example():
    """0.10 USD/kWh PEt reference + 0.02 distribucion + 0.01 cust +
    0.005 costamm + 0.008 comercializacion, hand-computed:
    (0.10 + 0.02 + 0.01 + 0.005 + 0.008) * 1.13
    """
    model = get_tariff_model("el_salvador")
    export_cost, import_cost = model.compute(
        [0.10], distribucion=0.02, cust=0.01, costamm=0.005, comercializacion=0.008
    )

    expected_import = (0.10 + 0.02 + 0.01 + 0.005 + 0.008) * 1.13

    assert export_cost == [0.10]
    assert import_cost == pytest.approx([expected_import])


def test_el_salvador_requires_all_regulated_components():
    model = get_tariff_model("el_salvador")
    with pytest.raises(TypeError):
        model.compute([0.10])


def test_compute_endpoint_returns_400_for_missing_params():
    resp = client.post("/tariffs/spain/compute", json={"spot_price": [0.05]})
    assert resp.status_code == 400


def test_compute_endpoint_returns_200_with_valid_params():
    resp = client.post(
        "/tariffs/spain/compute",
        json={"spot_price": [0.05], "params": {"peaje_energia": 0.005, "cargo_energia": 0.003}},
    )
    assert resp.status_code == 200
    body = resp.json()
    assert body["export_cost"] == [0.05]
    assert body["import_cost"][0] > 0.05


def test_compute_endpoint_returns_404_for_unknown_market():
    resp = client.post("/tariffs/nicaragua/compute", json={"spot_price": [10.0]})
    assert resp.status_code == 404
