use crate::font;
use crate::html::NodeType;
use crate::style::{Display, StyledNode};
use crate::values::{parse_color, parse_px, Color};

#[derive(Default, Clone, Copy, Debug)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Default, Clone, Copy, Debug)]
pub struct EdgeSizes {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

#[derive(Default, Clone, Copy, Debug)]
pub struct Dimensions {
    pub content: Rect,
    pub padding: EdgeSizes,
    pub border: EdgeSizes,
    pub margin: EdgeSizes,
}

impl Dimensions {
    pub fn padding_box(&self) -> Rect {
        expand(self.content, self.padding)
    }
    pub fn border_box(&self) -> Rect {
        expand(self.padding_box(), self.border)
    }
    pub fn margin_box(&self) -> Rect {
        expand(self.border_box(), self.margin)
    }
}

fn expand(r: Rect, e: EdgeSizes) -> Rect {
    Rect {
        x: r.x - e.left,
        y: r.y - e.top,
        width: r.width + e.left + e.right,
        height: r.height + e.top + e.bottom,
    }
}

pub struct TextLine {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub font_size: f32,
    pub color: Color,
}

pub struct LayoutBox<'a> {
    pub dimensions: Dimensions,
    pub styled: Option<&'a StyledNode<'a>>, // None for the anonymous root
    pub background: Option<Color>,
    pub border_color: Option<Color>,
    pub children: Vec<LayoutBox<'a>>,
    pub text_lines: Vec<TextLine>,
    pub tag: String,
}

fn get_num(node: &StyledNode, name: &str, default: f32) -> f32 {
    node.get(name).and_then(parse_px).unwrap_or(default)
}

fn collect_text(node: &StyledNode) -> String {
    // concatenate direct text-node children (ignores nested elements)
    let mut out = String::new();
    if let NodeType::Text(t) = &node.node.node_type {
        out.push_str(t);
    }
    out
}

/// Wraps `text` to fit within `max_width` px given a monospace-ish glyph
/// advance derived from font_size, returning line strings.
fn wrap_text(text: &str, font_size: f32, max_width: f32) -> Vec<String> {
    let advance = (font::GLYPH_W + 1.0) * (font_size / font::GLYPH_H);
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return vec![];
    }
    let mut lines = vec![];
    let mut current = String::new();
    let mut current_width = 0.0;
    for word in words {
        let word_width = word.chars().count() as f32 * advance;
        let space_width = if current.is_empty() { 0.0 } else { advance };
        if current_width + space_width + word_width > max_width && !current.is_empty() {
            lines.push(current.clone());
            current.clear();
            current_width = 0.0;
        }
        if !current.is_empty() {
            current.push(' ');
            current_width += advance;
        }
        current.push_str(word);
        current_width += word_width;
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

pub fn layout_tree<'a>(root: &'a StyledNode<'a>, containing_width: f32) -> LayoutBox<'a> {
    let mut root_box = LayoutBox {
        dimensions: Dimensions::default(),
        styled: None,
        background: None,
        border_color: None,
        children: vec![],
        text_lines: vec![],
        tag: "root".into(),
    };
    root_box.dimensions.content.width = containing_width;
    let mut cursor_y = 0.0;
    layout_block_children(root, containing_width, 0.0, &mut cursor_y, &mut root_box.children, 16.0, Color::BLACK);
    root_box.dimensions.content.height = cursor_y;
    root_box
}

/// Lays out the block-level and text content that are direct children of `node`,
/// stacking them vertically starting at (x, *cursor_y), advancing *cursor_y.
fn layout_block_children<'a>(
    node: &'a StyledNode<'a>,
    available_width: f32,
    x: f32,
    cursor_y: &mut f32,
    out: &mut Vec<LayoutBox<'a>>,
    inherited_font_size: f32,
    inherited_color: Color,
) {
    // first, any direct text nodes among children get wrapped into a text-only box
    for child in &node.children {
        match &child.node.node_type {
            NodeType::Text(_) => {
                let text = collect_text(child);
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let font_size = child.get("font-size").and_then(parse_px).unwrap_or(inherited_font_size);
                let color = child.get("color").and_then(parse_color).unwrap_or(inherited_color);
                let lines = wrap_text(trimmed, font_size, available_width);
                let line_height = font_size * 1.4;
                let mut text_lines = vec![];
                for line in lines {
                    text_lines.push(TextLine { text: line, x, y: *cursor_y, font_size, color });
                    *cursor_y += line_height;
                }
                out.push(LayoutBox {
                    dimensions: Dimensions {
                        content: Rect { x, y: *cursor_y, width: available_width, height: 0.0 },
                        ..Default::default()
                    },
                    styled: None,
                    background: None,
                    border_color: None,
                    children: vec![],
                    text_lines,
                    tag: "text".into(),
                });
            }
            NodeType::Element(data) => {
                if child.display() == Display::None {
                    continue;
                }
                if data.tag_name == "script" || data.tag_name == "style" || data.tag_name == "head" || data.tag_name == "title" {
                    continue;
                }
                let laid_out = layout_element(child, available_width, x, *cursor_y, inherited_font_size, inherited_color);
                *cursor_y += laid_out.dimensions.margin_box().height;
                out.push(laid_out);
            }
        }
    }
}

fn layout_element<'a>(
    node: &'a StyledNode<'a>,
    containing_width: f32,
    x: f32,
    y: f32,
    inherited_font_size: f32,
    inherited_color: Color,
) -> LayoutBox<'a> {
    let margin = EdgeSizes {
        top: get_num(node, "margin-top", get_num(node, "margin", 0.0)),
        right: get_num(node, "margin-right", get_num(node, "margin", 0.0)),
        bottom: get_num(node, "margin-bottom", get_num(node, "margin", 0.0)),
        left: get_num(node, "margin-left", get_num(node, "margin", 0.0)),
    };
    let border_width = EdgeSizes {
        top: get_num(node, "border-top-width", get_num(node, "border-width", 0.0)),
        right: get_num(node, "border-right-width", get_num(node, "border-width", 0.0)),
        bottom: get_num(node, "border-bottom-width", get_num(node, "border-width", 0.0)),
        left: get_num(node, "border-left-width", get_num(node, "border-width", 0.0)),
    };
    let padding = EdgeSizes {
        top: get_num(node, "padding-top", get_num(node, "padding", 0.0)),
        right: get_num(node, "padding-right", get_num(node, "padding", 0.0)),
        bottom: get_num(node, "padding-bottom", get_num(node, "padding", 0.0)),
        left: get_num(node, "padding-left", get_num(node, "padding", 0.0)),
    };

    let horizontal_extra = margin.left + margin.right + border_width.left + border_width.right + padding.left + padding.right;
    let content_width = node
        .get("width")
        .and_then(parse_px)
        .unwrap_or(containing_width - horizontal_extra)
        .max(0.0);

    let content_x = x + margin.left + border_width.left + padding.left;
    let content_y = y + margin.top + border_width.top + padding.top;

    let font_size = node.get("font-size").and_then(parse_px).unwrap_or(inherited_font_size);
    let color = node.get("color").and_then(parse_color).unwrap_or(inherited_color);

    let mut children = vec![];
    let mut cursor_y = content_y;
    layout_block_children(node, content_width, content_x, &mut cursor_y, &mut children, font_size, color);
    let auto_height = cursor_y - content_y;
    let content_height = node.get("height").and_then(parse_px).unwrap_or(auto_height).max(0.0);

    let dimensions = Dimensions {
        content: Rect { x: content_x, y: content_y, width: content_width, height: content_height },
        padding,
        border: border_width,
        margin,
    };

    let background = node.get("background-color").or_else(|| node.get("background")).and_then(parse_color);
    let border_color = if border_width.top + border_width.right + border_width.bottom + border_width.left > 0.0 {
        node.get("border-color").and_then(parse_color).or(Some(Color::BLACK))
    } else {
        None
    };

    let tag = match &node.node.node_type {
        NodeType::Element(d) => d.tag_name.clone(),
        _ => "anon".into(),
    };

    LayoutBox { dimensions, styled: Some(node), background, border_color, children, text_lines: vec![], tag }
}
