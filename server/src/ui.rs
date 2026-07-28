//! Server-rendered HTML UI (Askama + HTMX) for Phase 3 — Configurar.
//!
//! Deliberately kept separate from the JSON API in `projects.rs`: HTML forms
//! post `application/x-www-form-urlencoded`, not JSON, so rather than making
//! one route branch on Content-Type, the UI gets its own `/app/...` routes
//! that call straight into the same `db` functions.

use askama::Template;
use axum::extract::{Form, Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::get;
use axum::Router;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::{AuthUser, Role};
use crate::config::{ConnectionConfigData, GridSpecData, ProjectData, StorageSpecData};
use crate::error::ApiError;
use crate::projects::scope_for;
use crate::{db, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/app/login", get(login_page))
        .route("/app/projects", get(projects_list).post(create_project_form))
        .route("/app/projects/{id}", get(project_edit_page))
        .route("/app/projects/{id}/config", axum::routing::post(save_config))
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

    let default_data = serde_json::to_value(ProjectData::default())
        .expect("ProjectData always serializes");
    let project = db::create_project(&state.db, org_id, &form.name, &default_data).await?;
    Ok(Redirect::to(&format!("/app/projects/{}", project.id)))
}

#[derive(Template)]
#[template(path = "project_edit.html")]
struct ProjectEditTemplate {
    project_id: Uuid,
    project_name: String,
    project_data: ProjectData,
    missing: Vec<String>,
}

async fn project_edit_page(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let project = db::get_project(&state.db, id, scope_for(&user))
        .await?
        .ok_or(ApiError::NotFound)?;
    let data: ProjectData = serde_json::from_value(project.data).unwrap_or_default();
    let missing = data.missing();
    Ok(HtmlTemplate(ProjectEditTemplate {
        project_id: project.id,
        project_name: project.name,
        project_data: data,
        missing,
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
    // Confirm the project is in scope before writing to it.
    db::get_project(&state.db, id, scope_for(&user))
        .await?
        .ok_or(ApiError::NotFound)?;

    let data = ProjectData {
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

    let missing = data.missing();
    let json = serde_json::to_value(&data).expect("ProjectData always serializes");
    db::update_project_data(&state.db, id, &json).await?;

    Ok(HtmlTemplate(ValidationTemplate { missing }))
}
