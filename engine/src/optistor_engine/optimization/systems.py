"""System configurations for power system optimization.

This module provides pre-configured system architectures for different optimization
scenarios. It includes base configurations and specialized variants for energy and
cost optimization.
"""

from dataclasses import dataclass

from .base import System, PowerComponent, INF, CSEP
from .components import Prosumer, Grid, Storage

CONSUMER = "consumer"
GRID = "grid"
PRODUCER = "producer"
STORAGE = "storage"
BURNER = "burner"
BACKUP = "backup"


@dataclass
class ConnectionConfig:
    production_to_consumption: bool = True
    production_to_grid: bool = True
    production_to_storage: bool = True
    grid_to_consumption: bool = True
    grid_to_storage: bool = True
    storage_to_consumption: bool = True
    storage_to_grid: bool = True


class Generic(System):
    """Base class for generic power system configurations.

    Implements a flexible power system architecture that can include multiple
    consumers, producers, storage units, grid connections, backup power, and burners.

    Parameters
    ----------
    n_consumers : int, optional
        Number of consumer components, by default 1
    n_producers : int, optional
        Number of producer components, by default 1
    n_storages : int, optional
        Number of storage components, by default 1
    connection_config : ConnectionConfig, optional
        Configuration defining allowed connections between components
    peak_allowed : bool, optional
        Allow exceeding peak power limits, by default False
    *args : tuple
        Additional arguments passed to System
    **kwargs : dict
        Additional keyword arguments passed to System
    """

    def __init__(
        self,
        n_consumers: int = 1,
        n_producers: int = 1,
        n_storages: int = 1,
        connection_config: ConnectionConfig = ConnectionConfig(),
        peak_allowed: bool = False,
        *args,
        **kwargs,
    ):
        consumers = (
            [CONSUMER] if n_consumers < 2 else [f"{CONSUMER}{i}" for i in range(n_consumers)]
        )
        grids = [GRID]
        storages = (
            [STORAGE] if n_storages < 2 else [f"{STORAGE}{i}" for i in range(n_storages)]
        )
        producers = (
            [PRODUCER] if n_producers < 2 else [f"{PRODUCER}{i}" for i in range(n_producers)]
        )
        backups = [BACKUP]
        burners = [BURNER]

        from_consumers = []
        config = [[(Prosumer, c), from_consumers] for c in consumers]

        from_grids = consumers + storages
        config += [[(Grid, g), from_grids] for g in grids]

        from_storages = consumers + grids
        config += [[(Storage, s), from_storages] for s in storages]

        from_producers = consumers + storages + grids + burners
        config += [[(Prosumer, p), from_producers] for p in producers]

        from_burners = []
        config += [[(PowerComponent, b), from_burners] for b in burners]

        from_backups = consumers
        config += [[(PowerComponent, b), from_backups] for b in backups]

        self._consumers = consumers
        self._grids = grids
        self._storages = storages
        self._producers = producers

        super().__init__(config, *args, **kwargs)

        self._set_scenario(
            n_consumers=n_consumers,
            n_producers=n_producers,
            n_storages=n_storages,
            connection_config=connection_config,
            peak_allowed=peak_allowed,
        )

        self._power_max = {}
        self._power_ref = {}
        self._power_dev = {}
        self._energy_cost = {}
        self._energy_cost_rate = {}

    def _pre_solve(self):
        super()._pre_solve()

        self._objectives = []
        self._set_objectives()

    def _set_scenario(
        self,
        n_consumers: int,
        n_producers: int,
        n_storages: int,
        connection_config: ConnectionConfig,
        peak_allowed: bool,
    ):
        """Set scenario-specific constraints.

        Parameters
        ----------
        n_consumers : int
            Number of consumers in the system
        n_producers : int
            Number of producers in the system
        n_storages : int
            Number of storage units in the system
        connection_config : ConnectionConfig
            Configuration object defining allowed connections between components
        peak_allowed : bool
            Allow exceeding peak power limits
        """
        enabled_connections = {}
        for (so, si), _ in self._power_connections.items():
            if (PRODUCER in so) and (CONSUMER in si):
                enabled_connections.update({(so, si): connection_config.production_to_consumption})
            if (PRODUCER in so) and (GRID in si):
                enabled_connections.update({(so, si): connection_config.production_to_grid})
            if (PRODUCER in so) and (STORAGE in si):
                enabled_connections.update({(so, si): connection_config.production_to_storage})
            if (GRID in so) and (CONSUMER in si):
                enabled_connections.update({(so, si): connection_config.grid_to_consumption})
            if (GRID in so) and (STORAGE in si):
                enabled_connections.update({(so, si): connection_config.grid_to_storage})
            if (STORAGE in so) and (CONSUMER in si):
                enabled_connections.update({(so, si): connection_config.storage_to_consumption})
            if (STORAGE in so) and (GRID in si):
                enabled_connections.update({(so, si): connection_config.storage_to_grid})

        consumers = n_consumers > 0
        producers = n_producers > 0
        storages = n_storages > 0
        for (so, si), v in self._power_connections.items():
            if not consumers:
                if (CONSUMER in so) or (CONSUMER in si):
                    if not (BACKUP in so):
                        enabled_connections.update({(so, si): False})
            if not producers:
                if (PRODUCER in so) or (PRODUCER in si):
                    if not (BURNER in si):
                        enabled_connections.update({(so, si): False})
            if not storages:
                if (STORAGE in so) or (STORAGE in si):
                    enabled_connections.update({(so, si): False})

        self._enable_connections(enabled_connections)

        if peak_allowed:
            self.set_grid_power_cap = self.set_grid_power_ref
            self.set_grid_power_max = self.set_grid_power_ref
        else:
            self.set_grid_power_cap = self._set_grid_power_cap
            self.set_grid_power_max = self._set_grid_power_max

    def _set_objectives(self):
        """Set additional optimization objectives.

        Adds penalty terms for backup power usage and power burning to discourage
        their use in the optimization.
        """
        for k, v in self._power_connections.items():
            if BURNER in k[1]:
                self.Minimize(1000 * v)
            if BACKUP in k[0]:
                self.Minimize(1000 * v)

    def set_power_max(self, power_max: dict):
        """Set maximum power constraints for connections.

        Parameters
        ----------
        power_max : dict
            Dictionary mapping connection tuples to maximum power values.
            Keys can be (origin, destination) tuples for specific connections,
            or (component, None) / (None, component) for aggregated flows.
        """
        for conn, power in power_max.items():
            if conn not in self._power_max:
                if all(conn):
                    name = CSEP.join(conn) + "_power_max"
                    p_max = self.create_gk_var("Param", name, 1)
                    p_o2p_i = self._power_connections[conn]
                    self.Equation(p_o2p_i <= p_max)

                elif any(conn):
                    c = 0 if conn[0] else 1
                    in_out = "in" if c else "out"
                    name = conn[c] + f"_power_{in_out}_max"
                    p_max = self.create_gk_var("Param", name, 1)
                    powers = list(
                        p for _conn, p in self._power_connections.items() if conn[c] == _conn[c]
                    )
                    self.Equation(self.sum(powers) <= p_max)

                self._power_max.update({conn: p_max})

            self.set_input(self._power_max[conn], power)

    def set_power_ref(self, power_ref: dict):
        """Set reference power for connections.

        Creates reference power parameters and corresponding deviation intermediates
        used to measure the difference between actual power flows and target values.

        Parameters
        ----------
        power_ref : dict
            Dictionary mapping connection tuples to reference power values.
            Keys can be (origin, destination) tuples for specific connections,
            or (component, None) / (None, component) for aggregated flows.
        """
        for conn, power in power_ref.items():
            if conn not in self._power_ref:
                if all(conn):
                    cname = CSEP.join(conn)
                    p_ref = self.create_gk_var("Param", f"{cname}_power_ref", 1)
                    p_o2p_i = self._power_connections[conn]
                    p_dev = self.create_gk_intermediate(p_o2p_i - p_ref, f"{cname}_power_dev")

                elif any(conn):
                    c = 0 if conn[0] else 1
                    in_out = "in" if c else "out"
                    cname = conn[c]
                    name = f"{cname}_power_{in_out}_ref"
                    p_ref = self.create_gk_var("Param", name, 1)
                    powers = list(
                        p for _conn, p in self._power_connections.items() if conn[c] == _conn[c]
                    )
                    p_dev = self.create_gk_intermediate(
                        self.sum(powers) - p_ref, f"{cname}_power_{in_out}_dev"
                    )

                self._power_ref.update({conn: p_ref})
                self._power_dev.update({conn: p_dev})

            self.set_input(self._power_ref[conn], power)

    def set_energy_cost(self, energy_cost: dict):
        """Set energy cost for connections.

        Creates energy cost parameters and cost rate intermediates for economic
        optimization. Cost rates are computed as power x cost.

        Parameters
        ----------
        energy_cost : dict
            Dictionary mapping connection tuples to energy cost values (e.g., $/kWh).
            Keys can be (origin, destination) tuples for specific connections,
            or (component, None) / (None, component) for aggregated flows.
        """
        for conn, power in energy_cost.items():
            if conn not in self._energy_cost:
                if all(conn):
                    cname = CSEP.join(conn)
                    e_cost = self.create_gk_var("Param", f"{cname}_energy_cost", 1)
                    p_o2p_i = self._power_connections[conn]
                    e_cost_rate = self.create_gk_intermediate(
                        p_o2p_i * e_cost, f"{cname}_energy_cost_rate"
                    )

                elif any(conn):
                    c = 0 if conn[0] else 1
                    in_out = "in" if c else "out"
                    cname = conn[c]
                    name = f"{cname}_energy_{in_out}_cost"
                    e_cost = self.create_gk_var("Param", name, 1)
                    powers = list(
                        p for _conn, p in self._power_connections.items() if conn[c] == _conn[c]
                    )
                    e_cost_rate = self.create_gk_intermediate(
                        self.sum(powers) * e_cost, f"{cname}_energy_{in_out}_cost_rate"
                    )

                self._energy_cost.update({conn: e_cost})
                self._energy_cost_rate.update({conn: e_cost_rate})

            self.set_input(self._energy_cost[conn], power)

    def set_consumption(self, consumption: dict):
        """Set consumption profiles for consumer components.

        Parameters
        ----------
        consumption : dict
            Dictionary mapping consumer names to consumption power profiles (arrays).
        """
        for c in self._consumers:
            self._components[c].set_consumption(consumption[c])

    def _set_grid_power_cap(self, power_cap: dict):
        """Set power capacity constraints for grid connections.

        Parameters
        ----------
        power_cap : dict
            Dictionary mapping grid names to (export_max, import_max) tuples in kW.
        """
        for g in self._grids:
            self._components[g].set_power_cap(*power_cap[g])

    def _set_grid_power_max(self, power_max: dict):
        """Set maximum power constraints for grid connections.

        Alternative method to set grid power limits using the generic set_power_max
        interface. (None, grid) corresponds to inbound/import limits and (grid, None)
        corresponds to outbound/export limits.

        Parameters
        ----------
        power_max : dict
            Dictionary mapping grid names to (export_max, import_max) tuples
        """
        for c in self._grids:
            if c in power_max:
                _power_max = {
                    (None, c): power_max[c][0],
                    (c, None): power_max[c][-1],
                }
                self.set_power_max(_power_max)

    def set_grid_power_ref(self, power_ref: dict):
        """Set reference power for grid connections.

        Parameters
        ----------
        power_ref : dict
            Dictionary mapping grid names to (export_ref, import_ref) tuples
        """
        for c in self._grids:
            if c in power_ref:
                _power_ref = {
                    (None, c): power_ref[c][0],
                    (c, None): power_ref[c][-1],
                }
                self.set_power_ref(_power_ref)

    def set_grid_energy_cost(self, power_cost: dict):
        """Set energy cost for grid connections.

        Parameters
        ----------
        power_cost : dict
            Dictionary mapping grid names to (export_cost, import_cost) tuples
        """
        for c in self._grids:
            if c in power_cost:
                _power_cost = {
                    (None, c): power_cost[c][0],
                    (c, None): power_cost[c][-1],
                }
                self.set_energy_cost(_power_cost)

    def _free_grid_power_constraint(self):
        """Remove power capacity constraints for grid connections.

        Sets grid power capacity to infinity, effectively removing power limits.
        """
        for g in self._grids:
            self._components[g].set_power_cap(INF, INF)

    def set_storage_power_cap(self, power_cap: dict):
        """Set power capacity constraints for storage components.

        Parameters
        ----------
        power_cap : dict
            Dictionary mapping storage names to (charge_max, discharge_max) tuples in kW.
        """
        for c in self._storages:
            self._components[c].set_power_cap(*power_cap[c])

    def set_storage_power_ref(self, power_ref: dict):
        """Set reference power for storage connections.

        Parameters
        ----------
        power_ref : dict
            Dictionary mapping storage names to (charge_ref, discharge_ref) tuples
        """
        for c in self._storages:
            if c in power_ref:
                _power_ref = {
                    (None, c): power_ref[c][0],
                    (c, None): power_ref[c][-1],
                }
                self.set_power_ref(_power_ref)

    def set_storage_eff(self, efficiency: dict):
        """Set efficiency values for storage components.

        Parameters
        ----------
        efficiency : dict
            Dictionary mapping storage names to (charge_eff, discharge_eff) tuples
        """
        for c in self._storages:
            self._components[c].set_eff(*efficiency[c])

    def set_storage_energy_cap(self, energy_cap: dict):
        """Set energy capacity constraints for storage components.

        Parameters
        ----------
        energy_cap : dict
            Dictionary mapping storage names to (min_energy, max_energy) tuples
        """
        for c in self._storages:
            self._components[c].set_energy_cap(*energy_cap[c])

    def set_storage_energy_cap_degradation(self, degradation_profile: dict):
        """Set energy capacity degradation profiles for storage components.

        Parameters
        ----------
        degradation_profile : dict
            Dictionary mapping storage names to degradation profile dictionaries
        """
        for c in self._storages:
            self._components[c].set_energy_cap_degradation(degradation_profile[c])

    def set_storage_cycle_max(self, cycle_max: dict):
        """Set maximum cycle limits for storage components.

        Parameters
        ----------
        cycle_max : dict
            Dictionary mapping storage names to maximum cycle counts
        """
        for c in self._storages:
            self._components[c].set_cycle_max(cycle_max[c])

    def set_production(self, production: dict):
        """Set production profiles for producer components.

        Parameters
        ----------
        production : dict
            Dictionary mapping producer names to production profiles
        """
        for c in self._producers:
            self._components[c].set_production(production[c])


class GenericPeakOpt(Generic):
    """Energy optimization variant of generic system.

    Extends Generic system with objective function focused on peak power minimization.
    """

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)

        self._power_peak = {}

    def _set_objectives(self):
        """Extend model with peak-minimization objectives."""
        super()._set_objectives()

        for k, v in self._power_dev.items():
            if (k[1] is None) and (GRID in k[0]):
                if k not in self._power_peak:
                    name = k[0] + "_power_in_peak"
                    self._power_peak[k] = self.create_gk_var("Var", name, 1, lb=0)
                    self._m.Equations([self._power_peak[k] >= v])
                self.Minimize(self._power_peak[k])

            if (k[0] is None) and (GRID in k[1]):
                if k not in self._power_peak:
                    name = k[1] + "_power_out_peak"
                    self._power_peak[k] = self.create_gk_var("Var", name, 1, lb=0)
                    self._m.Equations([self._power_peak[k] >= v])
                self.Minimize(self._power_peak[k])


class GenericGridTrack(Generic):
    """Grid tracking variant of generic system.

    Extends Generic system with objective function focused on grid power tracking.
    """

    obj_target = "grid_tracking"

    def _set_objectives(self):
        """Extend model with grid-tracking objectives."""
        super()._set_objectives()

        for k, v in self._power_dev.items():
            if (k[1] is None) and (GRID in k[0]):
                self.Minimize(v**2)

            if (k[0] is None) and (GRID in k[1]):
                self.Minimize(v**2)


class GenericBatteryTrack(Generic):
    """Battery tracking variant of generic system.

    Extends Generic system with objective function focused on battery state tracking.
    """

    obj_target = "battery_tracking"

    def _set_objectives(self):
        """Extend model with battery-tracking objectives."""
        super()._set_objectives()

        for k, v in self._power_dev.items():
            if (k[1] is None) and (STORAGE in k[0]):
                self.Minimize(v**2)

            if (k[0] is None) and (STORAGE in k[1]):
                self.Minimize(v**2)


class GenericEnergyOpt(Generic):
    """Energy optimization variant of generic system.

    Extends Generic system with objective function focused on energy optimization.
    Maximizes self-consumption and minimizes grid dependence.
    """

    obj_target = "energy"

    def _set_objectives(self):
        """Extend model with self-consumption maximization objectives."""
        super()._set_objectives()

        for k, v in self._power_connections.items():
            if GRID in k[0]:
                self.Minimize(v)

            if (PRODUCER in k[0]) and (CONSUMER in k[1]):
                self.Maximize(v)


class GenericCostOpt(Generic):
    """Cost optimization variant of generic system.

    Extends Generic system with objective function focused on cost optimization.
    Minimizes total operational costs considering energy prices.
    """

    obj_target = "cost"

    def _set_objectives(self):
        """Extend model with cost-minimization objectives."""
        super()._set_objectives()

        for k, v in self._energy_cost_rate.items():
            if (k[1] is None) and (GRID in k[0]):
                self.Minimize(v)

            if (k[0] is None) and (GRID in k[1]):
                self.Minimize(v)


class StorageProducerGridConsumer(object):
    """Factory class for creating power system configurations.

    Creates an optimized system based on objective and configuration parameters.

    Parameters
    ----------
    objective : str, optional
        Optimization objective ('energy' or 'cost'), by default 'energy'
    peak_minimization : bool, optional
        Enable peak power minimization, by default False
    consumption : bool, optional
        Include consumer component, by default True
    production : bool, optional
        Include producer component, by default True
    storage : bool, optional
        Include storage component, by default True
    connection_config : ConnectionConfig, optional
        Configuration for component connections, by default ConnectionConfig()
    peak_allowed : bool, optional
        Allow peak power usage, by default False
    *args : tuple
        Additional arguments passed to system class
    **kwargs : dict
        Additional keyword arguments passed to system class
    """

    def __new__(
        cls,
        objective: str = "energy",
        peak_minimization: bool = False,
        consumption: bool = True,
        production: bool = True,
        storage: bool = True,
        connection_config: ConnectionConfig = ConnectionConfig(),
        peak_allowed: bool = False,
        *args,
        **kwargs,
    ):
        match objective:
            case "energy":
                if peak_minimization:

                    class Model(GenericPeakOpt, GenericEnergyOpt):
                        pass
                else:

                    class Model(GenericEnergyOpt):
                        pass
            case "cost":
                if peak_minimization:

                    class Model(GenericPeakOpt, GenericCostOpt):
                        pass
                else:

                    class Model(GenericCostOpt):
                        pass
            case _:
                raise ValueError(f"Unknown objective: {objective!r}. Expected 'energy' or 'cost'.")

        return Model(
            n_consumers=int(consumption),
            n_producers=int(production),
            n_storages=int(storage),
            connection_config=connection_config,
            peak_allowed=peak_allowed,
            *args,
            **kwargs,
        )
