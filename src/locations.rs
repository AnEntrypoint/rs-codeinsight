use std::collections::HashMap;

pub struct KeyLocation {
    pub label: String,
    pub path: String,
    pub count: u32,
}

pub struct KeyLocations {
    pub locations: Vec<KeyLocation>,
}

/// Detect key locations by analyzing relative file paths (no extra filesystem access).
pub fn detect_key_locations_from_paths(all_rel_paths: &[String]) -> KeyLocations {
    // Map from directory path -> count of files
    let mut dir_counts: HashMap<String, u32> = HashMap::new();

    for rel_path in all_rel_paths {
        let normalized = rel_path.replace('\\', "/");
        let parts: Vec<&str> = normalized.split('/').collect();
        // Collect all directory components (up to 2 levels deep from root)
        // e.g., for "src/components/Button.tsx", we look at "src/" and "src/components/"
        for depth in 0..parts.len().saturating_sub(1).min(3) {
            let dir_path = parts[..=depth].join("/");
            *dir_counts.entry(dir_path).or_insert(0) += 1;
        }
    }

    let patterns: &[(&[&str], &str)] = &[
        (&["api", "routes", "handlers"], "API routes"),
        (&["components", "ui"], "Components"),
        (&["pages", "app"], "Pages"),
        (&["models", "entities", "domain"], "Models"),
        (&["types", "interfaces"], "Types"),
        (&["lib", "utils", "helpers"], "Utilities"),
        (&["hooks"], "Hooks"),
        (&["middleware"], "Middleware"),
        (&["services", "service"], "Services"),
        (&["config", "configs"], "Config"),
        (&["tests", "__tests__", "test"], "Tests"),
        (&["migrations", "migrate"], "Migrations"),
        (&["public", "static", "assets"], "Static assets"),
        (&["cmd"], "Commands (Go)"),
        (&["internal"], "Internal (Go)"),
        (&["pkg"], "Packages (Go)"),
        (&["prisma"], "Prisma"),
        (&["store", "stores", "state"], "State management"),
    ];

    let mut locations: Vec<KeyLocation> = Vec::new();
    let mut used_dirs: HashMap<String, bool> = HashMap::new();

    for (dir_path, count) in &dir_counts {
        if *count < 1 {
            continue;
        }

        // Get the last component of the directory path to match against patterns
        let last_component = dir_path
            .rsplit('/')
            .next()
            .unwrap_or(dir_path);

        for (dir_names, label) in patterns {
            if dir_names.contains(&last_component) {
                // Avoid duplicating the same label for different paths unless they are distinct
                let key = format!("{}:{}", label, dir_path);
                if used_dirs.contains_key(&key) {
                    continue;
                }
                used_dirs.insert(key, true);

                locations.push(KeyLocation {
                    label: label.to_string(),
                    path: dir_path.clone(),
                    count: *count,
                });
            }
        }
    }

    // Sort by count descending
    locations.sort_by(|a, b| b.count.cmp(&a.count));

    KeyLocations { locations }
}
