pub fn parse_px(s: &str) -> Option<f32> {
    let s = s.trim();
    if let Some(stripped) = s.strip_suffix("px") {
        return stripped.trim().parse().ok();
    }
    s.parse().ok()
}

#[derive(Clone, Copy, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Color = Color { r: 0, g: 0, b: 0, a: 255 };
    pub const WHITE: Color = Color { r: 255, g: 255, b: 255, a: 255 };
    pub const TRANSPARENT: Color = Color { r: 0, g: 0, b: 0, a: 0 };

    pub fn as_u32(&self) -> u32 {
        (self.r as u32) << 16 | (self.g as u32) << 8 | (self.b as u32)
    }
}

const NAMED_COLORS: &[(&str, Color)] = &[
    ("black", Color { r: 0, g: 0, b: 0, a: 255 }),
    ("white", Color { r: 255, g: 255, b: 255, a: 255 }),
    ("red", Color { r: 220, g: 40, b: 40, a: 255 }),
    ("green", Color { r: 30, g: 150, b: 60, a: 255 }),
    ("blue", Color { r: 40, g: 90, b: 220, a: 255 }),
    ("yellow", Color { r: 230, g: 200, b: 30, a: 255 }),
    ("orange", Color { r: 230, g: 140, b: 30, a: 255 }),
    ("purple", Color { r: 140, g: 50, b: 180, a: 255 }),
    ("pink", Color { r: 230, g: 130, b: 170, a: 255 }),
    ("gray", Color { r: 130, g: 130, b: 130, a: 255 }),
    ("grey", Color { r: 130, g: 130, b: 130, a: 255 }),
    ("lightgray", Color { r: 210, g: 210, b: 210, a: 255 }),
    ("lightgrey", Color { r: 210, g: 210, b: 210, a: 255 }),
    ("darkgray", Color { r: 80, g: 80, b: 80, a: 255 }),
    ("navy", Color { r: 20, g: 30, b: 90, a: 255 }),
    ("teal", Color { r: 20, g: 130, b: 130, a: 255 }),
    ("transparent", Color { r: 0, g: 0, b: 0, a: 0 }),
];

pub fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim().to_lowercase();
    if let Some(hex) = s.strip_prefix('#') {
        return parse_hex(hex);
    }
    if let Some(inner) = s.strip_prefix("rgb(").and_then(|r| r.strip_suffix(')')) {
        let parts: Vec<Option<u8>> = inner.split(',').map(|p| p.trim().parse::<u8>().ok()).collect();
        if parts.len() == 3 {
            if let (Some(r), Some(g), Some(b)) = (parts[0], parts[1], parts[2]) {
                return Some(Color { r, g, b, a: 255 });
            }
        }
    }
    if let Some(inner) = s.strip_prefix("rgba(").and_then(|r| r.strip_suffix(')')) {
        let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
        if parts.len() == 4 {
            let r = parts[0].parse::<u8>().ok()?;
            let g = parts[1].parse::<u8>().ok()?;
            let b = parts[2].parse::<u8>().ok()?;
            let a = (parts[3].parse::<f32>().ok()? * 255.0) as u8;
            return Some(Color { r, g, b, a });
        }
    }
    NAMED_COLORS.iter().find(|(name, _)| *name == s).map(|(_, c)| *c)
}

fn parse_hex(hex: &str) -> Option<Color> {
    let expand = |c: char| -> Option<u8> { c.to_digit(16).map(|d| (d * 16 + d) as u8) };
    match hex.len() {
        3 => {
            let chars: Vec<char> = hex.chars().collect();
            Some(Color { r: expand(chars[0])?, g: expand(chars[1])?, b: expand(chars[2])?, a: 255 })
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(Color { r, g, b, a: 255 })
        }
        _ => None,
    }
}
