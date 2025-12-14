use std::{
    collections::{HashMap, HashSet},
    ffi::c_void,
};

use glam::{Vec2, Vec4};
use image::GenericImageView;
use winit::{event::MouseButton, keyboard::KeyCode};

use crate::{
    entity_manager::EntityManager, enums_types::CameraState, gl_call, input::InputState,
    platform::CursorMode, shaders::Shader,
};

use super::{
    color::hex_to_vec4,
    font::FontManager,
    message_queue::{MessageQueue, UiMessage},
};

pub struct TextureCache {
    pub map: HashMap<String, u32>,
}

impl TextureCache {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn get_or_load(&mut self, path: &str) -> u32 {
        if let Some(&id) = self.map.get(path) {
            return id;
        }

        let id = create_2d_texture(path);
        self.map.insert(path.to_string(), id);
        id
    }
}

pub struct GameUiContext {
    pub vao: u32,
    pub vbo: u32,
    pub quad_vertices: Vec<f32>,
    pub tex_cache: TextureCache,

    pub font_manager: FontManager,
    pub want_capture_mouse: bool,
}

impl GameUiContext {
    pub fn new() -> Self {
        let mut font_manager = FontManager::new();
        font_manager.load_chars(
            "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789:,.!?()[]{}<> ",
        );
        font_manager.setup_buffers();

        let mut vao = 0;
        let mut vbo = 0;

        unsafe {
            gl_call!(gl::GenVertexArrays(1, &mut vao));
            gl_call!(gl::GenBuffers(1, &mut vbo));

            gl_call!(gl::BindVertexArray(vao));
            gl_call!(gl::BindBuffer(gl::ARRAY_BUFFER, vbo));

            // Just allocate enough space once — data will be updated with glBufferSubData.
            gl_call!(gl::BufferData(
                gl::ARRAY_BUFFER,
                1024 * 1024,
                std::ptr::null(),
                gl::DYNAMIC_DRAW
            ));

            let stride = 9 * 4;

            gl_call!(gl::EnableVertexAttribArray(0));
            gl_call!(gl::VertexAttribPointer(
                0,
                3,
                gl::FLOAT,
                gl::FALSE,
                stride,
                std::ptr::null()
            ));

            gl_call!(gl::EnableVertexAttribArray(1));
            gl_call!(gl::VertexAttribPointer(
                1,
                2,
                gl::FLOAT,
                gl::FALSE,
                stride,
                (3 * 4) as *const _
            ));

            gl_call!(gl::EnableVertexAttribArray(2));
            gl_call!(gl::VertexAttribPointer(
                2,
                4,
                gl::FLOAT,
                gl::FALSE,
                stride,
                (5 * 4) as *const _
            ));
        }

        Self {
            vao,
            vbo,
            quad_vertices: vec![0.0; 54],
            tex_cache: TextureCache::new(),
            font_manager,
            want_capture_mouse: false,
        }
    }
}

#[derive(Clone)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub color: Vec4,
    pub text: String,
    pub texture_id: Option<u32>,
}

pub fn do_ui(
    fb_width: f32,
    fb_height: f32,
    mouse_pos: Vec2,
    shader: &Shader,
    font_shader: &Shader,
    mq: &mut MessageQueue,
    paused: &mut bool,
    cm: CursorMode,
    cs: &CameraState,
    ui_ctx: &mut GameUiContext,
    render_gizmos: &mut bool,
    input: &mut InputState,
    em: &mut EntityManager,
) {
    let mut rects = vec![];
    // =============================================================
    // PAUSE PANEL
    // =============================================================
    if *paused {
        let mut w = fb_width * 0.25;
        let h = fb_height * 0.45;

        let main_container = Rect {
            x: (fb_width / 2.0) - (w / 2.0),
            y: (fb_height / 2.0) - (h / 2.0),
            w,
            h,
            color: hex_to_vec4("#030712"),
            text: "".to_string(),
            texture_id: None,
        };

        rects.push(main_container.clone());

        let button_h = main_container.h * 0.15;
        w = main_container.w * 0.95;

        let x = main_container.x + (main_container.w / 2.0) - (w / 2.0);
        // Bottom to top layout
        let mut y = main_container.y + main_container.h - button_h;

        let gap = 15.0; // Pixels

        y -= gap;
        if button(
            "Quit Game",
            x,
            y,
            w,
            button_h,
            mouse_pos,
            mq,
            &mut rects,
            cm,
            None,
            input,
        ) {
            mq.send(UiMessage::WindowShouldClose);
        }

        y -= button_h + gap;
        if button(
            "Save Player Data",
            x,
            y,
            w,
            button_h,
            mouse_pos,
            mq,
            &mut rects,
            cm,
            None,
            input,
        ) {
            em.serialize_entity_data("config/player_data.json");
        }

        y -= button_h + gap;
        if button(
            "Reload World Data",
            x,
            y,
            w,
            button_h,
            mouse_pos,
            mq,
            &mut rects,
            cm,
            None,
            input,
        ) {
            mq.send(UiMessage::ReloadWorldData);
        }

        y -= button_h + gap;
        if button(
            "Gizmo Rendering",
            x,
            y,
            w,
            button_h,
            mouse_pos,
            mq,
            &mut rects,
            cm,
            None,
            input,
        ) {
            *render_gizmos = !*render_gizmos;
        }

        // x button (close window)
        let exit_size = button_h / 3.0;
        let ex = (main_container.x + main_container.w) - (exit_size + gap);
        let ey = main_container.y + gap;

        if button(
            "X", ex, ey, exit_size, exit_size, mouse_pos, mq, &mut rects, cm, None, input,
        ) {
            //mq.send(UiMessage::PauseToggle);
            *paused = false;
        }
    }

    // =============================================================
    // LOWER RIGHT BOX
    // =============================================================
    // Main panel w/h
    // if *cs == CameraState::Third {
    //     let mut w = fb_width * 0.75;
    //     let h = 100.0;
    //     let gap = 10.0;

    //     let main_container = Rect {
    //         x: (fb_width / 2.0) - (w / 2.0),
    //         y: (fb_height - h) - gap,
    //         w,
    //         h,
    //         color: hex_to_vec4("#030712"),
    //         text: "".to_string(),
    //         texture_id: None,
    //     };

    //     let button_h = main_container.h;
    //     let button_w = button_h;

    //     let num_buttons = 10;

    //     let total_width = (button_w * num_buttons as f32) + (gap * (num_buttons as f32 - 1.0));

    //     let mut x = main_container.x + (main_container.w - total_width) / 2.0;
    //     let  y = main_container.y;

    //     for i in 0..num_buttons {
    //         let  label = if i == 9 { "0".to_string() } else { (i + 1).to_string() };

    //         match i {
    //             0 => {
    //                 if button(
    //                     &label,
    //                     x,
    //                     y,
    //                     button_w,
    //                     button_h,
    //                     mouse_pos,
    //                     mq,
    //                     &mut rects,
    //                     cm,
    //                     Some(
    //                         ui_ctx.tex_cache.get_or_load("resources/textures/guy.png")
    //                     ),
    //                     input,
    //                 ) {
    //                     println!("Activated 1");
    //                 }
    //             },
    //             1 => {
    //                 if button(
    //                     &label,
    //                     x,
    //                     y,
    //                     button_w,
    //                     button_h,
    //                     mouse_pos,
    //                     mq,
    //                     &mut rects,
    //                     cm,
    //                     Some(
    //                         ui_ctx.tex_cache.get_or_load("resources/textures/tree.png")
    //                     ),
    //                     input,
    //                 ) {
    //                     println!("Activated 2");
    //                 }
    //             },
    //             2 => {
    //                 if button(
    //                     &label,
    //                     x,
    //                     y,
    //                     button_w,
    //                     button_h,
    //                     mouse_pos,
    //                     mq,
    //                     &mut rects,
    //                     cm,
    //                     Some(
    //                         ui_ctx.tex_cache.get_or_load("resources/textures/moose.png")
    //                     ),
    //                     input,
    //                 ) {
    //                     println!("Activated 3");
    //                 }
    //             }
    //             _ => {
    //                 if button(
    //                     &label,
    //                     x,
    //                     y,
    //                     button_w,
    //                     button_h,
    //                     mouse_pos,
    //                     mq,
    //                     &mut rects,
    //                     cm,
    //                     None,
    //                     input,
    //                 ) {
    //                 }
    //             }
    //         }

    //         x += button_w + gap;
    //     }
    // }

    // =============================================================
    // DRAW ALL BOXES AT THE END
    // =============================================================
    draw_rects(rects, shader, fb_width, fb_height, font_shader, ui_ctx);
}

// =============================================================
// Rendering
// =============================================================
fn draw_rects(
    rects: Vec<Rect>,
    shader: &Shader,
    fb_width: f32,
    fb_height: f32,
    font_shader: &Shader,
    ui_ctx: &mut GameUiContext,
) {
    unsafe {
        gl_call!(gl::Enable(gl::BLEND));
        gl_call!(gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA));
        gl_call!(gl::Disable(gl::DEPTH_TEST));
    }

    for rect in rects.iter() {
        unsafe {
            gl_call!(gl::BindVertexArray(ui_ctx.vao));
            gl_call!(gl::BindBuffer(gl::ARRAY_BUFFER, ui_ctx.vbo));
        }
        shader.activate();
        let x0 = (rect.x / fb_width) * 2.0 - 1.0;
        let y0 = 1.0 - (rect.y / fb_height) * 2.0;
        let x1 = ((rect.x + rect.w) / fb_width) * 2.0 - 1.0;
        let y1 = 1.0 - ((rect.y + rect.h) / fb_height) * 2.0;

        let c = rect.color.to_array();

        unsafe {
            let verts = &mut ui_ctx.quad_vertices;

            let new_verts = vec![
                // pos          //uv         // color
                x0, y0, 0.0, 0.0, 0.0, c[0], c[1], c[2], c[3], x1, y0, 0.0, 1.0, 0.0, c[0], c[1],
                c[2], c[3], x1, y1, 0.0, 1.0, 1.0, c[0], c[1], c[2], c[3], x1, y1, 0.0, 1.0, 1.0,
                c[0], c[1], c[2], c[3], x0, y1, 0.0, 0.0, 1.0, c[0], c[1], c[2], c[3], x0, y0, 0.0,
                0.0, 0.0, c[0], c[1], c[2], c[3],
            ];

            verts.copy_from_slice(&new_verts);

            gl_call!(gl::BufferSubData(
                gl::ARRAY_BUFFER,
                0,
                (verts.len() * std::mem::size_of::<f32>()) as isize,
                verts.as_ptr() as *const _,
            ));

            if let Some(tex_id) = rect.texture_id {
                gl_call!(gl::ActiveTexture(gl::TEXTURE0));
                gl_call!(gl::BindTexture(gl::TEXTURE_2D, tex_id));
                shader.set_bool("use_texture", true);
            } else {
                shader.set_bool("use_texture", false);
            }

            gl_call!(gl::DrawArrays(gl::TRIANGLES, 0, 6));

            // Render font on top;
            if rect.texture_id.is_none() || !rect.text.is_empty() {
                let target_font_height = if rect.text == "X" {
                    rect.h * 0.9
                } else {
                    rect.h * 0.4
                };

                let scale = target_font_height / ui_ctx.font_manager.font_pixel_size;

                ui_ctx.font_manager.render_phrase_centered(
                    &rect.text,
                    rect,
                    fb_width,
                    fb_height,
                    font_shader,
                    scale,
                );
            }
        }
    }

    unsafe {
        gl_call!(gl::Enable(gl::DEPTH_TEST));
        gl_call!(gl::Disable(gl::BLEND));
    }
}

// =============================================================
// UI Elements
// =============================================================
pub fn button(
    label: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    mouse_pos: Vec2,
    mq: &mut MessageQueue,
    rects: &mut Vec<Rect>,
    cm: CursorMode,
    texture_id: Option<u32>,
    input: &mut InputState,
) -> bool {
    let color_900 = hex_to_vec4("#1c1917");
    let color_800 = hex_to_vec4("#292524");
    let mut clicked = false;
    let mut final_color = color_900;
    let mut hovered = false;

    let num_check = match label {
        "1" => Some(KeyCode::Numpad1),
        "2" => Some(KeyCode::Numpad2),
        "3" => Some(KeyCode::Numpad3),
        "4" => Some(KeyCode::Numpad4),
        "5" => Some(KeyCode::Numpad5),
        "6" => Some(KeyCode::Numpad6),
        "7" => Some(KeyCode::Numpad7),
        "8" => Some(KeyCode::Numpad8),
        "9" => Some(KeyCode::Numpad9),
        "0" => Some(KeyCode::Numpad0),
        _ => None,
    };

    if let Some(key) = num_check {
        if input.keys_current.contains(&key) {
            hovered = true;
            clicked = true;
        }
    } else if cm == CursorMode::Normal {
        hovered =
            mouse_pos.x >= x && mouse_pos.y >= y && mouse_pos.x <= x + w && mouse_pos.y <= y + h;

        clicked = hovered && input.left_mouse_just_pressed();

        if clicked {
            input.mouse_current.remove(&MouseButton::Left);
        }
    }

    final_color = if hovered { color_800 } else { color_900 };

    rects.push(Rect {
        x,
        y,
        w,
        h,
        color: final_color,
        text: label.to_string(),
        texture_id,
    });

    clicked
}

fn create_2d_texture(path: &str) -> u32 {
    let mut texture_id = 0;

    unsafe {
        gl_call!(gl::GenTextures(1, &mut texture_id));

        let img = match image::open(path) {
            Ok(data) => Some(data),
            Err(_) => panic!("Failed to load 2D GameUI image"),
        };

        if let Some(img) = img {
            let (img_width, img_height) = img.dimensions();
            let rgba = img.to_rgba8();
            let raw = rgba.as_raw();

            gl_call!(gl::BindTexture(gl::TEXTURE_2D, texture_id));
            gl_call!(gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA as i32,
                img_width as i32,
                img_height as i32,
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                raw.as_ptr() as *const c_void
            ));

            gl_call!(gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_WRAP_S,
                gl::CLAMP_TO_EDGE as i32
            ));
            gl_call!(gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_WRAP_T,
                gl::CLAMP_TO_EDGE as i32
            ));
            gl_call!(gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_MIN_FILTER,
                gl::NEAREST as i32
            ));
            gl_call!(gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_MAG_FILTER,
                gl::NEAREST as i32
            ));
            gl_call!(gl::GenerateMipmap(gl::TEXTURE_2D));
        };
    }

    texture_id
}
