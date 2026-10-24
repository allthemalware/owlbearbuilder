<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# Contributing to OwlbearBuilder

Thank you for contributing to OwlbearBuilder. Please follow these guidelines to keep the codebase clean, robust, and maintainable.

## Building and Testing

Always run tests with concurrency constrained to 2 jobs to keep builds polite on shared development machines:

```bash
cargo test -j 2 --workspace
```

All workspace tests must pass cleanly without warnings before submitting changes. For release builds:

```bash
cargo build -j 2 --release
```

## No External Crates

OwlbearBuilder relies strictly on the Rust standard library (`std`). External dependencies are not permitted:
- Every crate's `[dependencies]` table must remain empty, except for internal workspace path dependencies (e.g. `obb-core = { path = "../obb-core" }`).
- Do not add JavaScript build tooling, package managers, or bundlers. Owlbear Rodeo extension UI assets (HTML, CSS, JS, and SVG) are stored as plain files and embedded into the binary using `include_str!`.

## License Headers

Every source file must begin with an SPDX license identifier:
- Rust files (`.rs`): `// SPDX-License-Identifier: GPL-3.0-or-later`
- Shell scripts (`.sh`) and TOML configurations (`.toml`): `# SPDX-License-Identifier: GPL-3.0-or-later`

The project is licensed under GPL-3.0-or-later (see `LICENSE`).

## Comments

Write comments that explain *why*, not *what*. Code should be self-documenting for mechanics; comments should clarify design rationale or rules nuances.
- Example: explain why a natural 20 adjusts the degree of success one step higher *after* the initial comparison against the DC, rather than merely adjusting the numerical roll total.

## Rules Accuracy

All mechanics must adhere strictly to Pathfinder Second Edition Remaster rules. When encoding rules logic, cite the relevant source in code comments (e.g., "Player Core, Multiple Attack Penalty" or "Player Core, Degrees of Success") to document the governing rule.
