#![windows_subsystem = "windows"]

use screenshots::Screen;


use screenshots::image::RgbaImage;
use winit::dpi::{PhysicalSize, PhysicalPosition};
use winit::event::{ElementState, Event, KeyEvent, WindowEvent, MouseButton};
use winit::event_loop::EventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::monitor::MonitorHandle;
use winit::platform::windows::WindowExtWindows;
use winit::window::{Fullscreen, WindowBuilder, Window, self, CursorIcon};

#[cfg(target_os = "macos")]
use winit::platform::macos::WindowExtMacOS;

// Window fill references
use softbuffer::{Context, Surface, Buffer};
use std::collections::{HashMap};
use std::num::NonZeroU32;
use std::rc::Rc;
use winit::window::WindowId;

use imageproc::drawing::draw_hollow_rect_mut;
use imageproc::rect::Rect;
use image::{Rgba, EncodableLayout};


fn main() -> Result<(), winit::error::EventLoopError>{
    let event_loop = EventLoop::new().unwrap();

    // Capture screens first so we don't obscure them with new windows
    let screens = Screen::all().unwrap();
    let mut screen_caps: Vec<RgbaImage> = Vec::new();
    for screen in screens {
        screen_caps.push(screen.capture().unwrap());
    }
    let mut window_screens  = HashMap::new();
    let mut windows  = HashMap::new();


    let mut monitor_index = 0;
    //let mut windows = Vec::new();
    for monitor in event_loop.available_monitors() {
        let window = create_fake_desktop(&event_loop, &monitor, monitor_index, &screen_caps[monitor_index]);
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
    let mut current_window = WindowId::from(0);
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
                        current_window = window_id;
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
                    device_id,
                    position
                } => match position {
                    position => {
                        mouse_pos = position;
                        if mouse_down {
                            //println!("Mouse origin {:?} Current pos {:?}", mouse_down_pos, mouse_pos);
                            let rect = calculate_rect(mouse_down_pos, mouse_pos);
                            on_mouse_drag(&windows[&window_id], window_screens[&window_id], rect);
                        }
                    },
                    _ => ()
                }
                _ => (),
            }
        }
    });

    return result;
}


fn calculate_rect(pos1: PhysicalPosition<f64>, pos2: PhysicalPosition<f64>) -> Rect {
    let x1 = pos1.x as i32;
    let y1 = pos1.y as i32;
    let x2 = pos2.x as i32;
    let y2 = pos2.y as i32;

    let width = i32::abs(x1-x2) + 1;
    let height = i32::abs(y1-y2) + 1;

    Rect::at(x1.min(x2), y1.min(y2))
        .of_size(width as u32, height as u32)
}

fn create_fake_desktop(event_loop: &EventLoop<()>, monitor: &MonitorHandle, monitor_index: usize, image: &RgbaImage) -> Rc<Window> {
    let window =Rc::new(WindowBuilder::new()
        .with_title("")
        .with_visible(false)
        .build(&event_loop)
        .unwrap());
    window.set_skip_taskbar(true);
    window.set_cursor_icon(CursorIcon::Crosshair);
    
    let context = Context::new(window.clone()).unwrap();
    let mut surface = Surface::new(&context, window.clone()).unwrap();

    surface.resize(NonZeroU32::new(image.width()).unwrap(), NonZeroU32::new(image.height()).unwrap());

    let mut buffer = surface.buffer_mut().unwrap();
    
    buffer_image(&mut buffer, image);

    buffer.present();

    let fullscreen = Some(Fullscreen::Borderless(Some(monitor.clone())));
    window.set_fullscreen(fullscreen);
    window.set_visible(true);

    return window;
}

fn on_mouse_drag(window: &Rc<Window>, image: &RgbaImage, rect: Rect) {
    let context = Context::new(window.clone()).unwrap();
    let mut surface = softbuffer::Surface::new(&context, window.clone()).unwrap();

    surface.resize(NonZeroU32::new(image.width()).unwrap(), NonZeroU32::new(image.height()).unwrap());

    let mut buffer = surface.buffer_mut().unwrap();

    let image_with_square = draw_square_on_image(image, rect);
    buffer_image(&mut buffer, &image_with_square);
    buffer.present();
}


fn draw_square_on_image(image: &RgbaImage, rect: Rect) -> RgbaImage {

    let mut out_image = image.clone();

    draw_hollow_rect_mut(&mut out_image, rect, Rgba([255,0,0,0]));
    return out_image;
}


fn buffer_image(buffer: &mut Buffer<'_, Rc<Window>, Rc<Window>>, image: &RgbaImage) {
    let mut i = 0;

    for pixel in image.pixels() {
        let red = pixel.0[0] as u32;
        let green = pixel.0[1] as u32;
        let blue = pixel.0[2] as u32;

        let color = blue | (green << 8) | (red << 16) ;
        buffer[i] = color;
        i = i+1;
    }
}

fn process_image(image: &RgbaImage, rect: Rect) {
    use rusty_tesseract::{Image, Args};
    use image::DynamicImage;
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