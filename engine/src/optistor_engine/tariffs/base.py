"""Pluggable per-jurisdiction tariff models.

Each country/market's regulated tariff structure gets its own module here,
selected by that market's `tariff_model_key` (see the server's `markets`
table, Phase 5). A model takes whatever raw inputs its jurisdiction needs
(typically a wholesale spot price series plus regulatory add-ons) and
returns the effective energy cost to feed into `set_energy_cost` on the
optimization model (see `optimization/systems.py`).

Projects are free to have no market assigned at all — `engine_client.rs` on
the server side falls back to a clearly-labeled provisional flat tariff in
that case, or if the assigned market's model raises `TariffPending`.
"""

from abc import ABC, abstractmethod


class TariffPending(Exception):
    """A market is registered but its formula hasn't been validated yet.

    Not a bug — this is the honest, explicit state for a jurisdiction whose
    tariff structure a domain/finance expert hasn't confirmed (Phase 5,
    FIN-01), as opposed to silently producing a wrong number.
    """

    def __init__(self, market_key: str):
        super().__init__(
            f"El modelo de tarifa '{market_key}' esta pendiente de validacion "
            "por un experto de dominio/finanzas (Fase 5, FIN-01)."
        )
        self.market_key = market_key


class TariffModel(ABC):
    """Base interface every jurisdiction's tariff module implements."""

    key: str

    @abstractmethod
    def compute(self, spot_price: list[float], **params) -> tuple[list[float], list[float]]:
        """Returns (export_cost, import_cost) arrays, one value per time step.

        Raises `TariffPending` if this jurisdiction's formula isn't
        validated yet.
        """
        raise NotImplementedError
