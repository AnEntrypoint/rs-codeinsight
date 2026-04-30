use tree_sitter::Language;

pub static NODE_BUILTINS: &[&str] = &[
    "fs", "path", "os", "util", "crypto", "http", "https", "url", "stream",
    "events", "child_process", "assert", "buffer", "querystring", "zlib",
    "net", "tls", "dns", "cluster", "readline", "worker_threads",
    "node:fs", "node:path", "node:os", "node:util", "node:crypto",
    "node:http", "node:https", "node:url", "node:stream", "node:events",
    "node:child_process", "node:assert", "node:buffer", "node:test",
];

pub static KNOWN_SERVICES: &[(&str, &str)] = &[
    ("stripe", "Stripe"), ("@stripe", "Stripe"),
    ("redis", "Redis"), ("ioredis", "Redis"),
    ("prisma", "Prisma"), ("@prisma", "Prisma"),
    ("drizzle-orm", "Drizzle"), ("mongoose", "MongoDB"), ("mongodb", "MongoDB"),
    ("pg", "PostgreSQL"), ("mysql2", "MySQL"),
    ("@aws-sdk", "AWS"), ("aws-sdk", "AWS"),
    ("firebase", "Firebase"), ("@supabase", "Supabase"),
    ("socket.io", "Socket.IO"), ("graphql", "GraphQL"), ("@apollo", "Apollo"),
    ("tailwindcss", "Tailwind"), ("@sentry", "Sentry"), ("sentry", "Sentry"),
    ("zod", "Zod"), ("trpc", "tRPC"), ("@trpc", "tRPC"),
];

pub struct LangDef {
    pub name: &'static str,
    pub language: Language,
}

pub fn get_language(ext: &str) -> Option<LangDef> {
    match ext {
        #[cfg(feature = "javascript")]
        ".js" | ".mjs" | ".cjs" | ".jsx" => Some(LangDef { name: "JavaScript", language: tree_sitter_javascript::LANGUAGE.into() }),
        #[cfg(feature = "typescript")]
        ".ts" => Some(LangDef { name: "TypeScript", language: tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into() }),
        #[cfg(feature = "typescript")]
        ".tsx" => Some(LangDef { name: "TSX", language: tree_sitter_typescript::LANGUAGE_TSX.into() }),
        #[cfg(feature = "python")]
        ".py" => Some(LangDef { name: "Python", language: tree_sitter_python::LANGUAGE.into() }),
        #[cfg(feature = "rust")]
        ".rs" => Some(LangDef { name: "Rust", language: tree_sitter_rust::LANGUAGE.into() }),
        #[cfg(feature = "go")]
        ".go" => Some(LangDef { name: "Go", language: tree_sitter_go::LANGUAGE.into() }),
        #[cfg(feature = "c")]
        ".c" | ".h" => Some(LangDef { name: "C", language: tree_sitter_c::LANGUAGE.into() }),
        #[cfg(feature = "cpp")]
        ".cpp" | ".cc" | ".cxx" | ".hpp" => Some(LangDef { name: "C++", language: tree_sitter_cpp::LANGUAGE.into() }),
        #[cfg(feature = "java")]
        ".java" => Some(LangDef { name: "Java", language: tree_sitter_java::LANGUAGE.into() }),
        #[cfg(feature = "ruby")]
        ".rb" => Some(LangDef { name: "Ruby", language: tree_sitter_ruby::LANGUAGE.into() }),
        #[cfg(feature = "json")]
        ".json" => Some(LangDef { name: "JSON", language: tree_sitter_json::LANGUAGE.into() }),
        #[cfg(feature = "php")]
        ".php" => Some(LangDef { name: "PHP", language: tree_sitter_php::LANGUAGE_PHP.into() }),
        #[cfg(feature = "csharp")]
        ".cs" => Some(LangDef { name: "C#", language: tree_sitter_c_sharp::LANGUAGE.into() }),
        #[cfg(feature = "html")]
        ".html" | ".htm" => Some(LangDef { name: "HTML", language: tree_sitter_html::LANGUAGE.into() }),
        #[cfg(feature = "css")]
        ".css" => Some(LangDef { name: "CSS", language: tree_sitter_css::LANGUAGE.into() }),
        #[cfg(feature = "bash")]
        ".sh" | ".bash" | ".zsh" => Some(LangDef { name: "Bash", language: tree_sitter_bash::LANGUAGE.into() }),
        #[cfg(feature = "yaml")]
        ".yaml" | ".yml" => Some(LangDef { name: "YAML", language: tree_sitter_yaml::LANGUAGE.into() }),
        #[cfg(feature = "markdown")]
        ".md" | ".markdown" => Some(LangDef { name: "Markdown", language: tree_sitter_md::LANGUAGE.into() }),
        #[cfg(feature = "scala")]
        ".scala" | ".sc" => Some(LangDef { name: "Scala", language: tree_sitter_scala::LANGUAGE.into() }),
        #[cfg(feature = "haskell")]
        ".hs" => Some(LangDef { name: "Haskell", language: tree_sitter_haskell::LANGUAGE.into() }),
        #[cfg(feature = "ocaml")]
        ".ml" | ".mli" => Some(LangDef { name: "OCaml", language: tree_sitter_ocaml::LANGUAGE_OCAML.into() }),
        #[cfg(feature = "elixir")]
        ".ex" | ".exs" => Some(LangDef { name: "Elixir", language: tree_sitter_elixir::LANGUAGE.into() }),
        #[cfg(feature = "erlang")]
        ".erl" | ".hrl" => Some(LangDef { name: "Erlang", language: tree_sitter_erlang::LANGUAGE.into() }),
        #[cfg(feature = "zig")]
        ".zig" => Some(LangDef { name: "Zig", language: tree_sitter_zig::LANGUAGE.into() }),
        #[cfg(feature = "lua")]
        ".lua" => Some(LangDef { name: "Lua", language: tree_sitter_lua::LANGUAGE.into() }),
        #[cfg(feature = "regex")]
        ".regex" => Some(LangDef { name: "Regex", language: tree_sitter_regex::LANGUAGE.into() }),
        #[cfg(feature = "r")]
        ".r" | ".R" => Some(LangDef { name: "R", language: tree_sitter_r::LANGUAGE.into() }),
        #[cfg(feature = "julia")]
        ".jl" => Some(LangDef { name: "Julia", language: tree_sitter_julia::LANGUAGE.into() }),
        #[cfg(feature = "swift")]
        ".swift" => Some(LangDef { name: "Swift", language: tree_sitter_swift::LANGUAGE.into() }),
        _ => None,
    }
}

pub fn lang_abbrev(name: &str) -> &str {
    match name {
        "JavaScript" => "JS",
        "TypeScript" => "TS",
        "TSX" => "TSX",
        "Python" => "Py",
        "Rust" => "Rs",
        "Go" => "Go",
        "C" => "C",
        "C++" => "C++",
        "Java" => "Java",
        "Ruby" => "Rb",
        "JSON" => "JSON",
        "PHP" => "PHP",
        "C#" => "C#",
        "HTML" => "HTML",
        "CSS" => "CSS",
        "Bash" => "Bash",
        "YAML" => "YAML",
        "Markdown" => "MD",
        "Scala" => "Sc",
        "Haskell" => "Hs",
        "OCaml" => "OCaml",
        "Elixir" => "Ex",
        "Erlang" => "Erl",
        "Zig" => "Zig",
        "Lua" => "Lua",
        "Regex" => "Regex",
        "R" => "R",
        "Julia" => "Jl",
        "Swift" => "Swift",
        _ => &name[..4.min(name.len())],
    }
}
