use axum::{
    extract::{Path, Query, State},
    Json,
};
use axum_extra::extract::cookie::CookieJar;
use serde::{Deserialize, Serialize};
use sqlx::types::chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{AppError, state::PlatformState};

// ===== Course Types =====

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Course {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub slug: String,
    pub content: serde_json::Value,
    pub order_num: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct CourseListResponse {
    pub courses: Vec<Course>,
    pub total: i64,
}

// ===== User Progress Types =====

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserProgress {
    pub id: Uuid,
    pub user_id: Uuid,
    pub course_id: Uuid,
    pub completed: bool,
    pub progress_data: serde_json::Value,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProgressRequest {
    pub progress_data: serde_json::Value,
    pub completed: Option<bool>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ProgressWithCourse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub course_id: Uuid,
    pub completed: bool,
    pub progress_data: serde_json::Value,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub course_title: String,
    pub course_slug: String,
}

// ===== Badge Types =====

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Badge {
    pub id: Uuid,
    pub course_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub criteria: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct BadgeWithDetails {
    pub id: Uuid,
    pub course_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub criteria: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub earned_at: Option<DateTime<Utc>>,
}

// ===== Course Endpoints =====

#[derive(Debug, Deserialize)]
pub struct CourseQueryParams {
    #[serde(default)]
    pub skip: i64,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    100
}

pub async fn list_courses(
    Query(params): Query<CourseQueryParams>,
    State(state): State<PlatformState>,
) -> Result<Json<CourseListResponse>, AppError> {
    let courses = sqlx::query_as::<_, Course>(
        "SELECT * FROM courses WHERE is_active = true ORDER BY order_num LIMIT $1 OFFSET $2"
    )
    .bind(params.limit)
    .bind(params.skip)
    .fetch_all(&state.db)
    .await?;

    let total: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM courses WHERE is_active = true"
    )
    .fetch_one(&state.db)
    .await?;

    Ok(Json(CourseListResponse {
        courses,
        total: total.0,
    }))
}

pub async fn get_course_by_id(
    Path(course_id): Path<Uuid>,
    State(state): State<PlatformState>,
) -> Result<Json<Course>, AppError> {
    let course = sqlx::query_as::<_, Course>(
        "SELECT * FROM courses WHERE id = $1"
    )
    .bind(course_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("Course not found".into()))?;

    Ok(Json(course))
}

pub async fn get_course_by_slug(
    Path(slug): Path<String>,
    State(state): State<PlatformState>,
) -> Result<Json<Course>, AppError> {
    let course = sqlx::query_as::<_, Course>(
        "SELECT * FROM courses WHERE slug = $1"
    )
    .bind(slug)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("Course not found".into()))?;

    Ok(Json(course))
}

// ===== Progress Endpoints =====

pub async fn get_user_progress(
    State(state): State<PlatformState>,
    user_id: Uuid,
) -> Result<Json<Vec<ProgressWithCourse>>, AppError> {
    let progress = sqlx::query_as::<_, ProgressWithCourse>(
        "SELECT 
            up.id, up.user_id, up.course_id, up.completed, 
            up.progress_data, up.completed_at, up.created_at, up.updated_at,
            c.title as course_title, c.slug as course_slug
         FROM user_progress up
         JOIN courses c ON c.id = up.course_id
         WHERE up.user_id = $1
         ORDER BY c.order_num"
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(progress))
}

pub async fn get_course_progress(
    Path(course_id): Path<Uuid>,
    State(state): State<PlatformState>,
    user_id: Uuid,
) -> Result<Json<UserProgress>, AppError> {
    let progress = sqlx::query_as::<_, UserProgress>(
        "SELECT * FROM user_progress WHERE user_id = $1 AND course_id = $2"
    )
    .bind(user_id)
    .bind(course_id)
    .fetch_optional(&state.db)
    .await?;

    match progress {
        Some(p) => Ok(Json(p)),
        None => {
            // Create new progress entry
            let new_progress = sqlx::query_as::<_, UserProgress>(
                "INSERT INTO user_progress (user_id, course_id, progress_data) 
                 VALUES ($1, $2, '{}'::jsonb) 
                 RETURNING *"
            )
            .bind(user_id)
            .bind(course_id)
            .fetch_one(&state.db)
            .await?;
            Ok(Json(new_progress))
        }
    }
}

pub async fn update_course_progress(
    Path(course_id): Path<Uuid>,
    State(state): State<PlatformState>,
    user_id: Uuid,
    Json(req): Json<UpdateProgressRequest>,
) -> Result<Json<UserProgress>, AppError> {
    let completed_at = if req.completed.unwrap_or(false) {
        Some("NOW()")
    } else {
        None
    };

    let query = if let Some(completed_at_val) = completed_at {
        format!(
            "INSERT INTO user_progress (user_id, course_id, progress_data, completed, completed_at)
             VALUES ($1, $2, $3, $4, {})
             ON CONFLICT (user_id, course_id)
             DO UPDATE SET 
                progress_data = $3,
                completed = $4,
                completed_at = {},
                updated_at = NOW()
             RETURNING *",
            completed_at_val, completed_at_val
        )
    } else {
        "INSERT INTO user_progress (user_id, course_id, progress_data, completed)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (user_id, course_id)
         DO UPDATE SET 
            progress_data = $3,
            completed = $4,
            updated_at = NOW()
         RETURNING *".to_string()
    };

    let progress = sqlx::query_as::<_, UserProgress>(&query)
        .bind(user_id)
        .bind(course_id)
        .bind(&req.progress_data)
        .bind(req.completed.unwrap_or(false))
        .fetch_one(&state.db)
        .await?;

    // Check if user earned any badges
    if req.completed.unwrap_or(false) {
        let _ = check_and_award_badges(&state, user_id, Some(course_id)).await;
    }

    Ok(Json(progress))
}

// ===== Badge Endpoints =====

pub async fn list_badges(
    State(state): State<PlatformState>,
) -> Result<Json<Vec<Badge>>, AppError> {
    let badges = sqlx::query_as::<_, Badge>(
        "SELECT * FROM badges ORDER BY created_at"
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(badges))
}

pub async fn get_user_badges(
    State(state): State<PlatformState>,
    user_id: Uuid,
) -> Result<Json<Vec<BadgeWithDetails>>, AppError> {
    let badges = sqlx::query_as::<_, BadgeWithDetails>(
        "SELECT b.*, ub.earned_at
         FROM badges b
         JOIN user_badges ub ON ub.badge_id = b.id
         WHERE ub.user_id = $1
         ORDER BY ub.earned_at DESC"
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(badges))
}

pub async fn get_course_badges(
    Path(course_id): Path<Uuid>,
    State(state): State<PlatformState>,
) -> Result<Json<Vec<Badge>>, AppError> {
    let badges = sqlx::query_as::<_, Badge>(
        "SELECT * FROM badges WHERE course_id = $1"
    )
    .bind(course_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(badges))
}

// ===== Helper Functions =====

async fn check_and_award_badges(
    state: &PlatformState,
    user_id: Uuid,
    completed_course_id: Option<Uuid>,
) -> Result<(), AppError> {
    // Check for course completion badges
    if let Some(course_id) = completed_course_id {
        let course_badges = sqlx::query_as::<_, Badge>(
            "SELECT * FROM badges WHERE course_id = $1"
        )
        .bind(course_id)
        .fetch_all(&state.db)
        .await?;

        for badge in course_badges {
            // Award badge if criteria met and not already earned
            let already_earned = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM user_badges WHERE user_id = $1 AND badge_id = $2)"
            )
            .bind(user_id)
            .bind(badge.id)
            .fetch_one(&state.db)
            .await?;

            if !already_earned {
                let _ = sqlx::query(
                    "INSERT INTO user_badges (user_id, badge_id) VALUES ($1, $2) ON CONFLICT DO NOTHING"
                )
                .bind(user_id)
                .bind(badge.id)
                .execute(&state.db)
                .await;
            }
        }
    }

    // Check for global badges (e.g., complete N courses)
    let completed_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM user_progress WHERE user_id = $1 AND completed = true"
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;

    let global_badges = sqlx::query_as::<_, Badge>(
        "SELECT * FROM badges WHERE course_id IS NULL"
    )
    .fetch_all(&state.db)
    .await?;

    for badge in global_badges {
        if let Some(courses_required) = badge.criteria.get("courses_completed").and_then(|v| v.as_i64()) {
            if completed_count >= courses_required {
                let already_earned = sqlx::query_scalar::<_, bool>(
                    "SELECT EXISTS(SELECT 1 FROM user_badges WHERE user_id = $1 AND badge_id = $2)"
                )
                .bind(user_id)
                .bind(badge.id)
                .fetch_one(&state.db)
                .await?;

                if !already_earned {
                    let _ = sqlx::query(
                        "INSERT INTO user_badges (user_id, badge_id) VALUES ($1, $2) ON CONFLICT DO NOTHING"
                    )
                    .bind(user_id)
                    .bind(badge.id)
                    .execute(&state.db)
                    .await;
                }
            }
        }
    }

    Ok(())
}

// ===== Session-Protected Handlers =====

async fn get_session_user_id(
    state: &PlatformState,
    jar: &CookieJar,
) -> Result<Uuid, AppError> {
    let session_id = jar.get("session_id")
        .map(|c| c.value().to_string())
        .ok_or(AppError::Unauthorized)?;

    let session = state.sessions.get_session(&session_id)
        .await
        .map_err(|_| AppError::Internal)?
        .ok_or(AppError::Unauthorized)?;

    // Refresh session on activity
    let _ = state.sessions.refresh_session(&session_id).await;

    Uuid::parse_str(&session.user_id)
        .map_err(|_| AppError::Internal)
}

pub async fn get_my_progress(
    State(state): State<PlatformState>,
    jar: CookieJar,
) -> Result<Json<Vec<ProgressWithCourse>>, AppError> {
    let user_id = get_session_user_id(&state, &jar).await?;
    get_user_progress(State(state), user_id).await
}

pub async fn get_my_course_progress(
    Path(course_id): Path<Uuid>,
    State(state): State<PlatformState>,
    jar: CookieJar,
) -> Result<Json<UserProgress>, AppError> {
    let user_id = get_session_user_id(&state, &jar).await?;
    get_course_progress(Path(course_id), State(state), user_id).await
}

pub async fn update_my_course_progress(
    Path(course_id): Path<Uuid>,
    State(state): State<PlatformState>,
    jar: CookieJar,
    Json(req): Json<UpdateProgressRequest>,
) -> Result<Json<UserProgress>, AppError> {
    let user_id = get_session_user_id(&state, &jar).await?;
    update_course_progress(Path(course_id), State(state), user_id, Json(req)).await
}

pub async fn get_my_badges(
    State(state): State<PlatformState>,
    jar: CookieJar,
) -> Result<Json<Vec<BadgeWithDetails>>, AppError> {
    let user_id = get_session_user_id(&state, &jar).await?;
    get_user_badges(State(state), user_id).await
}
