use crate::Problem;
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

fn prompt(p: &Problem, language: &str, code: &str, result: &serde_json::Value) -> String {
    let sig = serde_json::to_string(&p.function).unwrap_or_default();
    format!(
        r#"You are a senior engineer coaching someone who is practicing coding interview problems.
They just submitted a solution. Review it and reply in Markdown with exactly these sections:

## Verdict
One or two sentences: correct or not, and why. If tests failed, explain the actual bug (use the failing input/expected/actual below), not just the symptom.

## Complexity
Time and space of their solution, and whether a better bound exists.

## Code review
Concrete feedback on clarity, idioms for {language}, naming, and anything a reviewer would flag. Keep it short; skip praise filler.

## Edge cases
Edge cases their code mishandles or that the test suite may not cover (if any).

## Follow-ups
Three follow-up questions an interviewer might ask next, from easier to harder (e.g. changed constraints, streaming input, a variant, scaling). Give a one-line hint for each, no full solutions.

# Problem: {title} ({difficulty})
{description}

Function signature (JSON): {sig}

# Submission ({language})
```
{code}
```

# Test results
{result}
"#,
        title = p.title,
        difficulty = p.difficulty,
        description = p.description,
        result = summarize(result),
    )
}

pub async fn review(p: &Problem, language: &str, code: &str, result: &serde_json::Value) -> Result<String, String> {
    let mut child = Command::new("claude")
        .args([
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
        ])
        .current_dir(std::env::temp_dir())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("could not launch the `claude` CLI: {e}"))?;
    let mut stdin = child.stdin.take().unwrap();
    let input = prompt(p, language, code, result);
    eprintln!("review prompt: {} chars (~{} tokens)", input.len(), input.len() / 4);
    tokio::spawn(async move {
        let _ = stdin.write_all(input.as_bytes()).await;
    });
    let out = timeout(REVIEW_TIMEOUT, child.wait_with_output())
        .await
        .map_err(|_| "review timed out".to_string())?
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(format!("claude exited with {}: {}", out.status, String::from_utf8_lossy(&out.stderr)))
    }
}
