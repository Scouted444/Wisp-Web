use crate::css::{Selector, SimpleSelectorPart, Stylesheet};
use crate::html::{Node, NodeType};
use std::collections::HashMap;

pub type PropertyMap = HashMap<String, String>;

pub struct StyledNode<'a> {
    pub node: &'a Node,
    pub specified: PropertyMap,
    pub children: Vec<StyledNode<'a>>,
}

impl<'a> StyledNode<'a> {
    pub fn get(&self, name: &str) -> Option<&str> {
        self.specified.get(name).map(|s| s.as_str())
    }

    pub fn get_or(&self, name: &str, default: &str) -> String {
        self.specified.get(name).cloned().unwrap_or_else(|| default.to_string())
    }

    pub fn display(&self) -> Display {
        match self.get("display") {
            Some("none") => Display::None,
            Some("inline") => Display::Inline,
            _ => Display::Block,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Display {
    Block,
    Inline,
    None,
}

fn matches_simple(elem_tag: &str, elem_id: Option<&str>, elem_classes: &[&str], sel: &crate::css::SimpleSelector) -> bool {
    for part in &sel.parts {
        let ok = match part {
            SimpleSelectorPart::Universal => true,
            SimpleSelectorPart::Tag(t) => t == elem_tag,
            SimpleSelectorPart::Id(i) => elem_id == Some(i.as_str()),
            SimpleSelectorPart::Class(c) => elem_classes.contains(&c.as_str()),
        };
        if !ok {
            return false;
        }
    }
    true
}

// ancestors[0] = immediate parent, ancestors[1] = grandparent, etc.
fn matches_selector(
    elem_tag: &str,
    elem_id: Option<&str>,
    elem_classes: &[&str],
    ancestors: &[(&str, Option<&str>, Vec<&str>)],
    sel: &Selector,
) -> bool {
    if sel.chain.is_empty() {
        return false;
    }
    let last = sel.chain.last().unwrap();
    if !matches_simple(elem_tag, elem_id, elem_classes, last) {
        return false;
    }
    // remaining ancestor simple-selectors must be found, in order, among ancestors
    let mut ancestor_idx = 0;
    for simple in sel.chain[..sel.chain.len() - 1].iter().rev() {
        let mut found = false;
        while ancestor_idx < ancestors.len() {
            let (tag, id, classes) = &ancestors[ancestor_idx];
            ancestor_idx += 1;
            if matches_simple(tag, *id, classes, simple) {
                found = true;
                break;
            }
        }
        if !found {
            return false;
        }
    }
    true
}

fn specified_values(
    tag: &str,
    id: Option<&str>,
    classes: &[&str],
    ancestors: &[(&str, Option<&str>, Vec<&str>)],
    inline_style: Option<&str>,
    stylesheet: &Stylesheet,
) -> PropertyMap {
    let mut matched: Vec<(u32, u32, u32, usize, &crate::css::Rule, &Selector)> = vec![];
    for (rule_idx, rule) in stylesheet.rules.iter().enumerate() {
        for sel in &rule.selectors {
            if matches_selector(tag, id, classes, ancestors, sel) {
                let (a, b, c) = sel.specificity();
                matched.push((a, b, c, rule_idx, rule, sel));
            }
        }
    }
    // sort by specificity, then source order, ascending so later/higher wins on insert
    matched.sort_by_key(|m| (m.0, m.1, m.2, m.3));

    let mut values = PropertyMap::new();
    for (_, _, _, _, rule, _) in matched {
        for decl in &rule.declarations {
            values.insert(decl.name.clone(), decl.value.clone());
        }
    }

    if let Some(style_attr) = inline_style {
        let decls = crate::css::parse(format!("dummy{{{}}}", style_attr));
        if let Some(rule) = decls.rules.first() {
            for decl in &rule.declarations {
                values.insert(decl.name.clone(), decl.value.clone());
            }
        }
    }

    values
}

const INHERITED: &[&str] = &["color", "font-size", "font-weight", "text-align", "font-family", "line-height"];

pub fn style_tree<'a>(root: &'a Node, stylesheet: &Stylesheet) -> StyledNode<'a> {
    build(root, stylesheet, &[], &PropertyMap::new())
}

fn build<'a>(
    node: &'a Node,
    stylesheet: &Stylesheet,
    ancestors: &[(&'a str, Option<&'a str>, Vec<&'a str>)],
    parent_inherited: &PropertyMap,
) -> StyledNode<'a> {
    match &node.node_type {
        NodeType::Text(_) => StyledNode {
            node,
            specified: parent_inherited.clone(),
            children: vec![],
        },
        NodeType::Element(data) => {
            let id = data.id().map(|s| s.as_str());
            let classes = data.classes();
            let mut specified = specified_values(
                &data.tag_name,
                id,
                &classes,
                ancestors,
                data.attributes.get("style").map(|s| s.as_str()),
                stylesheet,
            );

            // apply inheritance for anything not explicitly set
            for key in INHERITED {
                if !specified.contains_key(*key) {
                    if let Some(v) = parent_inherited.get(*key) {
                        specified.insert(key.to_string(), v.clone());
                    }
                }
            }

            let mut new_ancestors: Vec<(&'a str, Option<&'a str>, Vec<&'a str>)> = Vec::with_capacity(ancestors.len() + 1);
            new_ancestors.push((data.tag_name.as_str(), id, classes.clone()));
            new_ancestors.extend(ancestors.iter().cloned());

            let inherited_for_children: PropertyMap = specified
                .iter()
                .filter(|(k, _)| INHERITED.contains(&k.as_str()))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();

            let children = node
                .children
                .iter()
                .map(|c| build(c, stylesheet, &new_ancestors, &inherited_for_children))
                .collect();

            StyledNode { node, specified, children }
        }
    }
}
