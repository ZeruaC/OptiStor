"""Stateless tariff computation — no session involved, just raw market
inputs in, effective cost arrays out. Kept separate from the session API
(`routes.py`) since tariff calculation is a pricing concern, not a dispatch
one; `engine_client.rs` on the server calls this before assembling a
session's `/grid` body.
"""

from fastapi import APIRouter, HTTPException
from pydantic import BaseModel

from ..tariffs.base import TariffPending
from ..tariffs.registry import get_tariff_model

router = APIRouter(prefix="/tariffs", tags=["tariffs"])


class ComputeTariffIn(BaseModel):
    spot_price: list[float]
    params: dict = {}


class ComputeTariffOut(BaseModel):
    export_cost: list[float]
    import_cost: list[float]


@router.post("/{key}/compute", response_model=ComputeTariffOut)
async def compute_tariff(key: str, body: ComputeTariffIn) -> ComputeTariffOut:
    try:
        model = get_tariff_model(key)
    except KeyError as exc:
        raise HTTPException(status_code=404, detail=str(exc))

    try:
        export_cost, import_cost = model.compute(body.spot_price, **body.params)
    except TariffPending as exc:
        raise HTTPException(status_code=501, detail=str(exc))

    return ComputeTariffOut(export_cost=export_cost, import_cost=import_cost)
