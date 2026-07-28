//! The shape of `projects.data` (see `db.rs`): system topology + input data
//! for one project, plus its most recent solve result. Field names in
//! `ProjectData` deliberately mirror the pydantic schemas in
//! `engine/src/optistor_engine/api/schemas.py` so `engine_client.rs` can
//! forward it straight into the engine's session endpoints with minimal
//! translation.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The full stored record for a project: its Configurar-phase config, plus
/// whatever the last Simular-phase solve produced (if any).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectRecord {
    #[serde(default)]
    pub config: ProjectData,
    #[serde(default)]
    pub last_solve: Option<SolveResultData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolveResultData {
    pub time: Vec<f64>,
    pub flows: BTreeMap<String, Vec<f64>>,
    pub kpis: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfigData {
    pub production_to_consumption: bool,
    pub production_to_grid: bool,
    pub production_to_storage: bool,
    pub grid_to_consumption: bool,
    pub grid_to_storage: bool,
    pub storage_to_consumption: bool,
    pub storage_to_grid: bool,
}

impl Default for ConnectionConfigData {
    fn default() -> Self {
        // Matches optimization/systems.py's ConnectionConfig defaults: everything enabled.
        Self {
            production_to_consumption: true,
            production_to_grid: true,
            production_to_storage: true,
            grid_to_consumption: true,
            grid_to_storage: true,
            storage_to_consumption: true,
            storage_to_grid: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StorageSpecData {
    pub charge_max: Option<f64>,
    pub discharge_max: Option<f64>,
    pub energy_min: Option<f64>,
    pub energy_max: Option<f64>,
    pub charge_eff: Option<f64>,
    pub discharge_eff: Option<f64>,
    pub cycle_max: Option<f64>,
    pub energy_init: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GridSpecData {
    pub export_max: Option<f64>,
    pub import_max: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectData {
    #[serde(default)]
    pub objective: Option<String>, // "energy" | "cost"
    #[serde(default = "default_true")]
    pub consumption_enabled: bool,
    #[serde(default = "default_true")]
    pub production_enabled: bool,
    #[serde(default = "default_true")]
    pub storage_enabled: bool,
    #[serde(default)]
    pub connections: ConnectionConfigData,
    #[serde(default)]
    pub time_period: Option<i64>,
    #[serde(default)]
    pub time_step: Option<f64>,
    #[serde(default)]
    pub consumption_values: Option<Vec<f64>>,
    #[serde(default)]
    pub production_values: Option<Vec<f64>>,
    #[serde(default)]
    pub storage: StorageSpecData,
    #[serde(default)]
    pub grid: GridSpecData,
}

fn default_true() -> bool {
    true
}

impl Default for ProjectData {
    fn default() -> Self {
        Self {
            objective: None,
            consumption_enabled: true,
            production_enabled: true,
            storage_enabled: true,
            connections: ConnectionConfigData::default(),
            time_period: None,
            time_step: None,
            consumption_values: None,
            production_values: None,
            storage: StorageSpecData::default(),
            grid: GridSpecData::default(),
        }
    }
}

impl ProjectData {
    /// CONF-05: what's missing before this configuration could be solved.
    /// Empty result means the configuration is complete.
    pub fn missing(&self) -> Vec<String> {
        let mut missing = Vec::new();

        if self.objective.is_none() {
            missing.push("Elige un objetivo de optimizacion (energia o coste).".to_string());
        }
        if self.time_period.is_none() || self.time_step.is_none() {
            missing.push("Define el horizonte temporal (periodo y paso).".to_string());
        }
        if self.consumption_enabled
            && self.consumption_values.as_ref().is_none_or(|v| v.is_empty())
        {
            missing.push("Falta el perfil de consumo.".to_string());
        }
        if self.production_enabled
            && self.production_values.as_ref().is_none_or(|v| v.is_empty())
        {
            missing.push("Falta el perfil de produccion.".to_string());
        }
        if self.storage_enabled {
            if self.storage.charge_max.is_none() || self.storage.discharge_max.is_none() {
                missing.push("Falta la potencia de carga/descarga de la bateria.".to_string());
            }
            if self.storage.energy_max.is_none() {
                missing.push("Falta la capacidad de energia maxima de la bateria.".to_string());
            }
            if self.storage.cycle_max.is_none() {
                missing.push("Falta el maximo de ciclos de la bateria.".to_string());
            }
        }
        if self.grid.export_max.is_none() || self.grid.import_max.is_none() {
            missing.push("Falta la potencia maxima de importacion/exportacion de red.".to_string());
        }

        missing
    }

    /// Renders a stored profile as a comma-separated string for a textarea's
    /// prefilled value (the inverse of `parse_series`).
    pub fn consumption_text(&self) -> String {
        series_text(&self.consumption_values)
    }

    pub fn production_text(&self) -> String {
        series_text(&self.production_values)
    }
}

fn series_text(values: &Option<Vec<f64>>) -> String {
    values
        .as_ref()
        .map(|v| v.iter().map(f64::to_string).collect::<Vec<_>>().join(", "))
        .unwrap_or_default()
}

/// Parses a textarea of comma/newline/space-separated numbers into a `Vec<f64>`,
/// skipping blank entries. Used for manual profile entry (CONF-04).
pub fn parse_series(raw: &str) -> Result<Vec<f64>, String> {
    raw.split(|c: char| c == ',' || c.is_whitespace())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.parse::<f64>().map_err(|_| format!("'{s}' no es un numero valido")))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_series_handles_commas_newlines_and_blank_entries() {
        assert_eq!(parse_series("1, 2,3\n4  5").unwrap(), vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(parse_series("").unwrap(), Vec::<f64>::new());
        assert_eq!(parse_series("  ").unwrap(), Vec::<f64>::new());
    }

    #[test]
    fn parse_series_rejects_non_numeric_entries() {
        assert!(parse_series("1, abc, 3").is_err());
    }

    #[test]
    fn fresh_project_is_missing_everything() {
        let data = ProjectData::default();
        let missing = data.missing();
        // objective, time, consumption, production, 3 storage items, grid = 8
        assert_eq!(missing.len(), 8);
    }

    #[test]
    fn disabling_a_component_removes_its_requirement() {
        let mut data = ProjectData {
            objective: Some("cost".to_string()),
            time_period: Some(7),
            time_step: Some(1.0),
            grid: GridSpecData { export_max: Some(200.0), import_max: Some(200.0) },
            ..ProjectData::default()
        };
        data.consumption_enabled = false;
        data.production_enabled = false;
        data.storage_enabled = false;

        assert!(data.missing().is_empty());
    }

    #[test]
    fn complete_configuration_has_nothing_missing() {
        let data = ProjectData {
            objective: Some("cost".to_string()),
            consumption_enabled: true,
            production_enabled: true,
            storage_enabled: true,
            connections: ConnectionConfigData::default(),
            time_period: Some(7),
            time_step: Some(1.0),
            consumption_values: Some(vec![100.0, 120.0]),
            production_values: Some(vec![0.0, 10.0]),
            storage: StorageSpecData {
                charge_max: Some(50.0),
                discharge_max: Some(50.0),
                energy_max: Some(100.0),
                cycle_max: Some(1.0),
                ..StorageSpecData::default()
            },
            grid: GridSpecData { export_max: Some(200.0), import_max: Some(200.0) },
        };

        assert!(data.missing().is_empty());
    }
}
