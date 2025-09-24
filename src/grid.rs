#![allow(dead_code)]
use std::fs::read_to_string;

use glam::{vec2, Vec3, Vec4};
use glam::vec3;
use image::{ImageBuffer, Rgba};

use crate::animation::animation::{texture_from_file, Model, Vertex};
use crate::enums_types::{TextureProfile, TextureType};
use crate::some_data::MAX_BONE_INFLUENCE;
use crate::{enums_types::CellType, shaders::Shader};

#[derive(Debug)]
pub struct GridCell {
    pub id: usize,
    pub position:  Vec3,
    pub width: f32,
    // used to determine if this is traversable in A*
    pub blocked: bool,
    // used to precalculate the neighbors for A*
    pub adjacent_cells: Vec<usize>,
    pub cell_type: CellType,
}

#[derive(Debug)]
pub struct Grid {
    // pub cells: HashMap<usize, GridCell>,
    pub cells: Vec<GridCell>,
    pub next_cell_id: usize,
    pub model: Model,
    pub cell_size: f32,
    pub width: usize,
    pub height: usize,
}

impl Grid {
    pub fn new(width: usize, height: usize, cell_size: f32) -> Grid {
        Grid {
            cells: Vec::with_capacity(width * height) ,
            next_cell_id: 0,
            model: Model::new(),
            cell_size,
            width,
            height,
        }
    }

    pub fn parse_grid_data(file_path: &str, cell_size: f32) -> Grid {
        let file = read_to_string(file_path).unwrap().replace(' ', "");

        let grid_width = file.lines().last().unwrap().len();
        let grid_height = file.lines().count();

        let mut grid = Grid::new(grid_width, grid_height, cell_size);
        grid.generate();

        for (y, l) in file.lines().enumerate() {
            for (x, c) in l.char_indices() {
                let index = y * grid_width + x;
                let cell = grid.cells.get_mut(index).expect("grid cell didn't exist");
                match c {
                    '0'=> { cell.cell_type = CellType::Grass },
                    'S'=> { cell.cell_type = CellType::Path },
                    'T'=> { cell.cell_type = CellType::Tree },
                    'X'=> { cell.cell_type = CellType::Path },
                    'E'=> { cell.cell_type = CellType::Path },
                    _  => { cell.cell_type = CellType::Grass },
                }
            }
        }

        grid
    }

    pub fn generate(&mut self) {
        Self::generate_texture();
        Self::generate_height_map();
        self.generate_model();
    }

    fn generate_model(&mut self) {
        // TODO: This only works with even numbered grid sizes, fix
        let mut model = self.generate_grid_mesh();
        model.directory = "resources/textures".to_string();

        texture_from_file(&mut model, "half_dark_half_light.png".to_string(), TextureType::Diffuse, TextureProfile::BroadDefault);
        self.model = model;
    }

    fn generate_grid_mesh(&mut self) -> Model {
        let mut vertices = Vec::<Vertex>::new();
        let mut indices = Vec::<u32>::new();
        let mut model = Model::new();
        let mut dark = false;

        let total_width = self.width as f32 * self.cell_size;
        let total_height = self.height as f32 * self.cell_size;

        for row in 0..self.width {
            for col in 0..self.height {
                let x = (col as f32 * self.cell_size) - (total_width / 2.0);
                let z = (row as f32 * self.cell_size) - (total_height / 2.0);


                let (bl, br, tr, tl) = if dark {
                    (
                        vec2(0.0, 0.0),
                        vec2(0.5, 0.0),
                        vec2(0.5, 1.0),
                        vec2(0.0, 1.0)
                    )
                } else {
                    ( 
                        vec2(0.5, 0.0),
                        vec2(1.0, 0.0),
                        vec2(1.0, 1.0),
                        vec2(0.5, 1.0),
                    )
                };

                dark = !dark;

                let base_index = vertices.len() as u32;


                // Add vertices for the cell
                vertices.push(Vertex { position: vec3(x, 0.0, z), normal: vec3(0.0, 1.0, 0.0), uv: bl, bone_ids: [-1; MAX_BONE_INFLUENCE], bone_weights: [0.0; MAX_BONE_INFLUENCE], base_color: Vec4::splat(1.0) });
                vertices.push(Vertex { position: vec3(x + self.cell_size, 0.0, z), normal: vec3(0.0, 1.0, 0.0), uv: br,bone_ids: [-1; MAX_BONE_INFLUENCE], bone_weights: [0.0; MAX_BONE_INFLUENCE], base_color: Vec4::splat(1.0) });
                vertices.push(Vertex { position: vec3(x + self.cell_size, 0.0, z + self.cell_size), normal: vec3(0.0, 1.0, 0.0), uv: tr, bone_ids: [-1; MAX_BONE_INFLUENCE], bone_weights: [0.0; MAX_BONE_INFLUENCE], base_color: Vec4::splat(1.0) });
                vertices.push(Vertex { position: vec3(x, 0.0, z + self.cell_size), normal: vec3(0.0, 1.0, 0.0), uv: tl, bone_ids: [-1; MAX_BONE_INFLUENCE], bone_weights: [0.0; MAX_BONE_INFLUENCE], base_color: Vec4::splat(1.0) });

                // Add indices for two triangles
                // flipped winding
                indices.push(base_index);
                indices.push(base_index + 2);
                indices.push(base_index + 1);
                indices.push(base_index);
                indices.push(base_index + 3);
                indices.push(base_index + 2);

                // indices.push(base_index);
                // indices.push(base_index + 1);
                // indices.push(base_index + 2);
                // indices.push(base_index);
                // indices.push(base_index + 2);
                // indices.push(base_index + 3);

                // =============================================================
                // Create cell object
                // =============================================================
                let cell = GridCell {
                    id: self.next_cell_id,
                    position: vec3(x + (self.cell_size / 2.0), 0.0, z + (self.cell_size / 2.0)),
                    width: self.cell_size, 
                    blocked: false,
                    adjacent_cells: vec![],
                    cell_type: CellType::Grass,
                };

                self.cells.push(cell);
                self.next_cell_id += 1;
            }
            
            if self.width % 2 == 0 {
                dark = !dark;
            } 
        }

        model.vertices.append(&mut vertices);
        model.indices.append(&mut indices);
        model.setup_opengl();

        model
    }

    pub fn generate_texture() {
        let width: u32 = 10;
        let height: u32 = 10;
        let color_dark: u8 = 87;
        let color_light: u8 = 91;

        let mut imgbuf = ImageBuffer::new(width, height);

        for (x, _y, pixel) in imgbuf.enumerate_pixels_mut() {
            if x < width / 2 {
                *pixel = Rgba([color_dark, color_dark, color_dark, 90]);
            } else {
                *pixel = Rgba([color_light, color_light, color_light, 90]);
            }
        }

        imgbuf
            .save("resources/textures/half_dark_half_light.png")
            .expect("Failed to save half dark / half light texture");
    }

    pub fn get_cell_from_position(&self, input: Vec3) -> Option<&GridCell> {
        let cell_x = (input.x / self.cell_size).floor() as usize;
        let cell_z = (input.z / self.cell_size).floor() as usize;

        // TODO: If you are out of bounds do something else, like a -1 or something to signify.
        let cell_id = cell_z * self.width + cell_x + 1;
        
        self.cells.get(cell_id)
    }

    pub fn draw(&mut self, shader: &mut Shader) {
        self.model.draw(shader);
    }

    pub fn generate_height_map() {
        let width: u32 = 10;
        let height: u32 = 10;
        let flat_value: u8 = 128;

        let mut imgbuf = ImageBuffer::new(width, height);

        for (_x, _y, pixel) in imgbuf.enumerate_pixels_mut() {
            *pixel = Rgba([flat_value, flat_value, flat_value, 255]);
        }

        imgbuf
            .save("resources/textures/grid_height.png")
            .expect("Failed to save grid height");
    }

}
