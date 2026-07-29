//! SQLite persistence for markets, organizations (clients), and projects.
//!
//! IDs are stored as TEXT (human-readable UUIDs) rather than relying on
//! sqlx's automatic UUID<->BLOB encoding, so the database stays easy to
//! inspect directly with any SQLite browser during development.
//!
//! `projects.data` is an opaque JSON blob — its shape belongs to the
//! Configurar/Simular UI (Phases 3-4), not to this schema. A project's
//! `market_id` is a real column, not part of that blob, since it's a
//! relational link (which shared tariff jurisdiction this project uses),
//! not an input value — see Phase 5 / `markets`.

use serde::Serialize;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{FromRow, SqlitePool};
use std::str::FromStr;
use uuid::Uuid;

pub async fn connect(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::from_str(database_url)?.create_if_missing(true);
    let pool = SqlitePoolOptions::new().max_connections(5).connect_with(options).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

#[derive(Debug, Serialize)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
}

/// Shared reference data: one per country/jurisdiction's tariff structure,
/// reusable across many organizations and projects (see Phase 5).
#[derive(Debug, Serialize)]
pub struct Market {
    pub id: Uuid,
    pub name: String,
    pub country_code: String,
    pub tariff_model_key: String,
}

#[derive(FromRow)]
struct MarketRow {
    id: String,
    name: String,
    country_code: String,
    tariff_model_key: String,
}

impl TryFrom<MarketRow> for Market {
    type Error = sqlx::Error;

    fn try_from(row: MarketRow) -> Result<Self, Self::Error> {
        Ok(Market {
            id: Uuid::parse_str(&row.id).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            name: row.name,
            country_code: row.country_code,
            tariff_model_key: row.tariff_model_key,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct Project {
    pub id: Uuid,
    pub org_id: Uuid,
    pub market_id: Option<Uuid>,
    pub name: String,
    pub data: serde_json::Value,
}

#[derive(FromRow)]
struct ProjectRow {
    id: String,
    org_id: String,
    market_id: Option<String>,
    name: String,
    data: String,
}

impl TryFrom<ProjectRow> for Project {
    type Error = sqlx::Error;

    fn try_from(row: ProjectRow) -> Result<Self, Self::Error> {
        let market_id = row
            .market_id
            .map(|s| Uuid::parse_str(&s).map_err(|e| sqlx::Error::Decode(Box::new(e))))
            .transpose()?;

        Ok(Project {
            id: Uuid::parse_str(&row.id).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            org_id: Uuid::parse_str(&row.org_id).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            market_id,
            name: row.name,
            data: serde_json::from_str(&row.data).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
        })
    }
}

pub async fn create_organization(pool: &SqlitePool, name: &str) -> Result<Organization, sqlx::Error> {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name) VALUES (?, ?)")
        .bind(id.to_string())
        .bind(name)
        .execute(pool)
        .await?;
    Ok(Organization { id, name: name.to_string() })
}

pub async fn organization_exists(pool: &SqlitePool, org_id: Uuid) -> Result<bool, sqlx::Error> {
    let row: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM organizations WHERE id = ?")
        .bind(org_id.to_string())
        .fetch_optional(pool)
        .await?;
    Ok(row.is_some())
}

pub async fn create_market(
    pool: &SqlitePool,
    name: &str,
    country_code: &str,
    tariff_model_key: &str,
) -> Result<Market, sqlx::Error> {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO markets (id, name, country_code, tariff_model_key) VALUES (?, ?, ?, ?)")
        .bind(id.to_string())
        .bind(name)
        .bind(country_code)
        .bind(tariff_model_key)
        .execute(pool)
        .await?;
    Ok(Market {
        id,
        name: name.to_string(),
        country_code: country_code.to_string(),
        tariff_model_key: tariff_model_key.to_string(),
    })
}

pub async fn list_markets(pool: &SqlitePool) -> Result<Vec<Market>, sqlx::Error> {
    let rows: Vec<MarketRow> =
        sqlx::query_as("SELECT id, name, country_code, tariff_model_key FROM markets ORDER BY name")
            .fetch_all(pool)
            .await?;
    rows.into_iter().map(Market::try_from).collect()
}

pub async fn get_market(pool: &SqlitePool, id: Uuid) -> Result<Option<Market>, sqlx::Error> {
    let row: Option<MarketRow> =
        sqlx::query_as("SELECT id, name, country_code, tariff_model_key FROM markets WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(pool)
            .await?;
    row.map(Market::try_from).transpose()
}

pub async fn market_exists(pool: &SqlitePool, id: Uuid) -> Result<bool, sqlx::Error> {
    let row: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM markets WHERE id = ?")
        .bind(id.to_string())
        .fetch_optional(pool)
        .await?;
    Ok(row.is_some())
}

pub async fn create_project(
    pool: &SqlitePool,
    org_id: Uuid,
    market_id: Option<Uuid>,
    name: &str,
    data: &serde_json::Value,
) -> Result<Project, sqlx::Error> {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO projects (id, org_id, market_id, name, data) VALUES (?, ?, ?, ?, ?)")
        .bind(id.to_string())
        .bind(org_id.to_string())
        .bind(market_id.map(|m| m.to_string()))
        .bind(name)
        .bind(data.to_string())
        .execute(pool)
        .await?;
    Ok(Project { id, org_id, market_id, name: name.to_string(), data: data.clone() })
}

/// `org_id = None` means "internal user" — see all projects across every organization.
pub async fn list_projects(pool: &SqlitePool, org_id: Option<Uuid>) -> Result<Vec<Project>, sqlx::Error> {
    let rows: Vec<ProjectRow> = match org_id {
        Some(org_id) => {
            sqlx::query_as("SELECT id, org_id, market_id, name, data FROM projects WHERE org_id = ?")
                .bind(org_id.to_string())
                .fetch_all(pool)
                .await?
        }
        None => {
            sqlx::query_as("SELECT id, org_id, market_id, name, data FROM projects")
                .fetch_all(pool)
                .await?
        }
    };
    rows.into_iter().map(Project::try_from).collect()
}

pub async fn update_project_data(
    pool: &SqlitePool,
    id: Uuid,
    data: &serde_json::Value,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE projects SET data = ?, updated_at = datetime('now') WHERE id = ?")
        .bind(data.to_string())
        .bind(id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

/// `org_id = None` means "internal user" — can fetch any project regardless of owner.
pub async fn get_project(
    pool: &SqlitePool,
    id: Uuid,
    org_id: Option<Uuid>,
) -> Result<Option<Project>, sqlx::Error> {
    let row: Option<ProjectRow> = match org_id {
        Some(org_id) => {
            sqlx::query_as(
                "SELECT id, org_id, market_id, name, data FROM projects WHERE id = ? AND org_id = ?",
            )
            .bind(id.to_string())
            .bind(org_id.to_string())
            .fetch_optional(pool)
            .await?
        }
        None => {
            sqlx::query_as("SELECT id, org_id, market_id, name, data FROM projects WHERE id = ?")
                .bind(id.to_string())
                .fetch_optional(pool)
                .await?
        }
    };
    row.map(Project::try_from).transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_db() -> SqlitePool {
        connect("sqlite::memory:").await.expect("failed to set up in-memory test db")
    }

    #[tokio::test]
    async fn partner_only_sees_own_org_projects() {
        let pool = test_db().await;

        let org_a = create_organization(&pool, "Holcim El Salvador").await.unwrap();
        let org_b = create_organization(&pool, "Holcim Nicaragua").await.unwrap();

        create_project(&pool, org_a.id, None, "El Ronco backup sizing", &serde_json::json!({}))
            .await
            .unwrap();
        create_project(&pool, org_b.id, None, "Nagarote PV+BESS", &serde_json::json!({}))
            .await
            .unwrap();

        // Internal (org_id = None) sees everything.
        let all = list_projects(&pool, None).await.unwrap();
        assert_eq!(all.len(), 2);

        // Partner scoped to org_a sees only their own project.
        let scoped = list_projects(&pool, Some(org_a.id)).await.unwrap();
        assert_eq!(scoped.len(), 1);
        assert_eq!(scoped[0].org_id, org_a.id);
    }

    #[tokio::test]
    async fn get_project_returns_none_for_wrong_org_not_an_error() {
        let pool = test_db().await;

        let org_a = create_organization(&pool, "Holcim El Salvador").await.unwrap();
        let org_b = create_organization(&pool, "Holcim Nicaragua").await.unwrap();
        let project =
            create_project(&pool, org_b.id, None, "Nagarote PV+BESS", &serde_json::json!({}))
                .await
                .unwrap();

        // A partner scoped to org_a must not be able to fetch org_b's project by id.
        let result = get_project(&pool, project.id, Some(org_a.id)).await.unwrap();
        assert!(result.is_none());

        // Internal (org_id = None) can fetch it regardless of owning org.
        let result = get_project(&pool, project.id, None).await.unwrap();
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn create_project_rejects_unknown_organization() {
        let pool = test_db().await;
        assert!(!organization_exists(&pool, Uuid::new_v4()).await.unwrap());
    }

    #[tokio::test]
    async fn markets_are_shared_reference_data() {
        let pool = test_db().await;

        let spain = create_market(&pool, "Espana (OMIE)", "ES", "spain").await.unwrap();
        create_market(&pool, "El Salvador", "SV", "el_salvador").await.unwrap();

        let org_a = create_organization(&pool, "Holcim El Salvador").await.unwrap();
        let org_b = create_organization(&pool, "Otro cliente en Espana").await.unwrap();

        // Two different orgs' projects can reference the same market.
        create_project(&pool, org_a.id, Some(spain.id), "Proyecto 1", &serde_json::json!({}))
            .await
            .unwrap();
        create_project(&pool, org_b.id, Some(spain.id), "Proyecto 2", &serde_json::json!({}))
            .await
            .unwrap();

        assert_eq!(list_markets(&pool).await.unwrap().len(), 2);
        assert!(market_exists(&pool, spain.id).await.unwrap());
        assert!(!market_exists(&pool, Uuid::new_v4()).await.unwrap());

        let all = list_projects(&pool, None).await.unwrap();
        assert!(all.iter().all(|p| p.market_id == Some(spain.id)));
    }

    #[tokio::test]
    async fn project_without_market_is_allowed() {
        let pool = test_db().await;
        let org = create_organization(&pool, "Holcim El Salvador").await.unwrap();
        let project =
            create_project(&pool, org.id, None, "Sin mercado asignado", &serde_json::json!({}))
                .await
                .unwrap();
        assert_eq!(project.market_id, None);
    }
}
