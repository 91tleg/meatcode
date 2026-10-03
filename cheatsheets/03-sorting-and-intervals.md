# Sorting, Intervals and the Sweep Line

## The idea

Many problems about **time ranges, bookings or overlaps** become easy once you sort. Sorting costs O(n log n), and afterwards a single pass solves the rest.

## Half-open intervals: `[start, end)`

A booking `[5, 10)` is busy from 5 up to, but **not including**, 10. So `[5, 10)` and `[10, 15)` do not overlap. Decide this before you code, because the **touching case** is where most bugs live.

## Technique 1: The sweep line (count what is active)

Turn every interval into two events, then walk the events in time order while keeping a running count.

```python
def max_overlap(intervals):
    events = []
    for start, end in intervals:
        events.append((start, 1))      # something begins
        events.append((end, -1))       # something ends
    events.sort()                      # same time: -1 sorts before +1
    active = best = 0
    for _, change in events:
        active += change
        best = max(best, active)
    return best
```

**Why `-1` before `+1` at the same time?** Tuples sort by their second item when the times tie, and `-1 < 1`. So an ending is processed before a starting at the same moment, which is exactly the half-open rule. Flip it and touching intervals will wrongly count as overlapping.

The answer is the maximum number overlapping at any instant. That is also the minimum number of rooms, bays or servers you need. Practice: *Minimum Test Bays*.

## Technique 2: A min-heap of end times

Sort by start. Keep a min-heap of the end times of resources in use. For each interval:

- If the earliest end is `<=` the new start, reuse that resource: replace it.
- Otherwise open a new one.

```python
import heapq

def min_bays(bookings):
    ends = []
    for start, end in sorted(bookings):
        if ends and ends[0] <= start:
            heapq.heapreplace(ends, end)    # reuse the freed bay
        else:
            heapq.heappush(ends, end)       # need another bay
    return len(ends)
```

In Rust `BinaryHeap` is a **max**-heap, so store `std::cmp::Reverse(end)` to get the smallest first.

## Technique 3: Merge overlapping intervals

Sort by start, then extend the last merged interval whenever the next one begins before it ends.

```python
def merge(intervals):
    out = []
    for s, e in sorted(intervals):
        if out and s < out[-1][1]:         # use <= if touching ranges should merge
            out[-1][1] = max(out[-1][1], e)
        else:
            out.append([s, e])
    return out
```

## Pitfalls

- **Unsorted input.** Never assume the input is sorted unless the statement says so.
- **The touching case.** Test `[[1,3],[3,5]]` by hand.
- **Sorting only by start** loses information when you also need end times. Sorting events, or a heap of ends, keeps it.
- **Big coordinates.** Sweep over *events*, not over every time unit.

## Complexity

O(n log n) for the sort, then O(n). The sweep line uses O(n) extra space for the events.
