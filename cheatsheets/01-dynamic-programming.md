# Dynamic Programming

## The idea in one sentence

**DP is recursion that remembers its answers.** It works when a problem has two properties:

1. **Overlapping subproblems.** The same smaller question gets asked again and again.
2. **Optimal substructure.** The best answer to the big question is built from the best answers to smaller ones.

If brute force means "try every choice at every step" and you notice it repeats the same work, DP is the fix.

## When to suspect DP

- The question asks for a **min, max, count of ways, or yes/no possible**.
- You make a **choice at each step** (take or skip, go left or right, use this coin or that one).
- A **greedy** idea fails on a small counterexample.
- The sizes are moderate. O(n²) DP is fine up to about n = 5,000. Beyond that you probably need a better idea.

## The 5-step recipe

Do these in order, and write each one down before you code.

1. **State.** Say in plain words what `dp[i]` means. If you can't say it in one sentence, you don't have a state yet.
2. **Transition.** How does `dp[i]` depend on smaller states? The best question to ask: *"What was the last decision?"*
3. **Base cases.** The smallest states you can answer directly.
4. **Order.** Fill smaller states before the larger states that need them.
5. **Answer.** Which cell holds the final answer? Often `dp[n]`, but not always.

Tip: define `dp[i]` as "the answer for the **first i items**", with `dp[0]` meaning "nothing yet". An array of size `n + 1` avoids most off-by-one bugs.

## Three ways to write the same DP

For every problem you can write it **top-down** (recursion plus a cache), **bottom-up** (a table filled in a loop), and often with **less memory** (keep only the rows you need). They are the same idea in different clothes. Top-down is usually easier to *derive*, and bottom-up is safer in practice because recursion depth is limited (Python stops at about 1,000).

---

## Example 1: Climbing stairs (counting ways)

You can climb 1 or 2 steps at a time. How many ways are there to reach step `n`?

| Step | Answer |
|---|---|
| State | `ways[i]` = number of ways to reach step `i` |
| Last decision | the last move was 1 step (from `i-1`) or 2 steps (from `i-2`) |
| Transition | `ways[i] = ways[i-1] + ways[i-2]` |
| Base | `ways[0] = 1`, `ways[1] = 1` |
| Answer | `ways[n]` |

```python
# 1. Plain recursion: correct, but it recomputes the same values -> O(2^n)
def ways(i):
    if i <= 1:
        return 1
    return ways(i - 1) + ways(i - 2)

# 2. Top-down: the same recursion plus a cache -> O(n)
from functools import lru_cache

@lru_cache(None)
def ways(i):
    if i <= 1:
        return 1
    return ways(i - 1) + ways(i - 2)

# 3. Bottom-up: fill a table from small to large -> O(n)
def climb(n):
    dp = [1] * (n + 1)
    for i in range(2, n + 1):
        dp[i] = dp[i - 1] + dp[i - 2]
    return dp[n]

# 4. Keep only the last two values -> O(n) time, O(1) memory
def climb(n):
    a, b = 1, 1
    for _ in range(n - 1):
        a, b = b, a + b
    return b
```

The lesson: version 1 to version 2 is one line, and it turns exponential into linear.

---

## Example 2: House robber (take it or skip it)

Houses in a row hold `nums[i]` money. You can't rob two neighbors. Maximize the total.

| Step | Answer |
|---|---|
| State | `best[i]` = most money using only the **first i houses** |
| Last decision | skip house `i-1`, or rob it |
| Transition | `best[i] = max(best[i-1], best[i-2] + nums[i-1])` |
| Base | `best[0] = 0`, `best[1] = nums[0]` |
| Answer | `best[n]` |

```python
def rob(nums):
    best = [0] * (len(nums) + 1)
    if nums:
        best[1] = nums[0]
    for i in range(2, len(nums) + 1):
        best[i] = max(best[i - 1], best[i - 2] + nums[i - 1])
    return best[-1]
```

Walk through `[2, 7, 9, 3, 1]`: `best = [0, 2, 7, 11, 11, 12]`, so the answer is **12** (2 + 9 + 1).

Every "take or skip" problem has this shape: one option keeps the old answer, the other adds something to an older answer.

---

## Example 3: Coin change (fewest coins)

Given coin values and an amount, find the fewest coins that add up to it, or `-1`.

| Step | Answer |
|---|---|
| State | `fewest[a]` = fewest coins to make amount `a` |
| Last decision | which coin `c` was added last |
| Transition | `fewest[a] = 1 + min(fewest[a - c])` over every coin `c <= a` |
| Base | `fewest[0] = 0`, everything else starts at infinity |
| Answer | `fewest[amount]` (or `-1` if still infinity) |

```python
def coin_change(coins, amount):
    INF = float("inf")
    fewest = [0] + [INF] * amount
    for a in range(1, amount + 1):
        for c in coins:
            if c <= a:
                fewest[a] = min(fewest[a], fewest[a - c] + 1)
    return -1 if fewest[amount] == INF else fewest[amount]
```

**Why not greedy?** For coins `[1, 3, 4]` and amount `6`, greedy takes `4 + 1 + 1` (3 coins). DP finds `3 + 3` (2 coins).

Time is **states x work per state** = `amount x len(coins)`.

---

## Example 4: Two strings (a 2D table)

Longest common subsequence (LCS): the longest sequence of characters that appears in both strings in the same order, not necessarily next to each other.

| Step | Answer |
|---|---|
| State | `lcs[i][j]` = LCS length of the first `i` chars of `a` and the first `j` chars of `b` |
| Transition | if `a[i-1] == b[j-1]`: `lcs[i-1][j-1] + 1`, else `max(lcs[i-1][j], lcs[i][j-1])` |
| Base | row 0 and column 0 are all 0 (one string is empty) |
| Answer | `lcs[len(a)][len(b)]` |

```python
def lcs(a, b):
    dp = [[0] * (len(b) + 1) for _ in range(len(a) + 1)]
    for i in range(1, len(a) + 1):
        for j in range(1, len(b) + 1):
            if a[i - 1] == b[j - 1]:
                dp[i][j] = dp[i - 1][j - 1] + 1
            else:
                dp[i][j] = max(dp[i - 1][j], dp[i][j - 1])
    return dp[len(a)][len(b)]
```

The table for `a = "ACE"` and `b = "ABCDE"`:

|  |  | A | B | C | D | E |
|---|---|---|---|---|---|---|
| **""** | 0 | 0 | 0 | 0 | 0 | 0 |
| **A** | 0 | 1 | 1 | 1 | 1 | 1 |
| **C** | 0 | 1 | 1 | 2 | 2 | 2 |
| **E** | 0 | 1 | 1 | 2 | 2 | **3** |

Each cell only looks at its **up-left, up and left** neighbors, so filling row by row works. Edit distance and grid-path problems have the same shape.

---

## Example 5: Partition DP (the "Split Jobs" problem in this app)

Split `jobs` into `workers` consecutive groups to minimize the biggest group total.

| Step | Answer |
|---|---|
| State | `f[j][i]` = smallest possible "busiest worker" time when the **first i jobs** go to **j workers** |
| Last decision | where the last worker's group starts (job `k`) |
| Transition | `f[j][i] = min over k of max(f[j-1][k], pre[i] - pre[k])` |
| Base | `f[0][0] = 0`, everything else infinity |
| Answer | `f[workers][n]` |

`pre` is the prefix-sum array (`pre[i]` = total of the first `i` jobs), so a group's total is one subtraction.

```python
def min_finish_time(jobs, workers):
    n = len(jobs)
    pre = [0]
    for x in jobs:
        pre.append(pre[-1] + x)
    INF = float("inf")
    f = [[INF] * (n + 1) for _ in range(workers + 1)]
    f[0][0] = 0
    for j in range(1, workers + 1):
        for i in range(j, n + 1):
            f[j][i] = min(
                max(f[j - 1][k], pre[i] - pre[k]) for k in range(j - 1, i)
            )
    return f[workers][n]
```

This is O(n² x workers), which is correct but too slow for n = 10,000. That is why the problem also has a binary-search solution (see the *Binary Search on the Answer* sheet). A good habit: **write the slow DP first, then use it to check a faster idea.** This is how the expected answers for the problems in this app were verified.

---

## Pattern catalog

| Pattern | State looks like | Classic problems |
|---|---|---|
| Linear | `dp[i]` from `dp[i-1]`, `dp[i-2]` | climbing stairs, house robber, max subarray |
| Unbounded knapsack | `dp[amount]` | coin change, number of ways to make change |
| 0/1 knapsack | `dp[capacity]` (loop **downward**) | subset sum, partition equal subset |
| Grid | `dp[r][c]` from up and left | unique paths, minimum path sum |
| Two strings | `dp[i][j]` | LCS, edit distance |
| Subsequence ending at `i` | `dp[i]` = best ending **at** `i` | longest increasing subsequence |
| Interval | `dp[l][r]` | burst balloons, palindrome partitioning |
| Partition | `dp[k][i]` | Split Jobs, split array largest sum |

### 0/1 versus unbounded: the loop direction

If each item can be used **once**, loop the capacity **downward** so an item can't feed itself. If items are **unlimited**, loop upward.

```python
def subset_sum(nums, target):
    can = [True] + [False] * target
    for x in nums:
        for s in range(target, x - 1, -1):   # downward: x is used at most once
            can[s] = can[s] or can[s - x]
    return can[target]
```

---

## Common mistakes

- **A vague state.** "dp[i] is the answer for i" is not a definition. Is it the best *ending at* `i`, or *within the first* `i`?
- **Off-by-one errors.** Use size `n + 1` and let `dp[i]` mean "first `i` items".
- **Wrong starting values.** Minimizing starts at infinity, not 0. Maximizing with negatives starts at negative infinity.
- **Overflow from "infinity".** In Rust or C++ use a large sentinel such as `10^18` in an `i64`, and check before adding to it.
- **Assuming the answer is `dp[n]`.** For longest increasing subsequence it is the *maximum* over all cells.
- **Deep recursion.** Python's limit is about 1,000 frames. Go bottom-up for large `n`.
- **Wrong loop direction** for 0/1 versus unbounded knapsack.

## How to practice and debug

1. Write the **brute force** first (try all choices). It is slow, but obviously right.
2. Write the DP and **compare the two on thousands of small random inputs**.
3. Print the table for a tiny input and check a few cells by hand.
4. If the transition won't come, ask: *"What was the last decision I made?"*
5. Derive it top-down, then convert to bottom-up.

## Complexity in one line

**Time = (number of states) x (work per state).** Climbing stairs: `n x 1`. Coin change: `amount x coins`. LCS: `n x m x 1`. Split Jobs: `n x workers x n`.
