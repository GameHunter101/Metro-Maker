use core::f32;

use image::{EncodableLayout, ImageBuffer};
use nalgebra::Vector2;
use street_plan::{smooth_path, trace_street_plan};
use tensor_field::{DesignElement, EvalEigenvectors, GRID_SIZE, TensorField};
use v4::{
    builtin_components::mesh_component::{MeshComponent, VertexDescriptor},
    engine_support::texture_support::Texture,
    scene,
};
use wgpu::vertex_attr_array;

mod aabb;
mod street_plan;
mod tensor_field;

#[tokio::main]
async fn main() {
    let grid_element = DesignElement::Grid {
        center: Vector2::new(100.0, 100.0),
        theta: -std::f32::consts::FRAC_PI_3 * 2.0,
        // theta: 0.0,
        length: 500.0,
    };

    let grid_element_2 = DesignElement::Grid {
        center: Vector2::new(300.0, 400.0),
        theta: 0.1,
        length: 200.0,
    };

    let radial_element = DesignElement::Radial {
        center: Vector2::new(200.0, 200.0),
    };

    let grid_element_3 = DesignElement::Grid {
        center: Vector2::new(0.0, 400.0),
        theta: 0.7,
        length: 10.0,
    };

    let city_center = *radial_element.center().as_ref().unwrap();

    let tensor_field = TensorField::new(
        vec![grid_element, radial_element, grid_element_2, grid_element_3],
        0.0004,
    );

    let trace = trace_street_plan(&tensor_field, 30, city_center, 16, 16);

    /* let sample_points = [
        Vector2::<f32>::new(0.0, 0.4),
        Vector2::<f32>::new(1.06, 1.84),
        Vector2::<f32>::new(2.0, 1.2),
        Vector2::<f32>::new(3.91, 2.48),
        Vector2::<f32>::new(4.85, 1.06),
        Vector2::<f32>::new(6.22, 2.34),
    ].map(|x| x * 30.0).to_vec();

    let trace = vec![sample_points.clone(), smooth_path(sample_points, 2)]; */

    let mut engine = v4::V4::builder()
        .features(wgpu::Features::POLYGON_MODE_LINE)
        .window_settings(
            GRID_SIZE as u32 * 2,
            GRID_SIZE as u32 * 2,
            "Visualizer",
            None,
        )
        .build()
        .await;

    let sample_factor = 14;

    let mut norm_tex = ImageBuffer::new(GRID_SIZE, GRID_SIZE);

    for (x, y, pix) in norm_tex.enumerate_pixels_mut() {
        let val = (tensor_field
            .evaluate_field_at_point(Vector2::new(x as f32, y as f32))
            .norm()
            > 0.01) as u8
            * 255;
        *pix = image::Rgba([val, val, val, 100]);
    }

    let rendering_manager = engine.rendering_manager();
    let device = rendering_manager.device();
    let queue = rendering_manager.queue();

    let vector_opacity = 0.4;

    scene! {
        scene: visualizer,
        "eigenvectors" = {
            material: {
                pipeline: {
                    vertex_shader_path: "./shaders/visualizer_vertex.wgsl",
                    fragment_shader_path: "./shaders/visualizer_fragment.wgsl",
                    vertex_layouts: [Vertex::vertex_layout()],
                    uses_camera: false,
                    geometry_details: {
                        topology: wgpu::PrimitiveTopology::LineList,
                        polygon_mode: wgpu::PolygonMode::Line,
                    },
                },
            },
            components: [
                MeshComponent(
                    vertices: vec![
                        (0..GRID_SIZE / sample_factor).flat_map(|x| (0..GRID_SIZE / sample_factor).flat_map(|y| {
                            let point = Vector2::new(x as f32 * sample_factor as f32, y as f32 * sample_factor as f32);
                            let tensor = tensor_field.evaluate_field_at_point(point);
                            let eigenvectors = tensor.eigenvectors();
                            let maj = eigenvectors.major.normalize() * (sample_factor - 1) as f32;
                            let min = eigenvectors.minor.normalize() * (sample_factor - 1) as f32;
                            let maj_point = normalize_vector(point + maj);
                            let min_point = normalize_vector(point + min);
                            let norm_point = normalize_vector(point);
                            [
                                Vertex {pos: [norm_point.x, norm_point.y, 0.0], col: [1.0, 0.0, 0.0, vector_opacity]}, Vertex {pos: [maj_point.x, maj_point.y, 0.0], col: [1.0, 0.0, 0.0, vector_opacity]},
                                Vertex {pos: [norm_point.x, norm_point.y, 0.0], col: [0.0, 1.0, 0.0, vector_opacity]}, Vertex {pos: [min_point.x, min_point.y, 0.0], col: [0.0, 1.0, 0.0, vector_opacity]}
                            ]
                        }).collect::<Vec<_>>()).collect()
                    ],
                    enabled_models: vec![(0, None)]
                ),
            ]
        },
        "degenerate_points" = {
            material: {
                pipeline: {
                    vertex_shader_path: "./shaders/degenerate_point_vert.wgsl",
                    fragment_shader_path: "./shaders/degenerate_point_frag.wgsl",
                    vertex_layouts: [TexVertex::vertex_layout()],
                    uses_camera: false,
                },
                attachments: [
                    Texture(
                    texture: v4::ecs::material::GeneralTexture::Regular(
                        Texture::from_bytes(
                            norm_tex.as_bytes(),
                            (GRID_SIZE, GRID_SIZE),
                            device,
                            queue,
                            wgpu::TextureFormat::Rgba8Unorm,
                            false,
                            true,
                        )
                    ),
                    visibility: wgpu::ShaderStages::FRAGMENT,
                )],
            },
            components: [
                MeshComponent(
                    vertices: vec![vec![
                        TexVertex {
                            pos: [-1.0, 1.0, 0.1],
                            tex_coords: [0.0, 1.0]
                        },
                        TexVertex {
                            pos: [-1.0, -1.0, 0.1],
                            tex_coords: [0.0, 0.0]
                        },
                        TexVertex {
                            pos: [1.0, -1.0, 0.1],
                            tex_coords: [1.0, 0.0]
                        },
                        TexVertex {
                            pos: [1.0, 1.0, 0.1],
                            tex_coords: [1.0, 1.0]
                        },
                    ]],
                    indices: vec![vec![0,1,2,0,2,3]],
                    enabled_models: vec![(0, None)]
                )
            ]
        },
        "path" = {
            material: {
                pipeline: {
                    vertex_shader_path: "./shaders/visualizer_vertex.wgsl",
                    fragment_shader_path: "./shaders/visualizer_fragment.wgsl",
                    vertex_layouts: [Vertex::vertex_layout()],
                    uses_camera: false,
                    geometry_details: {
                        topology: wgpu::PrimitiveTopology::LineStrip,
                        polygon_mode: wgpu::PolygonMode::Line,
                    }
                }
            },
            components: [
                MeshComponent(
                    vertices:
                        trace.iter().map(|arr| {
                            arr.iter().map(|vec| {
                                    Vertex {
                                        pos: [
                                            2.0 * vec.x / GRID_SIZE as f32 - 1.0,
                                            2.0 * vec.y / GRID_SIZE as f32 - 1.0,
                                            0.0,
                                        ],
                                        col: [0.0, 0.0, 1.0, 1.0]
                                    }
                            }).collect::<Vec<_>>()
                        }).collect(),
                    enabled_models: trace.iter().enumerate().map(|(i, _)| (i, None)).collect()
                )
            ]
        }
    }

    engine.attach_scene(visualizer);

    engine.main_loop().await;
}

fn normalize_vector(vec: Vector2<f32>) -> Vector2<f32> {
    Vector2::new(
        2.0 * vec.x / GRID_SIZE as f32 - 1.0,
        2.0 * vec.y / GRID_SIZE as f32 - 1.0,
    )
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    pos: [f32; 3],
    col: [f32; 4],
}

impl VertexDescriptor for Vertex {
    const ATTRIBUTES: &[wgpu::VertexAttribute] =
        &vertex_attr_array![0 => Float32x3, 1 => Float32x4];

    fn from_pos_normal_coords(pos: Vec<f32>, _normal: Vec<f32>, _tex_coords: Vec<f32>) -> Self {
        Self {
            pos: pos.try_into().unwrap(),
            col: [1.0; 4],
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct TexVertex {
    pos: [f32; 3],
    tex_coords: [f32; 2],
}

impl VertexDescriptor for TexVertex {
    const ATTRIBUTES: &[wgpu::VertexAttribute] =
        &vertex_attr_array![0 => Float32x3, 1 => Float32x2];

    fn from_pos_normal_coords(pos: Vec<f32>, _normal: Vec<f32>, tex_coords: Vec<f32>) -> Self {
        Self {
            pos: pos.try_into().unwrap(),
            tex_coords: tex_coords.try_into().unwrap(),
        }
    }
}
