# Changelog

## 0.1.0 (2026-03-27)

Initial release.

### Features

- Tree-sitter AST analysis for 11 languages (JS, TS, TSX, Python, Rust, Go, C, C++, Java, Ruby, JSON)
- Framework detection (Next.js, React, Gin, Express, Vue, Angular, Svelte, Nuxt, Remix, Astro, and more)
- Data model detection (Prisma, GORM, TypeORM, Drizzle, Mongoose, Sequelize)
- Dependency graph with circular dependency detection
- Dead code detection (framework-aware — excludes Next.js pages, Go files, config files)
- Git context (branch, recent commits, uncommitted changes, hot files)
- Security scanning (eval usage, hardcoded secrets, SQL injection)
- Test coverage mapping (source-to-test file matching)
- Developer notes (TODO, FIXME, HACK comment extraction)
- Tooling detection (TypeScript config, ESLint, Prettier, Jest, Vitest, CI/CD, Docker)
- Key code locations map (handlers, components, services, middleware, etc.)
- Project metadata from package.json, go.mod, tsconfig.json
- Monorepo/subdirectory scanning for framework detection
- Parallel file processing via rayon
- Compact markdown output optimized for AI tool consumption
