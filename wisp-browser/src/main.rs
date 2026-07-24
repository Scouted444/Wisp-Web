#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::num::NonZeroU32;
use std::rc::Rc;

use browser_engine::{paint::Canvas, values::Color};
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

const BAR_HEIGHT: f32 = 36.0;

struct BrowserState {
    address: String,
    status: String,
    page: Option<Canvas>,
    scroll: f32,
}

impl BrowserState {
    fn navigate(&mut self, width: f32) {
        let (addr, path) = wisp_protocol::client::parse_wisp_url(&self.address);
        self.status = format!("loading {addr}{path} ...");
        match wisp_protocol::client::fetch(&addr, &path) {
            Ok(resp) => {
                let body = String::from_utf8_lossy(&resp.body).to_string();
                self.status = format!("{} {} — {} bytes", resp.status, resp.reason, resp.body.len());
                let canvas = browser_engine::render_html(body, width, 400);
                self.page = Some(canvas);
                self.scroll = 0.0;
            }
            Err(e) => {
                self.status = format!("failed: {e}");
                let error_html = format!(
                    "<body><h1>Could not load page</h1><p>{}</p></body>",
                    e.to_string().replace('<', "&lt;")
                );
                self.page = Some(browser_engine::render_html(error_html, width, 400));
            }
        }
    }
}

fn composite(window_w: usize, window_h: usize, state: &BrowserState, focused: bool) -> Canvas {
    let mut canvas = Canvas::new(window_w, window_h, Color { r: 235, g: 235, b: 238, a: 255 });

    // address bar background
    canvas.fill_rect(0.0, 0.0, window_w as f32, BAR_HEIGHT, Color::WHITE);
    canvas.fill_rect(0.0, BAR_HEIGHT - 1.0, window_w as f32, 1.0, Color { r: 200, g: 200, b: 200, a: 255 });
    // input box
    let box_color = if focused { Color { r: 250, g: 250, b: 255, a: 255 } } else { Color { r: 240, g: 240, b: 240, a: 255 } };
    canvas.fill_rect(8.0, 6.0, window_w as f32 - 16.0, BAR_HEIGHT - 12.0, box_color);
    canvas.draw_rect_border(8.0, 6.0, window_w as f32 - 16.0, BAR_HEIGHT - 12.0, 1.0, Color { r: 180, g: 180, b: 180, a: 255 });
    let display_addr = if state.address.is_empty() { "wisp://host:port/path".to_string() } else { state.address.clone() };
    canvas.draw_text(&display_addr, 14.0, 14.0, 14.0, Color { r: 40, g: 40, b: 40, a: 255 });
    if focused {
        let cursor_x = 14.0 + state.address.chars().count() as f32 * ((browser_engine::font::GLYPH_W + 1.0) * (14.0 / browser_engine::font::GLYPH_H));
        canvas.fill_rect(cursor_x, 12.0, 2.0, 16.0, Color { r: 40, g: 100, b: 220, a: 255 });
    }

    // page content, offset below the bar, clipped to window, with scroll
    if let Some(page) = &state.page {
        let dest_y_start = BAR_HEIGHT as usize;
        for y in 0..window_h.saturating_sub(dest_y_start) {
            let src_y = y + state.scroll as usize;
            if src_y >= page.height {
                break;
            }
            for x in 0..window_w.min(page.width) {
                canvas.pixels[(y + dest_y_start) * window_w + x] = page.pixels[src_y * page.width + x];
            }
        }
    } else {
        canvas.draw_text(&state.status, 12.0, BAR_HEIGHT as f32 + 12.0, 14.0, Color { r: 100, g: 100, b: 100, a: 255 });
    }

    canvas
}

fn main() {
    let event_loop = EventLoop::new();
    let window = Rc::new(
        WindowBuilder::new()
            .with_title("wisp browser")
            .with_inner_size(winit::dpi::LogicalSize::new(1000.0, 750.0))
            .build(&event_loop)
            .unwrap(),
    );

    let context = unsafe { softbuffer::Context::new(&*window) }.unwrap();
    let mut surface = unsafe { softbuffer::Surface::new(&context, &*window) }.unwrap();

    let start_url = std::env::args().nth(1).unwrap_or_default();
    let mut state = BrowserState {
        address: start_url.clone(),
        status: "enter a wisp:// address and press Enter".to_string(),
        page: None,
        scroll: 0.0,
    };
    let focused = true; // this toy browser treats the address bar as always-focused for simplicity
    if !start_url.is_empty() {
        let size = window.inner_size();
        state.navigate(size.width as f32);
    }

    let mut needs_redraw = true;

    event_loop.run(move |event, _elwt, control_flow| {
        control_flow.set_wait();
        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                control_flow.set_exit();
            }
            Event::WindowEvent { event: WindowEvent::Resized(size), .. } => {
                if size.width > 0 && size.height > 0 {
                    surface
                        .resize(NonZeroU32::new(size.width).unwrap(), NonZeroU32::new(size.height).unwrap())
                        .unwrap();
                    needs_redraw = true;
                }
            }
            Event::WindowEvent { event: WindowEvent::ReceivedCharacter(ch), .. } => {
                if !ch.is_control() {
                    state.address.push(ch);
                    needs_redraw = true;
                }
            }
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput {
                    input: KeyboardInput { state: ElementState::Pressed, virtual_keycode: Some(key), .. },
                    ..
                },
                ..
            } => match key {
                VirtualKeyCode::Back => {
                    state.address.pop();
                    needs_redraw = true;
                }
                VirtualKeyCode::Return => {
                    let width = window.inner_size().width as f32;
                    state.navigate(width);
                    needs_redraw = true;
                }
                VirtualKeyCode::Down => {
                    state.scroll += 40.0;
                    needs_redraw = true;
                }
                VirtualKeyCode::Up => {
                    state.scroll = (state.scroll - 40.0).max(0.0);
                    needs_redraw = true;
                }
                _ => {}
            },
            Event::MainEventsCleared => {
                if needs_redraw {
                    needs_redraw = false;
                    let size = window.inner_size();
                    if size.width == 0 || size.height == 0 {
                        return;
                    }
                    surface
                        .resize(NonZeroU32::new(size.width).unwrap(), NonZeroU32::new(size.height).unwrap())
                        .unwrap();
                    let canvas = composite(size.width as usize, size.height as usize, &state, focused);
                    let mut buffer = surface.buffer_mut().unwrap();
                    for i in 0..buffer.len().min(canvas.pixels.len()) {
                        buffer[i] = canvas.pixels[i];
                    }
                    buffer.present().unwrap();
                }
            }
            _ => {}
        }
    });
}
