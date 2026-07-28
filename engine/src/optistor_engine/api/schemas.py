"""Request/response models for the session API.

Field shapes mirror the existing methods on the ported `Generic`/
`StorageProducerGridConsumer` classes as closely as possible (see
`optimization/systems.py`) rather than inventing a new data model.
"""

from typing import Literal

from pydantic import BaseModel, Field


class ConnectionConfigIn(BaseModel):
    production_to_consumption: bool = True
    production_to_grid: bool = True
    production_to_storage: bool = True
    grid_to_consumption: bool = True
    grid_to_storage: bool = True
    storage_to_consumption: bool = True
    storage_to_grid: bool = True


class TopologyIn(BaseModel):
    objective: Literal["energy", "cost"] = "cost"
    peak_minimization: bool = False
    consumption: bool = True
    production: bool = True
    storage: bool = True
    peak_allowed: bool = False
    connection_config: ConnectionConfigIn = ConnectionConfigIn()


class SessionOut(BaseModel):
    session_id: str


class TimeIn(BaseModel):
    period: int = Field(..., gt=1, description="Number of time points in the horizon, including t=0")
    step: float = Field(1.0, gt=0, description="Time step size in hours")


class ProfileIn(BaseModel):
    values: list[float]


class StorageSpecIn(BaseModel):
    power_cap: tuple[float, float] = Field(..., description="(charge_max, discharge_max) in kW")
    energy_cap: tuple[float, float] = Field(..., description="(min, max) in kWh")
    efficiency: tuple[float, float] = Field((1.0, 1.0), description="(charge_eff, discharge_eff)")
    cycle_max: float = Field(..., description="Maximum number of full cycles allowed over the horizon")
    degradation_profile: dict[float, float] = Field(
        default_factory=lambda: {0.0: 1.0},
        description="Cycle count -> capacity retention factor (0-1)",
    )
    energy_init: float | None = Field(None, description="Initial stored energy in kWh")


class GridSpecIn(BaseModel):
    power_cap: tuple[float, float] = Field(..., description="(export_max, import_max) in kW")
    energy_cost: tuple[list[float], list[float]] | None = Field(
        None, description="(export_cost, import_cost) arrays, one value per time step"
    )


class SolveResult(BaseModel):
    time: list[float]
    flows: dict[str, list[float]]
    kpis: dict[str, float]
