"""Maps a market's `tariff_model_key` (see the server's `markets` table) to
its `TariffModel` instance.
"""

from .base import TariffModel
from .el_salvador import ElSalvadorTariff
from .spain import SpainTariff

_REGISTRY: dict[str, TariffModel] = {
    "spain": SpainTariff(),
    "el_salvador": ElSalvadorTariff(),
}


def get_tariff_model(key: str) -> TariffModel:
    try:
        return _REGISTRY[key]
    except KeyError:
        raise KeyError(f"Modelo de tarifa desconocido: '{key}'")
