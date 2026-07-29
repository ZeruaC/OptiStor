"""Tariff framework: registry lookup and the explicit "pending" state for
jurisdictions whose formula hasn't been validated yet (Phase 5, FIN-01)."""

import pytest
from fastapi.testclient import TestClient

from optistor_engine.main import app
from optistor_engine.tariffs.base import TariffPending
from optistor_engine.tariffs.registry import get_tariff_model

client = TestClient(app)


def test_registry_has_spain_and_el_salvador():
    assert get_tariff_model("spain").key == "spain"
    assert get_tariff_model("el_salvador").key == "el_salvador"


def test_unknown_market_key_raises():
    with pytest.raises(KeyError):
        get_tariff_model("nicaragua")


def test_pending_models_raise_not_silently_compute():
    for key in ("spain", "el_salvador"):
        with pytest.raises(TariffPending):
            get_tariff_model(key).compute([10.0, 20.0])


def test_compute_endpoint_returns_501_for_pending_market():
    resp = client.post("/tariffs/spain/compute", json={"spot_price": [10.0, 20.0]})
    assert resp.status_code == 501
    assert "pendiente de validacion" in resp.json()["detail"]


def test_compute_endpoint_returns_404_for_unknown_market():
    resp = client.post("/tariffs/nicaragua/compute", json={"spot_price": [10.0]})
    assert resp.status_code == 404
