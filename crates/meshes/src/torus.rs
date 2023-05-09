use ambient_std::mesh::Mesh;
use glam::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TorusMesh {
    pub inner_radius: f32,
    pub outer_radius: f32,
    pub slices: usize,
    pub loops: usize,
}

impl Default for TorusMesh {
    fn default() -> Self {
        TorusMesh {
            inner_radius: 0.25,
            outer_radius: 1.0,
            slices: 16,
            loops: 16,
        }
    }
}

impl From<TorusMesh> for Mesh {
    fn from(torus: TorusMesh) -> Self {
        let TorusMesh {
            inner_radius,
            outer_radius,
            slices,
            loops,
        } = torus;

        let vertex_count = (slices + 1) * (loops + 1);
        let mut vertices = Vec::with_capacity(vertex_count);
        let mut normals = Vec::with_capacity(vertex_count);
        let mut texcoords = Vec::with_capacity(vertex_count);

        let ring_factor = std::f32::consts::PI * 2.0 / slices as f32;
        let loop_factor = std::f32::consts::PI * 2.0 / loops as f32;

        for i in 0..=loops {
            let u = i as f32 * loop_factor;
            let cos_u = u.cos();
            let sin_u = u.sin();

            for j in 0..=slices {
                let v = j as f32 * ring_factor;
                let cos_v = v.cos();
                let sin_v = v.sin();

                let r = outer_radius + inner_radius * cos_v;
                let x = r * cos_u;
                let y = r * sin_u;
                let z = inner_radius * sin_v;

                vertices.push(Vec3::new(x, y, z));

                let nv = Vec3::new(cos_v * cos_u, cos_v * sin_u, sin_v);
                normals.push(nv.normalize());

                texcoords.push(Vec2::new(v / ring_factor, 1.0 - u / loop_factor));
            }
        }

        let index_count = slices * loops * 6;
        let mut indices = Vec::with_capacity(index_count);

        for i in 0..loops {
            for j in 0..slices {
                let a = i * (slices + 1) + j;
                let b = a + slices + 1;

                indices.push(a as u32);
                indices.push(b as u32);
                indices.push((a + 1) as u32);

                indices.push(b as u32);
                indices.push((b + 1) as u32);
                indices.push((a + 1) as u32);
            }
        }

        let mut mesh = Mesh {
            name: "torus".into(),
            positions: Some(vertices),
            colors: None,
            normals: Some(normals),
            tangents: None,
            texcoords: vec![texcoords],
            joint_indices: None,
            joint_weights: None,
            indices: Some(indices),
        };
        mesh.create_tangents();
        mesh

    }
}