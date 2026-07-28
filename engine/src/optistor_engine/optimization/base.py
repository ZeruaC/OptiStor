"""Base classes for power system optimization.

This module provides the foundational classes for building and optimizing power systems.
It includes base classes for system components, power components, and energy components.
"""

import os
import json
from typing import TypeVar

import numpy as np
import pandas as pd

from gekko import GEKKO
from gekko.gk_operators import GK_Operators
from gekko.gk_parameter import GKParameter, GK_FV

from .utils import HiddenPrints

INF = 1e20
CSEP = "_2_"

ParentComponent = TypeVar("ParentComponent", bound="Component")


class Base(object):
    """Base class providing common utility methods for all components.

    This class implements shared functionality for variable creation and management
    in the GEKKO optimization environment.
    """

    def create_gk_var(self, kind: str, name: str, n: int = 1, **args) -> GK_Operators:
        """Create GEKKO variable(s) with appropriate naming convention.

        Parameters
        ----------
        kind : str
            Type of GEKKO variable to create (e.g., 'Var', 'MV', 'SV', 'CV', 'FV')
        name : str
            Base name for the variable(s)
        n : int, optional
            Number of variables to create, by default 1
        **args : dict
            Additional arguments passed to GEKKO variable creation

        Returns
        -------
        GK_Operators or numpy.ndarray
            Single GEKKO variable if n=1, otherwise array of GEKKO variables
        """
        p_name = self._name
        v_name = "_".join([p_name, name])

        create = getattr(self._m, kind)

        if n == 1:
            return create(name=v_name, **args)

        return np.array([create(name=f"{v_name}{i}", **args) for i in range(n)])

    def create_gk_intermediate(self, exp, name: str) -> GK_Operators:
        """Create GEKKO intermediate variable with appropriate naming.

        Parameters
        ----------
        exp : GK_Operators
            Expression to be assigned to intermediate variable
        name : str
            Name for the intermediate variable

        Returns
        -------
        GK_Operators
            GEKKO intermediate variable
        """
        return self._m.Intermediate(exp, "_".join([self._name, name]))

    def get_value(self, var: GK_Operators | np.ndarray | int | float) -> np.ndarray:
        """Get numerical value from GEKKO variable or regular numeric type.

        Parameters
        ----------
        var : GK_Operators or numpy.ndarray or int or float
            Variable to extract value from

        Returns
        -------
        numpy.ndarray
            Flattened array of values
        """
        if hasattr(var, "VALUE"):
            val = var.VALUE.value
        else:
            val = var

        return np.array([val]).flatten()

    def set_input(self, input_var: GKParameter, newval: np.ndarray | int | float):
        """Set value of GEKKO input parameter.

        Parameters
        ----------
        input_var : GKParameter
            GEKKO parameter to set
        newval : numpy.ndarray or int or float
            New value(s) to assign
        """
        if not (type(newval) is np.ndarray):
            newval = self.get_value(newval)

        if type(input_var) is GK_FV:
            input_var.VALUE = newval[0]
        else:
            val = self.get_value(input_var)
            if newval.size == 1:
                if val.size == 1:
                    input_var.VALUE = newval[0]
                else:
                    val = np.roll(val, -1)
                    val[-1] = newval[0]
                    input_var.VALUE = val
            else:
                input_var.VALUE = newval

    def _pre_solve(self):
        """Pre-solve operations. To be implemented by subclasses."""
        pass


class System(GEKKO, Base):
    """Core system class implementing the optimization model.

    This class inherits from GEKKO to provide optimization capabilities and from Base
    to utilize common utility methods. It handles model setup, component connections,
    and solving the optimization problem.

    Parameters
    ----------
    config : list
        System configuration specifying components and their connections
    *args : tuple
        Additional arguments passed to GEKKO
    **kwargs : dict
        Additional keyword arguments passed to GEKKO
    """

    def __init__(self, config, *args, **kwargs):
        super().__init__(*args, **kwargs)

        self._config = config
        self._set_model()

        self.options.SOLVER = 3
        self.options.IMODE = 6
        self.options.EV_TYPE = 1
        self.options.NODES = 2

        self._results = pd.DataFrame()

    def _set_model(self):
        """Set up the system model based on configuration.

        Creates components and establishes power and energy connections between them.
        """
        _, sinks = zip(*self._config)

        self._model_components = {}
        for source, sink in self._config:
            c_type, c_name = source
            n_inlet = len(list(c_name for s in sinks if c_name in s))
            n_outlet = len(sink)
            component = c_type(self, c_name, n_inlet, n_outlet)
            self._model_components.update({c_name: component})

        self._power_connections = {}
        self._energy_connections = {}
        p_i_used = []
        for source, sink in self._config:
            _, so_name = source
            so = self._components[so_name]
            for si_name, p_o in zip(sink, so.power_outlet):
                si = self._components[si_name]
                for i, p_i in enumerate(si.power_inlet):
                    if p_i.name not in p_i_used:
                        break
                p_i_used += [p_i.name]

                p_o2p_i_name = f"{so_name}{CSEP}{si_name}"

                p_o2p_i = self.create_gk_var(
                    "MV",
                    f"{p_o2p_i_name}_power",
                    1,
                    lb=0,
                    fixed_initial=True,  # fixed_initial is set True for shift function correctness
                )
                p_o2p_i.STATUS = 1
                p_o2p_i.FSTATUS = 0
                self.Connection(p_o2p_i, p_i)
                self.Connection(p_o2p_i, p_o)
                self._power_connections.update({(so_name, si_name): p_o2p_i})

                e_o2e_i = self.create_gk_var("Param", f"{p_o2p_i_name}_energy", 1)
                self._energy_connections.update({(so_name, si_name): e_o2e_i})

        self._enabled_connections = {conn: True for conn in self._power_connections}

    def _enable_connections(self, enabled_connections):
        for conn, enable in enabled_connections.items():
            if conn in self._enabled_connections:
                self._enabled_connections[conn] = enable

        for conn, p_o2p_i in self._power_connections.items():
            if self._enabled_connections[conn]:
                self.free(p_o2p_i)
                self.fix_initial(p_o2p_i)
            else:
                self.fix(p_o2p_i, val=0)

        return self._enabled_connections

    @property
    def _components(self):
        return self._model_components

    @property
    def _name(self):
        return self._model_name

    @property
    def _m(self):
        return self

    def set_time(self, period, step=1, shift=1):  # step in hours (energy in kWh)
        """Set time parameters for the optimization.

        Parameters
        ----------
        period : int
            Total time period to optimize over
        step : int, optional
            Time step size in hours, by default 1
        shift : int, optional
            Number of steps to shift in rolling horizon, by default 1

        Returns
        -------
        numpy.ndarray
            Array of time points
        """
        self.time = np.arange(0, period, step)
        self._model_timestep = step
        self._model_shiftstep = shift

        return self.time

    @property
    def _timestep(self):
        return self._model_timestep

    def _save_results(self, n_steps: int):
        """Save optimization results.

        Parameters
        ----------
        n_steps : int
            Number of time steps to save
        """
        if n_steps < 1:
            n_steps = -1
        self._results = pd.concat(
            [self._results, self.results.iloc[:n_steps, :]], ignore_index=True
        )

    def shift_model(self):
        """Shift the model forward in time for rolling horizon optimization."""
        self._save_results(self._model_shiftstep)

        for component in self._components.values():
            component.shift_model(self._model_shiftstep)

    def _load_results_json(self):
        data = {}
        if os.path.isfile(os.path.join(self._path, "results.json")):
            with open(os.path.join(self._path, "results.json")) as f:
                data = json.load(f)
        else:
            print("Error: 'results.json' not found. Check above for additional error details")

        return pd.DataFrame.from_dict(data).iloc[:, :]

    def _load_results_csv(self):
        file_name = self._name + ".csv"
        file_path = os.path.join(self._path, file_name)
        if os.path.isfile(file_path):
            data = pd.read_csv(file_path)
        else:
            print(f"Error: '{file_name}' not found. Check above for additional error details")
            data = pd.DataFrame.from_dict({})

        return data.iloc[:, :]

    def solve(self, disp: bool = True, *args, **kwargs):
        """Solve the optimization problem.

        Parameters
        ----------
        disp : bool, optional
            Display solver output, by default True
        *args : tuple
            Additional arguments passed to GEKKO solver
        **kwargs : dict
            Additional keyword arguments passed to GEKKO solver
        """
        self._pre_solve()
        for c in self._components.values():
            c._pre_solve()

        if disp:
            super().solve(disp=disp, *args, **kwargs)
        else:
            with HiddenPrints():
                super().solve(disp=disp, *args, **kwargs)
        self.results = self._load_results_json()

        for k, v in self._energy_connections.items():
            p_v = self._power_connections[k]
            self.set_input(
                v, (np.cumsum(p_v) - p_v.VALUE[0]) * self._timestep + v.VALUE[0]
            )

        for c in self._components.values():
            c._post_solve()

        self._m._write_csv()

        results_csv = self._load_results_csv()
        self.results.loc[:, results_csv.columns] = results_csv.values


class Component(Base):
    """Base class for all system components.

    Parameters
    ----------
    parent : System or Component
        Parent system or component
    name : str
        Component name
    """

    def __init__(self, parent: type[System] | type[ParentComponent], name: str):
        self._parent_model = parent._m

        p_name = parent._name
        c_name = name.lower().replace(" ", "")
        self._component_name = "_".join([p_name, c_name])

        self._set_model()

    def _set_model(self):
        """Set up component model. To be implemented by subclasses."""
        pass

    def _post_solve(self):
        """Post-solve operations. To be implemented by subclasses."""
        pass

    def shift_model(self, n_steps: int):
        """Shift component model forward in time.

        Parameters
        ----------
        n_steps : int
            Number of time steps to shift
        """
        results = self._m._load_results_csv()
        for var in self._variables + self._parameters:
            if var.name in results.columns:
                v = self.get_value(var)
                v[:n_steps] = v[-1]
                var.VALUE = np.roll(v, -n_steps)

    @property
    def _name(self):
        return self._component_name

    @property
    def _m(self):
        return self._parent_model

    @property
    def _parameters(self):
        return list(p for p in self._m._parameters if p.name.startswith(self._name))

    @property
    def _variables(self):
        return list(v for v in self._m._variables if v.name.startswith(self._name))


class PowerComponent(Component):
    """Base class for components with power flows.

    Parameters
    ----------
    parent : System or Component
        Parent system or component
    name : str
        Component name
    n_power_inlet : int
        Number of power inlets
    n_power_outlet : int
        Number of power outlets
    """

    def __init__(
        self,
        parent: type[System] | type[Component],
        name: str,
        n_power_inlet: int,
        n_power_outlet: int,
    ):
        self._n_power_inlet = max(n_power_inlet, 0)
        self._n_power_outlet = max(n_power_outlet, 0)

        super().__init__(parent, name)

    def _set_model(self):
        """Set up power component model with inlets and outlets."""
        super()._set_model()

        self.power_inlet = np.array(
            self.create_gk_var("Var", "power_inlet", self._n_power_inlet)
        ).flatten()
        self.power_outlet = np.array(
            self.create_gk_var("Var", "power_outlet", self._n_power_outlet)
        ).flatten()

        self._set_power_in_out()

        self._m.Equations(
            [
                self.power_in == self._m.sum(self.power_inlet),
                self.power_out == -self._m.sum(self.power_outlet),
            ]
        )

    def _set_power_in(self):
        self.power_in = self.create_gk_var("Var", "power_in", 1)

    def _set_power_out(self):
        self.power_out = self.create_gk_var("Var", "power_out", 1)

    def _set_power_in_out(self):
        self._set_power_in()
        self._set_power_out()

    def set_power_cap(self, power_max_in: float, power_max_out: float):
        """Set power capacity constraints.

        Parameters
        ----------
        power_max_in : float
            Maximum input power
        power_max_out : float
            Maximum output power
        """
        self.power_in.UPPER = power_max_in
        self.power_out.LOWER = -power_max_out


class EnergyComponent(PowerComponent):
    """Base class for components with energy storage.

    Inherits from PowerComponent and adds energy-related variables and constraints.
    """

    def _set_model(self):
        """Set up energy component model."""
        super()._set_model()

        self.energy = self.create_gk_var("Var", "energy", 1)
        self.eff_in = self.create_gk_var("Param", "eff_in", 1)
        self.eff_out = self.create_gk_var("Param", "eff_out", 1)

        self._m.Equations(
            [self.energy.dt() == self.power_in * self.eff_in + self.power_out / self.eff_out]
        )

        # Defaults
        self.set_eff(1.0, 1.0)
        self.set_energy_cap(0.0, self.energy.UPPER)
        self.set_energy_init(0.0)

    def shift_model(self, n_steps: int):
        """Shift the component and preserve the solved energy state.

        The energy reached at ``n_steps`` in the solved horizon becomes
        the fixed initial energy of the following MPC window.
        """
        solved_results = self._m.results

        if self.energy.name not in solved_results.columns:
            available_energy_columns = [
                column for column in solved_results.columns if "energy" in column.lower()
            ]
            raise KeyError(
                f"'{self.energy.name}' not found in self._m.results. "
                f"Energy-related columns available: {available_energy_columns}"
            )

        solved_energy = solved_results[self.energy.name].astype(float).to_numpy()

        if n_steps < 1 or n_steps >= len(solved_energy):
            raise ValueError(
                f"n_steps={n_steps} is not valid for a window of {len(solved_energy)} points."
            )

        super().shift_model(n_steps)

        next_initial_energy = float(solved_energy[n_steps])

        shifted_energy = np.concatenate(
            [
                solved_energy[n_steps:],
                np.full(n_steps, solved_energy[-1], dtype=float),
            ]
        )

        self.energy.VALUE = shifted_energy

        self._m.free_initial(self.energy)
        self._m.fix_initial(self.energy, next_initial_energy)

    def set_eff(self, eff_in: float, eff_out: float):
        """Set energy conversion efficiencies.

        Parameters
        ----------
        eff_in : float
            Input (charging) efficiency
        eff_out : float
            Output (discharging) efficiency
        """
        self.set_input(self.eff_in, eff_in)
        self.set_input(self.eff_out, eff_out)

    def set_energy_cap(self, energy_min: float, energy_max: float):
        """Set energy capacity constraints.

        Parameters
        ----------
        energy_min : float
            Minimum energy level
        energy_max : float
            Maximum energy level
        """
        self.energy.LOWER = energy_min
        self.energy.UPPER = energy_max

    def set_energy_init(self, energy_init: float):
        """Set initial energy level.

        Parameters
        ----------
        energy_init : float
            Initial energy level
        """
        self.energy.VALUE = energy_init
