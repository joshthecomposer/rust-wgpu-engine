use glam::{Mat4, Quat, Vec3};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::panic;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::{
    assets,
    config::{game_config::GameConfig, Config},
    game::Game,
    platform::{self, RenderSurface},
    shaders::{Shader, ShaderProfile},
};

const WEB_GAME_ASSETS: &[WebSceneEntityDescriptor] = &[
    WebSceneEntityDescriptor {
        model_path: "resources/models/static/weapons/swords/001_orc_sword.txt",
        position: Vec3::new(-0.75, 0.0, 0.0),
        rotation_axis: Vec3::Y,
        rotation_speed: 0.65,
        base_rotation_xyz: Vec3::new(-0.35, 0.0, 0.0),
        scale: 1.45,
    },
    WebSceneEntityDescriptor {
        model_path: "resources/models/static/weapons/swords/001_double_axe_new.txt",
        position: Vec3::new(0.85, -0.08, 0.0),
        rotation_axis: Vec3::Y,
        rotation_speed: -0.48,
        base_rotation_xyz: Vec3::new(0.0, 0.0, 0.25),
        scale: 1.25,
    },
];

pub struct WebGameRuntime {
    game: Game,
    start_ms: f64,
    frame_count: u64,
}

impl WebGameRuntime {
    fn new() -> Result<Self, JsValue> {
        let mut platform = platform::web_canvas::WebCanvasPlatform::new("game-canvas", 1280, 720)?;
        platform.load_gl();

        let mut config = GameConfig::load_or_create_default("config/game_config.json");
        config.win_width = platform.fb_width as f32;
        config.win_height = platform.fb_height as f32;
        let game = Game::new(platform, config);

        Ok(Self {
            game,
            start_ms: 0.0,
            frame_count: 0,
        })
    }

    fn tick(&mut self, now_ms: f64) {
        if self.frame_count == 0 {
            self.start_ms = now_ms;
        }

        let elapsed = ((now_ms - self.start_ms) * 0.001) as f32;
        self.game.tick(elapsed);
        self.frame_count += 1;

        if self.frame_count == 1 {
            web_sys::console::log_1(
                &format!(
                    "real Game::tick running with {} populated transforms",
                    self.game.world.ecs.transforms.iter().count()
                )
                .into(),
            );
        }

        if self.frame_count % 60 == 0 && self.game.input.wasd_is_down() {
            let camera = &self.game.world.camera;
            web_sys::console::log_1(
                &format!(
                    "web input active: camera={:?} pos=({:.2}, {:.2}, {:.2}) paused={}",
                    camera.move_state,
                    camera.position.x,
                    camera.position.y,
                    camera.position.z,
                    self.game.paused
                )
                .into(),
            );
        }
    }
}

struct WebGameState {
    renderer: WebSceneRenderer,
    world: WebGameWorld,
    camera: WebCamera,
}

impl WebGameState {
    fn new(surface: RenderSurface<'_>) -> Result<Self, JsValue> {
        let renderer = WebSceneRenderer::new(surface)?;
        let world = WebGameWorld::load()?;
        let camera = WebCamera::new();

        Ok(Self {
            renderer,
            world,
            camera,
        })
    }

    fn render(&mut self, elapsed: f32, surface: RenderSurface<'_>) {
        self.world.update(elapsed);
        self.camera.update(elapsed, &surface);

        let pulse = elapsed.sin() * 0.5 + 0.5;
        unsafe {
            gl::Viewport(0, 0, surface.fb_width as i32, surface.fb_height as i32);
            gl::ClearColor(0.04 + 0.04 * pulse, 0.07, 0.13 + 0.08 * (1.0 - pulse), 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }

        self.renderer
            .render_world(&self.world, &self.camera, elapsed);
    }
}

struct WebCamera {
    projection: Mat4,
    view: Mat4,
}

impl WebCamera {
    fn new() -> Self {
        Self {
            projection: Mat4::IDENTITY,
            view: Mat4::IDENTITY,
        }
    }

    fn update(&mut self, elapsed: f32, surface: &RenderSurface<'_>) {
        let aspect = surface.fb_width as f32 / surface.fb_height.max(1) as f32;
        let orbit = elapsed * 0.16;
        let camera_pos = Vec3::new(orbit.sin() * 0.7, 1.15, 4.35 + orbit.cos() * 0.35);

        self.projection = Mat4::perspective_rh_gl(45.0_f32.to_radians(), aspect, 0.1, 100.0);
        self.view = Mat4::look_at_rh(camera_pos, Vec3::new(0.0, 0.05, 0.0), Vec3::Y);
    }
}

struct WebSceneRenderer {
    shader: Shader,
}

impl WebSceneRenderer {
    fn new(surface: RenderSurface<'_>) -> Result<Self, JsValue> {
        if !surface.capabilities.is_gles_like {
            return Err(JsValue::from_str(
                "web game renderer expected WebGL capabilities",
            ));
        }

        let shader = Shader::new_with_profile(
            "resources/shaders/web_game_scene_es300.glsl",
            ShaderProfile::GlslEs300,
        );
        shader.activate();
        shader.set_int("diffuse_texture", 0);

        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::DepthFunc(gl::LEQUAL);
        }

        Ok(Self { shader })
    }

    fn render_world(&self, world: &WebGameWorld, camera: &WebCamera, elapsed: f32) {
        self.shader.activate();

        self.shader.set_mat4("projection", camera.projection);
        self.shader.set_mat4("view", camera.view);
        self.shader.set_float("elapsed", elapsed);

        for entity in &world.entities {
            let model = entity.model_matrix();
            self.shader.set_mat4("model", model);
            self.shader
                .set_bool("use_texture", entity.renderable.use_texture);
            entity.renderable.draw();
        }
    }
}

struct WebGameWorld {
    entities: Vec<WebGameEntity>,
}

impl WebGameWorld {
    fn load() -> Result<Self, JsValue> {
        let entities = WEB_GAME_ASSETS
            .iter()
            .map(WebGameEntity::load)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self { entities })
    }

    fn update(&mut self, elapsed: f32) {
        for entity in &mut self.entities {
            entity.update(elapsed);
        }
    }
}

struct WebSceneEntityDescriptor {
    model_path: &'static str,
    position: Vec3,
    rotation_axis: Vec3,
    rotation_speed: f32,
    base_rotation_xyz: Vec3,
    scale: f32,
}

struct WebGameEntity {
    position: Vec3,
    rotation_axis: Vec3,
    rotation_speed: f32,
    base_rotation: Quat,
    current_rotation: Quat,
    scale: f32,
    renderable: WebRenderableModel,
}

impl WebGameEntity {
    fn load(desc: &WebSceneEntityDescriptor) -> Result<Self, JsValue> {
        Ok(Self {
            position: desc.position,
            rotation_axis: desc.rotation_axis,
            rotation_speed: desc.rotation_speed,
            base_rotation: Quat::from_euler(
                glam::EulerRot::XYZ,
                desc.base_rotation_xyz.x,
                desc.base_rotation_xyz.y,
                desc.base_rotation_xyz.z,
            ),
            current_rotation: Quat::IDENTITY,
            scale: desc.scale,
            renderable: WebRenderableModel::load(desc.model_path)?,
        })
    }

    fn update(&mut self, elapsed: f32) {
        let spin = Quat::from_axis_angle(
            self.rotation_axis.normalize(),
            elapsed * self.rotation_speed,
        );
        self.current_rotation = spin * self.base_rotation;
    }

    fn model_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(
            Vec3::splat(self.scale),
            self.current_rotation,
            self.position,
        )
    }
}

struct WebRenderableModel {
    vao: u32,
    _vbo: u32,
    _ebo: u32,
    texture: u32,
    use_texture: bool,
    index_count: i32,
}

impl WebRenderableModel {
    fn load(path: &str) -> Result<Self, JsValue> {
        let mesh = load_web_model_mesh(path).map_err(|error| JsValue::from_str(&error))?;
        let texture = match mesh.diffuse_texture.as_deref() {
            Some(path) => upload_web_texture(path).map_err(|error| JsValue::from_str(&error))?,
            None => 0,
        };
        let use_texture = texture != 0;
        let mut vao = 0;
        let mut vbo = 0;
        let mut ebo = 0;

        unsafe {
            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(1, &mut vbo);
            gl::GenBuffers(1, &mut ebo);
            gl::BindVertexArray(vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (mesh.vertices.len() * std::mem::size_of::<f32>()) as isize,
                mesh.vertices.as_ptr().cast(),
                gl::STATIC_DRAW,
            );
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (mesh.indices.len() * std::mem::size_of::<u32>()) as isize,
                mesh.indices.as_ptr().cast(),
                gl::STATIC_DRAW,
            );

            let stride = (12 * std::mem::size_of::<f32>()) as i32;
            gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, stride, std::ptr::null());
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(
                1,
                3,
                gl::FLOAT,
                gl::FALSE,
                stride,
                (3 * std::mem::size_of::<f32>()) as *const std::ffi::c_void,
            );
            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(
                2,
                2,
                gl::FLOAT,
                gl::FALSE,
                stride,
                (6 * std::mem::size_of::<f32>()) as *const std::ffi::c_void,
            );
            gl::EnableVertexAttribArray(2);
            gl::VertexAttribPointer(
                3,
                4,
                gl::FLOAT,
                gl::FALSE,
                stride,
                (8 * std::mem::size_of::<f32>()) as *const std::ffi::c_void,
            );
            gl::EnableVertexAttribArray(3);
            gl::BindVertexArray(0);
        }

        Ok(Self {
            vao,
            _vbo: vbo,
            _ebo: ebo,
            texture,
            use_texture,
            index_count: mesh.indices.len() as i32,
        })
    }

    fn draw(&self) {
        unsafe {
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, self.texture);
            gl::BindVertexArray(self.vao);
            gl::DrawElements(
                gl::TRIANGLES,
                self.index_count,
                gl::UNSIGNED_INT,
                std::ptr::null(),
            );
            gl::BindVertexArray(0);
        }
    }
}

struct WebModelMesh {
    vertices: Vec<f32>,
    indices: Vec<u32>,
    diffuse_texture: Option<String>,
}

fn load_web_model_mesh(path: &str) -> Result<WebModelMesh, String> {
    let source =
        assets::read_text(path).map_err(|error| format!("failed to read {path}: {error}"))?;
    let mut lines = source.lines().peekable();
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut diffuse_texture = None;
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);

    while let Some(line) = lines.next() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "TEXTURE_DIFFUSE:" => {
                let texture = parts.get(1).ok_or("missing diffuse texture path")?.trim();
                diffuse_texture = Some(resolve_web_asset_relative_path(path, texture));
            }
            "VERT:" => {
                let position = parse_web_vec3(lines.next().ok_or("missing vertex position")?)?;
                let normal = parse_web_vec3(lines.next().ok_or("missing vertex normal")?)?;
                let uv = parse_web_vec2(lines.next().ok_or("missing vertex uv")?)?;
                let mut color = [1.0, 1.0, 1.0, 1.0];

                if let Some(next) = lines.peek() {
                    if next.trim_start().starts_with("COLOR:") {
                        let color_line = lines.next().ok_or("missing vertex color")?;
                        color = parse_web_color(color_line)?;
                    }
                }

                min = min.min(position);
                max = max.max(position);
                vertices.extend_from_slice(&[
                    position.x, position.y, position.z, normal.x, normal.y, normal.z, uv[0], uv[1],
                    color[0], color[1], color[2], color[3],
                ]);
            }
            "INDEX_COUNT:" => {
                let expected_count = parts
                    .get(1)
                    .ok_or("missing index count")?
                    .parse::<usize>()
                    .map_err(|error| format!("invalid index count: {error}"))?;
                let index_line = lines.next().ok_or("missing index list")?;
                indices = index_line
                    .split_whitespace()
                    .map(|value| {
                        value
                            .parse::<u32>()
                            .map_err(|error| format!("invalid index '{value}': {error}"))
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                if indices.len() != expected_count {
                    return Err(format!(
                        "index count mismatch in {path}: expected {expected_count}, got {}",
                        indices.len()
                    ));
                }
            }
            _ => {}
        }
    }

    if vertices.is_empty() || indices.is_empty() {
        return Err(format!("model did not contain mesh data: {path}"));
    }

    normalize_web_model_vertices(&mut vertices, min, max);

    Ok(WebModelMesh {
        vertices,
        indices,
        diffuse_texture,
    })
}

fn normalize_web_model_vertices(vertices: &mut [f32], min: Vec3, max: Vec3) {
    let center = (min + max) * 0.5;
    let extents = max - min;
    let scale = 1.0 / extents.max_element().max(0.001);

    for vertex in vertices.chunks_exact_mut(12) {
        vertex[0] = (vertex[0] - center.x) * scale;
        vertex[1] = (vertex[1] - center.y) * scale;
        vertex[2] = (vertex[2] - center.z) * scale;
    }
}

fn resolve_web_asset_relative_path(model_path: &str, texture_path: &str) -> String {
    if texture_path.contains('/') || texture_path.contains('\\') {
        return texture_path.replace('\\', "/");
    }

    match model_path.rsplit_once('/') {
        Some((directory, _)) => format!("{directory}/{texture_path}"),
        None => texture_path.to_string(),
    }
}

fn upload_web_texture(path: &str) -> Result<u32, String> {
    let image = assets::load_image(path)?;
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    let mut texture = 0;

    unsafe {
        gl::GenTextures(1, &mut texture);
        gl::BindTexture(gl::TEXTURE_2D, texture);
        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGBA as i32,
            width as i32,
            height as i32,
            0,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            rgba.as_raw().as_ptr().cast(),
        );
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);
        gl::TexParameteri(
            gl::TEXTURE_2D,
            gl::TEXTURE_MIN_FILTER,
            gl::NEAREST_MIPMAP_LINEAR as i32,
        );
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
        gl::GenerateMipmap(gl::TEXTURE_2D);
        gl::BindTexture(gl::TEXTURE_2D, 0);
    }

    Ok(texture)
}

fn parse_web_vec3(line: &str) -> Result<Vec3, String> {
    let values = parse_web_floats(line, 3)?;
    Ok(Vec3::new(values[0], values[1], values[2]))
}

fn parse_web_vec2(line: &str) -> Result<[f32; 2], String> {
    let values = parse_web_floats(line, 2)?;
    Ok([values[0], values[1]])
}

fn parse_web_color(line: &str) -> Result<[f32; 4], String> {
    let values = parse_web_floats(line.trim_start_matches("COLOR:").trim(), 4)?;
    Ok([values[0], values[1], values[2], values[3]])
}

fn parse_web_floats(line: &str, expected: usize) -> Result<Vec<f32>, String> {
    let values = line
        .split_whitespace()
        .map(|value| {
            value
                .parse::<f32>()
                .map_err(|error| format!("invalid float '{value}': {error}"))
        })
        .collect::<Result<Vec<_>, _>>()?;

    if values.len() != expected {
        return Err(format!(
            "expected {expected} float values, got {} in '{line}'",
            values.len()
        ));
    }

    Ok(values)
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    install_panic_hook();

    let runtime = Rc::new(RefCell::new(WebGameRuntime::new()?));
    let input_queue: Rc<RefCell<VecDeque<WebInputEvent>>> = Rc::new(RefCell::new(VecDeque::new()));
    install_input_handlers(runtime.clone(), input_queue.clone())?;
    let frame_callback: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let frame_callback_handle = frame_callback.clone();
    let runtime_handle = runtime.clone();
    let input_queue_handle = input_queue.clone();

    *frame_callback_handle.borrow_mut() = Some(Closure::wrap(Box::new(move |now_ms: f64| {
        drain_and_apply_web_inputs(&runtime_handle, &input_queue_handle);

        runtime_handle.borrow_mut().tick(now_ms);

        drain_and_apply_web_inputs(&runtime_handle, &input_queue_handle);

        if let Some(window) = web_sys::window() {
            if let Some(callback) = frame_callback.borrow().as_ref() {
                let _ = window.request_animation_frame(callback.as_ref().unchecked_ref());
            }
        }
    }) as Box<dyn FnMut(f64)>));

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("missing window"))?;
    let callback_borrow = frame_callback_handle.borrow();
    let callback_ref = callback_borrow
        .as_ref()
        .ok_or_else(|| JsValue::from_str("missing frame callback"))?
        .as_ref()
        .unchecked_ref();
    window.request_animation_frame(callback_ref)?;

    web_sys::console::log_1(&"learn-opengl-rs real Game bootstrap initialized".into());
    Ok(())
}

#[derive(Clone)]
enum WebInputEvent {
    Key {
        keycode: winit::keyboard::KeyCode,
        state: winit::event::ElementState,
    },
    MouseMove {
        x: f32,
        y: f32,
        dx: f64,
        dy: f64,
    },
    MouseButton {
        button: winit::event::MouseButton,
        state: winit::event::ElementState,
    },
    Scroll {
        x: f32,
        y: f32,
    },
}

fn apply_pending_web_inputs(rt: &mut WebGameRuntime, event: WebInputEvent) {
    match event {
        WebInputEvent::Key { keycode, state } => {
            if state == winit::event::ElementState::Pressed && is_movement_key(keycode) {
                web_sys::console::log_1(
                    &format!("web keydown movement: mapped={keycode:?}").into(),
                );
            }
            rt.game.handle_web_keyboard_input(keycode, state);
        }
        WebInputEvent::MouseMove { x, y, dx, dy } => {
            rt.game.handle_web_mouse_move(x, y, dx, dy);
        }
        WebInputEvent::MouseButton { button, state } => {
            rt.game.handle_web_mouse_button(button, state);
        }
        WebInputEvent::Scroll { x, y } => {
            rt.game.handle_web_scroll(x, y);
        }
    }
}

/// Apply DOM input that was queued while `runtime` was already mutably borrowed
/// (e.g. events fired during `request_animation_frame`).
fn drain_and_apply_web_inputs(
    runtime: &RefCell<WebGameRuntime>,
    queue: &RefCell<VecDeque<WebInputEvent>>,
) {
    let drained: Vec<_> = queue.borrow_mut().drain(..).collect();
    if drained.is_empty() {
        return;
    }
    let mut rt = runtime.borrow_mut();
    for event in drained {
        apply_pending_web_inputs(&mut rt, event);
    }
}

fn install_panic_hook() {
    panic::set_hook(Box::new(|info| {
        let payload = info
            .payload()
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| info.payload().downcast_ref::<String>().map(String::as_str))
            .unwrap_or("panic payload was not a string");
        let location = info
            .location()
            .map(|location| {
                format!(
                    "{}:{}:{}",
                    location.file(),
                    location.line(),
                    location.column()
                )
            })
            .unwrap_or_else(|| "unknown location".to_string());

        web_sys::console::error_1(&format!("Rust panic at {location}: {payload}").into());
    }));
}

fn install_input_handlers(
    runtime: Rc<RefCell<WebGameRuntime>>,
    input_queue: Rc<RefCell<VecDeque<WebInputEvent>>>,
) -> Result<(), JsValue> {
    let canvas = runtime.borrow().game.platform.canvas.clone();
    focus_canvas(&canvas);
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("missing document"))?;

    {
        let input_queue = input_queue.clone();
        let closure = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::wrap(Box::new(
            move |event: web_sys::KeyboardEvent| {
            if let Some(keycode) = map_keyboard_code(&event.code()) {
                event.prevent_default();
                input_queue.borrow_mut().push_back(WebInputEvent::Key {
                    keycode,
                    state: winit::event::ElementState::Pressed,
                });
            }
        },
        ));
        document.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    {
        let input_queue = input_queue.clone();
        let closure = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::wrap(Box::new(
            move |event: web_sys::KeyboardEvent| {
            if let Some(keycode) = map_keyboard_code(&event.code()) {
                event.prevent_default();
                input_queue.borrow_mut().push_back(WebInputEvent::Key {
                    keycode,
                    state: winit::event::ElementState::Released,
                });
            }
        },
        ));
        document.add_event_listener_with_callback("keyup", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    {
        let input_queue = input_queue.clone();
        let closure = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
            event.prevent_default();
            let x = event.offset_x() as f32;
            let y = event.offset_y() as f32;
            let dx = event.movement_x() as f64;
            let dy = event.movement_y() as f64;
            input_queue
                .borrow_mut()
                .push_back(WebInputEvent::MouseMove { x, y, dx, dy });
        },
        ));
        canvas.add_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    {
        let runtime = runtime.clone();
        let input_queue = input_queue.clone();
        let canvas_for_focus = canvas.clone();
        let closure = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
            if let Some(button) = map_mouse_button(event.button()) {
                event.prevent_default();
                focus_canvas(&canvas_for_focus);

                if let Ok(mut rt) = runtime.try_borrow_mut() {
                    if button == winit::event::MouseButton::Left
                        && !rt.game.paused
                        && !rt.game.cursor_unlocked()
                    {
                        rt.game.platform.request_pointer_lock();
                    }
                    rt.game
                        .handle_web_mouse_button(button, winit::event::ElementState::Pressed);
                } else {
                    input_queue
                        .borrow_mut()
                        .push_back(WebInputEvent::MouseButton {
                            button,
                            state: winit::event::ElementState::Pressed,
                        });
                }
            }
        },
        ));
        canvas.add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    {
        let input_queue = input_queue.clone();
        let closure = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
            if let Some(button) = map_mouse_button(event.button()) {
                event.prevent_default();
                input_queue
                    .borrow_mut()
                    .push_back(WebInputEvent::MouseButton {
                        button,
                        state: winit::event::ElementState::Released,
                    });
            }
        },
        ));
        canvas.add_event_listener_with_callback("mouseup", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    {
        let input_queue = input_queue.clone();
        let closure = Closure::<dyn FnMut(web_sys::WheelEvent)>::wrap(Box::new(
            move |event: web_sys::WheelEvent| {
            event.prevent_default();
            input_queue.borrow_mut().push_back(WebInputEvent::Scroll {
                x: event.delta_x() as f32,
                y: event.delta_y() as f32,
            });
        },
        ));
        canvas.add_event_listener_with_callback("wheel", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    Ok(())
}

fn focus_canvas(canvas: &web_sys::HtmlCanvasElement) {
    if let Some(element) = canvas.dyn_ref::<web_sys::HtmlElement>() {
        let _ = element.focus();
    }
}

fn map_keyboard_code(code: &str) -> Option<winit::keyboard::KeyCode> {
    use winit::keyboard::KeyCode;

    Some(match code {
        "KeyW" => KeyCode::KeyW,
        "KeyA" => KeyCode::KeyA,
        "KeyS" => KeyCode::KeyS,
        "KeyD" => KeyCode::KeyD,
        "KeyF" => KeyCode::KeyF,
        "KeyG" => KeyCode::KeyG,
        "KeyL" => KeyCode::KeyL,
        "KeyU" => KeyCode::KeyU,
        "ShiftLeft" => KeyCode::ShiftLeft,
        "Space" => KeyCode::Space,
        "Escape" => KeyCode::Escape,
        "Tab" => KeyCode::Tab,
        _ => return None,
    })
}

fn is_movement_key(keycode: winit::keyboard::KeyCode) -> bool {
    matches!(
        keycode,
        winit::keyboard::KeyCode::KeyW
            | winit::keyboard::KeyCode::KeyA
            | winit::keyboard::KeyCode::KeyS
            | winit::keyboard::KeyCode::KeyD
    )
}

fn map_mouse_button(button: i16) -> Option<winit::event::MouseButton> {
    Some(match button {
        0 => winit::event::MouseButton::Left,
        1 => winit::event::MouseButton::Middle,
        2 => winit::event::MouseButton::Right,
        _ => return None,
    })
}
