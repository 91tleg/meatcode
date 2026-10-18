# Binary Search

Binary search has two faces. **Part 1** searches a sorted array. **Part 2** searches the answer itself.

## Part 1: Searching a sorted array

### The idea

If the data is **sorted**, you never need to look at everything. Check the middle: that tells you which half can be thrown away. Repeat, and n items take about log2(n) steps (a million items take 20).

### The two functions that solve almost everything

Don't write a different binary search for every question. Write **one** well, then build on it.

> `lower(x)` = the index of the **first** element that is `>= x` (or `n` if there is none).

```python
def lower(a, x):
    lo, hi = 0, len(a)          # the answer is somewhere in [lo, hi]; hi = n means "none"
    while lo < hi:
        mid = (lo + hi) // 2
        if a[mid] < x:
            lo = mid + 1        # a[mid] is too small, so the answer is to the right
        else:
            hi = mid            # a[mid] could be the answer, so keep it
    return lo
```

For **integers**, the first element `> x` is just `lower(a, x + 1)`. Everything else comes from these two:

| Question | Answer |
|---|---|
| Insert position for `x` | `lower(a, x)` |
| Is `x` present? | `i = lower(a, x)`, then `i < len(a) and a[i] == x` |
| First position of `x` | `lower(a, x)` (if present) |
| Last position of `x` | `lower(a, x + 1) - 1` (if present) |
| How many copies of `x`? | `lower(a, x + 1) - lower(a, x)` |
| How many values in `[s, e]` (both inclusive)? | `lower(a, e + 1) - lower(a, s)` |
| How many values `< x`? | `lower(a, x)` |

```python
def count_in_range(a, s, e):         # s <= value <= e
    return lower(a, e + 1) - lower(a, s)
```

### Names in each language

| Language | `lower(x)` | first element `> x` |
|---|---|---|
| Python | `bisect.bisect_left(a, x)` | `bisect.bisect_right(a, x)` |
| C++ | `lower_bound(a.begin(), a.end(), x)` | `upper_bound(...)` |
| Rust | `a.partition_point(\|&v\| v < x)` | `a.partition_point(\|&v\| v <= x)` |
| JavaScript | write `lower` yourself | `lower(x + 1)` |

### Why `lo < hi` and `hi = mid`

Keep one rule in your head: the answer is always inside `[lo, hi]`, and the loop shrinks that range every time.

- `hi = len(a)`, not `len(a) - 1`, so "no element qualifies" has somewhere to land.
- `lo = mid + 1` when `mid` is ruled out, and `hi = mid` when `mid` might still be the answer.
- **Never write `lo = mid`** with `mid = (lo + hi) // 2`: when `hi = lo + 1` the range stops shrinking and the loop runs forever.
- In C++ and Rust, `lo + (hi - lo) / 2` avoids overflow with huge indexes.

### Check these by hand before submitting

- the **empty** array,
- a **single** element (hit and miss),
- **all elements equal**,
- the value is **smaller than everything** and **larger than everything**,
- **duplicates** at the edges of your range (does `[s, e]` include them?).

Practice: *Log Range Counts*.

---

## Part 2: Binary search on the answer

### The idea

Normally you binary search an array. Here you binary search the **answer itself**. It works whenever:

1. You can write a function `ok(x)` that answers **"is x enough?"** quickly.
2. That function is **monotonic**: once it becomes true it stays true as `x` grows (`no, no, no, yes, yes, yes`).

Then the smallest `x` where `ok(x)` is true is found in O(log range) calls.

### The template

```python
def smallest_ok(lo, hi):
    # invariant: the answer is in [lo, hi]
    while lo < hi:
        mid = (lo + hi) // 2
        if ok(mid):
            hi = mid          # mid works, so the answer is mid or smaller
        else:
            lo = mid + 1      # mid fails, so the answer is bigger
    return lo
```

For "largest x that works", flip it: `if ok(mid): lo = mid + 1` and use `mid = (lo + hi + 1) // 2`.

### Worked example: Split Jobs Across Workers

Split jobs into `workers` consecutive groups to minimize the biggest group total.

- **Guess** a time limit `cap`. Can the jobs fit into `workers` groups, each totaling at most `cap`?
- **Check greedily:** fill a group until the next job would overflow `cap`, then start a new group.
- **Monotonic:** a larger `cap` never needs more groups.
- **Bounds:** `lo = max(jobs)` (no group can be smaller than the biggest job) and `hi = sum(jobs)` (one worker does everything).

```python
def min_finish_time(jobs, workers):
    def ok(cap):
        groups, current = 1, 0
        for x in jobs:
            if current + x > cap:
                groups += 1
                current = x
            else:
                current += x
        return groups <= workers

    lo, hi = max(jobs), sum(jobs)
    while lo < hi:
        mid = (lo + hi) // 2
        if ok(mid):
            hi = mid
        else:
            lo = mid + 1
    return lo
```

Cost: `ok` is O(n) and runs about `log2(sum)` times, so O(n log sum). Compare that to the O(n² x workers) DP.

### How to spot it

- The question says **"minimize the maximum"** or **"maximize the minimum"**.
- You can easily *check* a proposed answer but not *construct* the best one directly.
- Words like "at most k groups / days / machines".

### Choosing the bounds

- `lo` is the smallest value that could possibly work. Starting too low can let impossible answers through.
- `hi` is a value that surely works.
- Make sure the true answer is always inside `[lo, hi]`. `hi = sum - 1` breaks the one-worker case.

### Pitfalls

- **Overflow.** Sums can pass 32 bits. Use 64-bit integers in C++ and Rust. (`lo + hi` can overflow too; `lo + (hi - lo) // 2` is safe.)
- **Infinite loop.** With `mid = (lo + hi) // 2`, use `lo = mid + 1` (never `lo = mid`).
- **Not monotonic.** If `ok` isn't `no..no, yes..yes`, binary search gives garbage. Test it on a few values by hand.

Practice: *Split Jobs Across Workers*.
