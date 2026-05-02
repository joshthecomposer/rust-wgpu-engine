use glam::Vec3;
use serde::Deserialize;

use crate::animation::model::{Model, Vertex};
use crate::renderer::Renderer;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum Dimension {
    Cuboid { w: f32, h: f32, d: f32 },
    Cylinder { r: f32, h: f32 },
    Pill { r: f32, h: f32 },
}

pub struct Cuboid {
    pub w: f32,
    pub h: f32,
    pub d: f32,
}

impl Cuboid {
    pub fn create_model(&self) -> Model {
        let mut cuboid = Model::new();

        let max_x = self.w / 2.0;
        let min_x = -max_x;

        let min_y = 0.0;
        let max_y = self.h;

        let max_z = self.d / 2.0;
        let min_z = -max_z;

        let px = Vec3::new(1.0, 0.0, 0.0);
        let nx = Vec3::new(-1.0, 0.0, 0.0);
        let py = Vec3::new(0.0, 1.0, 0.0);
        let ny = Vec3::new(0.0, -1.0, 0.0);
        let pz = Vec3::new(0.0, 0.0, 1.0);
        let nz = Vec3::new(0.0, 0.0, -1.0);

        let vertices = vec![
            // Right (+X)
            Vertex::new(Vec3::new(max_x, min_y, min_z), px),
            Vertex::new(Vec3::new(max_x, max_y, min_z), px),
            Vertex::new(Vec3::new(max_x, max_y, max_z), px),
            Vertex::new(Vec3::new(max_x, min_y, max_z), px),
            // Left (-X)
            Vertex::new(Vec3::new(min_x, min_y, max_z), nx),
            Vertex::new(Vec3::new(min_x, max_y, max_z), nx),
            Vertex::new(Vec3::new(min_x, max_y, min_z), nx),
            Vertex::new(Vec3::new(min_x, min_y, min_z), nx),
            // Top (+Y)
            Vertex::new(Vec3::new(min_x, max_y, min_z), py),
            Vertex::new(Vec3::new(min_x, max_y, max_z), py),
            Vertex::new(Vec3::new(max_x, max_y, max_z), py),
            Vertex::new(Vec3::new(max_x, max_y, min_z), py),
            // Bottom (-Y)
            Vertex::new(Vec3::new(min_x, min_y, max_z), ny),
            Vertex::new(Vec3::new(min_x, min_y, min_z), ny),
            Vertex::new(Vec3::new(max_x, min_y, min_z), ny),
            Vertex::new(Vec3::new(max_x, min_y, max_z), ny),
            // Front (+Z)
            Vertex::new(Vec3::new(max_x, min_y, max_z), pz),
            Vertex::new(Vec3::new(max_x, max_y, max_z), pz),
            Vertex::new(Vec3::new(min_x, max_y, max_z), pz),
            Vertex::new(Vec3::new(min_x, min_y, max_z), pz),
            // Back (-Z)
            Vertex::new(Vec3::new(min_x, min_y, min_z), nz),
            Vertex::new(Vec3::new(min_x, max_y, min_z), nz),
            Vertex::new(Vec3::new(max_x, max_y, min_z), nz),
            Vertex::new(Vec3::new(max_x, min_y, min_z), nz),
        ];

        let indices = vec![
            0, 1, 2, 0, 2, 3, // Right
            4, 5, 6, 4, 6, 7, // Left
            8, 9, 10, 8, 10, 11, // Top
            12, 13, 14, 12, 14, 15, // Bottom
            16, 17, 18, 16, 18, 19, // Front
            20, 21, 22, 20, 22, 23, // Back
        ];

        cuboid.vertices = vertices;
        cuboid.indices = indices;
        Renderer::upload_model_mesh(&mut cuboid);

        cuboid
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Cylinder {
    pub r: f32,
    pub h: f32,
}

impl Cylinder {
    pub fn create_model(&self, segments: u32) -> Model {
        let mut model = Model::new();
        let mut vertices = vec![];
        let mut indices = vec![];

        let angle_step = std::f32::consts::TAU / segments as f32;

        // bottom center vertex
        let bottom_center_index = vertices.len() as u32;
        vertices.push(Vertex::new(Vec3::new(0.0, 0.0, 0.0), Vec3::NEG_Y));

        // Bottom Ring
        for i in 0..segments {
            let theta = i as f32 * angle_step;
            let x = self.r * theta.cos();
            let z = self.r * theta.sin();

            vertices.push(Vertex::new(Vec3::new(x, 0.0, z), Vec3::NEG_Y));
        }

        // Bottom cap indices (triangle fan)
        for i in 0..segments {
            let next = (i + 1) % segments;
            indices.push(bottom_center_index);
            indices.push(bottom_center_index + 1 + next);
            indices.push(bottom_center_index + 2 + i);
        }

        // top center vertex
        let top_center_vertex = vertices.len() as u32;
        vertices.push(Vertex::new(Vec3::new(0.0, self.h, 0.0), Vec3::Y));

        // top ring
        let top_ring_start = vertices.len() as u32;
        for i in 0..segments {
            let theta = i as f32 * angle_step;
            let x = self.r * theta.cos();
            let z = self.r * theta.sin();

            vertices.push(Vertex::new(Vec3::new(x, self.h, z), Vec3::Y));
        }

        // top cap indices ( triangle fan )
        for i in 0..segments {
            let next = (i + 1) % segments;
            indices.push(top_center_vertex);
            indices.push(top_ring_start + i);
            indices.push(top_ring_start + next);
        }

        // build side wall
        let side_start_index = vertices.len() as u32;
        for i in 0..segments {
            let theta = i as f32 * angle_step;
            let x = self.r * theta.cos();
            let z = self.r * theta.sin();
            let normal = Vec3::new(x, 0.0, z).normalize();

            // bottom vertex of the side
            vertices.push(Vertex::new(Vec3::new(x, 0.0, z), normal));
            // top vertex of the side
            vertices.push(Vertex::new(Vec3::new(x, self.h, z), normal));
        }

        // side indices (split the quads into two triangles)
        for i in 0..segments {
            let next = (i + 1) % segments;
            let base = side_start_index;

            let bottom_i = base + i * 2;
            let top_i = base + i * 2 + 1;
            let bottom_next = base + next * 2;
            let top_next = base + next * 2 + 1;

            // triangle 1
            indices.push(bottom_i);
            indices.push(bottom_next);
            indices.push(top_i);

            // triangle 2
            indices.push(top_i);
            indices.push(bottom_next);
            indices.push(top_next);
        }

        model.vertices = vertices;
        model.indices = indices;
        Renderer::upload_model_mesh(&mut model);

        model
    }
}

pub struct Sphere {
    pub r: f32,
}

impl Sphere {
    pub fn create_model(&self, segments: u32, rings: u32, offset: f32) -> Model {
        let mut model = Model::new();
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();

        let segs = segments.max(3); // avoid degenerate
        let rings = rings.max(2);

        let d_theta = std::f32::consts::TAU / segs as f32;
        let d_phi = std::f32::consts::PI / rings as f32;

        // === Vertices ===
        for ring in 0..=rings {
            let phi = ring as f32 * d_phi; // 0..PI
            let sp = phi.sin();
            let cp = phi.cos();

            for seg in 0..=segs {
                let theta = seg as f32 * d_theta; // 0..TAU
                let ct = theta.cos();
                let st = theta.sin();

                let x = self.r * sp * ct;
                let y = self.r * cp;
                let z = self.r * sp * st;

                let pos = Vec3::new(x, y + offset, z);

                // Normal based on the unoffset sphere center.
                // Equivalent to (x,y,z).normalize() because it's on radius r.
                let normal = Vec3::new(x, y, z).normalize();

                vertices.push(Vertex::new(pos, normal));
            }
        }

        let row_stride = segs + 1;
        for ring in 0..rings {
            for seg in 0..segs {
                let i0 = ring * row_stride + seg;
                let i1 = i0 + 1;
                let i2 = (ring + 1) * row_stride + seg;
                let i3 = i2 + 1;

                indices.extend_from_slice(&[i0, i1, i2, i1, i3, i2]);
            }
        }

        model.vertices = vertices;
        model.indices = indices;
        Renderer::upload_model_mesh(&mut model);
        model
    }
}

pub struct Pill {
    pub r: f32,
    // Total height including hemispheres
    pub h: f32,
}

impl Pill {
    pub fn create_model(&self, segments: u32, rings: u32, offset: f32) -> Model {
        let mut model = Model::new();
        let mut vertices = vec![];
        let mut indices = vec![];

        let cylinder_bottom = self.r;
        let cylinder_top = self.h - self.r;

        // === Cylinder Section ===
        let angle_step = std::f32::consts::TAU / segments as f32;
        for i in 0..segments {
            let theta = i as f32 * angle_step;
            let next_theta = ((i + 1) % segments) as f32 * angle_step;

            let x0 = self.r * theta.cos();
            let z0 = self.r * theta.sin();
            let x1 = self.r * next_theta.cos();
            let z1 = self.r * next_theta.sin();

            let normal0 = Vec3::new(x0, 0.0, z0).normalize();
            let normal1 = Vec3::new(x1, 0.0, z1).normalize();

            let base = vertices.len() as u32;

            vertices.push(Vertex::new(
                Vec3::new(x0, cylinder_bottom + offset, z0),
                normal0,
            ));
            vertices.push(Vertex::new(
                Vec3::new(x0, cylinder_top + offset, z0),
                normal0,
            ));
            vertices.push(Vertex::new(
                Vec3::new(x1, cylinder_top + offset, z1),
                normal1,
            ));
            vertices.push(Vertex::new(
                Vec3::new(x1, cylinder_bottom + offset, z1),
                normal1,
            ));

            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }

        // === Hemispheres (top and bottom) ===
        let ring_step = std::f32::consts::FRAC_PI_2 / rings as f32;

        for &hemisphere_sign in &[-1.0, 1.0] {
            let y_offset = if hemisphere_sign < 0.0 {
                cylinder_bottom
            } else {
                cylinder_top
            };

            let ring_base = vertices.len() as u32;

            for ring in 0..=rings {
                let phi = ring as f32 * ring_step;
                let y = phi.sin() * self.r;
                let r = phi.cos() * self.r;

                for seg in 0..=segments {
                    let theta = seg as f32 * angle_step;
                    let x = r * theta.cos();
                    let z = r * theta.sin();
                    let normal = Vec3::new(x, hemisphere_sign * y, z).normalize();

                    vertices.push(Vertex::new(
                        Vec3::new(x, y_offset + hemisphere_sign * y + offset, z),
                        normal,
                    ));
                }
            }

            for ring in 0..rings {
                for seg in 0..segments {
                    let i0 = ring * (segments + 1) + seg;
                    let i1 = i0 + 1;
                    let i2 = i0 + segments + 1;
                    let i3 = i2 + 1;

                    let base = ring_base;

                    if hemisphere_sign < 0.0 {
                        // bottom hemisphere
                        indices.extend_from_slice(&[
                            base + i0,
                            base + i2,
                            base + i1,
                            base + i1,
                            base + i2,
                            base + i3,
                        ]);
                    } else {
                        // top hemisphere
                        indices.extend_from_slice(&[
                            base + i0,
                            base + i1,
                            base + i2,
                            base + i1,
                            base + i3,
                            base + i2,
                        ]);
                    }
                }
            }
        }

        model.vertices = vertices;
        model.indices = indices;
        Renderer::upload_model_mesh(&mut model);
        model
    }
}
