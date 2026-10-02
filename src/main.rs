mod db;
mod harness;
mod review;
mod runner;
mod types;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use axum::http::{header::CONTENT_TYPE, HeaderValue, Method};
use sqlx::SqlitePool;
use tower_http::cors::CorsLayer;

#[derive(Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub args: Vec<serde_json::Value>,
    pub expected: serde_json::Value,
    #[serde(default)]
    pub hidden: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Problem {
    pub slug: String,
    pub title: String,
    pub difficulty: String,
    pub description: String,
    pub function: types::Signature,
    /// Compare the returned array ignoring element order.
    #[serde(default)]
    pub sort_result: bool,
    pub tests: Vec<TestCase>,
}

#[derive(Clone)]
struct AppState {
    dir: Arc<PathBuf>,
    db: SqlitePool,
}

// Problems are re-read from disk on every request so new files appear without a restart.
fn load_all(dir: &PathBuf) -> Vec<Problem> {
    let mut out: Vec<Problem> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .filter_map(|e| {
            let s = std::fs::read_to_string(e.path()).ok()?;
            match serde_json::from_str(&s) {
                Ok(p) => Some(p),
                Err(err) => {
                    eprintln!("skipping {:?}: {err}", e.path());
                    None
                }
            }
        })
        .collect();
    out.sort_by(|a: &Problem, b| a.title.cmp(&b.title));
    out
}

fn find(dir: &PathBuf, slug: &str) -> Option<Problem> {
    load_all(dir).into_iter().find(|p| p.slug == slug)
}

#[derive(Serialize)]
struct Summary {
    slug: String,
    title: String,
    difficulty: String,
}

async fn list(State(s): State<AppState>) -> Json<Vec<Summary>> {
    Json(
        load_all(&s.dir)
            .into_iter()
            .map(|p| Summary { slug: p.slug, title: p.title, difficulty: p.difficulty })
            .collect(),
    )
}

#[derive(Serialize)]
struct ProblemView {
    #[serde(flatten)]
    problem: Problem,
    starter: HashMap<String, String>,
}

async fn get_problem(
    State(s): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<ProblemView>, StatusCode> {
    let mut p = find(&s.dir, &slug).ok_or(StatusCode::NOT_FOUND)?;
    // Never send hidden tests to the browser.
    p.tests.retain(|t| !t.hidden);
    let starter = harness::starters(&p.function).map_err(|e| {
        eprintln!("{slug}: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(ProblemView { problem: p, starter }))
}

#[derive(Deserialize)]
struct RunReq {
    language: String,
    code: String,
    #[serde(default)]
    submit: bool,
}

fn internal(e: impl std::fmt::Display) -> StatusCode {
    eprintln!("error: {e}");
    StatusCode::INTERNAL_SERVER_ERROR
}

async fn run(
    State(s): State<AppState>,
    Path(slug): Path<String>,
    Json(req): Json<RunReq>,
) -> Result<Json<runner::RunResult>, StatusCode> {
    let p = find(&s.dir, &slug).ok_or(StatusCode::NOT_FOUND)?;
    let tests: Vec<TestCase> = if req.submit {
        p.tests.clone()
    } else {
        p.tests.iter().filter(|t| !t.hidden).cloned().collect()
    };
    let mut result = runner::run(&req.language, &req.code, &p, &tests).await;
    if req.submit {
        let json = serde_json::to_string(&result).map_err(internal)?;
        let id = db::insert_submission(&s.db, &slug, &req.language, &req.code, result.passed, result.total, &json)
            .await
            .map_err(internal)?;
        result.submission_id = Some(id);
    }
    Ok(Json(result))
}

async fn list_submissions(
    State(s): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Vec<db::SubmissionSummary>>, StatusCode> {
    Ok(Json(db::list_submissions(&s.db, &slug).await.map_err(internal)?))
}

async fn get_submission(
    State(s): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<db::Submission>, StatusCode> {
    db::get_submission(&s.db, id).await.map_err(internal)?.map(Json).ok_or(StatusCode::NOT_FOUND)
}

#[derive(Deserialize)]
struct ReviewReq {
    submission_id: i64,
    /// Ask Claude again even if a saved review exists.
    #[serde(default)]
    force: bool,
}

#[derive(Serialize)]
struct ReviewResp {
    review: String,
}

async fn review(
    State(s): State<AppState>,
    Json(req): Json<ReviewReq>,
) -> Result<Json<ReviewResp>, (StatusCode, String)> {
    let err = |e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    let sub = db::get_submission(&s.db, req.submission_id)
        .await
        .map_err(err)?
        .ok_or((StatusCode::NOT_FOUND, "no such submission".into()))?;
    if let (Some(saved), false) = (&sub.review, req.force) {
        return Ok(Json(ReviewResp { review: saved.clone() }));
    }
    let p = find(&s.dir, &sub.slug).ok_or((StatusCode::NOT_FOUND, "problem no longer exists".into()))?;
    let text = review::review(&p, &sub.language, &sub.code, &sub.results)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e))?;
    db::save_review(&s.db, sub.id, &text).await.map_err(err)?;
    Ok(Json(ReviewResp { review: text }))
}

async fn get_drafts(
    State(s): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<HashMap<String, String>>, StatusCode> {
    Ok(Json(db::get_drafts(&s.db, &slug).await.map_err(internal)?))
}

#[derive(Deserialize)]
struct DraftReq {
    code: String,
}

async fn put_draft(
    State(s): State<AppState>,
    Path((slug, lang)): Path<(String, String)>,
    Json(req): Json<DraftReq>,
) -> Result<StatusCode, StatusCode> {
    db::put_draft(&s.db, &slug, &lang, &req.code).await.map_err(internal)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_draft(
    State(s): State<AppState>,
    Path((slug, lang)): Path<(String, String)>,
) -> Result<StatusCode, StatusCode> {
    db::delete_draft(&s.db, &slug, &lang).await.map_err(internal)?;
    Ok(StatusCode::NO_CONTENT)
}

#[tokio::main]
async fn main() {
    let dir = std::env::var("PROBLEMS_DIR").unwrap_or_else(|_| "problems".into());
    let db_path = std::env::var("DATABASE_PATH").unwrap_or_else(|_| "meatcode.db".into());
    let db = db::connect(&db_path).await.expect("could not open the SQLite database");
    let state = AppState { dir: Arc::new(PathBuf::from(dir)), db };
    let app = Router::new()
        .route("/api/problems", get(list))
        .route("/api/problems/{slug}", get(get_problem))
        .route("/api/problems/{slug}/run", post(run))
        .route("/api/problems/{slug}/submissions", get(list_submissions))
        .route("/api/problems/{slug}/drafts", get(get_drafts))
        .route("/api/problems/{slug}/draft/{lang}", put(put_draft).delete(delete_draft))
        .route("/api/submissions/{id}", get(get_submission))
        .route("/api/review", post(review))
        // Only the local Vite dev server may call the API from a browser; other sites are blocked.
        .layer(
            CorsLayer::new()
                .allow_origin([
                    HeaderValue::from_static("http://localhost:5173"),
                    HeaderValue::from_static("http://127.0.0.1:5173"),
                ])
                .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
                .allow_headers([CONTENT_TYPE]),
        )
        .with_state(state);
    let l = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("meatcode API on http://127.0.0.1:3000");
    axum::serve(l, app).await.unwrap();
}
