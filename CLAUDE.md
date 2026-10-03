# meatcode

Local coding interview practice app: Axum API in `src/`, React/Vite UI in `web/`, problems in `problems/*.json`
(format documented in README.md). User data (drafts, submissions, reviews) lives in SQLite via sqlx, schema in `src/db.rs`. The review feature shells out to the local `claude -p` CLI (no API key).

## Authoring a problem (when asked to add one)

Never hand-write expected outputs. Generate them with a script and verify every one two independent ways.

1. Write a reference solution AND an independent brute force (different algorithm). `assert ref == brute` on every
   case small enough to brute force; use a second fast method (e.g. heap vs sweep line) on big cases.
2. Where the input domain is small, enumerate it exhaustively against the brute force to validate the reference
   (these checks need not all become tests; each test costs a process spawn, so keep suites to ~15-30 tests).
3. If the statement promises a unique answer, assert uniqueness for every test (random data often breaks this).
4. First 3 tests are visible examples; everything else is `"hidden": true`.
5. Cover, as applicable:
   - smallest legal input (empty, single element, length 2), and the largest legal input (performance:
     the O(n^2) approach should time out at 5s, the intended one should pass)
   - boundary values of every constraint (min/max coordinates, 0, negatives, values past 32-bit)
   - duplicates / all-identical / all-distinct, sorted / reverse-sorted / shuffled
   - the "touching" boundary of any half-open or inclusive range
   - answer at the start, middle and end
   - the classic wrong approaches for this problem (reusing an element, case folding by bit tricks,
     counting distinct vs total, 32-bit overflow, ignoring unsorted input)
6. Mutation-test the suite: run 4-6 plausible buggy solutions through `POST /api/problems/<slug>/run`
   (`"submit": true`); every one must fail at least one test. Then run a correct solution in all four languages.
7. Strings must be ASCII with no newlines; `int` is 64-bit; floats compare with 1e-6 tolerance.

## Cheatsheets
Concept guides live in `cheatsheets/*.md` (served read-only by the API). When writing or editing one, run every
Python snippet against a brute force before saving; a wrong example is worse than none.
