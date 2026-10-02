# meatcode

Local LeetCode-style practice. Axum API (`src/`) + React/Vite UI (`web/`).
Languages: Python, JavaScript, Rust, C++ (uses local `python3`, `node`, `rustc`, `c++`).

    cargo run                         # API on :3000
    cd web && npm run dev             # UI on :5173

## Storage
SQLite (`meatcode.db`, gitignored, created on first run; override with `DATABASE_PATH`) holds your drafts,
every submission (code + per-test results) and saved Claude reviews. Use the **History** dropdown to reopen
an attempt along with its review. Problems stay as JSON files in `problems/`.

## Claude review
After a Submit, click **Review with Claude** for a verdict, complexity, code review, missed edge cases and
three follow-up questions. Reviews are saved per submission (**Regenerate review** asks again). It runs the local `claude` CLI (needs Claude Code installed and logged in).

## Adding problems
Drop a JSON file in `problems/` (picked up without restart). Users only write a function;
the server generates the starter code and a hidden `main` per language. See `problems/two-sum.json`.

- `function`: `name` (snake_case; camelCase in JS), `params` [{name, type}], `returns`
- types: `int` (64-bit), `float`, `bool`, `string`, `int[]`, `float[]`, `string[]`, `int[][]`
- `tests`: `args` (JSON, one per param) and `expected` (JSON); `"hidden": true` runs only on Submit
- `"sort_result": true` accepts the returned array in any order
