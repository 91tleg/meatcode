# meatcode

Local LeetCode-style practice. Axum API (`src/`) + React/Vite UI (`web/`).
Languages: Python, JavaScript, Rust, C++, C (uses local `python3`, `node`, `rustc`, `c++`, `cc`).

    cargo run                         # API on :3000
    cd web && npm run dev             # UI on :5173

## Cheatsheets
Click **Cheatsheet** in the top bar for a slide-in panel of concept guides (dynamic programming, hash maps,
intervals, binary search on the answer, graphs, stacks). They are Markdown files in `cheatsheets/`, ordered by
filename; the first `# ` heading is the title. Add your own and they appear without a restart. Every Python
snippet in the shipped sheets was run against a brute force.

## Storage
SQLite (`meatcode.db`, gitignored, created on first run; override with `DATABASE_PATH`) holds your drafts,
every submission (code + per-test results) and saved Claude reviews. Use the **History** dropdown to reopen
an attempt along with its review. Problems stay as JSON files in `problems/`.

## Claude review
After a Submit, click **Review with Claude** for a verdict, complexity, code review and missed edge cases, plus
**one** follow-up. The follow-up keeps the same function signature and adds a constraint judged from the code
(e.g. "no hash set", "O(1) extra space"), so the existing tests still apply.

Click **Work on this follow-up** to edit your solution in place; the question stays pinned above the editor and
**Compare with first attempt** shows a diff. Submit again and **Review follow-up** checks the tests and whether the
constraint is really met. That is the only follow-up round for a problem. Reviews are saved per submission
(**Regenerate review** asks again). It runs the local `claude` CLI (needs Claude Code installed and logged in).

## C
Your solution may not allocate. `malloc`, `calloc`, `realloc`, `free`, `strdup`,
`getline` and `alloca` are poisoned, so using one is a compile error. The test harness around it is not restricted.
Plain C11 (`cc -O2 -std=c11 -lm`), fixed-width types only: `int` is `int64_t`, lengths are `int32_t`, plus `double`,
`bool` and `const char*`; stdio, stdlib, string, stdint, inttypes, limits, float, math, ctype and assert are pre-included.

Inputs are read-only and carry their length (`xs, xs_len`); matrices come as `m, m_rows, m_cols`. Results go into a buffer
the caller provides, and the function returns how many items it wrote (an out-of-range count fails the test):
- `int[]` / `float[]`: `int32_t f(..., int64_t* ret, int32_t ret_cap)` (or `double*`)
- `string`: `int32_t f(..., char* ret, int32_t ret_cap)`, returns the length
- `string[]`: same signature, strings back to back each NUL-terminated, returns the count
- `int[][]`: `int32_t f(..., int64_t* ret, int32_t ret_cap, int32_t* ret_cols, int32_t ret_rows_cap)`, rows row-major, each
  width in `ret_cols[i]`, returns the row count

Buffers hold 1,048,576 elements (262,144 rows). The starter code shows the exact signature for each problem.

## Adding problems
Drop a JSON file in `problems/` (picked up without restart). Users only write a function;
the server generates the starter code and a hidden `main` per language. See `problems/two-sum.json`.

- `function`: `name` (snake_case; camelCase in JS), `params` [{name, type}], `returns`
- types: `int` (64-bit), `float`, `bool`, `string`, `int[]`, `float[]`, `string[]`, `int[][]`
- `tests`: `args` (JSON, one per param) and `expected` (JSON); `"hidden": true` runs only on Submit
- `"sort_result": true` accepts the returned array in any order

### SQL problems
Add a `sql` section instead of `function`: `{"schema": "CREATE TABLE ...", "starter": "...", "ordered": true}`.
Each test's `args` is `[setup]` (the INSERT statements for that test) and `expected` is
`{"columns": [...], "rows": [[...], ...]}`. Rows are compared as a set unless `ordered` is true (use it when the
statement requires an `ORDER BY`). A submission is one `SELECT` run in a fresh in-memory SQLite database inside a
`python3` subprocess; writes, `PRAGMA`, `ATTACH` and extensions are blocked, a query gets 3 seconds, and a
submission gets a 10 second budget. Keep to portable SQL and avoid NULL ordering and date functions.
