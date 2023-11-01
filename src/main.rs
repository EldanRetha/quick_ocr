#![windows_subsystem = "windows"]

use screenshots::Screen;


use screenshots::image::RgbaImage;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, Event, KeyEvent, WindowEvent, MouseButton};
use winit::event_loop::EventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::monitor::MonitorHandle;
use winit::platform::windows::WindowExtWindows;
use winit::window::{Fullscreen, WindowBuilder, Window, CursorIcon};

#[cfg(target_os = "macos")]
use winit::platform::macos::WindowExtMacOS;

// Window fill references
use softbuffer::{Context, Surface, Buffer};
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::rc::Rc;

use imageproc::drawing::draw_hollow_rect_mut;
use imageproc::rect::Rect;
use image::{Rgba, DynamicImage, GenericImageView, GenericImage};


fn main() -> Result<(), winit::error::EventLoopError>{
    let event_loop = EventLoop::new().unwrap();

    // Capture screens first so we don't obscure them with new windows
    let screens = Screen::all().unwrap();
    let mut screen_caps = Vec::new();
    for screen in screens {
        screen_caps.push(screen.capture().unwrap());
    }
    let mut window_screens  = HashMap::new();
    let mut windows  = HashMap::new();


    let mut monitor_index = 0;
    //let mut windows = Vec::new();
    for monitor in event_loop.available_monitors() {
        let window = create_fake_desktop(&event_loop, &monitor, &screen_caps[monitor_index]);
        let window_ref = window.clone();
        // Transfer ownership of the window to windows so that window outlives the for loop 
        windows.insert(window.id(), window);
        window_screens.insert(window_ref.id(), &screen_caps[monitor_index]);
        monitor_index = monitor_index + 1;
    }

    let mut mouse_pos = PhysicalPosition { x: 0f64, y: 0f64 };
    let mut mouse_down = false;
    let mut mouse_down_pos = PhysicalPosition { x: 0f64, y: 0f64 };
    let mut mouse_up_pos = PhysicalPosition { x: 0f64, y: 0f64 };
    let result = event_loop.run(move |event, elwt| {
        if let Event::WindowEvent { event, window_id } = event {
            match event {
                WindowEvent::CloseRequested => elwt.exit(),
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            logical_key: key,
                            state: ElementState::Pressed,
                            ..
                        },
                    ..
                } => match key {
                    Key::Named(NamedKey::Escape) => elwt.exit(),
                    _ => (),
                },
                WindowEvent::RedrawRequested => {
                    
                }
                WindowEvent::MouseInput { 
                    button,
                    state,
                    ..
                } => match (button, state) {
                    (MouseButton::Left, ElementState::Pressed) => {
                        mouse_down_pos = mouse_pos;
                        mouse_down = true;
                    },
                    (MouseButton::Left, ElementState::Released) => {
                        mouse_up_pos = mouse_pos;
                        mouse_down = false;
                        println!("Mouse moved from {:?} to {:?} on window {:?}", mouse_down_pos, mouse_up_pos, window_id);
                        let rect = calculate_rect(mouse_down_pos, mouse_up_pos);
                        process_image(window_screens[&window_id], rect);
                        elwt.exit();
                    },
                    _ => ()
                },
                WindowEvent::CursorMoved {
                    position,
                    ..
                } => match position {
                    position => {
                        mouse_pos = position;
                        if mouse_down {
                            let screen =  window_screens[&window_id];
                            mouse_pos = keep_mouse_in_window(mouse_pos, screen.width() as f64, screen.height() as f64);
                            let rect = calculate_rect(mouse_down_pos, mouse_pos);
                            on_mouse_drag(&windows[&window_id], screen, rect);
                        }
                    }
                }
                _ => (),
            }
        }
    });

    return result;
}

fn keep_mouse_in_window(mouse_pos: PhysicalPosition<f64>, x_max: f64, y_max: f64) -> PhysicalPosition<f64>{
    PhysicalPosition{
        x: mouse_pos.x.min(x_max).max(0f64),
        y: mouse_pos.y.min(y_max).max(0f64)
    }
}

fn calculate_rect(pos1: PhysicalPosition<f64>, pos2: PhysicalPosition<f64>) -> Rect {
    let x1 = pos1.x as i32;
    let y1 = pos1.y as i32;
    let x2 = pos2.x as i32;
    let y2 = pos2.y as i32;

    let width = i32::abs(x1-x2).max(1);
    let height = i32::abs(y1-y2).max(1);

    Rect::at(x1.min(x2), y1.min(y2))
        .of_size(width as u32, height as u32)
}

fn create_fake_desktop(event_loop: &EventLoop<()>, monitor: &MonitorHandle, image: &RgbaImage) -> Rc<Window> {
    let window =Rc::new(WindowBuilder::new()
        .with_title("")
        .with_visible(false)
        .build(&event_loop)
        .unwrap());
    let fullscreen = Some(Fullscreen::Borderless(Some(monitor.clone())));
    window.set_fullscreen(fullscreen);
    window.set_skip_taskbar(true);
    window.set_cursor_icon(CursorIcon::Crosshair);
    
    let context = Context::new(window.clone()).unwrap();
    let mut surface = Surface::new(&context, window.clone()).unwrap();

    surface.resize(NonZeroU32::new(image.width()).unwrap(), NonZeroU32::new(image.height()).unwrap()).expect("Failed to resize window");

    let mut buffer = surface.buffer_mut().unwrap();
    
    draw_shaded_image(&mut buffer, image);

    // Presenting the buffering after appears to cause an issue where the window isn't actually made visible
    // immediately
    window.set_visible(true);
    buffer.present().expect("Failed to present buffer");

    return window;
}

fn on_mouse_drag(window: &Rc<Window>, image: &RgbaImage, rect: Rect) {
    let context = Context::new(window.clone()).unwrap();
    let mut surface = softbuffer::Surface::new(&context, window.clone()).unwrap();

    surface.resize(NonZeroU32::new(image.width()).unwrap(), NonZeroU32::new(image.height()).unwrap()).expect("Failed to resize window");

    let mut buffer = surface.buffer_mut().unwrap();
    let mut wrapped_buffer = BufferWrapper{
        buffer: &mut buffer,
        width: image.width(),
        height: image.height(),
        x: rect.left() as u32,
        y: rect.top() as u32
    };
    draw_shaded_image(&mut wrapped_buffer.buffer, image);
    let sub_image = image.view(rect.left() as u32, rect.top() as u32, rect.width(), rect.height());
    wrapped_buffer.copy_from(&*sub_image, rect.left() as u32, rect.top() as u32).expect("Failed to copy subimage");
    draw_hollow_rect_mut(&mut wrapped_buffer, rect, Rgba([255,0,0,255]));

    buffer.present().expect("Failed to present buffer");
}



struct BufferWrapper<'a, 'b> {
    buffer: &'a mut Buffer<'b, Rc<Window>,Rc<Window>>,
    width: u32,
    height: u32,
    x: u32,
    y: u32
}

impl<'a, 'b> GenericImageView for BufferWrapper<'a, 'b> {
    type Pixel = image::Rgba<u8>;

    fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
    fn bounds(&self) -> (u32, u32, u32, u32) {
        (self.x, self.y, self.width, self.height)
    }

    fn get_pixel(&self, x: u32, y: u32) -> Self::Pixel {
        let color_u32 = self.buffer[(y * self.width + x) as usize];
        let alpha = (color_u32 & 0xFF000000 >> 24) as u8;
        let red = (color_u32 & 0x00FF0000 >> 16) as u8;
        let green = (color_u32 & 0x0000FF00 >> 8) as u8;
        let blue = (color_u32 & 0x000000FF) as u8;
        Rgba::from([red, green, blue, alpha])
    }

}

impl<'a, 'b> GenericImage for BufferWrapper<'a, 'b> {
    fn put_pixel(&mut self, x: u32, y: u32, pixel: Self::Pixel) {
        let red = pixel[0]  as u32;
        let green = pixel[1] as u32;
        let blue = pixel[2] as u32;
        let alpha = pixel[3] as u32;
        let color_u32 = blue | (green << 8) | (red << 16) | (alpha << 24);
        self.buffer[(y * self.width + x) as usize] = color_u32;
    }

    #[allow(unused_variables)]
    fn get_pixel_mut(&mut self, x: u32, y: u32) -> &mut Self::Pixel {
        todo!();
    }

    #[allow(unused_variables)]
    fn blend_pixel(&mut self, x: u32, y: u32, pixel: Self::Pixel) {
        todo!();
    }
}

fn draw_shaded_image(buffer: &mut Buffer<'_, Rc<Window>, Rc<Window>>, image: &RgbaImage) {
    for (i, pixel) in image.pixels().enumerate() {
        let red;
        let green;
        let blue;
        let u8max = u8::MAX as u32;
        
        red = (pixel.0[0] as u32 + 50).min(u8max) ;
        green = (pixel.0[1] as u32 + 50).min(u8max);
        blue = (pixel.0[2] as u32 + 50).min(u8max);

        let color = blue | (green << 8) | (red << 16) ;
        buffer[i] = color;
    }
}

fn process_image(image: &RgbaImage, rect: Rect) {
    use rusty_tesseract::{Image, Args};
    use clipboard_win::{Clipboard, formats, Setter};
    
    // TODO: Add bounds checks
    let sub_image = image::imageops::crop_imm(image, rect.left() as u32, rect.top() as u32, rect.width(), rect.height());

    let cropped_image = sub_image.to_image();

    //cropped_image.save("testImage.png");

    let dyn_image = Image::from_dynamic_image(&DynamicImage::ImageRgba8(cropped_image)).unwrap();

    let args = Args {
        lang: "jpn_vert".to_string(),
        dpi: Some(150),
        psm: Some(5),
        oem: Some(3),
        config_variables: HashMap::new()
    };

    let output = rusty_tesseract::image_to_string(&dyn_image, &args).unwrap();
    println!("Text: {:?}", &output);

    let _clip = Clipboard::new_attempts(10).expect("Open clipboard");
    formats::Unicode.write_clipboard(&output).expect("Write sample");

}