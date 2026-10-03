mod db;
mod harness;
mod review;
mod runner;
mod sql;
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
pub struct SqlSpec {
    /// CREATE TABLE statements shared by every test; each test's `args[0]` holds that test's INSERTs.
    pub schema: String,
    /// What the editor starts with.
    pub starter: String,
    /// Compare rows in order (the problem requires an ORDER BY) instead of as a set.
    #[serde(default)]
    pub ordered: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Problem {
    pub slug: String,
    pub title: String,
    pub difficulty: String,
    pub description: String,
    /// Code problems define a function signature; SQL problems define `sql` instead.
    pub function: Option<types::Signature>,
    pub sql: Option<SqlSpec>,
    /// Compare the returned array ignoring element order.
    #[serde(default)]
    pub sort_result: bool,
    pub tests: Vec<TestCase>,
}

#[derive(Clone)]
struct AppState {
    dir: Arc<PathBuf>,
    sheets: Arc<PathBuf>,
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
    /// Some submission passed every test in the current suite.
    solved: bool,
}

async fn list(State(s): State<AppState>) -> Result<Json<Vec<Summary>>, StatusCode> {
    let passed = db::fully_passed(&s.db).await.map_err(log_err)?;
    Ok(Json(
        load_all(&s.dir)
            .into_iter()
            .map(|p| Summary {
                solved: passed.contains(&(p.slug.clone(), p.tests.len() as i64)),
                slug: p.slug,
                title: p.title,
                difficulty: p.difficulty,
            })
            .collect(),
    ))
}

#[derive(Serialize)]
struct SheetSummary {
    slug: String,
    title: String,
}

#[derive(Serialize)]
struct Sheet {
    slug: String,
    title: String,
    markdown: String,
}

/// The title is the first `# ` heading of the file.
fn sheet_title(md: &str, fallback: &str) -> String {
    md.lines().find_map(|l| l.strip_prefix("# ")).unwrap_or(fallback).trim().to_string()
}

// Cheatsheets are Markdown files in `cheatsheets/`, ordered by filename (e.g. `01-dynamic-programming.md`).
async fn list_sheets(State(s): State<AppState>) -> Json<Vec<SheetSummary>> {
    let mut files: Vec<_> = std::fs::read_dir(&*s.sheets)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "md"))
        .collect();
    files.sort();
    Json(
        files
            .iter()
            .filter_map(|p| {
                let slug = p.file_stem()?.to_str()?.to_string();
                let md = std::fs::read_to_string(p).ok()?;
                Some(SheetSummary { title: sheet_title(&md, &slug), slug })
            })
            .collect(),
    )
}

async fn get_sheet(State(s): State<AppState>, Path(slug): Path<String>) -> Result<Json<Sheet>, StatusCode> {
    // Only plain file names: no separators or dots, so a request can't escape the directory.
    if slug.is_empty() || !slug.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        return Err(StatusCode::NOT_FOUND);
    }
    let markdown = std::fs::read_to_string(s.sheets.join(format!("{slug}.md"))).map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(Sheet { title: sheet_title(&markdown, &slug), slug, markdown }))
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
    let starter = match (&p.sql, &p.function) {
        (Some(q), _) => HashMap::from([("sql".to_string(), q.starter.clone())]),
        (None, Some(f)) => harness::starters(f).map_err(|e| {
            eprintln!("{slug}: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?,
        (None, None) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };
    Ok(Json(ProblemView { problem: p, starter }))
}

#[derive(Deserialize)]
struct RunReq {
    language: String,
    code: String,
    #[serde(default)]
    submit: bool,
    /// Submitting a follow-up attempt: the id of the reviewed attempt it builds on.
    parent_id: Option<i64>,
}

fn log_err(e: impl std::fmt::Display) -> StatusCode {
    eprintln!("error: {e}");
    StatusCode::INTERNAL_SERVER_ERROR
}

fn internal(e: impl std::fmt::Display) -> (StatusCode, String) {
    (log_err(e), "internal error".into())
}

async fn run(
    State(s): State<AppState>,
    Path(slug): Path<String>,
    Json(req): Json<RunReq>,
) -> Result<Json<runner::RunResult>, (StatusCode, String)> {
    let bad = |m: &str| (StatusCode::BAD_REQUEST, m.to_string());
    let p = find(&s.dir, &slug).ok_or((StatusCode::NOT_FOUND, "no such problem".into()))?;
    // A follow-up must build on a reviewed first attempt of this problem; there is only one round.
    let parent = match (req.submit, req.parent_id) {
        (true, Some(id)) => {
            let base = db::get_submission(&s.db, id).await.map_err(internal)?.ok_or_else(|| bad("no such parent attempt"))?;
            let followup = base.proposed_followup.clone();
            if base.slug != slug || base.parent_id.is_some() {
                return Err(bad("a follow-up must build on a first attempt of the same problem"));
            }
            Some((id, followup.ok_or_else(|| bad("that attempt has no follow-up; review it first"))?))
        }
        _ => None,
    };
    let tests: Vec<TestCase> = if req.submit {
        p.tests.clone()
    } else {
        p.tests.iter().filter(|t| !t.hidden).cloned().collect()
    };
    let mut result = runner::run(&req.language, &req.code, &p, &tests).await;
    if req.submit {
        let json = serde_json::to_string(&result).map_err(internal)?;
        let id = db::insert_submission(
            &s.db,
            &slug,
            &req.language,
            &req.code,
            result.passed,
            result.total,
            &json,
            parent.as_ref().map(|(id, f)| (*id, f)),
        )
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
    Ok(Json(db::list_submissions(&s.db, &slug).await.map_err(log_err)?))
}

async fn get_submission(
    State(s): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<db::Submission>, StatusCode> {
    db::get_submission(&s.db, id).await.map_err(log_err)?.map(Json).ok_or(StatusCode::NOT_FOUND)
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
    followup: Option<review::Followup>,
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
        return Ok(Json(ReviewResp { review: saved.clone(), followup: sub.proposed_followup.clone() }));
    }
    let p = find(&s.dir, &sub.slug).ok_or((StatusCode::NOT_FOUND, "problem no longer exists".into()))?;
    // A follow-up attempt is reviewed against the attempt it was built from, and is the last round.
    let base = match sub.parent_id {
        Some(id) => Some(
            db::get_submission(&s.db, id)
                .await
                .map_err(err)?
                .ok_or((StatusCode::NOT_FOUND, "parent attempt is missing".into()))?,
        ),
        None => None,
    };
    let kind = match (&base, &sub.followup) {
        (Some(b), Some(f)) => review::Kind::FollowUp { question: &f.question, base_code: &b.code },
        _ => review::Kind::Initial,
    };
    let out = review::review(&p, &sub.language, &sub.code, &sub.results, kind)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e))?;
    db::save_review(&s.db, sub.id, &out.markdown, out.followup.as_ref()).await.map_err(err)?;
    Ok(Json(ReviewResp { review: out.markdown, followup: out.followup }))
}

async fn get_drafts(
    State(s): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<HashMap<String, String>>, StatusCode> {
    Ok(Json(db::get_drafts(&s.db, &slug).await.map_err(log_err)?))
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
    db::put_draft(&s.db, &slug, &lang, &req.code).await.map_err(log_err)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_draft(
    State(s): State<AppState>,
    Path((slug, lang)): Path<(String, String)>,
) -> Result<StatusCode, StatusCode> {
    db::delete_draft(&s.db, &slug, &lang).await.map_err(log_err)?;
    Ok(StatusCode::NO_CONTENT)
}

#[tokio::main]
async fn main() {
    let dir = std::env::var("PROBLEMS_DIR").unwrap_or_else(|_| "problems".into());
    let db_path = std::env::var("DATABASE_PATH").unwrap_or_else(|_| "meatcode.db".into());
    let db = db::connect(&db_path).await.expect("could not open the SQLite database");
    let sheets = std::env::var("CHEATSHEETS_DIR").unwrap_or_else(|_| "cheatsheets".into());
    let state = AppState { dir: Arc::new(PathBuf::from(dir)), sheets: Arc::new(PathBuf::from(sheets)), db };
    let app = Router::new()
        .route("/api/problems", get(list))
        .route("/api/problems/{slug}", get(get_problem))
        .route("/api/cheatsheets", get(list_sheets))
        .route("/api/cheatsheets/{slug}", get(get_sheet))
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
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());
    let l = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await.unwrap();
    println!("meatcode API on http://127.0.0.1:{port}");
    axum::serve(l, app).await.unwrap();
}
