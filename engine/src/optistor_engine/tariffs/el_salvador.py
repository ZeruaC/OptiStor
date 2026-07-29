"""El Salvador tariff model — PENDING domain/finance validation (Phase 5, FIN-01).

El Salvador trades on the regional Central American market (MER, coordinated
by the EOR/CRIE) but does not share a single unified price with its
neighbors — April 2026 data showed El Salvador at 75-146 USD/MWh against
Nicaragua's 156-178 USD/MWh in the same month, so this stays its own market
entity rather than a shared "Central America" one. The actual regulated
tariff structure (transmission/distribution add-ons, taxes) hasn't been
provided yet.
"""

from .base import TariffModel, TariffPending


class ElSalvadorTariff(TariffModel):
    key = "el_salvador"

    def compute(self, spot_price: list[float], **params) -> tuple[list[float], list[float]]:
        raise TariffPending(self.key)
