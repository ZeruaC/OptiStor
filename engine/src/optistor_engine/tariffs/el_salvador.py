"""El Salvador tariff model — validated 2026-07-28 (Phase 5, FIN-01/02).

Confirmed against official/independent sources — see
`.planning/research/tariff_spain_el_salvador.md` for the full trail:

- The energy reference price (PEt) is set quarterly by SIGET as a weighted
  average of Mercado Regulador del Sistema (MRS, spot) and long-term
  contract prices (Reglamento de la Ley General de Electricidad, Art. 90) —
  independently confirmed via SIGET's own quarterly-adjustment pages.
- Losses are NOT a separate multiplier here (unlike Spain) — SIGET's
  distribution-charge methodology already folds recognized technical/
  non-technical losses into the approved `Cargo por Distribucion`, so this
  model doesn't apply an extra loss coefficient.
- Transmission is broken into two distinct, SIGET-regulated charges rather
  than bundled into distribution: CUST (Cargo por Uso del Sistema de
  Transmision, paid to ETESAL for grid use) and COSTAMM (the wholesale
  market operation/administration charge) — both independently confirmed
  as real, separately-tracked charges.
- IVA (13%) DOES apply: the Ley de IVA Art. 46(h) exemption is worded for
  electricity supplied by *public institutions*; El Salvador's AES-footprint
  distributors (CAESS, CLESA, EEO, DEUSEM) are privately owned, so the
  exemption doesn't extend to them.

NOT in scope here: exact current numeric values for `distribucion`/`cust`/
`costamm`/`comercializacion` are not hardcoded — SIGET republishes
distributor pliegos tarifarios annually (by the first business day of
December) and they vary by distributor/voltage level, so they're required
parameters, not module constants that would silently go stale.

Also NOT in scope: export (excess-injection) pricing — `export_cost` here
is a placeholder (the raw energy reference price, no distribution/
transmission/IVA add-ons), not something this pass's research covered.
"""

from .base import TariffModel

IVA_RATE = 0.13


class ElSalvadorTariff(TariffModel):
    key = "el_salvador"

    def compute(
        self,
        spot_price: list[float],
        *,
        distribucion: float,
        cust: float,
        costamm: float,
        comercializacion: float,
        iva_rate: float = IVA_RATE,
        **_ignored,
    ) -> tuple[list[float], list[float]]:
        """`distribucion`/`cust`/`costamm`/`comercializacion` are required —
        a distributor's actual approved pliego tarifario values, not
        something this module should guess or default (see module
        docstring). `spot_price` here represents the quarterly PEt energy
        reference, not a raw MRS tick price."""
        import_cost = []
        export_cost = []
        for energia in spot_price:
            base = energia + distribucion + cust + costamm + comercializacion
            final_price = base * (1 + iva_rate)
            import_cost.append(final_price)
            export_cost.append(energia)

        return export_cost, import_cost
