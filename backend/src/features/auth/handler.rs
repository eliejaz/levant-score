use axum::{
    extract::{FromRequestParts, State},
    http::{header::AUTHORIZATION, request::Parts},
    Json,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::Utc;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use uuid::Uuid;

use super::model::*;
use crate::{
    api::error::{require, ApiError, ApiResult},
    AppState,
};

pub async fn signup(
    State(state): State<AppState>,
    Json(request): Json<SignupRequest>,
) -> ApiResult<Json<AuthResponse>> {
    let display_name = request.display_name.trim();
    let email = request.email.trim().to_lowercase();
    require(!display_name.is_empty(), "Display name is required")?;
    require(
        display_name.chars().count() <= 80,
        "Display name must be at most 80 characters",
    )?;
    require(valid_email(&email), "Email must be valid")?;
    require(
        (8..=100).contains(&request.password.chars().count()),
        "Password must be between 8 and 100 characters",
    )?;
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE lower(email)=lower($1))")
            .bind(&email)
            .fetch_one(&state.pool)
            .await?;
    if exists {
        return Err(ApiError::Conflict(
            "An account already exists for this email".into(),
        ));
    }
    let user = UserRow {
        id: Uuid::new_v4(),
        email,
        display_name: display_name.into(),
        password_hash: hash(&request.password, DEFAULT_COST)
            .map_err(|e| ApiError::Internal(e.to_string()))?,
    };
    let result = sqlx::query("INSERT INTO users (id,email,display_name,password_hash,created_at) VALUES ($1,$2,$3,$4,now())")
        .bind(user.id).bind(&user.email).bind(&user.display_name).bind(&user.password_hash).execute(&state.pool).await;
    if let Err(sqlx::Error::Database(db)) = &result {
        if db.is_unique_violation() {
            return Err(ApiError::Conflict(
                "An account already exists for this email".into(),
            ));
        }
    }
    result?;
    Ok(Json(response(&state, &user)?))
}

pub async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> ApiResult<Json<AuthResponse>> {
    let email = request.email.trim();
    require(valid_email(email), "Email must be valid")?;
    let user = sqlx::query_as::<_, UserRow>(
        "SELECT id,email,display_name,password_hash FROM users WHERE lower(email)=lower($1)",
    )
    .bind(email)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::Unauthorized("Email or password is incorrect".into()))?;
    if !verify(&request.password, &user.password_hash).unwrap_or(false) {
        return Err(ApiError::Unauthorized(
            "Email or password is incorrect".into(),
        ));
    }
    Ok(Json(response(&state, &user)?))
}

fn response(state: &AppState, user: &UserRow) -> ApiResult<AuthResponse> {
    let now = Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: user.id.to_string(),
        email: user.email.clone(),
        iat: now,
        exp: now + state.jwt_expiration_hours as usize * 3600,
    };
    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )
    .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(AuthResponse {
        token,
        user: UserView {
            id: user.id,
            display_name: user.display_name.clone(),
            email: user.email.clone(),
        },
    })
}

fn valid_email(value: &str) -> bool {
    let mut parts = value.split('@');
    matches!((parts.next(), parts.next(), parts.next()), (Some(left), Some(right), None) if !left.is_empty() && right.contains('.') && !right.starts_with('.') && !right.ends_with('.'))
}

#[derive(Clone, Copy, Debug)]
pub struct AuthUser(pub Uuid);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;
    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "))
            .ok_or_else(|| ApiError::Unauthorized("Unauthorized".into()))?;
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        let claims = decode::<Claims>(
            header,
            &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
            &validation,
        )
        .map_err(|_| ApiError::Unauthorized("Unauthorized".into()))?
        .claims;
        let id = Uuid::parse_str(&claims.sub)
            .map_err(|_| ApiError::Unauthorized("Unauthorized".into()))?;
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE id=$1)")
            .bind(id)
            .fetch_one(&state.pool)
            .await?;
        if !exists {
            return Err(ApiError::Unauthorized("Unauthorized".into()));
        }
        Ok(Self(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_email_shape() {
        assert!(valid_email("a@b.com"));
        assert!(!valid_email("wrong"));
    }
    #[test]
    fn verifies_spring_compatible_bcrypt() {
        let encoded = hash("correct horse", 10).unwrap();
        assert!(verify("correct horse", &encoded).unwrap());
    }
}
