use crate::Problem;
use serde::{Deserialize, Serialize};
use std::{process::Stdio, time::Duration};
use tokio::{io::AsyncWriteExt, process::Command, time::timeout};

const REVIEW_TIMEOUT: Duration = Duration::from_secs(180);

/// Shorten long text (stress-test inputs can be hundreds of KB) so the review prompt stays small.
fn clip(s: &str, max: usize) -> String {
    let n = s.chars().count();
    if n <= max {
        return s.to_string();
    }
    let head: String = s.chars().take(max).collect();
    format!("{head}… [{n} chars total]")
}

/// Condense a stored RunResult: only failing tests get details, and everything is clipped.
fn summarize(result: &serde_json::Value) -> String {
    const MAX_FAILURES: usize = 5;
    let get = |v: &serde_json::Value, k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    let mut out = format!(
        "Passed {}/{} tests.\n",
        result["passed"].as_u64().unwrap_or(0),
        result["total"].as_u64().unwrap_or(0)
    );
    if let Some(e) = result["compile_error"].as_str() {
        out += &format!("Compile error:\n{}\n", clip(e, 1500));
    }
    let tests = result["results"].as_array().cloned().unwrap_or_default();
    let failed: Vec<_> = tests.iter().enumerate().filter(|(_, t)| t["passed"] != true).collect();
    for (i, t) in failed.iter().take(MAX_FAILURES) {
        out += &format!(
            "\nFailed test {}{} — {} ({}ms)\n  input: {}\n  expected: {}\n  actual: {}\n",
            i + 1,
            if t["hidden"] == true { " (hidden)" } else { "" },
            get(t, "status"),
            t["ms"],
            clip(&get(t, "input"), 300).replace('\n', " | "),
            clip(&get(t, "expected"), 300),
            clip(&get(t, "actual"), 300),
        );
        let err = get(t, "stderr");
        if !err.is_empty() {
            out += &format!("  stderr: {}\n", clip(&err, 600));
        }
    }
    if failed.len() > MAX_FAILURES {
        out += &format!("\n…and {} more failing tests.\n", failed.len() - MAX_FAILURES);
    }
    let slowest = tests.iter().filter(|t| t["passed"] == true).filter_map(|t| t["ms"].as_u64()).max();
    if let Some(ms) = slowest {
        out += &format!("\nSlowest passing test: {ms}ms (the time limit is 5000ms per test).\n");
    }
    out
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Followup {
    pub question: String,
    pub hint: String,
}

pub struct ReviewOut {
    pub markdown: String,
    /// Only the first review of a problem proposes a follow-up; the follow-up review is the last round.
    pub followup: Option<Followup>,
}

pub enum Kind<'a> {
    Initial,
    /// A second attempt that edits `base_code` to satisfy `question`.
    FollowUp { question: &'a str, base_code: &'a str },
}

const SCHEMA: &str = r#"{"type":"object","properties":{"review":{"type":"string"},"followup":{"type":"object","properties":{"question":{"type":"string"},"hint":{"type":"string"}},"required":["question","hint"]}},"required":["review","followup"]}"#;

fn problem_block(p: &Problem) -> String {
    format!(
        "# Problem: {} ({})\n{}\n\nFunction signature (JSON): {}\n",
        p.title,
        p.difficulty,
        p.description,
        serde_json::to_string(&p.function).unwrap_or_default()
    )
}

fn prompt(p: &Problem, language: &str, code: &str, result: &serde_json::Value, kind: &Kind) -> String {
    let problem = problem_block(p);
    let results = summarize(result);
    match kind {
        Kind::Initial => format!(
            r#"You are a senior engineer coaching someone who is practicing coding interview problems.
They just submitted a solution. Return JSON with two fields.

`review`: Markdown with exactly these sections:
## Verdict
One or two sentences: correct or not, and why. If tests failed, explain the actual bug (use the failing input/expected/actual below), not just the symptom.
## Complexity
Time and space of their solution, and whether a better bound exists.
## Code review
Concrete feedback on clarity, idioms for {language}, naming, and anything a reviewer would flag. Keep it short; skip praise filler.
## Edge cases
Edge cases their code mishandles or that the test suite may not cover (if any).

`followup`: the ONE follow-up an interviewer would ask next, with a one-line `hint` (no full solution). Rules:
- Same function signature and same input/output format: the person will edit their code in place and the existing tests must still pass.
- It adds a new constraint on technique, time or space that can be judged by reading the code (e.g. O(1) extra space, a single pass, no sorting, no hash map, iterative instead of recursive).
- A correct solution under the constraint must exist, run within the 5s per-test limit on the largest tests, and pass every existing test. If unsure, pick a weaker constraint.
- Choose what is most worth practicing given THEIR solution. If it already meets the obvious optimum, pick a constraint that forces a different technique.

{problem}
# Submission ({language})
```
{code}
```

# Test results
{results}
"#
        ),
        Kind::FollowUp { question, base_code } => format!(
            r#"You are a senior engineer coaching someone who is practicing coding interview problems.
They solved the problem, then you asked this follow-up, and they edited their code in place to answer it.

Follow-up: {question}

Review the new attempt in Markdown with exactly these sections:
## Verdict
Did they answer the follow-up? Say whether the tests pass and whether the constraint is actually met. If tests failed, explain the real bug.
## Constraint check
Judge from the code whether the constraint holds (cite the relevant lines or variables). State the new time and space complexity.
## What changed
How the new version differs from the first, and the tradeoff they accepted.
## Takeaway
One sentence on how to explain this in an interview.

This is the last round for this problem: do not propose another follow-up.

{problem}
# First attempt ({language})
```
{base_code}
```

# New attempt ({language})
```
{code}
```

# Test results for the new attempt
{results}
"#
        ),
    }
}

pub async fn review(
    p: &Problem,
    language: &str,
    code: &str,
    result: &serde_json::Value,
    kind: Kind<'_>,
) -> Result<ReviewOut, String> {
    let mut args: Vec<&str> = vec![
        "-p",
        "--tools",
        "",
        "--no-session-persistence",
        // Skip Claude Code's large default prompt, user settings, MCP servers and skills: ~6k -> ~0.4k tokens.
        "--system-prompt",
        "You are a concise, precise coding interview coach.",
        "--setting-sources",
        "",
        "--strict-mcp-config",
        "--disable-slash-commands",
        "--output-format",
        "json",
    ];
    if matches!(kind, Kind::Initial) {
        args.extend(["--json-schema", SCHEMA]);
    }
    let mut child = Command::new("claude")
        .args(&args)
        .current_dir(std::env::temp_dir())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("could not launch the `claude` CLI: {e}"))?;
    let mut stdin = child.stdin.take().unwrap();
    let input = prompt(p, language, code, result, &kind);
    eprintln!("review prompt: {} chars (~{} tokens)", input.len(), input.len() / 4);
    tokio::spawn(async move {
        let _ = stdin.write_all(input.as_bytes()).await;
    });
    let out = timeout(REVIEW_TIMEOUT, child.wait_with_output())
        .await
        .map_err(|_| "review timed out".to_string())?
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!("claude exited with {}: {}", out.status, String::from_utf8_lossy(&out.stderr)));
    }
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).map_err(|e| format!("bad claude output: {e}"))?;
    if v["is_error"] == true {
        return Err(format!("claude error: {}", v["result"].as_str().unwrap_or("unknown")));
    }
    match kind {
        Kind::FollowUp { .. } => Ok(ReviewOut { markdown: v["result"].as_str().unwrap_or("").trim().to_string(), followup: None }),
        Kind::Initial => {
            let so = &v["structured_output"];
            let markdown = so["review"].as_str().ok_or("claude returned no review")?.trim().to_string();
            let followup = serde_json::from_value(so["followup"].clone()).ok();
            Ok(ReviewOut { markdown, followup })
        }
    }
}
