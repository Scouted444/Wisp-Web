use crate::font;
use crate::layout::LayoutBox;
use crate::values::Color;

pub struct Canvas {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>, // 0x00RRGGBB, row-major
}

impl Canvas {
    pub fn new(width: usize, height: usize, bg: Color) -> Canvas {
        Canvas { width, height, pixels: vec![bg.as_u32(); width * height] }
    }

    pub fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        if color.a == 0 {
            return;
        }
        let x0 = x.max(0.0) as i32;
        let y0 = y.max(0.0) as i32;
        let x1 = ((x + w).min(self.width as f32)) as i32;
        let y1 = ((y + h).min(self.height as f32)) as i32;
        for py in y0.max(0)..y1.max(0) {
            for px in x0.max(0)..x1.max(0) {
                if (px as usize) < self.width && (py as usize) < self.height {
                    self.blend(px as usize, py as usize, color);
                }
            }
        }
    }

    fn blend(&mut self, x: usize, y: usize, color: Color) {
        let idx = y * self.width + x;
        if color.a == 255 {
            self.pixels[idx] = color.as_u32();
        } else {
            let bg = self.pixels[idx];
            let br = ((bg >> 16) & 0xff) as f32;
            let bg_ = ((bg >> 8) & 0xff) as f32;
            let bb = (bg & 0xff) as f32;
            let a = color.a as f32 / 255.0;
            let r = color.r as f32 * a + br * (1.0 - a);
            let g = color.g as f32 * a + bg_ * (1.0 - a);
            let b = color.b as f32 * a + bb * (1.0 - a);
            self.pixels[idx] = (r as u32) << 16 | (g as u32) << 8 | (b as u32);
        }
    }

    fn draw_line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, color: Color, thickness: f32) {
        let dx = x1 - x0;
        let dy = y1 - y0;
        let len = (dx * dx + dy * dy).sqrt().max(0.001);
        let steps = len.ceil() as i32;
        let half = thickness / 2.0;
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let x = x0 + dx * t;
            let y = y0 + dy * t;
            self.fill_rect(x - half, y - half, thickness.max(1.0), thickness.max(1.0), color);
        }
    }

    pub fn draw_text(&mut self, text: &str, x: f32, y: f32, font_size: f32, color: Color) {
        let scale = font_size / font::GLYPH_H;
        let advance = (font::GLYPH_W + 1.0) * scale;
        let mut cx = x;
        for ch in text.chars() {
            for (x0, y0, x1, y1) in font::glyph_segments(ch) {
                self.draw_line(cx + x0 * scale, y + y0 * scale, cx + x1 * scale, y + y1 * scale, color, (scale * 0.9).max(1.0));
            }
            cx += advance;
        }
    }

    pub fn draw_rect_border(&mut self, x: f32, y: f32, w: f32, h: f32, thickness: f32, color: Color) {
        if thickness <= 0.0 {
            return;
        }
        self.fill_rect(x, y, w, thickness, color); // top
        self.fill_rect(x, y + h - thickness, w, thickness, color); // bottom
        self.fill_rect(x, y, thickness, h, color); // left
        self.fill_rect(x + w - thickness, y, thickness, h, color); // right
    }
}

pub fn paint(canvas: &mut Canvas, layout_box: &LayoutBox) {
    let border_box = layout_box.dimensions.border_box();
    let padding_box = layout_box.dimensions.padding_box();

    if let Some(bg) = layout_box.background {
        canvas.fill_rect(padding_box.x, padding_box.y, padding_box.width, padding_box.height, bg);
    }
    if let Some(bc) = layout_box.border_color {
        let b = layout_box.dimensions.border;
        let max_thickness = b.top.max(b.right).max(b.bottom).max(b.left);
        canvas.draw_rect_border(border_box.x, border_box.y, border_box.width, border_box.height, max_thickness, bc);
    }

    for line in &layout_box.text_lines {
        canvas.draw_text(&line.text, line.x, line.y, line.font_size, line.color);
    }

    for child in &layout_box.children {
        paint(canvas, child);
    }
}
