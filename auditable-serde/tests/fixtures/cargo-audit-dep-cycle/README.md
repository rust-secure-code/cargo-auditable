# Minimal example for cyclic dependency graph in audit data

When building this project with `cargo auditable build`, and then running `cargo audit` on it, this error is printed:

```
error: parse error: Failed to deserialize audit data from JSON: The input JSON specifies a cyclic dependency graph
```

This repository serves as a minimal example for reproducing the issue.

The issue was reported [here](https://github.com/rustsec/rustsec/issues/1043).
