# Graphs, Cycles and Dependency Order

## The idea

A **graph** is things (nodes) connected by relationships (edges). Build systems, task schedulers and "A needs B" rules are all graphs. A prerequisite `[a, b]` ("b must happen before a") is an edge from `b` to `a`.

Two questions come up constantly:

1. **Is there a cycle?** If A needs B and B needs A, nothing can ever run.
2. **In what order can things run?** That is a *topological order*.

## Building the graph

```python
adj = [[] for _ in range(n)]       # adj[b] = tasks that wait for b
indegree = [0] * n                 # indegree[a] = how many prerequisites a has
for a, b in prerequisites:
    adj[b].append(a)
    indegree[a] += 1
```

## Kahn's algorithm (topological sort by peeling)

Repeatedly run the tasks that have **no remaining prerequisites**, then decrement the counts of tasks waiting on them.

```python
from collections import deque

def can_finish_all(n, prerequisites):
    adj = [[] for _ in range(n)]
    indegree = [0] * n
    for a, b in prerequisites:
        adj[b].append(a)
        indegree[a] += 1

    ready = deque(i for i in range(n) if indegree[i] == 0)
    done = 0
    while ready:
        task = ready.popleft()
        done += 1
        for nxt in adj[task]:
            indegree[nxt] -= 1
            if indegree[nxt] == 0:
                ready.append(nxt)
    return done == n               # anything left over sits on a cycle
```

If fewer than `n` tasks ran, the leftovers are stuck in (or behind) a cycle. The order in which tasks left the queue is a valid schedule. Practice: *Build Dependencies*.

## Cycle detection with DFS (three colours)

Mark each node **unvisited / in progress / finished**. Reaching an *in-progress* node means you came back around: a cycle. Reaching a *finished* node is fine; that is just a shared dependency (a diamond), not a cycle.

Write it **iteratively** (with your own stack) when `n` can be large. Recursion in Python stops at about 1,000 frames, so a chain of 10,000 tasks would crash a recursive version.

## Pitfalls

- **Visited is not enough.** A diamond (`A -> B -> D` and `A -> C -> D`) reaches `D` twice and is *not* a cycle. You need the in-progress state or in-degree counting.
- **Disconnected graphs.** Start the search from *every* unvisited node, not only node 0.
- **Self-loops** like `[0, 0]` are cycles of length one.
- **Duplicate edges** are harmless in Kahn's algorithm, as long as you count and decrement them consistently.
- **Undirected thinking.** Union-find detects cycles in *undirected* graphs. It is the wrong tool here.

## Complexity

O(n + edges) time and space for both Kahn's algorithm and DFS.
