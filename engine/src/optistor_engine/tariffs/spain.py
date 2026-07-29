"""Spain (OMIE) tariff model — PENDING domain/finance validation (Phase 5, FIN-01).

The predecessor prototype (`F:\\batopt\\src\\batopt\\utils.py`) had two
mutually exclusive versions of this formula in the same file — one active,
one commented out flagged "check bracket" — differing by an extra
`+ municipality` term. Rather than guess which is correct for a formula that
feeds client-facing commercial proposals, this stays an explicit pending
stub until Balore confirms the real formula (and clarifies the `shift`
question in the old `adjust_index_tariff` too).
"""

from .base import TariffModel, TariffPending


class SpainTariff(TariffModel):
    key = "spain"

    def compute(self, spot_price: list[float], **params) -> tuple[list[float], list[float]]:
        raise TariffPending(self.key)
