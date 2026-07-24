// Minimal CSS parser: supports tag/class/#id/universal selectors, simple
// descendant combinator ("div p"), comma-separated selector lists, and a
// small set of properties as raw string values (interpreted in layout.rs / paint.rs).

#[derive(Debug, Clone, PartialEq)]
pub enum SimpleSelectorPart {
    Tag(String),
    Class(String),
    Id(String),
    Universal,
}

#[derive(Debug, Clone)]
pub struct SimpleSelector {
    pub parts: Vec<SimpleSelectorPart>, // e.g. div.card#main -> [Tag(div), Class(card), Id(main)]
}

#[derive(Debug, Clone)]
pub struct Selector {
    // Ancestor chain for descendant combinators, most specific (rightmost) last.
    pub chain: Vec<SimpleSelector>,
}

impl Selector {
    pub fn specificity(&self) -> (u32, u32, u32) {
        // (id count, class count, tag count) across the whole chain
        let mut ids = 0;
        let mut classes = 0;
        let mut tags = 0;
        for simple in &self.chain {
            for part in &simple.parts {
                match part {
                    SimpleSelectorPart::Id(_) => ids += 1,
                    SimpleSelectorPart::Class(_) => classes += 1,
                    SimpleSelectorPart::Tag(_) => tags += 1,
                    SimpleSelectorPart::Universal => {}
                }
            }
        }
        (ids, classes, tags)
    }
}

#[derive(Debug, Clone)]
pub struct Declaration {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub selectors: Vec<Selector>,
    pub declarations: Vec<Declaration>,
}

#[derive(Debug, Clone, Default)]
pub struct Stylesheet {
    pub rules: Vec<Rule>,
}

struct Parser {
    pos: usize,
    input: Vec<char>,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }
    fn eof(&self) -> bool {
        self.pos >= self.input.len()
    }
    fn consume_char(&mut self) -> char {
        let c = self.input[self.pos];
        self.pos += 1;
        c
    }
    fn consume_while<F: FnMut(char) -> bool>(&mut self, mut test: F) -> String {
        let mut s = String::new();
        while !self.eof() && test(self.peek().unwrap()) {
            s.push(self.consume_char());
        }
        s
    }
    fn consume_whitespace_and_comments(&mut self) {
        loop {
            self.consume_while(|c| c.is_whitespace());
            if self.peek() == Some('/') && self.input.get(self.pos + 1) == Some(&'*') {
                self.pos += 2;
                while !self.eof() && !(self.peek() == Some('*') && self.input.get(self.pos + 1) == Some(&'/')) {
                    self.pos += 1;
                }
                self.pos = (self.pos + 2).min(self.input.len());
            } else {
                break;
            }
        }
    }

    fn parse_rules(&mut self) -> Vec<Rule> {
        let mut rules = vec![];
        loop {
            self.consume_whitespace_and_comments();
            if self.eof() {
                break;
            }
            rules.push(self.parse_rule());
        }
        rules
    }

    fn parse_rule(&mut self) -> Rule {
        let selectors = self.parse_selectors();
        let declarations = self.parse_declarations();
        Rule { selectors, declarations }
    }

    fn parse_selectors(&mut self) -> Vec<Selector> {
        let mut selectors = vec![];
        loop {
            self.consume_whitespace_and_comments();
            selectors.push(self.parse_selector());
            self.consume_whitespace_and_comments();
            match self.peek() {
                Some(',') => {
                    self.consume_char();
                }
                _ => break,
            }
        }
        selectors
    }

    fn parse_selector(&mut self) -> Selector {
        let mut chain = vec![self.parse_simple_selector()];
        loop {
            // whitespace between simple selectors = descendant combinator
            let start = self.pos;
            self.consume_whitespace_and_comments();
            if self.eof() || self.peek() == Some(',') || self.peek() == Some('{') {
                break;
            }
            if self.pos == start {
                break; // no combinator, shouldn't happen
            }
            chain.push(self.parse_simple_selector());
        }
        Selector { chain }
    }

    fn parse_simple_selector(&mut self) -> SimpleSelector {
        let mut parts = vec![];
        loop {
            match self.peek() {
                Some('*') => {
                    self.consume_char();
                    parts.push(SimpleSelectorPart::Universal);
                }
                Some('#') => {
                    self.consume_char();
                    let name = self.consume_while(valid_ident_char);
                    parts.push(SimpleSelectorPart::Id(name));
                }
                Some('.') => {
                    self.consume_char();
                    let name = self.consume_while(valid_ident_char);
                    parts.push(SimpleSelectorPart::Class(name));
                }
                Some(c) if c.is_alphanumeric() || c == '-' || c == '_' => {
                    let name = self.consume_while(valid_ident_char);
                    parts.push(SimpleSelectorPart::Tag(name.to_lowercase()));
                }
                _ => break,
            }
        }
        if parts.is_empty() {
            parts.push(SimpleSelectorPart::Universal);
        }
        SimpleSelector { parts }
    }

    fn parse_declarations(&mut self) -> Vec<Declaration> {
        self.consume_whitespace_and_comments();
        if self.peek() == Some('{') {
            self.consume_char();
        }
        let mut decls = vec![];
        loop {
            self.consume_whitespace_and_comments();
            if self.eof() || self.peek() == Some('}') {
                break;
            }
            let name = self.consume_while(|c| c != ':' && c != '}').trim().to_string();
            if self.peek() == Some(':') {
                self.consume_char();
            }
            let value = self.consume_while(|c| c != ';' && c != '}').trim().to_string();
            if self.peek() == Some(';') {
                self.consume_char();
            }
            if !name.is_empty() {
                decls.push(Declaration { name: name.to_lowercase(), value });
            }
        }
        if self.peek() == Some('}') {
            self.consume_char();
        }
        decls
    }
}

fn valid_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '-' || c == '_'
}

pub fn parse(source: String) -> Stylesheet {
    let rules = Parser { pos: 0, input: source.chars().collect() }.parse_rules();
    Stylesheet { rules }
}
