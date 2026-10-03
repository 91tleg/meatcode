# Stacks and String Parsing

## The idea

A **stack** is last in, first out. Use one whenever a later thing can **undo or cancel an earlier thing**: parentheses, `..` in a path, backspaces, "undo".

In Python a `list` is a stack: `append` pushes, `pop` removes the top.

## Worked example: normalize a file path

Process one segment at a time. A normal name pushes, `..` pops, and `.` or an empty segment is ignored.

```python
def normalize_path(path):
    stack = []
    for part in path.split("/"):
        if part in ("", "."):          # empty (from //) or the current directory
            continue
        if part == "..":
            if stack:                  # the parent of the root is the root
                stack.pop()
        else:
            stack.append(part)
    return "/" + "/".join(stack)
```

Splitting on `/` turns `//` into empty segments, which the first check discards. Practice: *Normalize File Path*.

## Second example: balanced brackets

```python
def balanced(s):
    pairs = {")": "(", "]": "[", "}": "{"}
    stack = []
    for ch in s:
        if ch in "([{":
            stack.append(ch)
        elif ch in pairs:
            if not stack or stack.pop() != pairs[ch]:
                return False
    return not stack                   # nothing left open
```

## Pitfalls

- **Popping an empty stack.** Guard it (`if stack:`). Python raises `IndexError`, and Rust's `pop()` returns an `Option`.
- **Names that look special.** Only the exact segments `.` and `..` are special. `...` and `.hidden` are ordinary names, so compare whole strings, not prefixes.
- **Whitespace is part of a name** unless the problem says otherwise. Don't `strip()` by habit.
- **Building strings in a loop** (`out = out + x`) can be O(n²) on huge inputs. Collect pieces in a list and `"".join(...)` once.
- **The result for an empty stack.** The root path is `"/"`, not an empty string.

## Complexity

O(n) time and O(n) space: each character is handled a constant number of times.
