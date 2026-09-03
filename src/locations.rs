use std::collections::{HashMap, HashSet};

pub struct KeyLocation {
    pub label: String,
    pub path: String,
    pub count: u32,
}

pub struct KeyLocations {
    pub locations: Vec<KeyLocation>,
}

pub fn detect_key_locations_from_paths(all_rel_paths: &[String]) -> KeyLocations {
    let mut dir_counts: HashMap<String, u32> = HashMap::new();

    for rel_path in all_rel_paths {
        let normalized = rel_path.replace('\\', "/");
        let parts: Vec<&str> = normalized.split('/').collect();
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
    let mut used_dirs: HashSet<String> = HashSet::new();

    for (dir_path, count) in &dir_counts {
        if *count < 1 {
            continue;
        }

        let last_component = dir_path
            .rsplit('/')
            .next()
            .unwrap_or(dir_path);

        for (dir_names, label) in patterns {
            if dir_names.contains(&last_component) {
                let key = format!("{}:{}", label, dir_path);
                if !used_dirs.insert(key) {
                    continue;
                }

                locations.push(KeyLocation {
                    label: label.to_string(),
                    path: dir_path.clone(),
                    count: *count,
                });
            }
        }
    }

    locations.sort_by_key(|l| std::cmp::Reverse(l.count));

    KeyLocations { locations }
}
