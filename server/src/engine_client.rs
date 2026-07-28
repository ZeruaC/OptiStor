//! Bridges a project's stored `ProjectData` (Phase 3) into the engine's
//! session API (Phase 1) to run a solve (Phase 4, SIML-01).
//!
//! Creates a fresh engine session per solve, configures it, solves, reads
//! back the result, and tears the session down again — the engine's
//! `SessionManager` is a per-process cache of in-flight work, not a place to
//! leave sessions lying around after we're done with them.

use serde_json::{json, Value};

use crate::config::{ProjectData, SolveResultData};

pub struct EngineClient {
    base_url: String,
    client: reqwest::Client,
}

#[derive(Debug)]
pub enum EngineError {
    Http(String),
    Api { status: u16, detail: String },
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineError::Http(msg) => write!(f, "no se pudo contactar con el motor: {msg}"),
            EngineError::Api { status, detail } => write!(f, "el motor devolvio {status}: {detail}"),
        }
    }
}

/// A blank efficiency (0.0) means "never set" (see Phase 3's known form
/// limitation), not a literal zero-efficiency battery.
fn effective_eff(v: Option<f64>) -> f64 {
    match v {
        None | Some(0.0) => 1.0,
        Some(x) => x,
    }
}

impl EngineClient {
    pub fn new(base_url: String) -> Self {
        Self { base_url, client: reqwest::Client::new() }
    }

    async fn post(&self, path: &str, body: Value) -> Result<Value, EngineError> {
        let resp = self
            .client
            .post(format!("{}{}", self.base_url, path))
            .json(&body)
            .send()
            .await
            .map_err(|e| EngineError::Http(e.to_string()))?;

        let status = resp.status();
        let body: Value = resp.json().await.unwrap_or(Value::Null);

        if !status.is_success() {
            let detail = body
                .get("detail")
                .and_then(|v| v.as_str())
                .unwrap_or("error desconocido")
                .to_string();
            return Err(EngineError::Api { status: status.as_u16(), detail });
        }
        Ok(body)
    }

    async fn delete_session(&self, session_id: &str) {
        // Best-effort cleanup: a failed delete just leaks one in-memory
        // engine session until that process restarts, not worth failing
        // the whole solve over.
        let _ = self
            .client
            .delete(format!("{}/sessions/{}", self.base_url, session_id))
            .send()
            .await;
    }

    /// Runs one full Configurar -> Simular cycle: builds a fresh engine
    /// session from `data`, solves it, and returns the result. Caller is
    /// expected to have already checked `data.missing().is_empty()`.
    pub async fn run_solve(&self, data: &ProjectData) -> Result<SolveResultData, EngineError> {
        let topology = json!({
            "objective": data.objective.clone().unwrap_or_else(|| "cost".to_string()),
            "consumption": data.consumption_enabled,
            "production": data.production_enabled,
            "storage": data.storage_enabled,
            "connection_config": {
                "production_to_consumption": data.connections.production_to_consumption,
                "production_to_grid": data.connections.production_to_grid,
                "production_to_storage": data.connections.production_to_storage,
                "grid_to_consumption": data.connections.grid_to_consumption,
                "grid_to_storage": data.connections.grid_to_storage,
                "storage_to_consumption": data.connections.storage_to_consumption,
                "storage_to_grid": data.connections.storage_to_grid,
            }
        });
        let session = self.post("/sessions", topology).await?;
        let session_id = session["session_id"]
            .as_str()
            .ok_or_else(|| EngineError::Api {
                status: 500,
                detail: "el motor no devolvio session_id".to_string(),
            })?
            .to_string();

        let result = self.configure_and_solve(&session_id, data).await;
        self.delete_session(&session_id).await;
        result
    }

    async fn configure_and_solve(
        &self,
        session_id: &str,
        data: &ProjectData,
    ) -> Result<SolveResultData, EngineError> {
        let period = data.time_period.unwrap_or(2);
        let step = data.time_step.unwrap_or(1.0);

        self.post(
            &format!("/sessions/{session_id}/time"),
            json!({ "period": period, "step": step }),
        )
        .await?;

        if data.consumption_enabled {
            self.post(
                &format!("/sessions/{session_id}/consumption"),
                json!({ "values": data.consumption_values.clone().unwrap_or_default() }),
            )
            .await?;
        }

        if data.production_enabled {
            self.post(
                &format!("/sessions/{session_id}/production"),
                json!({ "values": data.production_values.clone().unwrap_or_default() }),
            )
            .await?;
        }

        if data.storage_enabled {
            self.post(
                &format!("/sessions/{session_id}/storage"),
                json!({
                    "power_cap": [
                        data.storage.charge_max.unwrap_or(0.0),
                        data.storage.discharge_max.unwrap_or(0.0),
                    ],
                    "energy_cap": [
                        data.storage.energy_min.unwrap_or(0.0),
                        data.storage.energy_max.unwrap_or(0.0),
                    ],
                    "efficiency": [
                        effective_eff(data.storage.charge_eff),
                        effective_eff(data.storage.discharge_eff),
                    ],
                    "cycle_max": data.storage.cycle_max.unwrap_or(1.0),
                    "energy_init": data.storage.energy_init,
                }),
            )
            .await?;
        }

        // Provisional flat tariff so the "cost" objective has something to
        // optimize against before Phase 5 validates the real tariff
        // formulas — see FIN-01..03. Not used at all for "energy" objective.
        let mut grid_body = json!({
            "power_cap": [
                data.grid.export_max.unwrap_or(0.0),
                data.grid.import_max.unwrap_or(0.0),
            ],
        });
        if data.objective.as_deref() == Some("cost") {
            let n = period.max(0) as usize;
            grid_body["energy_cost"] = json!([vec![-0.05_f64; n], vec![0.20_f64; n]]);
        }
        self.post(&format!("/sessions/{session_id}/grid"), grid_body).await?;

        let result = self.post(&format!("/sessions/{session_id}/solve"), json!({})).await?;
        serde_json::from_value(result).map_err(|e| EngineError::Api {
            status: 500,
            detail: format!("respuesta de solve inesperada: {e}"),
        })
    }
}
