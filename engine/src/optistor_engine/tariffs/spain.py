"""Spain tariff model — validated 2026-07-28 (Phase 5, FIN-01/02).

Formula and every rate below were confirmed against official sources (not
guessed) after the predecessor prototype's own formula turned out to have
an unconfirmable "municipality" term and an ambiguous bracket. See
`.planning/research/tariff_spain_el_salvador.md` for the full research
trail and citations. Summary of what's confirmed:

- The "recargo municipal" is real: the "tasa por utilizacion privativa o
  aprovechamiento especial del dominio publico local" (TRLHL Art. 24.1.c,
  Real Decreto Legislativo 2/2004) — fixed by law at 1.5% of the
  distributor's gross billing revenue, passed through to the client's bill.
  The old prototype's author likely searched for the colloquial "recargo"
  rather than the legal term "tasa" and came up empty.
- The loss coefficient multiplies ONLY the wholesale (spot/pool) energy
  price, not peajes or cargos — those are fixed per unit of *metered*
  (delivered) energy, so they aren't subject to a loss adjustment.
- Tax stacking order: tasa municipal, then IEE (Impuesto Especial sobre la
  Electricidad), then IVA on top of everything including the IEE (tax on
  tax) — confirmed consistent across multiple independent sources.

NOT in scope here: the 6.3 TD tariff band's exact numeric peaje/cargo
values are NOT hardcoded — CNMC/MITECO republish these annually (2026
values were found during research but only partially cross-checked digit
by digit against the primary BOE source), and different clients sit on
different bands/voltage levels. They're required parameters here, to be
supplied per-project, not baked into this module as constants that would
silently go stale.

Also NOT in scope: time-of-use period mapping (P1-P6, which hour/month
maps to which period) — `peaje_energia`/`cargo_energia` are accepted as
single representative values per call, not per-period arrays. Splitting a
horizon's hours into P1-P6 is real, unbuilt work for whenever per-period
precision matters.

Also NOT in scope: export (excess-injection) pricing. Spain's real
"compensacion simplificada de excedentes" mechanism is a different,
separately-regulated thing from the import-side formula validated here —
`export_cost` here is a placeholder (wholesale spot price only, no
peajes/taxes), not something FIN-01's research covered or confirmed.
"""

from .base import TariffModel

DEFAULT_LOSS_COEFFICIENT = 1.05
TASA_MUNICIPAL_RATE = 0.015
IEE_RATE = 0.0511
IVA_RATE = 0.21


class SpainTariff(TariffModel):
    key = "spain"

    def compute(
        self,
        spot_price: list[float],
        *,
        peaje_energia: float,
        cargo_energia: float,
        loss_coefficient: float = DEFAULT_LOSS_COEFFICIENT,
        tasa_municipal: float = TASA_MUNICIPAL_RATE,
        iee_rate: float = IEE_RATE,
        iva_rate: float = IVA_RATE,
        **_ignored,
    ) -> tuple[list[float], list[float]]:
        """`peaje_energia`/`cargo_energia` are required — a client's actual
        contracted tariff band, not a value this module should guess or
        default (see module docstring)."""
        import_cost = []
        export_cost = []
        for spot in spot_price:
            energy_at_meter = spot * loss_coefficient
            base = energy_at_meter + peaje_energia + cargo_energia
            with_tasa = base * (1 + tasa_municipal)
            with_iee = with_tasa * (1 + iee_rate)
            final_price = with_iee * (1 + iva_rate)
            import_cost.append(final_price)
            export_cost.append(spot)

        return export_cost, import_cost
