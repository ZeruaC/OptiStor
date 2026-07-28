//! Server-rendered HTML UI (Askama + HTMX) for Phase 3 (Configurar) and
//! Phase 4 (Simular & Dashboard).
//!
//! Deliberately kept separate from the JSON API in `projects.rs`: HTML forms
//! post `application/x-www-form-urlencoded`, not JSON, so rather than making
//! one route branch on Content-Type, the UI gets its own `/app/...` routes
//! that call straight into the same `db` functions.

use std::collections::BTreeMap;

use askama::Template;
use axum::extract::{Form, Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::get;
use axum::Router;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::{AuthUser, Role};
use crate::config::{
    ConnectionConfigData, GridSpecData, ProjectData, ProjectRecord, SolveResultData,
    StorageSpecData,
};
use crate::error::ApiError;
use crate::projects::scope_for;
use crate::{db, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/app/login", get(login_page))
        .route("/app/projects", get(projects_list).post(create_project_form))
        .route("/app/projects/{id}", get(project_edit_page))
        .route("/app/projects/{id}/config", axum::routing::post(save_config))
        .route("/app/projects/{id}/solve", axum::routing::post(solve_project))
}

struct HtmlTemplate<T>(T);

impl<T: Template> IntoResponse for HtmlTemplate<T> {
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
        }
    }
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {
    supabase_url: String,
    supabase_anon_key: String,
}

async fn login_page() -> impl IntoResponse {
    HtmlTemplate(LoginTemplate {
        supabase_url: crate::supabase_url(),
        supabase_anon_key: crate::supabase_publishable_key(),
    })
}

#[derive(Template)]
#[template(path = "projects_list.html")]
struct ProjectsListTemplate {
    projects: Vec<db::Project>,
    is_internal: bool,
}

async fn projects_list(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, ApiError> {
    let projects = db::list_projects(&state.db, scope_for(&user)).await?;
    Ok(HtmlTemplate(ProjectsListTemplate { projects, is_internal: user.role == Role::Internal }))
}

#[derive(Deserialize)]
struct CreateProjectForm {
    name: String,
    #[serde(default)]
    org_id: Option<String>,
}

async fn create_project_form(
    State(state): State<AppState>,
    user: AuthUser,
    Form(form): Form<CreateProjectForm>,
) -> Result<impl IntoResponse, ApiError> {
    let org_id = match user.role {
        Role::Partner => user.org_id.expect("partner AuthUser always carries an org_id"),
        Role::Internal => {
            let raw = form
                .org_id
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| ApiError::BadRequest("org_id es obligatorio".into()))?;
            Uuid::parse_str(raw.trim()).map_err(|_| ApiError::BadRequest("org_id invalido".into()))?
        }
    };

    if !db::organization_exists(&state.db, org_id).await? {
        return Err(ApiError::BadRequest("organizacion desconocida".into()));
    }

    let default_data = serde_json::to_value(ProjectRecord::default())
        .expect("ProjectRecord always serializes");
    let project = db::create_project(&state.db, org_id, &form.name, &default_data).await?;
    Ok(Redirect::to(&format!("/app/projects/{}", project.id)))
}

/// Fields shared by the Configurar page and the standalone dashboard
/// fragment returned after a solve — Askama's `{% include %}` renders using
/// the *same* struct's fields, so both templates need identical field names.
#[derive(Clone)]
struct DashboardFields {
    show_error: bool,
    error_message: String,
    show_results: bool,
    show_soc_chart: bool,
    kpi_cards: Vec<KpiCard>,
    time_json: String,
    flows_json: String,
    soc_json: String,
}

#[derive(Clone)]
struct KpiCard {
    label: String,
    value: String,
}

impl DashboardFields {
    fn empty() -> Self {
        Self {
            show_error: false,
            error_message: String::new(),
            show_results: false,
            show_soc_chart: false,
            kpi_cards: Vec::new(),
            time_json: "[]".to_string(),
            flows_json: "{}".to_string(),
            soc_json: "null".to_string(),
        }
    }

    fn error(message: String) -> Self {
        Self { show_error: true, error_message: message, ..Self::empty() }
    }

    fn from_solve(solve: &SolveResultData) -> Self {
        let aggregated = aggregate_flows(solve.time.len(), &solve.flows);
        let soc = solve.flows.get("storage_soc_pct");
        Self {
            show_error: false,
            error_message: String::new(),
            show_results: true,
            show_soc_chart: soc.is_some(),
            kpi_cards: kpi_cards_from(&solve.kpis),
            time_json: serde_json::to_string(&solve.time).unwrap_or_else(|_| "[]".to_string()),
            flows_json: serde_json::to_string(&aggregated).unwrap_or_else(|_| "{}".to_string()),
            soc_json: soc
                .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "null".to_string()))
                .unwrap_or_else(|| "null".to_string()),
        }
    }
}

fn kpi_cards_from(kpis: &BTreeMap<String, f64>) -> Vec<KpiCard> {
    let mut cards = Vec::new();
    if let Some(v) = kpis.get("total_consumption_kwh") {
        cards.push(KpiCard { label: "Consumo total".into(), value: format!("{v:.0} kWh") });
    }
    if let Some(v) = kpis.get("total_production_kwh") {
        cards.push(KpiCard { label: "Produccion total".into(), value: format!("{v:.0} kWh") });
    }
    if let Some(v) = kpis.get("self_consumption_pct") {
        cards.push(KpiCard { label: "Autoconsumo".into(), value: format!("{v:.1}%") });
    }
    if let Some(v) = kpis.get("total_grid_import_kwh") {
        cards.push(KpiCard { label: "Importacion red".into(), value: format!("{v:.0} kWh") });
    }
    if let Some(v) = kpis.get("total_grid_export_kwh") {
        cards.push(KpiCard { label: "Exportacion red".into(), value: format!("{v:.0} kWh") });
    }
    if let Some(v) = kpis.get("battery_soh_pct") {
        cards.push(KpiCard { label: "SoH bateria".into(), value: format!("{v:.1}%") });
    }
    if let Some(v) = kpis.get("total_energy_cost") {
        cards.push(KpiCard { label: "Coste (provisional)".into(), value: format!("{v:.2}") });
    }
    cards
}

/// Aggregates raw per-connection power flows (e.g. `grid_2_consumer_power`)
/// into a handful of meaningful, chartable series, matched by name pattern
/// rather than by a fixed model-name prefix (the prefix varies per session).
fn aggregate_flows(len: usize, flows: &BTreeMap<String, Vec<f64>>) -> BTreeMap<String, Vec<f64>> {
    let mut out: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    let mut add = |label: &str, values: &Vec<f64>| {
        let entry = out.entry(label.to_string()).or_insert_with(|| vec![0.0; len]);
        for (i, v) in values.iter().enumerate() {
            if i < entry.len() {
                entry[i] += v;
            }
        }
    };

    for (name, values) in flows {
        if name == "storage_soc_pct" {
            continue;
        }
        if name.ends_with("_2_consumer_power") {
            add("Consumo", values);
        }
        if name.contains("producer_2_") {
            add("Produccion", values);
        }
        if name.starts_with("grid_2_") {
            add("Red importacion", values);
        }
        if name.ends_with("_2_grid_power") {
            add("Red exportacion", values);
        }
        if name.ends_with("_2_storage_power") {
            add("Bateria carga", values);
        }
        if name.starts_with("storage_2_") {
            add("Bateria descarga", values);
        }
    }
    out
}

#[derive(Template)]
#[template(path = "project_edit.html")]
struct ProjectEditTemplate {
    project_id: Uuid,
    project_name: String,
    project_data: ProjectData,
    missing: Vec<String>,
    show_error: bool,
    error_message: String,
    show_results: bool,
    show_soc_chart: bool,
    kpi_cards: Vec<KpiCard>,
    time_json: String,
    flows_json: String,
    soc_json: String,
}

async fn project_edit_page(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let project = db::get_project(&state.db, id, scope_for(&user))
        .await?
        .ok_or(ApiError::NotFound)?;
    let record: ProjectRecord = serde_json::from_value(project.data).unwrap_or_default();
    let missing = record.config.missing();
    let dashboard = record
        .last_solve
        .as_ref()
        .map(DashboardFields::from_solve)
        .unwrap_or_else(DashboardFields::empty);

    Ok(HtmlTemplate(ProjectEditTemplate {
        project_id: project.id,
        project_name: project.name,
        project_data: record.config,
        missing,
        show_error: dashboard.show_error,
        error_message: dashboard.error_message,
        show_results: dashboard.show_results,
        show_soc_chart: dashboard.show_soc_chart,
        kpi_cards: dashboard.kpi_cards,
        time_json: dashboard.time_json,
        flows_json: dashboard.flows_json,
        soc_json: dashboard.soc_json,
    }))
}

#[derive(Template)]
#[template(path = "_validation.html")]
struct ValidationTemplate {
    missing: Vec<String>,
}

#[derive(Deserialize)]
struct ConfigForm {
    objective: String,
    #[serde(default)]
    consumption_enabled: Option<String>,
    #[serde(default)]
    production_enabled: Option<String>,
    #[serde(default)]
    storage_enabled: Option<String>,
    #[serde(default)]
    production_to_consumption: Option<String>,
    #[serde(default)]
    production_to_grid: Option<String>,
    #[serde(default)]
    production_to_storage: Option<String>,
    #[serde(default)]
    grid_to_consumption: Option<String>,
    #[serde(default)]
    grid_to_storage: Option<String>,
    #[serde(default)]
    storage_to_consumption: Option<String>,
    #[serde(default)]
    storage_to_grid: Option<String>,
    #[serde(default)]
    time_period: String,
    #[serde(default)]
    time_step: String,
    #[serde(default)]
    consumption_values: String,
    #[serde(default)]
    production_values: String,
    #[serde(default)]
    charge_max: String,
    #[serde(default)]
    discharge_max: String,
    #[serde(default)]
    energy_min: String,
    #[serde(default)]
    energy_max: String,
    #[serde(default)]
    charge_eff: String,
    #[serde(default)]
    discharge_eff: String,
    #[serde(default)]
    cycle_max: String,
    #[serde(default)]
    energy_init: String,
    #[serde(default)]
    export_max: String,
    #[serde(default)]
    import_max: String,
}

fn parse_opt_f64(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.is_empty() {
        None
    } else {
        s.parse().ok()
    }
}

fn parse_opt_i64(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.is_empty() {
        None
    } else {
        s.parse().ok()
    }
}

async fn save_config(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Form(form): Form<ConfigForm>,
) -> Result<impl IntoResponse, ApiError> {
    let project = db::get_project(&state.db, id, scope_for(&user))
        .await?
        .ok_or(ApiError::NotFound)?;
    // Preserve any existing solve result — saving config shouldn't wipe it.
    let mut record: ProjectRecord = serde_json::from_value(project.data).unwrap_or_default();

    record.config = ProjectData {
        objective: Some(form.objective),
        consumption_enabled: form.consumption_enabled.is_some(),
        production_enabled: form.production_enabled.is_some(),
        storage_enabled: form.storage_enabled.is_some(),
        connections: ConnectionConfigData {
            production_to_consumption: form.production_to_consumption.is_some(),
            production_to_grid: form.production_to_grid.is_some(),
            production_to_storage: form.production_to_storage.is_some(),
            grid_to_consumption: form.grid_to_consumption.is_some(),
            grid_to_storage: form.grid_to_storage.is_some(),
            storage_to_consumption: form.storage_to_consumption.is_some(),
            storage_to_grid: form.storage_to_grid.is_some(),
        },
        time_period: parse_opt_i64(&form.time_period),
        time_step: parse_opt_f64(&form.time_step),
        consumption_values: crate::config::parse_series(&form.consumption_values).ok(),
        production_values: crate::config::parse_series(&form.production_values).ok(),
        storage: StorageSpecData {
            charge_max: parse_opt_f64(&form.charge_max),
            discharge_max: parse_opt_f64(&form.discharge_max),
            energy_min: parse_opt_f64(&form.energy_min),
            energy_max: parse_opt_f64(&form.energy_max),
            charge_eff: parse_opt_f64(&form.charge_eff),
            discharge_eff: parse_opt_f64(&form.discharge_eff),
            cycle_max: parse_opt_f64(&form.cycle_max),
            energy_init: parse_opt_f64(&form.energy_init),
        },
        grid: GridSpecData {
            export_max: parse_opt_f64(&form.export_max),
            import_max: parse_opt_f64(&form.import_max),
        },
    };

    let missing = record.config.missing();
    let json = serde_json::to_value(&record).expect("ProjectRecord always serializes");
    db::update_project_data(&state.db, id, &json).await?;

    Ok(HtmlTemplate(ValidationTemplate { missing }))
}

#[derive(Template)]
#[template(path = "_dashboard.html")]
struct DashboardTemplate {
    show_error: bool,
    error_message: String,
    show_results: bool,
    show_soc_chart: bool,
    kpi_cards: Vec<KpiCard>,
    time_json: String,
    flows_json: String,
    soc_json: String,
}

impl From<DashboardFields> for DashboardTemplate {
    fn from(f: DashboardFields) -> Self {
        Self {
            show_error: f.show_error,
            error_message: f.error_message,
            show_results: f.show_results,
            show_soc_chart: f.show_soc_chart,
            kpi_cards: f.kpi_cards,
            time_json: f.time_json,
            flows_json: f.flows_json,
            soc_json: f.soc_json,
        }
    }
}

async fn solve_project(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let project = db::get_project(&state.db, id, scope_for(&user))
        .await?
        .ok_or(ApiError::NotFound)?;
    let mut record: ProjectRecord = serde_json::from_value(project.data).unwrap_or_default();

    if !record.config.missing().is_empty() {
        return Ok(HtmlTemplate(DashboardTemplate::from(DashboardFields::error(
            "La configuracion esta incompleta.".to_string(),
        ))));
    }

    let dashboard = match state.engine.run_solve(&record.config).await {
        Ok(result) => {
            record.last_solve = Some(result.clone());
            let json = serde_json::to_value(&record).expect("ProjectRecord always serializes");
            db::update_project_data(&state.db, id, &json).await?;
            DashboardFields::from_solve(&result)
        }
        Err(err) => DashboardFields::error(err.to_string()),
    };

    Ok(HtmlTemplate(DashboardTemplate::from(dashboard)))
}
