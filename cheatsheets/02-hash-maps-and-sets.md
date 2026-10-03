# Hash Maps, Sets and Counting

## The idea

A hash map answers **"have I seen this before, and what do I know about it?"** in O(1) on average. Whenever your first thought is a nested loop that searches for something, ask whether a map could do that search instantly.

| Language | Map | Set | Counting |
|---|---|---|---|
| Python | `dict` | `set` | `collections.Counter` |
| JavaScript | `Map` | `Set` | `Map` with `(m.get(x) \|\| 0) + 1` |
| Rust | `HashMap` | `HashSet` | `*m.entry(x).or_insert(0) += 1` |
| C++ | `unordered_map` | `unordered_set` | `m[x]++` |

## Pattern 1: "Have I seen it?" (a set)

Walk once, check the set, then add to it. Order matters: **check first, add after.**

```python
def count_repeats(items):
    seen, repeats = set(), 0
    for x in items:
        if x in seen:
            repeats += 1
        seen.add(x)
    return repeats
```

Practice: *Duplicate Tickets*.

## Pattern 2: "Find my partner" (the complement lookup)

Instead of searching for a partner with a second loop, store what you have seen and look up what you need.

```python
def two_sum(nums, target):
    index = {}                          # value -> where we saw it
    for i, x in enumerate(nums):
        if target - x in index:         # does my partner already exist?
            return [index[target - x], i]
        index[x] = i                    # add ourselves after checking
```

Checking before inserting is what stops an element from pairing with itself. Practice: *Two Sum*.

## Pattern 3: Count, then rank (group by)

Count with a map, then sort the entries. This is SQL's `GROUP BY ... ORDER BY COUNT(*) DESC, code ASC LIMIT k`.

```python
from collections import Counter

def top_k(codes, k):
    counts = Counter(codes)
    ranked = sorted(counts.items(), key=lambda t: (-t[1], t[0]))   # count desc, then code asc
    return [code for code, _ in ranked[:k]]
```

- Sorting by a **tuple** key handles tie-breaks. Negating a number sorts it descending.
- `Counter.most_common(k)` does **not** break ties by value. It keeps first-seen order.
- With a huge number of distinct keys and a small `k`, a heap (`heapq.nsmallest`) avoids sorting everything.

Practice: *Top Error Codes*.

## Pitfalls

- **Normalize keys first.** Case-insensitive matching means lowercasing *before* using the key. Never lowercase only for display.
- **Don't rely on iteration order** of a hash map or set. Sort when order matters.
- **Mutable keys.** A Python list can't be a key; use a tuple.
- **Numbers past 32 bits.** In C++ and Rust use 64-bit integers for keys and counts.

## Complexity

Average O(1) per lookup or insert, so a single pass is O(n). Worst case is O(n) per operation with adversarial keys, which is rare in practice.
