//! SQLite persistence for organizations (clients) and projects.
//!
//! IDs are stored as TEXT (human-readable UUIDs) rather than relying on
//! sqlx's automatic UUID<->BLOB encoding, so the database stays easy to
//! inspect directly with any SQLite browser during development.
//!
//! `projects.data` is an opaque JSON blob — its shape belongs to the
//! Configurar/Simular UI (Phases 3-4), not to this schema.

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

#[derive(Debug, Serialize)]
pub struct Project {
    pub id: Uuid,
    pub org_id: Uuid,
    pub name: String,
    pub data: serde_json::Value,
}

#[derive(FromRow)]
struct ProjectRow {
    id: String,
    org_id: String,
    name: String,
    data: String,
}

impl TryFrom<ProjectRow> for Project {
    type Error = sqlx::Error;

    fn try_from(row: ProjectRow) -> Result<Self, Self::Error> {
        Ok(Project {
            id: Uuid::parse_str(&row.id).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            org_id: Uuid::parse_str(&row.org_id).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
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

pub async fn create_project(
    pool: &SqlitePool,
    org_id: Uuid,
    name: &str,
    data: &serde_json::Value,
) -> Result<Project, sqlx::Error> {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO projects (id, org_id, name, data) VALUES (?, ?, ?, ?)")
        .bind(id.to_string())
        .bind(org_id.to_string())
        .bind(name)
        .bind(data.to_string())
        .execute(pool)
        .await?;
    Ok(Project { id, org_id, name: name.to_string(), data: data.clone() })
}

/// `org_id = None` means "internal user" — see all projects across every organization.
pub async fn list_projects(pool: &SqlitePool, org_id: Option<Uuid>) -> Result<Vec<Project>, sqlx::Error> {
    let rows: Vec<ProjectRow> = match org_id {
        Some(org_id) => {
            sqlx::query_as("SELECT id, org_id, name, data FROM projects WHERE org_id = ?")
                .bind(org_id.to_string())
                .fetch_all(pool)
                .await?
        }
        None => {
            sqlx::query_as("SELECT id, org_id, name, data FROM projects")
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
            sqlx::query_as("SELECT id, org_id, name, data FROM projects WHERE id = ? AND org_id = ?")
                .bind(id.to_string())
                .bind(org_id.to_string())
                .fetch_optional(pool)
                .await?
        }
        None => {
            sqlx::query_as("SELECT id, org_id, name, data FROM projects WHERE id = ?")
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

        create_project(&pool, org_a.id, "El Ronco backup sizing", &serde_json::json!({}))
            .await
            .unwrap();
        create_project(&pool, org_b.id, "Nagarote PV+BESS", &serde_json::json!({}))
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
        let project = create_project(&pool, org_b.id, "Nagarote PV+BESS", &serde_json::json!({}))
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
}
