# Contributing to codeinsight

Thanks for your interest in contributing. Here's how to get started.

## Development setup

1. Install the Rust toolchain: https://rustup.rs/
2. Clone the repo:
   ```bash
   git clone https://github.com/faizelmahomed/codeinsight.git
   cd codeinsight
   ```
3. Build:
   ```bash
   cargo build --release
   ```
4. Run on a test project:
   ```bash
   cargo run --release -- /path/to/any/project
   ```

## Project structure

```
src/
  main.rs          — entry point, file collection, parallel dispatch
  lang.rs          — tree-sitter grammar registry
  analyzer.rs      — AST traversal and entity extraction
  depgraph.rs      — dependency graph and dead code detection
  formatter.rs     — markdown output generation
  project.rs       — package.json / go.mod metadata
  git.rs           — git context
  tooling.rs       — CI/linting/testing detection
  scanner.rs       — TODO/FIXME, security, test mapping
  models.rs        — data model detection
  locations.rs     — key directory recognition
```

## How to contribute

### Reporting bugs

Open an issue with:
- The command you ran
- The output you got
- What you expected instead
- Your OS and Rust version (`rustc --version`)

### Adding a language

1. Find the tree-sitter grammar crate on crates.io (search `tree-sitter-<language>`)
2. Add it to `Cargo.toml`
3. Add the extension mapping in `src/lang.rs` in the `get_language` function
4. Add the abbreviation in `lang_abbrev`
5. Test on a real project using that language

### Improving detection

If codeinsight misses a framework, ORM, or pattern:
1. Check which module should handle it (project.rs for frameworks, models.rs for ORMs, scanner.rs for patterns)
2. Add the detection logic
3. Update the formatter if a new section is needed
4. Test on a real project

### Reducing false positives

If codeinsight flags something incorrectly (e.g., marking a Next.js page as orphaned):
1. Check `depgraph.rs` for dead code exclusions
2. Check `scanner.rs` for security false positive filters
3. Add the appropriate exclusion pattern
4. Test to verify the false positive is gone without losing true positives

## Code style

- No comments unless the logic is non-obvious
- Keep functions focused — one function, one job
- Use `rustfmt` for formatting: `cargo fmt`
- Use `clippy` for linting: `cargo clippy`

## Pull requests

1. Fork the repo
2. Create a branch: `git checkout -b my-feature`
3. Make your changes
4. Run `cargo build --release` and `cargo clippy`
5. Test on at least one real project
6. Push and open a PR with a clear description of what changed and why
