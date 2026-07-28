"""Component implementations for power system optimization.

This module provides concrete implementations of power system components including
prosumers, grid connections, and storage elements. Each component inherits from
the base classes defined in base.py and implements specific behaviors and constraints.
"""

import numpy as np

from .base import PowerComponent, EnergyComponent, INF


class Prosumer(PowerComponent):
    """A component that can both produce and consume power.

    The Prosumer class represents entities in the power system that can both
    consume power (load) and produce power (generation). Examples include
    buildings with solar panels or industrial facilities with cogeneration.
    """

    def _set_power_in_out(self):
        """Configure power input/output variables and methods.

        Sets up the power flow variables and corresponding setter methods
        based on whether the component has inlets and/or outlets.
        """
        if self._n_power_inlet:
            self.power_in = self.create_gk_var("Param", "power_in", 1)
            self.set_consumption = self.__set_consumption
        else:
            self._set_power_in()

        if self._n_power_outlet:
            self.power_out = self.create_gk_var("Param", "power_out", 1)
            self.set_production = self.__set_production
        else:
            self._set_power_out()

    def __set_consumption(self, consumption_input):
        """Set the power consumption level.

        Parameters
        ----------
        consumption_input : float or array-like
            Power consumption values in appropriate units
        """
        self.set_input(self.power_in, consumption_input)

    def __set_production(self, production_input):
        """Set the power production level.

        Parameters
        ----------
        production_input : float or array-like
            Power production values in appropriate units.
            Note: Production is set as negative consumption.
        """
        self.set_input(self.power_out, -production_input)


class Grid(PowerComponent):
    """Represents a grid connection point in the power system.

    The Grid class models the interface between the local power system and the
    external electrical grid. It can handle both power import and export.
    """

    def _set_model(self):
        """Set up the grid component model.

        Initializes the base power component model without additional constraints.
        """
        super()._set_model()


class Storage(EnergyComponent):
    """Energy storage component with degradation modeling.

    The Storage class represents energy storage systems (e.g., batteries) with
    consideration of cycling degradation and state of health tracking.
    """

    def _set_model(self):
        """Set up the storage component model.

        Initializes the storage model with cycle counting and degradation tracking.
        Adds constraints for maximum cycle energy and degradation limits.
        """
        super()._set_model()

        self._cycle_energy = self.create_gk_var("Var", "cycle_energy", 1)
        self.cycles = self.create_gk_var("Param", "cycles", 1)
        self._cycle_energy_max = self.create_gk_var("Param", "cycle_energy_max", 1)

        self._m.Equations(
            [
                self._cycle_energy.dt() == (self.power_in - self.power_out) / 2,
                self._cycle_energy <= self._cycle_energy_max,
            ]
        )

        # Defaults
        self.set_energy_cap(0.0, INF)
        self.set_energy_cap_degradation({0: 1, INF: 1})

    def set_cycle_max(self, cycle_max: float):
        """Set maximum number of cycles allowed.

        Parameters
        ----------
        cycle_max : float
            Maximum number of cycles the storage can undergo
        """
        self._cycle_max = cycle_max
        self._cycle_energy_max.VALUE = self._cycle_max * self.energy.UPPER

    def set_energy_cap_degradation(self, degradation_profile: dict):
        """Set the capacity degradation profile.

        Parameters
        ----------
        degradation_profile : dict
            Dictionary mapping cycle counts to capacity retention factors (0-1)
        """
        self._degradation_profile = degradation_profile

    @property
    def _cycles(self):
        """Calculate the cumulative number of cycles.

        Returns
        -------
        numpy.ndarray
            Array of cumulative cycle counts
        """
        n_pred = self._m.time.size
        cycles = np.zeros(n_pred - 1)
        if self._m._results.size:
            c = self._m._results.loc[:, self.cycles.name].values[-(n_pred - 1):]
            cycles[-c.size:] = c

        return cycles

    @property
    def _SoH(self):
        """Calculate the current State of Health (SoH).

        Returns
        -------
        float
            State of Health as a fraction (0-1)
        """
        cycles = self._cycles[-1]
        xp, fp = list(zip(*self._degradation_profile.items()))
        SoH = np.interp(cycles, xp, fp)

        return SoH

    def shift_model(self, n_steps: int):
        """Shift the storage model forward in time.

        Parameters
        ----------
        n_steps : int
            Number of time steps to shift
        """
        super().shift_model(n_steps)

    def _pre_solve(self):
        """Pre-solve operations for storage component.

        Updates energy capacity based on current state of health and
        sets cycle energy constraints.
        """
        super()._pre_solve()

        self.set_energy_cap(self.energy.LOWER, self.energy.UPPER * self._SoH)

        cycles0 = self.get_value(self.cycles)[0]
        cycles = cycles0 - np.concatenate([self._cycles, [cycles0]])
        cycle_energy_max = (self._cycle_max - cycles) * self.energy.UPPER
        cycle_energy_max = np.clip(cycle_energy_max, 0.0, None)

        self._cycle_energy.VALUE = 0.0
        self.set_input(self._cycle_energy_max, cycle_energy_max)

    def _post_solve(self):
        """Post-solve operations for storage component.

        Updates cycle counts and adjusts energy capacity based on degradation.
        """
        super()._post_solve()

        cycle_energy = self.get_value(self._cycle_energy)
        cycles = (cycle_energy - cycle_energy[0]) / self.energy.UPPER

        self.set_input(self.cycles, cycles + self.cycles[0])

        self.set_energy_cap(self.energy.LOWER, self.energy.UPPER / self._SoH)
