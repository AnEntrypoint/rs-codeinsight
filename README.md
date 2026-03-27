# codeinsight

Fast codebase analyzer powered by tree-sitter. Gives AI coding tools (Claude Code, Cursor, Copilot, etc.) instant, comprehensive project understanding in a single pass.

Built in Rust for speed. Analyzes 500-file projects in under 200ms. Zero runtime dependencies — single static binary.

## Why

AI coding assistants waste their first few interactions exploring your codebase — reading package.json, grepping for patterns, checking directory structure. This burns tokens, adds latency, and still produces an incomplete picture.

codeinsight solves this by analyzing your entire codebase in one pass and producing a structured overview that tells the AI everything it needs to know: what frameworks you use, what your data models look like, where the key code lives, what's tested, what's broken, and what's actively being worked on.

One command. Under 200ms. The AI starts working immediately instead of exploring.

## What it produces

codeinsight outputs a structured markdown report with up to 21 sections, each designed to answer a specific question an AI assistant would otherwise need multiple tool calls to figure out:

| Section | What it answers |
|---|---|
| **Project Info** | What is this project? What version? What type (CLI, web app, library)? |
| **Quick Start** | How do I run this locally? |
| **Header** | How big is this codebase? How complex? |
| **Languages** | What languages is this written in? What's the primary one? |
| **Tech Stack** | What frameworks and services does this use? (Next.js, Gin, Stripe, Redis, etc.) |
| **Code Patterns** | Is this async-heavy? How much error handling? What internal functions are most called? |
| **I/O & Integration** | What env vars are needed? What external APIs are called? What routes exist? |
| **Features** | How is the code organized by feature area? |
| **Data Layer** | What are the domain models? Where's the schema? What ORM? |
| **Key Locations** | Where are the handlers, components, services, middleware, tests? |
| **Git Context** | What branch? Recent commits? Uncommitted work? What files change most? |
| **Tooling** | TypeScript strict mode? ESLint? Jest? CI/CD? Docker? |
| **Developer Notes** | Any TODOs, FIXMEs, or HACKs in the code? |
| **Security** | Any eval() usage, hardcoded secrets, SQL injection patterns? |
| **Test Map** | Which files have tests? Which don't? What's the coverage ratio? |
| **Code Organization** | Which files are too large? Which functions have too many parameters? |
| **Architecture** | What's the dependency graph? Which files are most connected? Any circular deps? |
| **API Surface** | What functions are exported? What classes exist? What are the entry points? |
| **Issues** | Code duplication, oversized files, overly complex functions |
| **Dead Code** | Unused exports, orphaned files, possibly dead code (framework-aware) |
| **Modules** | How is the project organized at the top level? |

Sections only appear when they have meaningful content. A 3-file project gets a compact report. A 500-file monorepo gets the full analysis.

## Performance

| Codebase | Files | Lines | Time |
|---|---|---|---|
| Small CLI tool | 14 | 1,600 | 60ms |
| Full-stack app (Next.js + Go) | 505 | 90,000 | 180ms |
| Large plugin ecosystem | 772 | 118,000 | 750ms |

For comparison, the Node.js-based alternative (mcp-thorns) takes 6-15 seconds on the same codebases. codeinsight is 25-45x faster.

## Installation

### From source (requires Rust)

```bash
git clone https://github.com/faizelmahomed/codeinsight.git
cd codeinsight
cargo build --release
```

The binary will be at `target/release/codeinsight` (or `target/release/codeinsight.exe` on Windows).

Optionally, copy it to a directory on your PATH:

```bash
# Linux / macOS
cp target/release/codeinsight ~/.local/bin/

# Windows (PowerShell)
Copy-Item target\release\codeinsight.exe $env:USERPROFILE\.local\bin\
```

### From crates.io (coming soon)

```bash
cargo install codeinsight
```

### Prebuilt binaries (coming soon)

Prebuilt binaries for Windows x64, macOS ARM, macOS Intel, and Linux x64 will be available on the [GitHub Releases](https://github.com/faizelmahomed/codeinsight/releases) page.

## Usage

### Basic usage

```bash
codeinsight /path/to/your/project
```

This prints the full analysis to stdout. Pipe it to a file or use it in scripts:

```bash
# Save to file
codeinsight ./my-project > analysis.md

# Use with AI tools
codeinsight ./my-project | pbcopy  # macOS — copies to clipboard
```

### With AI coding tools

#### Claude Code

Use codeinsight as part of a session-start hook or run it manually:

```bash
codeinsight .
```

The output is designed to be injected into Claude's context at session start, giving it full project understanding before the first interaction.

#### Cursor / Copilot / Other

Run codeinsight and paste the output into your AI tool's context or system prompt. The markdown format is designed to be token-efficient while maximizing information density.

#### CI / Code Review

Run codeinsight in CI to generate a project snapshot with each PR:

```bash
codeinsight . > .codeinsight-report.md
```

Use the Issues and Dead Code sections to flag problems in code review.

## Supported languages

codeinsight uses tree-sitter grammars for accurate AST-level analysis. The following languages are fully supported:

| Language | Extensions | Framework detection |
|---|---|---|
| JavaScript | `.js`, `.mjs`, `.cjs`, `.jsx` | React, Express, Fastify, Koa, Hono, Elysia |
| TypeScript | `.ts`, `.tsx` | Next.js, Angular, Svelte, Nuxt, Remix, Astro |
| Python | `.py` | Django, Flask, FastAPI (via requirements/pyproject) |
| Go | `.go` | Gin, Echo, Fiber, Chi, Gorilla Mux |
| Rust | `.rs` | Actix, Axum, Rocket (via Cargo.toml) |
| C | `.c`, `.h` | — |
| C++ | `.cpp`, `.cc`, `.cxx`, `.hpp` | — |
| Java | `.java` | Spring (via pom.xml/build.gradle) |
| Ruby | `.rb` | Rails (via Gemfile) |
| JSON | `.json` | package.json, tsconfig.json parsed for metadata |

Files over 200KB are automatically skipped (typically build artifacts or generated code).

## How it works

### 1. File discovery

codeinsight walks the project directory using the `ignore` crate, which respects `.gitignore` rules. It automatically skips:

- `node_modules`, `.git`, `dist`, `build`, `target`
- `.next`, `.nuxt`, `coverage`, `__pycache__`
- `.venv`, `vendor`, `.cache`, `.output`
- Files over 200KB

### 2. Parallel parsing

All discovered files are parsed in parallel using `rayon`. Each file is parsed with the appropriate tree-sitter grammar. This is where the speed comes from — tree-sitter parsing is fast, and parallelism saturates all CPU cores.

### 3. AST analysis

For each file, codeinsight traverses the AST and extracts:

- **Functions**: name, line number, parameter count, body length, structural hash (for duplication detection)
- **Classes/structs/enums**: name, line number
- **Imports/exports**: paths, names, CommonJS and ES module styles
- **Call patterns**: which functions are called most, internal vs external
- **Async patterns**: async/await usage, Promises, callbacks, .then/.catch chains
- **Error handling**: try/catch blocks, throw statements
- **Constants and global state**: module-level declarations
- **Environment variables**: `process.env.X` references
- **URLs and routes**: HTTP endpoints, API routes
- **Storage patterns**: file I/O, JSON operations, SQL queries
- **Event patterns**: emitters and listeners
- **Identifiers**: all variable/function/type names with frequency counts

### 4. Dependency graph

Import paths are resolved to actual files. codeinsight builds a full dependency graph showing:

- Which files import which
- Which files are imported by others (coupling)
- Circular dependency chains
- Cross-module dependencies
- Orphaned files (not imported by anything)
- Entry points (imported by others but don't import anything)

### 5. Dead code detection

Using the dependency graph, codeinsight identifies:

- **Unused exports**: files that export functions but nothing imports them
- **Orphaned files**: completely isolated files with no connections
- **Possibly dead**: files used by only one other file (leaf nodes)
- **Framework-aware**: Next.js pages (`page.tsx`, `layout.tsx`), Go files, and config files are excluded from false positive detection

### 6. Additional scanners

Beyond AST analysis, codeinsight runs text-level scanners for:

- **Developer notes**: TODO, FIXME, HACK comments with file and line context
- **Security signals**: `eval()` usage, potential hardcoded secrets (with false positive filtering for test files, JSX attributes, type definitions), SQL string interpolation
- **Test mapping**: matches source files to their test files by naming convention

### 7. Project context

codeinsight reads project metadata from:

- `package.json` (name, version, description, scripts, dependencies)
- `go.mod` (module path, Go dependencies)
- `tsconfig.json` (strict mode, target)
- CI configuration files (GitHub Actions, GitLab CI, etc.)
- Docker files
- Environment files (`.env`, `.env.example`)
- README files (first paragraph excerpt)

It scans subdirectories one level deep to detect monorepo structures with multiple package.json or go.mod files.

### 8. Compact formatting

All analysis results are formatted into a single markdown document optimized for AI consumption:

- Sections only appear when they contain meaningful data
- Lists are capped (typically 5-15 items) with `(+N)` overflow indicators
- File paths are shortened to filenames where unambiguous
- Numeric values use `k` suffix for thousands
- Emoji section headers for quick visual scanning
- The entire output is typically 1,500-3,000 tokens — less than what an AI would spend exploring the codebase manually

## Architecture

```
src/
  main.rs          — entry point, file collection, parallel dispatch
  lang.rs          — tree-sitter grammar registry (11 languages)
  analyzer.rs      — AST traversal and entity extraction
  depgraph.rs      — dependency graph, dead code, circular dep detection
  formatter.rs     — compact markdown output generation
  project.rs       — package.json / go.mod / project metadata
  git.rs           — git context (branch, commits, hot files)
  tooling.rs       — CI, linting, testing framework detection
  scanner.rs       — TODO/FIXME/HACK, security signals, test mapping
  models.rs        — data model and schema detection
  locations.rs     — key directory pattern recognition
```

## Contributing

Contributions are welcome. Please open an issue first to discuss what you'd like to change.

### Building from source

```bash
git clone https://github.com/faizelmahomed/codeinsight.git
cd codeinsight
cargo build --release
cargo test
```

### Adding a new language

1. Add the tree-sitter grammar crate to `Cargo.toml`
2. Add the extension mapping in `src/lang.rs`
3. The analyzer, dependency graph, and formatter work automatically for any tree-sitter grammar

### Running on your own project

```bash
cargo run --release -- /path/to/your/project
```

## License

MIT
