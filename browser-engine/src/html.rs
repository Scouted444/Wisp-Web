// A small HTML parser. Not spec-compliant (real HTML parsing has ~100 pages
// of error-recovery rules) but handles well-formed-ish documents: nested tags,
// attributes, text nodes, comments, self-closing/void elements.

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum NodeType {
    Text(String),
    Element(ElementData),
}

#[derive(Debug, Clone)]
pub struct ElementData {
    pub tag_name: String,
    pub attributes: HashMap<String, String>,
}

impl ElementData {
    pub fn id(&self) -> Option<&String> {
        self.attributes.get("id")
    }

    pub fn classes(&self) -> Vec<&str> {
        match self.attributes.get("class") {
            Some(s) => s.split_whitespace().collect(),
            None => vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct Node {
    pub children: Vec<Node>,
    pub node_type: NodeType,
}

pub fn text(data: String) -> Node {
    Node { children: vec![], node_type: NodeType::Text(data) }
}

pub fn elem(tag_name: String, attributes: HashMap<String, String>, children: Vec<Node>) -> Node {
    Node {
        children,
        node_type: NodeType::Element(ElementData { tag_name, attributes }),
    }
}

const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta",
    "param", "source", "track", "wbr",
];

struct Parser {
    pos: usize,
    input: Vec<char>,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    fn starts_with(&self, s: &str) -> bool {
        self.input[self.pos..].iter().collect::<String>().starts_with(s)
    }

    fn eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn consume_char(&mut self) -> char {
        let c = self.input[self.pos];
        self.pos += 1;
        c
    }

    fn consume_while<F>(&mut self, mut test: F) -> String
    where
        F: FnMut(char) -> bool,
    {
        let mut result = String::new();
        while !self.eof() && test(self.peek().unwrap()) {
            result.push(self.consume_char());
        }
        result
    }

    fn consume_whitespace(&mut self) {
        self.consume_while(|c| c.is_whitespace());
    }

    fn parse_nodes(&mut self) -> Vec<Node> {
        let mut nodes = vec![];
        loop {
            self.consume_whitespace();
            if self.eof() || self.starts_with("</") {
                break;
            }
            if self.starts_with("<!--") {
                self.consume_comment();
                continue;
            }
            if self.starts_with("<!") {
                // doctype or similar, skip to '>'
                self.consume_while(|c| c != '>');
                if !self.eof() {
                    self.consume_char();
                }
                continue;
            }
            nodes.push(self.parse_node());
        }
        nodes
    }

    fn consume_comment(&mut self) {
        // consume "<!--"
        for _ in 0..4 {
            if !self.eof() {
                self.consume_char();
            }
        }
        while !self.eof() && !self.starts_with("-->") {
            self.consume_char();
        }
        for _ in 0..3 {
            if !self.eof() {
                self.consume_char();
            }
        }
    }

    fn parse_node(&mut self) -> Node {
        if self.peek() == Some('<') {
            self.parse_element()
        } else {
            self.parse_text()
        }
    }

    fn parse_text(&mut self) -> Node {
        let raw = self.consume_while(|c| c != '<');
        text(decode_entities(&raw))
    }

    fn parse_tag_name(&mut self) -> String {
        self.consume_while(|c| c.is_alphanumeric() || c == '-')
    }

    fn parse_element(&mut self) -> Node {
        self.consume_char(); // '<'
        let tag_name = self.parse_tag_name().to_lowercase();
        let attributes = self.parse_attributes();

        // self-closing "<tag ... />"
        self.consume_whitespace();
        let mut self_closing = false;
        if self.peek() == Some('/') {
            self.consume_char();
            self_closing = true;
        }
        if self.peek() == Some('>') {
            self.consume_char();
        }

        // <script>/<style> contents are raw text until the matching close tag
        if !self_closing && (tag_name == "script" || tag_name == "style") {
            let closing = format!("</{}", tag_name);
            let mut raw = String::new();
            while !self.eof() && !self.starts_with(&closing) {
                raw.push(self.consume_char());
            }
            // consume closing tag
            self.consume_while(|c| c != '>');
            if !self.eof() {
                self.consume_char();
            }
            return elem(tag_name, attributes, vec![text(raw)]);
        }

        if self_closing || VOID_ELEMENTS.contains(&tag_name.as_str()) {
            return elem(tag_name, attributes, vec![]);
        }

        let children = self.parse_nodes();

        // consume closing tag "</tag>" if present
        if self.starts_with("</") {
            self.consume_while(|c| c != '>');
            if !self.eof() {
                self.consume_char();
            }
        }

        elem(tag_name, attributes, children)
    }

    fn parse_attributes(&mut self) -> HashMap<String, String> {
        let mut attributes = HashMap::new();
        loop {
            self.consume_whitespace();
            if self.peek() == Some('>') || self.peek() == Some('/') || self.eof() {
                break;
            }
            let name = self.consume_while(|c| c != '=' && c != '>' && c != '/' && !c.is_whitespace());
            if name.is_empty() {
                break;
            }
            self.consume_whitespace();
            let value = if self.peek() == Some('=') {
                self.consume_char();
                self.consume_whitespace();
                self.parse_attr_value()
            } else {
                String::new()
            };
            attributes.insert(name.to_lowercase(), value);
        }
        attributes
    }

    fn parse_attr_value(&mut self) -> String {
        match self.peek() {
            Some(q @ '"') | Some(q @ '\'') => {
                self.consume_char();
                let value = self.consume_while(|c| c != q);
                if !self.eof() {
                    self.consume_char();
                }
                decode_entities(&value)
            }
            _ => decode_entities(&self.consume_while(|c| !c.is_whitespace() && c != '>')),
        }
    }
}

fn decode_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&nbsp;", "\u{00A0}")
}

pub fn parse(source: String) -> Node {
    let mut nodes = Parser { pos: 0, input: source.chars().collect() }.parse_nodes();
    if nodes.len() == 1 {
        nodes.swap_remove(0)
    } else {
        elem("html".to_string(), HashMap::new(), nodes)
    }
}
