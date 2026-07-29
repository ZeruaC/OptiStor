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

    /// Attempts the real per-jurisdiction tariff via the engine's stateless
    /// `/tariffs/{key}/compute`. Returns `None` on *any* failure — unknown
    /// market key, the model still being `TariffPending` (Phase 5, FIN-01),
    /// or a network error — so the caller can fall back to the provisional
    /// flat tariff without the whole solve failing over a pricing detail.
    async fn try_compute_tariff(&self, key: &str, spot_price: &[f64]) -> Option<(Vec<f64>, Vec<f64>)> {
        let body = json!({ "spot_price": spot_price, "params": {} });
        let value = self.post(&format!("/tariffs/{key}/compute"), body).await.ok()?;
        let export_cost = serde_json::from_value(value.get("export_cost")?.clone()).ok()?;
        let import_cost = serde_json::from_value(value.get("import_cost")?.clone()).ok()?;
        Some((export_cost, import_cost))
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
    ///
    /// `tariff_model_key` is the project's assigned market's tariff model
    /// (see `db::Market`), if any — `None` when no market is assigned, in
    /// which case the provisional flat tariff is used directly.
    pub async fn run_solve(
        &self,
        data: &ProjectData,
        tariff_model_key: Option<&str>,
    ) -> Result<SolveResultData, EngineError> {
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

        let result = self.configure_and_solve(&session_id, data, tariff_model_key).await;
        self.delete_session(&session_id).await;
        result
    }

    async fn configure_and_solve(
        &self,
        session_id: &str,
        data: &ProjectData,
        tariff_model_key: Option<&str>,
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

        let mut grid_body = json!({
            "power_cap": [
                data.grid.export_max.unwrap_or(0.0),
                data.grid.import_max.unwrap_or(0.0),
            ],
        });
        if data.objective.as_deref() == Some("cost") {
            let n = period.max(0) as usize;

            // Try the project's assigned market's real tariff model first.
            // Its input shape is still a placeholder (no market-price
            // collection UI exists yet — the Configurar form doesn't ask
            // for one, since we don't know what each formula needs until
            // Phase 5 confirms it) — every registered model currently
            // raises TariffPending regardless, so this always falls
            // through to the provisional flat tariff today. The plumbing
            // is real; only the formulas and their real inputs are pending.
            let real_tariff = match tariff_model_key {
                Some(key) => self.try_compute_tariff(key, &vec![0.0_f64; n]).await,
                None => None,
            };

            let (export_cost, import_cost) =
                real_tariff.unwrap_or_else(|| (vec![-0.05_f64; n], vec![0.20_f64; n]));
            grid_body["energy_cost"] = json!([export_cost, import_cost]);
        }
        self.post(&format!("/sessions/{session_id}/grid"), grid_body).await?;

        let result = self.post(&format!("/sessions/{session_id}/solve"), json!({})).await?;
        serde_json::from_value(result).map_err(|e| EngineError::Api {
            status: 500,
            detail: format!("respuesta de solve inesperada: {e}"),
        })
    }
}
