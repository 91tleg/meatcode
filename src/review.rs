use crate::Problem;
use std::{process::Stdio, time::Duration};
use tokio::{io::AsyncWriteExt, process::Command, time::timeout};

const REVIEW_TIMEOUT: Duration = Duration::from_secs(180);

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

# Test results (JSON; `passed`, `status`, and for failures `input`/`expected`/`actual`)
{result}
"#,
        title = p.title,
        difficulty = p.difficulty,
        description = p.description,
    )
}

pub async fn review(p: &Problem, language: &str, code: &str, result: &serde_json::Value) -> Result<String, String> {
    let mut child = Command::new("claude")
        .args(["-p", "--tools", "", "--no-session-persistence"])
        .current_dir(std::env::temp_dir())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("could not launch the `claude` CLI: {e}"))?;
    let mut stdin = child.stdin.take().unwrap();
    let input = prompt(p, language, code, result);
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
