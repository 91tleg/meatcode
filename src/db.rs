use serde::Serialize;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    Row, SqlitePool,
};
use std::str::FromStr;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS submissions (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    slug       TEXT    NOT NULL,
    language   TEXT    NOT NULL,
    code       TEXT    NOT NULL,
    passed     INTEGER NOT NULL,
    total      INTEGER NOT NULL,
    results    TEXT    NOT NULL, -- full RunResult as JSON
    created_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
);
CREATE INDEX IF NOT EXISTS submissions_by_problem ON submissions (slug, id DESC);

CREATE TABLE IF NOT EXISTS reviews (
    submission_id INTEGER PRIMARY KEY REFERENCES submissions(id) ON DELETE CASCADE,
    markdown      TEXT    NOT NULL,
    created_at    INTEGER NOT NULL DEFAULT (strftime('%s','now'))
);

CREATE TABLE IF NOT EXISTS drafts (
    slug       TEXT    NOT NULL,
    language   TEXT    NOT NULL,
    code       TEXT    NOT NULL,
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
    PRIMARY KEY (slug, language)
);
";

pub async fn connect(path: &str) -> Result<SqlitePool, sqlx::Error> {
    let opts = SqliteConnectOptions::from_str(&format!("sqlite://{path}"))?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new().max_connections(4).connect_with(opts).await?;
    sqlx::raw_sql(SCHEMA).execute(&pool).await?;
    Ok(pool)
}

#[derive(Serialize)]
pub struct SubmissionSummary {
    pub id: i64,
    pub language: String,
    pub passed: i64,
    pub total: i64,
    pub created_at: i64,
    pub has_review: bool,
}

#[derive(Serialize)]
pub struct Submission {
    pub id: i64,
    pub slug: String,
    pub language: String,
    pub code: String,
    pub passed: i64,
    pub total: i64,
    pub results: serde_json::Value,
    pub review: Option<String>,
    pub created_at: i64,
}

pub async fn insert_submission(
    pool: &SqlitePool,
    slug: &str,
    language: &str,
    code: &str,
    passed: usize,
    total: usize,
    results_json: &str,
) -> Result<i64, sqlx::Error> {
    let r = sqlx::query("INSERT INTO submissions (slug, language, code, passed, total, results) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(slug)
        .bind(language)
        .bind(code)
        .bind(passed as i64)
        .bind(total as i64)
        .bind(results_json)
        .execute(pool)
        .await?;
    Ok(r.last_insert_rowid())
}

pub async fn list_submissions(pool: &SqlitePool, slug: &str) -> Result<Vec<SubmissionSummary>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT s.id, s.language, s.passed, s.total, s.created_at, r.submission_id IS NOT NULL AS has_review
         FROM submissions s LEFT JOIN reviews r ON r.submission_id = s.id
         WHERE s.slug = ? ORDER BY s.id DESC LIMIT 50",
    )
    .bind(slug)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .iter()
        .map(|r| SubmissionSummary {
            id: r.get("id"),
            language: r.get("language"),
            passed: r.get("passed"),
            total: r.get("total"),
            created_at: r.get("created_at"),
            has_review: r.get("has_review"),
        })
        .collect())
}

pub async fn get_submission(pool: &SqlitePool, id: i64) -> Result<Option<Submission>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT s.id, s.slug, s.language, s.code, s.passed, s.total, s.results, s.created_at, r.markdown
         FROM submissions s LEFT JOIN reviews r ON r.submission_id = s.id WHERE s.id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| Submission {
        id: r.get("id"),
        slug: r.get("slug"),
        language: r.get("language"),
        code: r.get("code"),
        passed: r.get("passed"),
        total: r.get("total"),
        results: serde_json::from_str(&r.get::<String, _>("results")).unwrap_or_default(),
        review: r.get("markdown"),
        created_at: r.get("created_at"),
    }))
}

pub async fn save_review(pool: &SqlitePool, submission_id: i64, markdown: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO reviews (submission_id, markdown) VALUES (?, ?)
         ON CONFLICT(submission_id) DO UPDATE SET markdown = excluded.markdown, created_at = strftime('%s','now')",
    )
    .bind(submission_id)
    .bind(markdown)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_drafts(pool: &SqlitePool, slug: &str) -> Result<std::collections::HashMap<String, String>, sqlx::Error> {
    let rows = sqlx::query("SELECT language, code FROM drafts WHERE slug = ?").bind(slug).fetch_all(pool).await?;
    Ok(rows.iter().map(|r| (r.get("language"), r.get("code"))).collect())
}

pub async fn put_draft(pool: &SqlitePool, slug: &str, language: &str, code: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO drafts (slug, language, code) VALUES (?, ?, ?)
         ON CONFLICT(slug, language) DO UPDATE SET code = excluded.code, updated_at = strftime('%s','now')",
    )
    .bind(slug)
    .bind(language)
    .bind(code)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_draft(pool: &SqlitePool, slug: &str, language: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM drafts WHERE slug = ? AND language = ?").bind(slug).bind(language).execute(pool).await?;
    Ok(())
}
