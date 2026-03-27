# Changelog

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
