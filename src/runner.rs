use crate::{harness, types, Problem, TestCase};
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
use tokio::{io::AsyncWriteExt, process::Command, time::timeout};

const RUN_TIMEOUT: Duration = Duration::from_secs(5);
const COMPILE_TIMEOUT: Duration = Duration::from_secs(60);
static COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Serialize)]
pub struct TestResult {
    pub passed: bool,
    pub hidden: bool,
    pub input: String,
    pub expected: String,
    pub actual: String,
    pub stderr: String,
    pub status: String, // ok | wrong | timeout | runtime_error
    pub ms: u128,
}

#[derive(Serialize, Default)]
pub struct RunResult {
    pub compile_error: Option<String>,
    pub results: Vec<TestResult>,
    pub passed: usize,
    pub total: usize,
}

// How to build (optional) and launch a language. Returns the argv to execute per test.
async fn prepare(lang: &str, code: &str, dir: &Path) -> Result<Vec<String>, String> {
    let file = |name: &str| dir.join(name).to_string_lossy().to_string();
    match lang {
        "python" => {
            std::fs::write(dir.join("main.py"), code).map_err(|e| e.to_string())?;
            Ok(vec!["python3".into(), file("main.py")])
        }
        "javascript" => {
            std::fs::write(dir.join("main.js"), code).map_err(|e| e.to_string())?;
            Ok(vec!["node".into(), file("main.js")])
        }
        "rust" => {
            std::fs::write(dir.join("main.rs"), code).map_err(|e| e.to_string())?;
            compile(&["rustc", "-O", "--edition", "2021", "-o", &file("out"), &file("main.rs")]).await?;
            Ok(vec![file("out")])
        }
        "cpp" => {
            std::fs::write(dir.join("main.cpp"), code).map_err(|e| e.to_string())?;
            compile(&["c++", "-O2", "-std=c++17", "-o", &file("out"), &file("main.cpp")]).await?;
            Ok(vec![file("out")])
        }
        other => Err(format!("unsupported language: {other}")),
    }
}

async fn compile(argv: &[&str]) -> Result<(), String> {
    let out = timeout(COMPILE_TIMEOUT, Command::new(argv[0]).args(&argv[1..]).output())
        .await
        .map_err(|_| "compilation timed out".to_string())?
        .map_err(|e| format!("failed to launch {}: {e}", argv[0]))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).into_owned())
    }
}

async fn run_one(argv: &[String], p: &Problem, ret: types::Ty, t: &TestCase, stdin_data: String) -> TestResult {
    let start = Instant::now();
    let mut res = TestResult {
        passed: false,
        hidden: t.hidden,
        input: p
            .function
            .params
            .iter()
            .zip(&t.args)
            .map(|(pm, a)| format!("{} = {a}", pm.name))
            .collect::<Vec<_>>()
            .join("\n"),
        expected: t.expected.to_string(),
        actual: String::new(),
        stderr: String::new(),
        status: "ok".into(),
        ms: 0,
    };
    let spawned = Command::new(&argv[0])
        .args(&argv[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn();
    let mut child = match spawned {
        Ok(c) => c,
        Err(e) => {
            res.status = "runtime_error".into();
            res.stderr = format!("failed to launch: {e}");
            return res;
        }
    };
    let mut stdin = child.stdin.take().unwrap();
    let input = stdin_data;
    tokio::spawn(async move {
        let _ = stdin.write_all(input.as_bytes()).await;
    });
    match timeout(RUN_TIMEOUT, child.wait_with_output()).await {
        Err(_) => res.status = "timeout".into(), // child dropped => killed
        Ok(Err(e)) => {
            res.status = "runtime_error".into();
            res.stderr = e.to_string();
        }
        Ok(Ok(out)) => {
            res.actual = String::from_utf8_lossy(&out.stdout).into_owned();
            res.stderr = String::from_utf8_lossy(&out.stderr).into_owned();
            if !out.status.success() {
                res.status = "runtime_error".into();
            } else {
                match ret.decode(&res.actual) {
                    Some(v) => {
                        res.actual = v.to_string();
                        let (a, b) = if p.sort_result {
                            (types::sorted(&v), types::sorted(&t.expected))
                        } else {
                            (v, t.expected.clone())
                        };
                        if types::equal(&a, &b) {
                            res.passed = true;
                        } else {
                            res.status = "wrong".into();
                        }
                    }
                    None => res.status = "wrong".into(),
                }
            }
        }
    }
    res.ms = start.elapsed().as_millis();
    res
}

pub async fn run(lang: &str, code: &str, p: &Problem, tests: &[TestCase]) -> RunResult {
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir: PathBuf = std::env::temp_dir().join(format!("meatcode-{}-{id}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let mut out = RunResult { total: tests.len(), ..Default::default() };
    let setup = (|| {
        let (ptys, ret) = p.function.types()?;
        let mut inputs = Vec::new();
        for t in tests {
            if t.args.len() != ptys.len() {
                return Err(format!("test has {} args but the function takes {}", t.args.len(), ptys.len()));
            }
            let mut s = String::new();
            for (ty, a) in ptys.iter().zip(&t.args) {
                s += &ty.encode(a)?;
            }
            inputs.push(s);
        }
        Ok::<_, String>((harness::wrap(lang, &p.function, code)?, ret, inputs))
    })();
    let prepared = match setup {
        Ok((src, ret, inputs)) => prepare(lang, &src, &dir).await.map(|argv| (argv, ret, inputs)),
        Err(e) => Err(e),
    };
    match prepared {
        Err(e) => out.compile_error = Some(e),
        Ok((argv, ret, inputs)) => {
            for (t, inp) in tests.iter().zip(inputs) {
                let r = run_one(&argv, p, ret, t, inp).await;
                if r.passed {
                    out.passed += 1;
                }
                out.results.push(r);
            }
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    out
}
