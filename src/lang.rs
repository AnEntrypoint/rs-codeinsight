use tree_sitter::Language;

pub struct LangDef {
    pub name: &'static str,
    pub language: Language,
}

pub fn get_language(ext: &str) -> Option<LangDef> {
    match ext {
        ".js" | ".mjs" | ".cjs" | ".jsx" => Some(LangDef {
            name: "JavaScript",
            language: tree_sitter_javascript::LANGUAGE.into(),
        }),
        ".ts" => Some(LangDef {
            name: "TypeScript",
            language: tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        }),
        ".tsx" => Some(LangDef {
            name: "TSX",
            language: tree_sitter_typescript::LANGUAGE_TSX.into(),
        }),
        ".py" => Some(LangDef {
            name: "Python",
            language: tree_sitter_python::LANGUAGE.into(),
        }),
        ".rs" => Some(LangDef {
            name: "Rust",
            language: tree_sitter_rust::LANGUAGE.into(),
        }),
        ".go" => Some(LangDef {
            name: "Go",
            language: tree_sitter_go::LANGUAGE.into(),
        }),
        ".c" | ".h" => Some(LangDef {
            name: "C",
            language: tree_sitter_c::LANGUAGE.into(),
        }),
        ".cpp" | ".cc" | ".cxx" | ".hpp" => Some(LangDef {
            name: "C++",
            language: tree_sitter_cpp::LANGUAGE.into(),
        }),
        ".java" => Some(LangDef {
            name: "Java",
            language: tree_sitter_java::LANGUAGE.into(),
        }),
        ".rb" => Some(LangDef {
            name: "Ruby",
            language: tree_sitter_ruby::LANGUAGE.into(),
        }),
        ".json" => Some(LangDef {
            name: "JSON",
            language: tree_sitter_json::LANGUAGE.into(),
        }),
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
        _ => &name[..4.min(name.len())],
    }
}
