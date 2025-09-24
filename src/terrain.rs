use gl::ActiveTexture;
use glam::{vec2, vec3, vec4, Vec3, Vec4};
use image::GenericImageView;
use imgui::sys::igSetWindowPosVec2;
use nalgebra::Point3;
use rapier3d::prelude::{Collider, ColliderBuilder};

use crate::{animation::animation::{texture_from_file, Model, Vertex}, enums_types::{TextureProfile, TextureType}, some_data::MAX_BONE_INFLUENCE};

pub struct Terrain {
    vertices: Vec<[f32; 3]>,
    indices: Vec<u32>,
    normals: Vec<[f32; 3]>,

    height_map: Vec<f32>,
    width: u32,
    height: u32,
    max_height: f32,
}

impl Terrain {
    pub fn from_height_map(path: &str) -> Self {
        let img = image::open(path).expect("Failed to load terrain image");
        let (width, height) = img.dimensions();

        let grayscale = img.to_luma8();

        let height_map: Vec<f32> = grayscale
            .pixels()
            // TODO: replace 10.0 with some max height value
            .map(|p| p[0] as f32 / 255.0 * 50.0)
            .collect();

        // VERTICES
        let mut vertices = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let i = (y * width + x) as usize;
                let z = height_map[i] - 10.0 / 2.0;
                let x_pos = x as f32 - width as f32 / 2.0;
                let z_pos = y as f32 - height as f32 / 2.0;
                vertices.push([x_pos, z, z_pos])
            }
        }
        
        // INDICES
        let mut indices = Vec::new();
        for y in 0..height - 1 {
            for x in 0..width - 1 {
                let top_left = y * width + x;
                let top_right = top_left + 1;
                let bottom_left = top_left + width;
                let bottom_right = bottom_left + 1;

                // first tri
                indices.push(top_left);
                indices.push(bottom_left);
                indices.push(top_right);

                // second tri
                indices.push(top_right);
                indices.push(bottom_left);
                indices.push(bottom_right);
            }
        }

        // NORMALS
        let mut normals = vec![[0.0; 3]; (width * height) as usize];

        for z in 0..height {
            for x in 0..width {
                let idx = (z * width + x) as usize;
                
                // surrounding heights
                let h_l = if x > 0 { height_map[(z * width + (x - 1)) as usize] } else { height_map[idx] };
                let h_r = if x < width - 1 { height_map[(z * width + (x + 1)) as usize] } else { height_map[idx] };
                let h_d = if z > 0 { height_map[((z - 1) * width + x) as usize] } else { height_map[idx] };
                let h_u = if z < height - 1 { height_map[((z + 1) * width + x) as usize] } else { height_map[idx] };
                
                // central difference
                let dx = h_l - h_r;
                let dz = h_d - h_u;

                let normal = Vec3::new(dx, 2.0, dz).normalize();

                normals[idx] = [normal.x, normal.y, normal.z];
            }
        }

        Self {
            vertices,
            indices,
            normals,

            height_map,

            width,
            height,
            max_height: 10.0,
        }
    }

    pub fn into_opengl_model(&mut self) -> Model {
        let mut model = Model::new();

        for (i, v) in self.vertices.iter().enumerate() {
            let n = self.normals[i];

            let tile_scale = 25.0;

            let uv = vec2(
                ((v[0] + self.width as f32 / 2.0) / self.width as f32) * tile_scale,
                ((v[2] + self.height as f32 / 2.0) / self.height as f32) * tile_scale,
            );

            model.vertices.push(Vertex {
                position: vec3(v[0], v[1], v[2]),
                normal: vec3(n[0], n[1], n[2]),
                uv,
                base_color: Vec4::splat(1.0),

                bone_ids: [-1; MAX_BONE_INFLUENCE],
                bone_weights: [0.0; MAX_BONE_INFLUENCE],
            });
        };

        model.directory = "resources/models/static/terrain".to_string();
        texture_from_file(&mut model, "ai_slop/dirt4.png".to_string(), TextureType::Diffuse, TextureProfile::BroadDefault);

        model.indices = self.indices.clone();
        model.setup_opengl();

        model
    }

    pub fn get_height_at(&self, x: f32, z: f32) -> f32 {
        let terrain_x = x + self.width as f32 / 2.0;
        let terrain_z = z + self.height as f32 / 2.0;

        let x0 = terrain_x.floor() as i32;
        let z0 = terrain_z.floor() as i32;
        let x1 = x0 + 1;
        let z1 = z0 + 1;

        if x0 < 0 || z0 < 0 || x1 >= self.width as i32 || z1 >= self.height as i32 {
            return 0.0; // out of bounds so return 0.       
        }

        let h00 = self.height_map[(z0 as u32 * self.width + x0 as u32) as usize];
        let h10 = self.height_map[(z0 as u32 * self.width + x1 as u32) as usize];
        let h01 = self.height_map[(z1 as u32 * self.width + x0 as u32) as usize];
        let h11 = self.height_map[(z1 as u32 * self.width + x1 as u32) as usize];

        let tx = terrain_x - x0 as f32;
        let tz = terrain_z - z0 as f32;

        let h0 = h00 * (1.0 - tx) + h10 * tx;
        let h1 = h01 * (1.0 - tx) + h11 * tx;
        let height = h0 * (1.0 - tz) + h1 * tz;

        height - (self.max_height / 2.0)

    }

    pub fn create_collider(&self) -> Collider {
        // Convert terrain vertex positions to Point3
        let vertices: Vec<Point3<f32>> = self.vertices
            .iter()
            .map(|v| Point3::new(v[0], v[1], v[2]))
            .collect();

        // Convert triangle indices (assumed u32 or usize)
        let indices: Vec<[u32; 3]> = self.indices
            .chunks(3)
            .map(|tri| [tri[0] as u32, tri[1] as u32, tri[2] as u32])
            .collect();

        ColliderBuilder::trimesh(vertices, indices)
            .expect("Some shit went wrong")
            .build()
    }
}
