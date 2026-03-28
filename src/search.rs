use std::collections::HashMap;

pub struct Bm25Index {
    docs: Vec<(String, String, Vec<String>)>,
    df: HashMap<String, usize>,
    avgdl: f64,
}

pub struct SearchResult {
    pub location: String,
    pub line: String,
    pub score: f64,
}

fn tokenize(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .collect()
}

pub fn build_bm25_index(lines: Vec<(String, String)>) -> Bm25Index {
    let docs: Vec<(String, String, Vec<String>)> = lines
        .into_iter()
        .map(|(loc, text)| {
            let tokens = tokenize(&text);
            (loc, text, tokens)
        })
        .collect();

    let mut df: HashMap<String, usize> = HashMap::new();
    for (_, _, tokens) in &docs {
        let mut seen = std::collections::HashSet::new();
        for t in tokens {
            if seen.insert(t.clone()) {
                *df.entry(t.clone()).or_insert(0) += 1;
            }
        }
    }

    let avgdl = if docs.is_empty() {
        1.0
    } else {
        docs.iter().map(|(_, _, t)| t.len() as f64).sum::<f64>() / docs.len() as f64
    };

    Bm25Index { docs, df, avgdl }
}

pub fn bm25_search(index: &Bm25Index, query: &str, top_k: usize) -> Vec<SearchResult> {
    let query_terms = tokenize(query);
    if query_terms.is_empty() {
        return Vec::new();
    }

    let n = index.docs.len() as f64;
    let k1: f64 = 1.5;
    let b: f64 = 0.75;

    let mut scores: Vec<(usize, f64)> = index
        .docs
        .iter()
        .enumerate()
        .filter_map(|(i, (_, _, tokens))| {
            let dl = tokens.len() as f64;
            let mut score = 0.0;

            for term in &query_terms {
                let df = *index.df.get(term).unwrap_or(&0) as f64;
                if df == 0.0 { continue; }
                let tf = tokens.iter().filter(|t| *t == term).count() as f64;
                let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();
                let tf_norm = tf * (k1 + 1.0) / (tf + k1 * (1.0 - b + b * dl / index.avgdl));
                score += idf * tf_norm;
            }

            if score > 0.0 { Some((i, score)) } else { None }
        })
        .collect();

    scores.sort_by(|a, b_val| b_val.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scores.truncate(top_k);

    scores
        .into_iter()
        .map(|(i, score)| {
            let (loc, line, _) = &index.docs[i];
            SearchResult { location: loc.clone(), line: line.trim().to_string(), score }
        })
        .collect()
}
