use std::collections::HashMap;

pub struct Doc {
    pub addr: String,
    pub path: String,
    pub title: String,
    pub text: String,
}

pub struct Index {
    pub docs: Vec<Doc>,
    // term -> (doc index -> occurrence count)
    postings: HashMap<String, HashMap<usize, u32>>,
}

impl Index {
    pub fn build(docs: Vec<Doc>) -> Index {
        let mut postings: HashMap<String, HashMap<usize, u32>> = HashMap::new();
        for (i, doc) in docs.iter().enumerate() {
            let combined = format!("{} {}", doc.title, doc.text);
            for term in tokenize(&combined) {
                *postings.entry(term).or_default().entry(i).or_insert(0) += 1;
            }
        }
        Index { docs, postings }
    }

    /// Returns (doc index, score) pairs, best first.
    pub fn search(&self, query: &str) -> Vec<(usize, u32)> {
        let mut scores: HashMap<usize, u32> = HashMap::new();
        for term in tokenize(query) {
            if let Some(hits) = self.postings.get(&term) {
                for (&doc_idx, &count) in hits {
                    // small title-match bonus so title hits rank above body-only hits
                    let title_bonus = if self.docs[doc_idx].title.to_lowercase().contains(&term) { 5 } else { 0 };
                    *scores.entry(doc_idx).or_insert(0) += count + title_bonus;
                }
            }
        }
        let mut ranked: Vec<(usize, u32)> = scores.into_iter().collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1));
        ranked
    }
}

fn tokenize(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| w.to_string())
        .collect()
}

/// Very naive HTML -> (title, plain text). Strips tags, unescapes a few
/// entities. Good enough for indexing purposes; not a real parser.
pub fn extract_title_and_text(html: &str) -> (String, String) {
    let mut title = String::new();
    let mut text = String::new();
    let mut in_tag = false;
    let mut current_tag = String::new();
    let mut in_title = false;
    let mut in_style_or_script = false;
    let mut current_tag_is_close = false;

    let chars: Vec<char> = html.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '<' {
            in_tag = true;
            current_tag.clear();
            current_tag_is_close = false;
            if i + 1 < chars.len() && chars[i + 1] == '/' {
                current_tag_is_close = true;
            }
        } else if c == '>' && in_tag {
            in_tag = false;
            let tag_name: String = current_tag.trim_start_matches('/').split_whitespace().next().unwrap_or("").to_lowercase();
            match tag_name.as_str() {
                "title" => in_title = !current_tag_is_close,
                "style" | "script" => in_style_or_script = !current_tag_is_close,
                _ => {}
            }
        } else if in_tag {
            current_tag.push(c);
        } else if in_style_or_script {
            // skip
        } else if in_title {
            title.push(c);
        } else {
            text.push(c);
        }
        i += 1;
    }

    let clean = |s: String| -> String {
        let s = s.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&nbsp;", " ");
        s.split_whitespace().collect::<Vec<_>>().join(" ")
    };
    (clean(title), clean(text))
}
