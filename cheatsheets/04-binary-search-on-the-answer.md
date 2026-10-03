# Binary Search on the Answer

## The idea

Normally you binary search an array. Here you binary search the **answer itself**. It works whenever:

1. You can write a function `ok(x)` that answers **"is x enough?"** quickly.
2. That function is **monotonic**: once it becomes true it stays true as `x` grows (`no, no, no, yes, yes, yes`).

Then the smallest `x` where `ok(x)` is true is found in O(log range) calls.

## The template

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

## Worked example: Split Jobs Across Workers

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

## How to spot it

- The question says **"minimize the maximum"** or **"maximize the minimum"**.
- You can easily *check* a proposed answer but not *construct* the best one directly.
- Words like "at most k groups / days / machines".

## Choosing the bounds

- `lo` is the smallest value that could possibly work. Starting too low can let impossible answers through.
- `hi` is a value that surely works.
- Make sure the true answer is always inside `[lo, hi]`. `hi = sum - 1` breaks the one-worker case.

## Pitfalls

- **Overflow.** Sums can pass 32 bits. Use 64-bit integers in C++ and Rust. (`lo + hi` can overflow too; `lo + (hi - lo) // 2` is safe.)
- **Infinite loop.** With `mid = (lo + hi) // 2`, use `lo = mid + 1` (never `lo = mid`).
- **Not monotonic.** If `ok` isn't `no..no, yes..yes`, binary search gives garbage. Test it on a few values by hand.

Practice: *Split Jobs Across Workers*.
