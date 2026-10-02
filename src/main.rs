mod harness;
mod review;
mod runner;
mod types;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
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
    Ok(Json(runner::run(&req.language, &req.code, &p, &tests).await))
}

#[derive(Deserialize)]
struct ReviewReq {
    language: String,
    code: String,
    result: serde_json::Value,
}

#[derive(Serialize)]
struct ReviewResp {
    review: String,
}

async fn review(
    State(s): State<AppState>,
    Path(slug): Path<String>,
    Json(req): Json<ReviewReq>,
) -> Result<Json<ReviewResp>, (StatusCode, String)> {
    let p = find(&s.dir, &slug).ok_or((StatusCode::NOT_FOUND, "no such problem".into()))?;
    match review::review(&p, &req.language, &req.code, &req.result).await {
        Ok(review) => Ok(Json(ReviewResp { review })),
        Err(e) => Err((StatusCode::BAD_GATEWAY, e)),
    }
}

#[tokio::main]
async fn main() {
    let dir = std::env::var("PROBLEMS_DIR").unwrap_or_else(|_| "problems".into());
    let state = AppState { dir: Arc::new(PathBuf::from(dir)) };
    let app = Router::new()
        .route("/api/problems", get(list))
        .route("/api/problems/{slug}", get(get_problem))
        .route("/api/problems/{slug}/run", post(run))
        .route("/api/problems/{slug}/review", post(review))
        .layer(CorsLayer::permissive())
        .with_state(state);
    let l = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("meatcode API on http://127.0.0.1:3000");
    axum::serve(l, app).await.unwrap();
}
