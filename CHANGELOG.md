# Changelog

## Unreleased

### Languages

Expanded `all-languages` from 12 to **28** by adding 16 tree-sitter grammars verified compatible with `tree-sitter@0.24`:

- **Web**: html, css
- **Shell + config**: bash, yaml
- **Functional**: haskell, ocaml, elixir, erlang
- **JVM-adjacent**: scala
- **Modern systems**: zig, swift
- **Scripting**: lua, r, julia
- **Markup + meta**: markdown, regex

Each language is a separate Cargo feature; `default = ["all-languages"]` enables every grammar. Per-language opt-out works via `default-features = false, features = ["javascript","typescript","python"]` etc.

#### Excluded — incompatible with `tree-sitter@0.24` core

The following crates were attempted but rejected after CI surfaced incompatibilities. Re-include when upstream publishes a 0.24-compatible release:

- **Old API (no `LANGUAGE` constant, no `language()` fn matching the expected signature)**: `tree-sitter-make`, `tree-sitter-graphql`, `tree-sitter-nix`, `tree-sitter-svelte`, `tree-sitter-dart`.
- **Pinned to old `tree-sitter` 0.19 / 0.20 transitively (multiple-version conflict)**: `tree-sitter-sql`, `tree-sitter-dockerfile`, `tree-sitter-vue`, `tree-sitter-kotlin`, `tree-sitter-toml`.

When a downstream cargo build sees two different `tree_sitter::Language` types from two different `tree-sitter` crate versions in the dep graph, it fails E0308 even when the rust source looks correct. The fix has to come from the grammar crate's maintainer republishing against a current `tree-sitter`.

Crate API note: modern crates export `tree_sitter_<lang>::LANGUAGE` (a `LanguageFn` that converts to `Language` via `.into()`). `lang.rs` uses this consistently for every supported grammar.

## 0.2.0 (2026-03-27)

### Features

- `--json` flag for structured JSON output — enables programmatic consumption by CI, dashboards, and other tools
- `--cache` flag writes output to `.codeinsight` file in project root for instant reads
- `--read-cache` reads cached output without re-analyzing (sub-millisecond)
- `.codeinsight.toml` config file support — custom ignore directories, ignore file patterns, max file size
- PHP language support (`.php` files)
- C# language support (`.cs` files)
- LLM-optimized output format — ~900 tokens vs ~2500, no emojis, no markdown overhead, all data preserved
- All env vars, routes, and models shown in full (no artificial caps)
- Monorepo script detection from subdirectory package.json and go.mod files
- Go entry point detection (`cmd/` directory or `main.go`)
- README excerpt shown as `About:` line for project purpose context
- Subdirectory README scanning for monorepos
- Framework-aware dead code detection (Next.js pages, Go files, config files excluded)
- Reduced security false positives (test files, JSX attributes, type definitions filtered)
- External dependency detection filters local paths and Node.js builtins
- Tech Stack shows detected frameworks instead of raw pattern counts

## 0.1.0 (2026-03-27)

Initial release.

### Features

- Tree-sitter AST analysis for 11 languages (JS, TS, TSX, Python, Rust, Go, C, C++, Java, Ruby, JSON)
- Framework detection (Next.js, React, Gin, Express, Vue, Angular, Svelte, Nuxt, Remix, Astro, and more)
- Data model detection (Prisma, GORM, TypeORM, Drizzle, Mongoose, Sequelize)
- Dependency graph with circular dependency detection
- Dead code detection
- Git context (branch, recent commits, uncommitted changes, hot files)
- Security scanning (eval usage, hardcoded secrets, SQL injection)
- Test coverage mapping (source-to-test file matching)
- Developer notes (TODO, FIXME, HACK comment extraction)
- Tooling detection (TypeScript config, ESLint, Prettier, Jest, Vitest, CI/CD, Docker)
- Key code locations map (handlers, components, services, middleware, etc.)
- Project metadata from package.json, go.mod, tsconfig.json
- Monorepo/subdirectory scanning for framework detection
- Parallel file processing via rayon
- Compact output optimized for AI tool consumption
