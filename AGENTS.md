# AGENTS.md

Standalone Rust crate (`codeinsight`), published independently of the gm/rs-* orchestration family. Conventions sourced from `CONTRIBUTING.md`.

## Project structure

```
src/
  main.rs          - entry point, file collection, parallel dispatch, CLI flags
  lang.rs          - tree-sitter grammar registry
  analyzer.rs      - AST traversal, entity extraction, convention counting
  depgraph.rs      - dependency graph, import resolution, dead code detection
  formatter.rs     - compact LLM-optimized text output
  json_output.rs   - structured JSON output
  project.rs       - package.json / go.mod / tsconfig.json metadata
  config.rs        - .codeinsight.toml configuration loading
  conventions.rs   - per-language convention aggregation
  git.rs           - git context (branch, commits, hot files)
  tooling.rs       - CI, linting, testing framework detection
  scanner.rs       - TODO/FIXME/HACK, security signals, test mapping
  models.rs        - data model and schema detection
  locations.rs     - key directory pattern recognition
```

## Code style

- No comments unless the logic is non-obvious.
- Keep functions focused: one function, one job.
- Format with `cargo fmt`; lint with `cargo clippy`.

## Development

```bash
cargo build --release
cargo run --release -- /path/to/any/project
```

## Adding a language

1. Find the tree-sitter grammar crate on crates.io (`tree-sitter-<language>`).
2. Add it to `Cargo.toml` as its own Cargo feature; include it in the `all-languages` feature set.
3. Add the extension mapping in `src/lang.rs` (`get_language`).
4. Add the abbreviation in `lang_abbrev`.
5. Test on a real project using that language.

Crates pinned to an incompatible `tree-sitter` version (multiple-version conflict, E0308) or missing the modern `LANGUAGE` constant API cannot be added until the grammar crate republishes against the current `tree-sitter`. See CHANGELOG.md for the current exclusion list.

## Improving detection / reducing false positives

- Frameworks -> `project.rs`; ORMs -> `models.rs`; TODO/security/test patterns -> `scanner.rs`.
- Dead-code exclusions -> `depgraph.rs`; security false-positive filters -> `scanner.rs`.
- Update `formatter.rs` if a new output section is needed.
- Verify against a real project, not a synthetic fixture.

## Testing

This crate is standalone and not bound by the gm-family no-test-framework rule. Prefer verifying changes by running `cargo build --release` and `cargo run --release -- <real project>` against real codebases (as CONTRIBUTING.md prescribes); idiomatic Rust `#[test]` unit/integration tests are acceptable where they add genuine regression coverage for parsing/detection logic.

## Pull requests

1. Branch off `main`.
2. Run `cargo build --release` and `cargo clippy`.
3. Test on at least one real project.
4. Open a PR describing what changed and why.

See `CONTRIBUTING.md` for the full contributor guide.
