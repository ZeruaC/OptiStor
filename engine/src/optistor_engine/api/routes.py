"""Session-scoped REST API for the optimization engine.

Every mutating call operates on a session created via POST /sessions and
identified by the returned `session_id`. See `sessions.py` for the
isolation model (ENG-05) and `kpis.py` for post-solve KPI computation.
"""

import anyio
import numpy as np
from fastapi import APIRouter, HTTPException

from ..optimization.systems import ConnectionConfig, STORAGE, GRID
from ..optimization.systems import StorageProducerGridConsumer
from .kpis import battery_soc_series, compute_kpis
from .schemas import (
    GridSpecIn,
    ProfileIn,
    SessionOut,
    SolveResult,
    StorageSpecIn,
    TimeIn,
    TopologyIn,
)
from .sessions import Session, SessionManager

router = APIRouter(prefix="/sessions", tags=["sessions"])
sessions = SessionManager()


async def _get_session(session_id: str) -> Session:
    try:
        return await sessions.get(session_id)
    except KeyError:
        raise HTTPException(status_code=404, detail=f"Unknown session '{session_id}'")


@router.post("", response_model=SessionOut)
async def create_session(topology: TopologyIn) -> SessionOut:
    """ENG-01: define which components exist and how they connect."""
    system = StorageProducerGridConsumer(
        objective=topology.objective,
        peak_minimization=topology.peak_minimization,
        consumption=topology.consumption,
        production=topology.production,
        storage=topology.storage,
        connection_config=ConnectionConfig(**topology.connection_config.model_dump()),
        peak_allowed=topology.peak_allowed,
        remote=False,
    )
    session_id = await sessions.create(system)
    return SessionOut(session_id=session_id)


@router.delete("/{session_id}")
async def delete_session(session_id: str) -> dict:
    await sessions.delete(session_id)
    return {"ok": True}


@router.post("/{session_id}/time")
async def set_time(session_id: str, body: TimeIn) -> dict:
    """ENG-02: define the horizon length and step size."""
    session = await _get_session(session_id)
    async with session.lock:
        session.system.set_time(body.period, body.step)
    return {"ok": True}


@router.post("/{session_id}/consumption")
async def set_consumption(session_id: str, body: ProfileIn) -> dict:
    """ENG-02: consumption profile for the (single) consumer component."""
    session = await _get_session(session_id)
    async with session.lock:
        session.system.set_consumption({"consumer": np.array(body.values)})
    return {"ok": True}


@router.post("/{session_id}/production")
async def set_production(session_id: str, body: ProfileIn) -> dict:
    """ENG-02: production profile for the (single) producer component."""
    session = await _get_session(session_id)
    async with session.lock:
        session.system.set_production({"producer": np.array(body.values)})
    return {"ok": True}


@router.post("/{session_id}/storage")
async def set_storage(session_id: str, body: StorageSpecIn) -> dict:
    """ENG-02: technical specs for the (single) storage component."""
    session = await _get_session(session_id)
    async with session.lock:
        system = session.system
        system.set_storage_power_cap({STORAGE: body.power_cap})
        system.set_storage_energy_cap({STORAGE: body.energy_cap})
        system.set_storage_eff({STORAGE: body.efficiency})
        system.set_storage_cycle_max({STORAGE: body.cycle_max})
        system.set_storage_energy_cap_degradation({STORAGE: body.degradation_profile})
        if body.energy_init is not None:
            system._components[STORAGE].set_energy_init(body.energy_init)
    return {"ok": True}


@router.post("/{session_id}/grid")
async def set_grid(session_id: str, body: GridSpecIn) -> dict:
    """ENG-02: power caps and (optionally) energy cost for the grid connection."""
    session = await _get_session(session_id)
    async with session.lock:
        system = session.system
        system.set_grid_power_cap({GRID: body.power_cap})
        if body.energy_cost is not None:
            export_cost, import_cost = body.energy_cost
            system.set_grid_energy_cost({GRID: (np.array(export_cost), np.array(import_cost))})
    return {"ok": True}


@router.post("/{session_id}/solve", response_model=SolveResult)
async def solve_session(session_id: str) -> SolveResult:
    """ENG-03/ENG-04: run a single-shot solve and return flows + KPIs.

    v1 only does single-shot solves (rolling horizon is v2, see ENG-06), so
    every call here is effectively a cold start: row 0 of GEKKO's results is
    the fixed initial condition for the power connections, not a real
    optimized value (see optimization/base.py notes), so it's dropped from
    the returned series rather than reported as a spurious zero.
    """
    session = await _get_session(session_id)
    async with session.lock:
        system = session.system
        try:
            await anyio.to_thread.run_sync(system.solve, False)
        except Exception as exc:
            raise HTTPException(status_code=400, detail=f"Solve failed: {exc}")

        flows = {
            name: system.results.loc[:, name].to_numpy()[1:].tolist()
            for name in (v.name for v in system._power_connections.values())
        }
        soc = battery_soc_series(system)
        if soc is not None:
            flows["storage_soc_pct"] = soc
        kpis = compute_kpis(system)

    return SolveResult(time=system.time[1:].tolist(), flows=flows, kpis=kpis)
