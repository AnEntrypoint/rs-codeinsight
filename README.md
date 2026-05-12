# codeinsight

[![CI](https://github.com/faizelmahomed/codeinsight/actions/workflows/ci.yml/badge.svg)](https://github.com/faizelmahomed/codeinsight/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![Languages](https://img.shields.io/badge/languages-13-green.svg)](#supported-languages)

Fast codebase analyzer powered by tree-sitter. Gives AI coding tools (Claude Code, Cursor, Copilot, etc.) instant, comprehensive project understanding in a single pass.

Built in Rust for speed. Analyzes 500-file projects in under 200ms. Zero runtime dependencies — single static binary.

## Why

AI coding assistants waste their first few interactions exploring your codebase — reading package.json, grepping for patterns, checking directory structure. This burns tokens, adds latency, and still produces an incomplete picture.

codeinsight solves this by analyzing your entire codebase in one pass and producing a token-efficient overview that tells the AI everything it needs to know: what frameworks you use, what your data models look like, where the key code lives, how code should be written, what's tested, what's broken, and what's actively being worked on.

One command. Under 200ms. The AI starts working immediately instead of exploring.

## What it produces

codeinsight outputs a compact, LLM-optimized report (~900 tokens for a 500-file project) covering:

| What Claude learns | Example |
|---|---|
| **Project identity** | `Project: creator-store v0.1.0 (web-app)` |
| **Purpose (from README)** | `About: A stan.store alternative for the Saudi market...` |
| **Tech stack** | `Stack: Next.js, React, Gin, Stripe, AWS, Tailwind, GORM` |
| **Scale** | `505 files \| 89.9kL \| Go 41%, TSX 31%, TS 26%` |
| **How to run** | `Dev: cd creator-store && next dev \| Build: next build \| Test: jest` |
| **All env vars** | `Env: DATABASE_URL, STRIPE_SECRET_KEY, REDIS_URL, ...` (full list, no caps) |
| **All API routes** | `Routes: /auth/login, /products/create, /subscriptions/checkout, ...` (full list) |
| **All domain models** | `Models: User, Store, Product, Subscription, Payout, Block, ...` (full list) |
| **Where code lives** | `Dirs: src/app(140), internal(123), components(88), handlers(45)` |
| **Git activity** | `Branch: main \| 4 uncommitted` + hot files |
| **Developer notes** | `TODO page.tsx:56 "Add sales count from Go backend"` |
| **Test coverage** | `Tests: 27/478 (5%)` |
| **Dependencies** | `Deps: react(146), next(103), gorm.io(66), gin(61)` |
| **Tooling** | `Tooling: ESLint, Prettier, Jest, GitHub Actions, Docker` |
| **Security signals** | `Security: eval() in 4 test files` |
| **Coding conventions** | `Conventions[TS]: 2-space, single quotes, arrow functions, named exports` |
| **Health issues** | `Issues: 40 orphaned, 4 unused exports, 35 large files` |
| **Exported API** | `Exports: auth-client.ts:33:getCSRFToken, badge.tsx:30:Badge` |

Every env var, every route, every model is shown in full — no artificial caps. Claude gets reference data it can use directly, not summaries it has to verify.

The output is designed for LLM consumption: no emojis, no markdown formatting overhead, no legends. Pure data, self-describing labels, minimal tokens.

## Example output

Here's real output from analyzing a full-stack creator store platform (Next.js + Go, 505 files):

<details>
<summary>Click to expand</summary>

```
Project: creator-store v0.1.0 (go)
About: A stan.store alternative built specifically for the Saudi Arabian market. This platform enables creators to build customizable link-in-bio storefronts to sell digital and physical products.
Stack: Next.js, React, Gin, Stripe, AWS, Tailwind, GORM
505 files | 89.9kL | Go 41%, TSX 31%, TS 26%, JSON 0%
Dev: cd creator-store && next dev -p 1234 | Build: cd creator-store && next build | Test: cd creator-store && jest

Env: AWS_ACCESS_KEY_ID, AWS_REGION, AWS_S3_BUCKET, AWS_SECRET_ACCESS_KEY, CI, DATABASE_URL, DOWNLOAD_TOKEN_SECRET, FROM_EMAIL, FROM_NAME, NEXT_PUBLIC_ALLOWED_ORIGINS, NEXT_PUBLIC_API_URL, NEXT_PUBLIC_APP_URL, NEXT_PUBLIC_STRIPE_PUBLISHABLE_KEY, NODE_ENV, RESEND_API_KEY, SECRET, SENTRY_ORG, SENTRY_PROJECT, STRIPE_SECRET_KEY, UPSTASH_REDIS_REST_TOKEN, UPSTASH_REDIS_REST_URL
Routes: /auth/login, /auth/register, /auth/logout, /auth/session, /auth/forgot-password, /auth/reset-password, /auth/verify-email, /user/me, /user/update-profile, /user/change-password, /user/sessions, /products/create, /products/list, /products/update, /products/delete, /subscriptions/plans, /subscriptions/checkout, /subscriptions/cancel, /subscriptions/portal, /blocks, /blocks/create, /blocks/update, /blocks/delete, /payouts/balance, /payouts/request, /payouts/transactions, /stores/check-availability, /stores/create-initial, /stores/me, /orders
Models: User, Account, Session, Store, StoreCustomization, Product, ProductVariant, Order, OrderItem, Download, Block, BlockClick, Experience, Education, Booking, Integration, Subscription, SubscriptionPlan, SubscriptionInvoice, APIKey, StripeAccount, PayoutRequest, CreatorBalance, EarningsTransaction, Analytics, DiscountCode, Affiliate (+19)
Schema: go-creator-store/internal/domain/

Dirs: creator-store/src/app(140), go-creator-store/internal(123), creator-store/src/components(88), creator-store/src/lib(69), go-creator-store/internal/handlers(45), creator-store/__tests__(27), go-creator-store/internal/domain(15), go-creator-store/internal/services(12), go-creator-store/internal/middleware(12)
Branch: main | 4 uncommitted
Hot: marketplace.json(95), pre-tool-use-hook.js(40), gm.md(21)
TODO page.tsx:56 "Add sales count from Go backend when available" | TODO webhook.go:358 "Notify store owner"
Tests: 27/478 (5%)
Deps: react(146), next(103), gorm.io(66), gin(61), lucide-react(58), framer-motion(29)
Security: eval() in CVE-008-xss-protection.test.ts:242, file_validator_test.go:794
Conventions[Go]: tabs, if err != nil, snake_case files
Conventions[TS]: 4-space, single quotes, no semicolons, arrow functions, named exports, kebab-case files
Conventions[TSX]: 4-space, no semicolons, arrow functions, named exports, PascalCase files

Issues: 40 orphaned files, 4 unused exports, 22 single-use files, 35 files >500 lines, 161 functions >50 lines, 109 duplicated groups
Exports: badge.tsx:30:Badge, skeleton.tsx:3:Skeleton, auth-client.ts:33:getCSRFToken, auth-client.ts:49:withCSRFToken
```

</details>

From this output, an AI assistant instantly knows: this is a **Next.js + Go/Gin** creator store for the Saudi market with **Stripe payments**, **Redis caching**, **46 GORM models**, handlers at `go-creator-store/internal/handlers/`, **5% test coverage**, two open TODOs, and that TypeScript uses single quotes with arrow functions while Go uses tabs with `if err != nil`.

## Inspired by

codeinsight was inspired by [mcp-thorns](https://github.com/AnEntrypoint/mcp-thorns) — a Node.js codebase analyzer that pioneered the idea of giving AI tools a one-shot project overview using tree-sitter AST analysis. mcp-thorns demonstrated that structured codebase context dramatically improves AI coding assistant performance.

codeinsight builds on that foundation with a Rust implementation for faster analysis, additional detection capabilities (git context, security scanning, test mapping, data model detection, convention detection), and framework-aware dead code analysis.

## Performance

| Codebase | Files | Lines | Time |
|---|---|---|---|
| Small CLI tool | 14 | 1,600 | 60ms |
| Full-stack app (Next.js + Go) | 505 | 90,000 | 180ms |
| Large plugin ecosystem | 772 | 118,000 | 750ms |

## How it compares

| Feature | codeinsight | tokei / scc / cloc | mcp-thorns | tree-sitter CLI |
|---|---|---|---|---|
| Line counting | Yes | Yes | Yes | No |
| Language detection | Yes | Yes | Yes | Yes |
| Framework detection | Yes (25+ frameworks) | No | Yes | No |
| Dependency graph | Yes (cross-language) | No | Yes (JS only) | No |
| Dead code detection | Yes (framework-aware) | No | Yes | No |
| Data model detection | Yes (6 ORMs) | No | No | No |
| Git context | Yes | No | No | No |
| Convention detection | Yes (per-language) | No | No | No |
| Security scanning | Yes | No | No | No |
| Test coverage mapping | Yes | No | No | No |
| TODO/FIXME scanning | Yes | No | No | No |
| Key locations map | Yes | No | No | No |
| tsconfig alias resolution | Yes | No | No | No |
| Go/Python/Rust imports | Yes | No | No | No |
| JSON output | Yes | Yes | No | Yes |
| Caching | Yes | No | No | No |
| Config file | Yes | Yes | No | No |
| AI-optimized output | Yes (~900 tokens) | No | Yes (~2500 tokens) | No |
| Speed (500 files) | ~180ms | ~50ms | ~6-15s | ~200ms |
| Runtime deps | None (static binary) | None | Node.js + 13 native modules | None |
| Languages supported | 13 | 200+ | 13 | Per grammar |

**tokei / scc / cloc** are excellent line-counting tools. They answer "how big is this codebase?" but not "what does this codebase do?" — no framework detection, no dependency analysis, no domain model extraction, no conventions.

**mcp-thorns** pioneered the AI-optimized codebase overview concept. codeinsight builds on that idea with Rust for speed, additional analysis capabilities, and a more token-efficient output format.

**tree-sitter CLI** provides raw AST output. codeinsight uses tree-sitter internally but adds the analysis layer on top — extracting meaning, not just structure.

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

### From npm (coming soon)

```bash
npm install -g codeinsight
```

### Prebuilt binaries (coming soon)

Prebuilt binaries for Windows x64, macOS ARM, macOS Intel, and Linux x64 will be available on the [GitHub Releases](https://github.com/faizelmahomed/codeinsight/releases) page.

## Usage

### Basic usage

```bash
codeinsight /path/to/your/project
```

### JSON output

```bash
codeinsight --json /path/to/your/project
```

Outputs structured JSON for programmatic consumption by CI pipelines, dashboards, or other tools.

### Caching

```bash
# Analyze and cache the result
codeinsight --cache /path/to/your/project

# Read cached result instantly (no re-analysis)
codeinsight --read-cache /path/to/your/project
```

The cache is written as a **pair** in the project root:

- `.codeinsight` — the analysis output (text or JSON, matching the flags used)
- `.codeinsight.digest` — an md5 of `(sorted [rel_path | mtime_secs], GIT_HEAD, git status --porcelain line count)` computed over every file `collect_files` walked

`--cache` writes both atomically. `--read-cache` re-computes the live digest, compares
against `.codeinsight.digest`, and serves the cached content verbatim on match. On
mismatch — file added, deleted, renamed, edited, branch switched, or working tree
dirtied — it transparently regenerates: runs a fresh analyze, rewrites both files,
and prints the fresh output. Consumers always get a result; there is no cache-miss
exit code. Stderr prints `[codeinsight cache stale or missing; running fresh analyze]`
on the regenerate path so callers can observe rebuilds.

Do not read `.codeinsight` directly without first validating `.codeinsight.digest`
against the live tree — the file alone cannot tell you whether it matches the
current source.

Use `--cache` in session-start hooks for instant reads on subsequent prompts.

### Configuration

Create a `.codeinsight.toml` in your project root to customize behavior:

```toml
[ignore]
dirs = ["generated", "vendor", "legacy"]
files = ["*.generated.ts", "*.min.js"]

[limits]
max_file_size = 200000
```

### With AI coding tools

#### Claude Code

Use codeinsight as part of a [sentinel-cc](https://github.com/faizelmahomed/sentinel-cc) session-start hook, or run it manually:

```bash
codeinsight .
```

The output is injected into Claude's context at session start, giving it full project understanding before the first interaction.

#### Cursor / Copilot / Other

Run codeinsight and paste the output into your AI tool's context or system prompt:

```bash
codeinsight . | pbcopy  # macOS — copies to clipboard
```

#### CI / Code Review

Run codeinsight in CI to generate a project snapshot with each PR:

```bash
codeinsight . > .codeinsight-report.md
```

Use the Issues section to flag problems in code review.

## Supported languages

codeinsight uses tree-sitter grammars for accurate AST-level analysis:

| Language | Extensions | Framework detection |
|---|---|---|
| JavaScript | `.js`, `.mjs`, `.cjs`, `.jsx` | React, Express, Fastify, Koa, Hono, Elysia |
| TypeScript | `.ts`, `.tsx` | Next.js, Angular, Svelte, Nuxt, Remix, Astro |
| Python | `.py` | Django, Flask, FastAPI |
| Go | `.go` | Gin, Echo, Fiber, Chi, Gorilla Mux |
| Rust | `.rs` | Actix, Axum, Rocket |
| C | `.c`, `.h` | — |
| C++ | `.cpp`, `.cc`, `.cxx`, `.hpp` | — |
| Java | `.java` | Spring |
| Ruby | `.rb` | Rails |
| PHP | `.php` | Laravel, Symfony |
| C# | `.cs` | .NET, ASP.NET |
| JSON | `.json` | package.json, tsconfig.json parsed for metadata |

## Dependency graph

codeinsight builds a full cross-language dependency graph with import resolution for:

- **JavaScript/TypeScript**: ES modules, CommonJS `require()`, dynamic `import()`, tsconfig path aliases (`@/` imports), barrel re-export tracing through `index.ts` files
- **Go**: package-level imports resolved to all files in the target package directory
- **Python**: dot-notation imports (`from module.sub import thing`), relative imports, `__init__.py` resolution
- **Rust**: `mod foo;` declarations resolved to `foo.rs` or `foo/mod.rs`

The graph powers dead code detection (framework-aware — excludes Next.js pages, Go files, config files), circular dependency detection, coupling analysis, and entry point identification.

## Convention detection

codeinsight detects per-language coding conventions from actual code patterns:

| Convention | JS/TS/TSX | Go | Python | Rust |
|---|---|---|---|---|
| Indent style | 2-space / 4-space / tabs | tabs | 4-space | 4-space |
| Quote style | single / double | — | single / double | — |
| Semicolons | yes / no | — | — | — |
| Function style | arrow / declaration | — | — | — |
| Export style | named / default | — | — | — |
| Import style | @/ / relative | — | — | — |
| File naming | kebab / camel / Pascal / snake | snake_case | snake_case | snake_case |
| Error handling | — | if err != nil | — | ? / unwrap / expect |

Output: one line per language, only showing conventions with a clear majority pattern.

```
Conventions[TS]: 2-space, single quotes, no semicolons, arrow functions, named exports, @/ imports
Conventions[Go]: tabs, if err != nil, snake_case files
```

## How it works

### 1. File discovery

Walks the project directory respecting `.gitignore` rules. Automatically skips `node_modules`, `.git`, `dist`, `build`, `target`, `.next`, `__pycache__`, `vendor`, and other common build/cache directories. Respects `.codeinsight.toml` custom ignores.

### 2. Parallel parsing

All files are parsed in parallel using `rayon`. Each file gets the appropriate tree-sitter grammar. Parallelism saturates all CPU cores — this is why a 500-file project finishes in 180ms.

### 3. AST analysis

Single traversal per file extracts: functions, classes, imports, exports, call patterns, async patterns, error handling, constants, global state, env vars, URLs, routes, storage patterns, event patterns, identifiers, and convention indicators (quotes, indentation, semicolons, function style, export style).

### 4. Dependency graph

Resolves imports across all supported languages (JS/TS path aliases, Go package imports, Python dot imports, Rust mod declarations). Traces through barrel re-exports. Builds coupling metrics, detects circular dependencies, identifies orphaned files and entry points.

### 5. Additional analysis

- **Data models**: Detects Prisma schemas, GORM structs, TypeORM/Drizzle/Mongoose/Sequelize entities
- **Key locations**: Maps directories to roles (handlers, components, services, middleware, tests)
- **Git context**: Branch, recent commits, uncommitted changes, most-changed files
- **Tooling**: TypeScript config, linting, testing frameworks, CI/CD, Docker, env files
- **Security**: eval() usage, potential hardcoded secrets, SQL injection patterns
- **Test mapping**: Matches source files to test files by naming convention
- **Conventions**: Per-language coding style detection

### 6. Compact output

Formatted for LLM consumption: no emojis, no markdown overhead, no legends. Self-describing labels. Full reference data (all env vars, all routes, all models). Typically ~900 tokens for a 500-file project.

## Architecture

```
src/
  main.rs          — entry point, file collection, parallel dispatch, CLI flags
  lang.rs          — tree-sitter grammar registry (13 languages)
  analyzer.rs      — AST traversal, entity extraction, convention counting
  depgraph.rs      — dependency graph, import resolution, dead code detection
  formatter.rs     — compact LLM-optimized text output
  json_output.rs   — structured JSON output
  project.rs       — package.json / go.mod / tsconfig.json metadata
  config.rs        — .codeinsight.toml configuration loading
  conventions.rs   — per-language convention aggregation
  git.rs           — git context (branch, commits, hot files)
  tooling.rs       — CI, linting, testing framework detection
  scanner.rs       — TODO/FIXME/HACK, security signals, test mapping
  models.rs        — data model and schema detection
  locations.rs     — key directory pattern recognition
```

## Why your README matters

codeinsight reads your project's README and includes an excerpt in the output. This gives AI tools something code analysis alone can never provide: the **purpose and intent** behind your project.

Code tells an AI *what* the project does technically. The README tells it *why* it exists, *who* it's for, and *what problem* it solves. An AI that knows "this is a stan.store alternative for the Saudi Arabian market" makes fundamentally different decisions than one that only sees "Next.js + Go + Stripe."

If your project doesn't have a README, or has a bare-bones one, codeinsight can't provide this context and the AI assistant starts with a purely technical view of your code. A good first paragraph in your README — one or two sentences explaining what the project is and who it's for — dramatically improves the quality of AI-assisted development on your codebase.

**A good README first paragraph looks like:**
- "A marketplace platform for creators to sell digital products, built for the Saudi Arabian market."
- "Internal CLI tool that generates compliance reports from our PostgreSQL audit logs."
- "React component library implementing our company's design system."

**A poor README first paragraph (from the AI's perspective):**
- "## Installation\n\nnpm install my-project" (no context about what the project does)
- "TODO: add description" (nothing to work with)
- No README at all

The investment is one sentence. The payoff is every AI interaction on your codebase understanding the context.

## Contributing

Contributions are welcome. Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT
