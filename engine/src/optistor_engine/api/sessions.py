"""In-memory, per-process session store for optimization runs.

Each session owns an isolated GEKKO ``Generic`` system instance. A registry
lock guards the session dict itself; a per-session lock serializes calls
against the same underlying GEKKO model, since GEKKO's model state is not
safe for concurrent mutation from multiple in-flight requests.

This is intentionally in-memory and single-process for v1 — durable,
cross-restart project storage is a Phase 2 concern (PROJ-02), not this one.
"""

import asyncio
import uuid
from dataclasses import dataclass, field

from ..optimization.systems import Generic


@dataclass
class Session:
    system: Generic
    lock: asyncio.Lock = field(default_factory=asyncio.Lock)


class SessionManager:
    """Registry of active optimization sessions, keyed by session id."""

    def __init__(self):
        self._sessions: dict[str, Session] = {}
        self._registry_lock = asyncio.Lock()

    async def create(self, system: Generic) -> str:
        session_id = uuid.uuid4().hex
        async with self._registry_lock:
            self._sessions[session_id] = Session(system=system)
        return session_id

    async def get(self, session_id: str) -> Session:
        async with self._registry_lock:
            session = self._sessions.get(session_id)
        if session is None:
            raise KeyError(session_id)
        return session

    async def delete(self, session_id: str) -> None:
        async with self._registry_lock:
            session = self._sessions.pop(session_id, None)
        if session is not None:
            try:
                session.system.cleanup()
            except Exception:
                pass
