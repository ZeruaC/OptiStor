//! Supabase JWT verification.
//!
//! `server/` never sees passwords — Supabase issues an ES256-signed JWT after
//! login, and every request here just needs to verify that token against
//! Supabase's public JWKS and read the `internal`/`partner` role + `org_id`
//! we store in the user's `app_metadata` for multi-tenancy (AUTH-03).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::FromRef;
use axum::extract::FromRequestParts;
use axum::http::{header, request::Parts, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use serde_json::json;
use tokio::sync::RwLock;
use uuid::Uuid;

const JWKS_CACHE_TTL: Duration = Duration::from_secs(600);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Internal,
    Partner,
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub role: Role,
    pub org_id: Option<Uuid>,
}

#[derive(Debug)]
pub enum AuthError {
    MissingToken,
    InvalidToken(String),
    IncompleteProfile,
    JwksFetch(String),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::MissingToken => write!(f, "missing or malformed Authorization header"),
            AuthError::InvalidToken(msg) => write!(f, "token verification failed: {msg}"),
            AuthError::IncompleteProfile => {
                write!(f, "user is missing required app_metadata (role/org_id)")
            }
            AuthError::JwksFetch(msg) => write!(f, "failed to fetch signing keys: {msg}"),
        }
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let status = match self {
            AuthError::JwksFetch(_) => StatusCode::SERVICE_UNAVAILABLE,
            _ => StatusCode::UNAUTHORIZED,
        };
        (status, Json(json!({ "error": self.to_string() }))).into_response()
    }
}

#[derive(Debug, Default, Deserialize)]
struct AppMetadata {
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    org_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct RawClaims {
    sub: Uuid,
    #[serde(default)]
    app_metadata: AppMetadata,
}

#[derive(Deserialize)]
struct Jwk {
    kid: String,
    alg: String,
    x: String,
    y: String,
}

#[derive(Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

pub struct JwtVerifier {
    jwks_url: String,
    client: reqwest::Client,
    cache: RwLock<Option<(Instant, Arc<HashMap<String, DecodingKey>>)>>,
}

impl JwtVerifier {
    pub fn new(supabase_url: &str) -> Self {
        Self {
            jwks_url: format!("{}/auth/v1/.well-known/jwks.json", supabase_url.trim_end_matches('/')),
            client: reqwest::Client::new(),
            cache: RwLock::new(None),
        }
    }

    async fn keys(&self) -> Result<Arc<HashMap<String, DecodingKey>>, AuthError> {
        {
            let cache = self.cache.read().await;
            if let Some((fetched_at, keys)) = cache.as_ref() {
                if fetched_at.elapsed() < JWKS_CACHE_TTL {
                    return Ok(keys.clone());
                }
            }
        }
        self.refresh_keys().await
    }

    async fn refresh_keys(&self) -> Result<Arc<HashMap<String, DecodingKey>>, AuthError> {
        let jwks: Jwks = self
            .client
            .get(&self.jwks_url)
            .send()
            .await
            .map_err(|e| AuthError::JwksFetch(e.to_string()))?
            .json()
            .await
            .map_err(|e| AuthError::JwksFetch(e.to_string()))?;

        let mut keys = HashMap::new();
        for jwk in jwks.keys {
            if jwk.alg != "ES256" {
                continue; // only asymmetric EC keys are meaningful here
            }
            let key = DecodingKey::from_ec_components(&jwk.x, &jwk.y)
                .map_err(|e| AuthError::JwksFetch(e.to_string()))?;
            keys.insert(jwk.kid, key);
        }
        let keys = Arc::new(keys);

        *self.cache.write().await = Some((Instant::now(), keys.clone()));
        Ok(keys)
    }

    pub async fn verify(&self, token: &str) -> Result<AuthUser, AuthError> {
        let header = decode_header(token).map_err(|e| AuthError::InvalidToken(e.to_string()))?;
        let kid = header
            .kid
            .ok_or_else(|| AuthError::InvalidToken("token has no key id".into()))?;

        let mut keys = self.keys().await?;
        if !keys.contains_key(&kid) {
            // Supabase may have rotated keys since our last fetch — try once more.
            keys = self.refresh_keys().await?;
        }
        let key = keys
            .get(&kid)
            .ok_or_else(|| AuthError::InvalidToken(format!("unknown key id '{kid}'")))?;

        let mut validation = Validation::new(Algorithm::ES256);
        validation.set_audience(&["authenticated"]);
        let data = decode::<RawClaims>(token, key, &validation)
            .map_err(|e| AuthError::InvalidToken(e.to_string()))?;
        let claims = data.claims;

        let role = match claims.app_metadata.role.as_deref() {
            Some("internal") => Role::Internal,
            Some("partner") => Role::Partner,
            _ => return Err(AuthError::IncompleteProfile),
        };
        if role == Role::Partner && claims.app_metadata.org_id.is_none() {
            return Err(AuthError::IncompleteProfile);
        }

        Ok(AuthUser {
            user_id: claims.sub,
            role,
            org_id: claims.app_metadata.org_id,
        })
    }
}

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    Arc<JwtVerifier>: FromRef<S>,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let verifier = Arc::<JwtVerifier>::from_ref(state);
        let token = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or(AuthError::MissingToken)?;
        verifier.verify(token).await
    }
}
