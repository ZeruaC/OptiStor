//! Organization + project endpoints, org-scoped for partner accounts (AUTH-03).

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::auth::{AuthUser, Role};
use crate::error::ApiError;
use crate::{db, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/organizations", post(create_organization))
        .route("/markets", post(create_market).get(list_markets))
        .route("/projects", post(create_project).get(list_projects))
        .route("/projects/{id}", get(get_project))
}

#[derive(Deserialize)]
struct CreateOrganizationIn {
    name: String,
}

async fn create_organization(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<CreateOrganizationIn>,
) -> Result<Json<db::Organization>, ApiError> {
    if user.role != Role::Internal {
        return Err(ApiError::Forbidden);
    }
    let org = db::create_organization(&state.db, &body.name).await?;
    Ok(Json(org))
}

#[derive(Deserialize)]
struct CreateMarketIn {
    name: String,
    country_code: String,
    tariff_model_key: String,
}

/// Markets are shared platform reference data (which jurisdictions/tariff
/// structures exist at all), not client-specific — internal-only, same as
/// organizations.
async fn create_market(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<CreateMarketIn>,
) -> Result<Json<db::Market>, ApiError> {
    if user.role != Role::Internal {
        return Err(ApiError::Forbidden);
    }
    let market =
        db::create_market(&state.db, &body.name, &body.country_code, &body.tariff_model_key)
            .await?;
    Ok(Json(market))
}

async fn list_markets(State(state): State<AppState>, _user: AuthUser) -> Result<Json<Vec<db::Market>>, ApiError> {
    Ok(Json(db::list_markets(&state.db).await?))
}

#[derive(Deserialize)]
struct CreateProjectIn {
    #[serde(default)]
    org_id: Option<Uuid>,
    #[serde(default)]
    market_id: Option<Uuid>,
    name: String,
    #[serde(default = "default_project_data")]
    data: Value,
}

fn default_project_data() -> Value {
    serde_json::json!({})
}

/// Partner accounts always write into their own org, regardless of what
/// `org_id` (if any) they sent — internal accounts must specify which
/// organization the project belongs to.
async fn create_project(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<CreateProjectIn>,
) -> Result<Json<db::Project>, ApiError> {
    let org_id = match user.role {
        Role::Partner => user.org_id.expect("partner AuthUser always carries an org_id"),
        Role::Internal => body
            .org_id
            .ok_or_else(|| ApiError::BadRequest("org_id is required for internal users".into()))?,
    };

    if !db::organization_exists(&state.db, org_id).await? {
        return Err(ApiError::BadRequest("unknown org_id".into()));
    }

    if let Some(market_id) = body.market_id {
        if !db::market_exists(&state.db, market_id).await? {
            return Err(ApiError::BadRequest("unknown market_id".into()));
        }
    }

    let project =
        db::create_project(&state.db, org_id, body.market_id, &body.name, &body.data).await?;
    Ok(Json(project))
}

pub fn scope_for(user: &AuthUser) -> Option<Uuid> {
    match user.role {
        Role::Partner => Some(user.org_id.expect("partner AuthUser always carries an org_id")),
        Role::Internal => None,
    }
}

async fn list_projects(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Vec<db::Project>>, ApiError> {
    let projects = db::list_projects(&state.db, scope_for(&user)).await?;
    Ok(Json(projects))
}

async fn get_project(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<db::Project>, ApiError> {
    let project = db::get_project(&state.db, id, scope_for(&user))
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(project))
}
