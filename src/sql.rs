//! SQL problems: each submission runs a single SELECT in a throwaway in-memory SQLite database.
//! The query runs in a `python3` subprocess (killed after the timeout), with an authorizer that only
//! allows reading, so a submission cannot write, attach files, change pragmas or load extensions.
use crate::{runner::{RunResult, TestResult}, Problem, TestCase};
use serde_json::Value;
use std::{process::Stdio, time::{Duration, Instant}};
use tokio::{io::AsyncWriteExt, process::Command, time::timeout};

const TIMEOUT: Duration = Duration::from_secs(4); // per test, a safety net above the 3s in-database limit
const BUDGET: Duration = Duration::from_secs(10); // per submission: after this, remaining tests are not run

const RUNNER: &str = r#"
import sys, json, sqlite3, time, math
req = json.load(sys.stdin)
try:
    con = sqlite3.connect(":memory:")
    con.executescript(req["schema"])
    con.executescript(req["setup"])
    con.commit()
    con.setlimit(sqlite3.SQLITE_LIMIT_LENGTH, 1000000)   # no giant strings or blobs
    DENY = {"load_extension", "readfile", "writefile", "edit", "fts3_tokenizer"}
    def auth(action, a1, a2, db, trigger):
        if action in (sqlite3.SQLITE_SELECT, sqlite3.SQLITE_READ, sqlite3.SQLITE_RECURSIVE):
            return sqlite3.SQLITE_OK
        if action == sqlite3.SQLITE_FUNCTION:
            return sqlite3.SQLITE_DENY if (a2 or "").lower() in DENY else sqlite3.SQLITE_OK
        return sqlite3.SQLITE_DENY
    con.set_authorizer(auth)
    deadline = time.time() + 3
    con.set_progress_handler(lambda: 1 if time.time() > deadline else 0, 10000)
    cur = con.execute(req["query"])   # raises if there is more than one statement
    cols = [d[0] for d in (cur.description or [])]
    rows = cur.fetchmany(5001)
    if len(rows) > 5000:
        raise Exception("the result has more than 5000 rows")
    def cell(v):
        if isinstance(v, (bytes, bytearray)): return v.hex()
        if isinstance(v, float) and not math.isfinite(v): return str(v)
        return v
    out = {"columns": cols, "rows": [[cell(v) for v in r] for r in rows]}
except Exception as e:
    out = {"error": "%s: %s" % (type(e).__name__, e)}
print(json.dumps(out))
"#;

enum Outcome {
    Table { columns: Vec<String>, rows: Vec<Vec<Value>> },
    Error(String),
    Timeout,
}

async fn exec(schema: &str, setup: &str, query: &str) -> Outcome {
    let req = serde_json::json!({ "schema": schema, "setup": setup, "query": query }).to_string();
    let spawned = Command::new("python3")
        .args(["-c", RUNNER])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn();
    let Ok(mut child) = spawned else { return Outcome::Error("could not launch python3".into()) };
    let mut stdin = child.stdin.take().unwrap();
    tokio::spawn(async move {
        let _ = stdin.write_all(req.as_bytes()).await;
    });
    let out = match timeout(TIMEOUT, child.wait_with_output()).await {
        Err(_) => return Outcome::Timeout,
        Ok(Err(e)) => return Outcome::Error(e.to_string()),
        Ok(Ok(o)) => o,
    };
    let Ok(v) = serde_json::from_slice::<Value>(&out.stdout) else {
        return Outcome::Error(String::from_utf8_lossy(&out.stderr).trim().to_string());
    };
    if let Some(e) = v["error"].as_str() {
        // The progress handler aborts runaway queries with "interrupted".
        return if e.contains("interrupted") { Outcome::Timeout } else { Outcome::Error(e.to_string()) };
    }
    let columns = v["columns"].as_array().map(|a| a.iter().map(|c| c.as_str().unwrap_or("").to_string()).collect()).unwrap_or_default();
    let rows = v["rows"].as_array().map(|a| a.iter().map(|r| r.as_array().cloned().unwrap_or_default()).collect()).unwrap_or_default();
    Outcome::Table { columns, rows }
}

fn show(v: &Value) -> String {
    match v {
        Value::Null => "NULL".into(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// A plain-text table, so wrong answers are easy to read.
pub fn table_text(columns: &[String], rows: &[Vec<Value>]) -> String {
    let mut lines = vec![columns.join(" | ")];
    lines.extend(rows.iter().map(|r| r.iter().map(show).collect::<Vec<_>>().join(" | ")));
    lines.join("\n")
}

/// Comparable form of a cell: numbers to 6 decimals (so 2 and 2.0 match), strings exact, NULL distinct.
fn key(v: &Value) -> String {
    match v {
        Value::Null => "n:".into(),
        Value::Number(n) => format!("d:{:.6}", n.as_f64().unwrap_or(f64::NAN)),
        Value::Bool(b) => format!("d:{:.6}", if *b { 1.0 } else { 0.0 }),
        Value::String(s) => format!("s:{s}"),
        other => format!("s:{other}"),
    }
}

fn same(expected: &[Vec<Value>], actual: &[Vec<Value>], ordered: bool) -> bool {
    let norm = |rows: &[Vec<Value>]| {
        let mut r: Vec<Vec<String>> = rows.iter().map(|row| row.iter().map(key).collect()).collect();
        if !ordered {
            r.sort();
        }
        r
    };
    norm(expected) == norm(actual)
}

pub async fn run(p: &Problem, code: &str, tests: &[TestCase]) -> RunResult {
    let spec = p.sql.as_ref().expect("sql problem");
    let mut out = RunResult { total: tests.len(), ..Default::default() };
    let began = Instant::now();
    for (i, t) in tests.iter().enumerate() {
        let setup = t.args.first().and_then(|a| a.as_str()).unwrap_or("");
        let want_cols: Vec<String> = t.expected["columns"].as_array().map(|a| a.iter().map(|c| c.as_str().unwrap_or("").to_string()).collect()).unwrap_or_default();
        let want_rows: Vec<Vec<Value>> = t.expected["rows"].as_array().map(|a| a.iter().map(|r| r.as_array().cloned().unwrap_or_default()).collect()).unwrap_or_default();
        let start = Instant::now();
        let mut r = TestResult {
            passed: false,
            hidden: t.hidden,
            input: setup.to_string(),
            expected: table_text(&want_cols, &want_rows),
            actual: String::new(),
            stderr: String::new(),
            status: "ok".into(),
            ms: 0,
        };
        let outcome = if began.elapsed() > BUDGET {
            r.stderr = "skipped: this submission already used its time budget".into();
            Outcome::Timeout
        } else {
            exec(&spec.schema, setup, code).await
        };
        match outcome {
            Outcome::Timeout => r.status = "timeout".into(),
            Outcome::Error(e) if i == 0 => {
                // Syntax errors and unknown columns fail every test the same way: report it once.
                out.compile_error = Some(e);
                out.results.clear();
                out.passed = 0;
                return out;
            }
            Outcome::Error(e) => {
                r.status = "runtime_error".into();
                r.stderr = e;
            }
            Outcome::Table { columns, rows } => {
                r.actual = table_text(&columns, &rows);
                if columns.len() == want_cols.len() && same(&want_rows, &rows, spec.ordered) {
                    r.passed = true;
                } else {
                    r.status = "wrong".into();
                }
            }
        }
        r.ms = start.elapsed().as_millis();
        if r.passed {
            out.passed += 1;
        }
        out.results.push(r);
    }
    out
}
