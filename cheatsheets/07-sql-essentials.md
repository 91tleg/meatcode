# SQL Essentials

## What this app runs

SQL problems run on **SQLite**, in a fresh in-memory database per test. Stick to standard SQL and your queries will carry over to SQL Server and Postgres. Only a few spellings differ:

| Task | SQLite / Postgres | SQL Server |
|---|---|---|
| First N rows | `LIMIT n` | `SELECT TOP n ...` |
| Replace NULL | `COALESCE(x, 0)` (works everywhere) | also `ISNULL(x, 0)` |
| Join strings | `a \|\| b` | `a + b` or `CONCAT(a, b)` |

A submission is **one `SELECT`** (or a `WITH ... SELECT`). Writes, `PRAGMA` and `ATTACH` are blocked.

## The sample tables used below

```sql
CREATE TABLE users   (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
CREATE TABLE tickets (id INTEGER PRIMARY KEY, team TEXT NOT NULL,
                      assignee_id INTEGER, status TEXT NOT NULL, priority INTEGER NOT NULL);
```

`assignee_id` is NULL while a ticket is unassigned.

## The order a query really runs in

You write `SELECT` first, but the database works in this order:

`FROM / JOIN` -> `WHERE` -> `GROUP BY` -> `HAVING` -> `SELECT` -> `DISTINCT` -> `ORDER BY` -> `LIMIT`

That explains most surprises: `WHERE` can't use an aggregate (the groups don't exist yet), and `HAVING` can.

## Joins

An **INNER JOIN** keeps only rows that match on both sides. A **LEFT JOIN** keeps every row from the left table and fills the right side with NULL when there is no match.

### Seeing the difference

A tiny example, separate from the sample tables above. A line shows where `tickets.assignee_id = users.id`.

```
users                    tickets
+----+------+            +----+-------------+
| id | name |            | id | assignee_id |
+----+------+            +----+-------------+
|  1 | Ada  |            |  1 |      1      |
|  2 | Ben  |            |  2 |    NULL     |
|  3 | Cy   |            |  3 |      3      |
|  4 | Dee  |            +----+-------------+
+----+------+

users.id                       tickets.assignee_id

 1  Ada  ---------------------->  1    (ticket 1)
 2  Ben       x  nothing points at 2
 3  Cy   ---------------------->  3    (ticket 3)
 4  Dee       x  nothing points at 4

                                NULL   (ticket 2)   x  matches nobody
```

**INNER JOIN** keeps only the rows that found a partner:

```sql
SELECT u.name, t.id AS ticket
FROM users u
JOIN tickets t ON t.assignee_id = u.id;
```

```
+------+--------+
| name | ticket |
+------+--------+
| Ada  |      1 |   <- matched
| Cy   |      3 |   <- matched
+------+--------+
Ben and Dee vanished (no ticket). Ticket 2 vanished too (no user).
```

**LEFT JOIN** keeps every row of the left table, with or without a partner. When there is none, the right-hand columns are filled with `NULL`:

```sql
SELECT u.name, t.id AS ticket
FROM users u
LEFT JOIN tickets t ON t.assignee_id = u.id;
```

```
+------+--------+
| name | ticket |
+------+--------+
| Ada  |      1 |   <- matched
| Ben  |   NULL |   <- no partner: right side is NULL
| Cy   |      3 |   <- matched
| Dee  |   NULL |   <- no partner: right side is NULL
+------+--------+
Every user is still here. Ticket 2 is still missing: only the LEFT table is protected.
```

**LEFT JOIN ... IS NULL** keeps just the rows that found nothing (the anti-join):

```sql
SELECT u.name
FROM users u
LEFT JOIN tickets t ON t.assignee_id = u.id
WHERE t.id IS NULL;
```

```
+------+--------+
| name | ticket |
+------+--------+
| Ada  |      1 |   x  dropped by WHERE
| Ben  |   NULL |   <-- kept
| Cy   |      3 |   x  dropped by WHERE
| Dee  |   NULL |   <-- kept
+------+--------+
Answer: Ben, Dee
```

Which rows survive:

| Join | Matched pairs | Left rows with no partner | Right rows with no partner |
|---|---|---|---|
| `JOIN` (inner) | kept | dropped | dropped |
| `LEFT JOIN` | kept | **kept** (right side NULL) | dropped |
| `LEFT JOIN ... WHERE right.id IS NULL` | dropped | **kept** | dropped |

Rule of thumb: use `JOIN` when you need the matches, `LEFT JOIN` when you must not lose the left rows (counting, listing everyone), and `LEFT JOIN ... IS NULL` for "who has none".

```sql
-- every user with the number of tickets assigned to them (0 for users with none)
SELECT u.name, COUNT(t.id) AS tickets
FROM users u
LEFT JOIN tickets t ON t.assignee_id = u.id
GROUP BY u.id, u.name
ORDER BY u.name;
```

### The anti-join: "rows with no match"

Two NULL-safe ways to find users with no tickets:

```sql
-- 1. LEFT JOIN, then keep the rows that found no partner
SELECT u.name
FROM users u
LEFT JOIN tickets t ON t.assignee_id = u.id
WHERE t.id IS NULL;

-- 2. NOT EXISTS
SELECT u.name
FROM users u
WHERE NOT EXISTS (SELECT 1 FROM tickets t WHERE t.assignee_id = u.id);
```

## NULL: the value that is never equal to anything

NULL means "unknown", so `NULL = NULL` is not true, it is also unknown. Use `IS NULL` and `IS NOT NULL`, never `= NULL`. A `WHERE` keeps a row only when the condition is **true**, so unknown drops the row.

### The `NOT IN` trap

```sql
-- WRONG: returns NO rows whenever any assignee_id is NULL
SELECT name FROM users
WHERE id NOT IN (SELECT assignee_id FROM tickets);
```

`id NOT IN (2, NULL)` means `id <> 2 AND id <> NULL`, and the second half is unknown, so the whole test is never true. Either filter the NULLs out of the subquery (`WHERE assignee_id IS NOT NULL`) or use `NOT EXISTS`, which is safe by construction.

### Counting with NULLs

- `COUNT(*)` counts rows.
- `COUNT(col)` counts rows where `col` is **not NULL**.
- After a LEFT JOIN, `COUNT(t.id)` gives 0 for a user with no tickets, but `COUNT(*)` gives 1 (the NULL-filled row).

## GROUP BY and HAVING

- `WHERE` filters **rows before** they are grouped. `HAVING` filters **groups after** they are formed.
- Every column in `SELECT` must be in `GROUP BY` or inside an aggregate (`COUNT`, `SUM`, `MIN`, `MAX`, `AVG`).
- **Group by the key, not the label.** Two different users can share a name. Grouping by `name` would merge them. Group by `u.id` (and keep `u.name` in the group list so you can select it).

```sql
-- users with at least 2 open tickets, busiest first
SELECT u.name, COUNT(*) AS open_tickets
FROM users u
JOIN tickets t ON t.assignee_id = u.id
WHERE t.status = 'open'
GROUP BY u.id, u.name
HAVING COUNT(*) >= 2
ORDER BY open_tickets DESC, u.name;
```

## Window functions

A window function computes a value **per row** looking at related rows, without collapsing them like `GROUP BY` does. `PARTITION BY` splits the rows into groups, and `ORDER BY` inside `OVER (...)` sets the order within each group.

The three ranking functions differ only in how they handle ties. For priorities `9, 9, 8`:

| Function | Gives | Meaning |
|---|---|---|
| `ROW_NUMBER()` | 1, 2, 3 | always unique; ties are split arbitrarily |
| `RANK()` | 1, 1, 3 | ties share a rank, then it **skips** |
| `DENSE_RANK()` | 1, 1, 2 | ties share a rank, **no gaps** |

### The top-N-per-group pattern

You can't filter on a window function in the same `SELECT`'s `WHERE` (it runs after `WHERE`), so wrap it in a subquery or a CTE.

```sql
-- the 2 highest distinct priorities in each team (all tickets that tie are kept)
SELECT team, id, priority
FROM (
  SELECT team, id, priority,
         DENSE_RANK() OVER (PARTITION BY team ORDER BY priority DESC) AS r
  FROM tickets
)
WHERE r <= 2
ORDER BY team, priority DESC, id;
```

Pick the function from the question: "exactly N rows" is `ROW_NUMBER`, "the top N values, ties included" is `DENSE_RANK`.

## CTEs (WITH): name a step to keep a query readable

```sql
WITH open_counts AS (
  SELECT assignee_id, COUNT(*) AS n
  FROM tickets
  WHERE status = 'open' AND assignee_id IS NOT NULL
  GROUP BY assignee_id
)
SELECT u.name, c.n
FROM open_counts c
JOIN users u ON u.id = c.assignee_id
ORDER BY c.n DESC, u.name;
```

## Common mistakes

- **`NOT IN` with a nullable column.** Use `NOT EXISTS`.
- **`= NULL`.** It is never true. Use `IS NULL`.
- **`GROUP BY name` when names repeat.** Group by the id.
- **An aggregate in `WHERE`.** Use `HAVING`.
- **`INNER JOIN` when you need the missing rows.** Use `LEFT JOIN`.
- **`DISTINCT` to hide a join bug.** It can also remove rows that should appear twice (two users with the same name).
- **No `ORDER BY`.** Row order is not guaranteed unless you ask for it, and these problems check the order.
- **`COUNT(*)` after a LEFT JOIN** counts the unmatched row. Count a column from the right table instead.

## Practice

*Quiet Users* (anti-join and NULLs), *Busiest Assignees* (join, group, having), *Top Tickets per Team* (window functions and ties).
