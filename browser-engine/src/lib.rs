pub mod css;
pub mod font;
pub mod html;
pub mod layout;
pub mod paint;
pub mod style;
pub mod values;

/// Convenience: parse HTML+CSS and produce a painted canvas in one call.
pub fn render_page(html_src: String, css_src: String, width: f32, min_height: usize) -> paint::Canvas {
    let dom = html::parse(html_src);
    let stylesheet = css::parse(css_src);
    let styled = style::style_tree(&dom, &stylesheet);
    let root_box = layout::layout_tree(&styled, width);
    let height = root_box.dimensions.content.height.max(1.0) as usize;
    let mut canvas = paint::Canvas::new(width as usize, height.max(min_height), values::Color::WHITE);
    paint::paint(&mut canvas, &root_box);
    canvas
}

/// Like `render_page`, but pulls CSS out of inline `<style>` tags in the HTML
/// itself rather than taking a separate stylesheet — this is what a fetched
/// single-file page (e.g. over wisp://) typically looks like.
pub fn render_html(html_src: String, width: f32, min_height: usize) -> paint::Canvas {
    let dom = html::parse(html_src);
    let css_src = extract_inline_styles(&dom);
    let stylesheet = css::parse(css_src);
    let styled = style::style_tree(&dom, &stylesheet);
    let root_box = layout::layout_tree(&styled, width);
    let height = root_box.dimensions.content.height.max(1.0) as usize;
    let mut canvas = paint::Canvas::new(width as usize, height.max(min_height), values::Color::WHITE);
    paint::paint(&mut canvas, &root_box);
    canvas
}

fn extract_inline_styles(node: &html::Node) -> String {
    let mut out = String::new();
    if let html::NodeType::Element(data) = &node.node_type {
        if data.tag_name == "style" {
            for child in &node.children {
                if let html::NodeType::Text(t) = &child.node_type {
                    out.push_str(t);
                    out.push('\n');
                }
            }
        }
    }
    for child in &node.children {
        out.push_str(&extract_inline_styles(child));
    }
    out
}
