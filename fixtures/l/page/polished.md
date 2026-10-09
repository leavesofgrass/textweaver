---
title: A polished page
lang: en
author: Ada Example
---

# A polished page

This page exercises every part of the HTML templates: code, tables,
footnotes,[^1] callouts, math, and figures. See [the table](#a-table).

## Code

```rust
// Add two numbers.
fn add(a: i32, b: i32) -> i32 {
    let total = a + b;
    total
}
```

```python
def greet(name):
    """Say hello."""
    return f"Hello, {name}"  # a comment
```

```
A block with no language stays plain.
```

## A table

| Planet | Moons | Notes |
|--------|------:|-------|
| Mercury | 0 | Closest to the sun |
| Earth | 1 | Home |
| Mars | 2 | Phobos and Deimos |

## Notes and math

> [!tip] Remember
> The type word comes first.

> [!warning]
> A warning with no title of its own.

The area of a circle is $\pi r^2$, and

$$
\sum_{i=1}^{n} i = \frac{n(n+1)}{2}
$$

### A figure

![A crow on a fence post at dusk](crow.png "A crow keeps watch")

[^1]: A footnote, with a link back to where it was cited.
