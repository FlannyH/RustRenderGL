use crate::bvh::Bvh;
use crate::graphics::Renderer;
use crate::material::Material;
use crate::structs::{Transform, Triangle};
use crate::{structs::Vertex, texture::Texture};
use glam::Vec4Swizzles;
use glam::{Mat4, Vec2, Vec3, Vec4};
use gltf::buffer::Data;
use std::sync::Arc;
use std::{collections::HashMap, path::Path};

#[derive(Clone)]
pub struct Mesh {
    pub verts: Vec<Vertex>,
    pub vao: u32,
    pub vbo: u32,
    pub bvh: Option<Arc<Bvh>>, // Used exclusively in raytracing
}

pub struct Model {
    pub meshes: Vec<(String, Mesh, Material)>,
}

// So what this function needs to do: &[u8] -(reinterpret)> &[SrcCompType] -(convert)> &[DstCompType]
fn reinterpret_then_convert<SrcCompType, DstCompType>(input_buffer: &[u8]) -> Vec<DstCompType>
where
    DstCompType: From<SrcCompType>,
    SrcCompType: Copy,
{
    // &[u8] -> &[SrcCompType]
    let input_ptr = input_buffer.as_ptr();
    let src_comp_buffer: &[SrcCompType] = unsafe {
        std::slice::from_raw_parts(
            std::mem::transmute(input_ptr),
            input_buffer.len() / std::mem::size_of::<SrcCompType>(),
        )
    };

    // &[SrcCompType] -> Vec<DstCompType>
    let mut dst_comp_vec = Vec::<DstCompType>::new();
    for item in src_comp_buffer {
        dst_comp_vec.push(DstCompType::from(*item));
    }

    // Return
    dst_comp_vec
}

fn convert_gltf_buffer_to_f32(input_buffer: &[u8], accessor: &gltf::Accessor) -> Vec<f32> {
    // Convert based on data type
    // First we make a f64 vector (this way we can do fancy generics magic and still convert u32 to f32)
    let values64 = match accessor.data_type() {
        gltf::accessor::DataType::I8 => reinterpret_then_convert::<i8, f64>(input_buffer),
        gltf::accessor::DataType::U8 => reinterpret_then_convert::<u8, f64>(input_buffer),
        gltf::accessor::DataType::I16 => reinterpret_then_convert::<i16, f64>(input_buffer),
        gltf::accessor::DataType::U16 => reinterpret_then_convert::<u16, f64>(input_buffer),
        gltf::accessor::DataType::U32 => reinterpret_then_convert::<u32, f64>(input_buffer),
        gltf::accessor::DataType::F32 => reinterpret_then_convert::<f32, f64>(input_buffer),
    };

    // Then we convert that to a f32 vector - this feels cursed as heck but let's ignore that, it'll be fine!
    let mut values32 = Vec::<f32>::new();
    values32.resize(values64.len(), 0.0);
    for i in 0..values32.len() {
        values32[i] = values64[i] as f32;
    }

    // Return
    values32
}

fn create_vertex_array(
    primitive: &gltf::Primitive,
    mesh_data: &[Data],
    local_matrix: Mat4,
) -> Mesh {
    let mut position_vec = Vec::<Vec3>::new();
    let mut normal_vec = Vec::<Vec3>::new();
    let mut tangent_vec = Vec::<Vec4>::new();
    let mut colour_vec = Vec::<Vec4>::new();
    let mut texcoord0_vec = Vec::<Vec2>::new();
    let mut texcoord1_vec = Vec::<Vec2>::new();
    let mut indices = Vec::<u16>::new();

    // Loop over all the primitive attributes
    for (name, accessor) in primitive.attributes() {
        // Get buffer view
        let bufferview = accessor.view().unwrap();

        // Find location in buffer
        let buffer_index = bufferview.buffer().index();
        let buffer_offset = bufferview.offset();
        let buffer_end = bufferview.offset() + bufferview.length();

        // Find location in buffer
        let buffer_base = &mesh_data[buffer_index].0;
        let buffer_slice = buffer_base.get(buffer_offset..buffer_end).unwrap();

        // Assign to the vectors
        match name.to_string().as_str() {
            "POSITION" => {
                let values = convert_gltf_buffer_to_f32(buffer_slice, &accessor);
                for i in (0..accessor.count() * 3).step_by(3) {
                    let slice = &values[i..i + 3];
                    position_vec.push(Vec3::from_slice(slice));
                }
            }
            "NORMAL" => {
                let values = convert_gltf_buffer_to_f32(buffer_slice, &accessor);
                for i in (0..accessor.count() * 3).step_by(3) {
                    let slice = &values[i..i + 3];
                    normal_vec.push(Vec3::from_slice(slice));
                }
            }
            "TANGENT" => {
                let values = convert_gltf_buffer_to_f32(buffer_slice, &accessor);
                for i in (0..accessor.count() * 4).step_by(4) {
                    let slice = &values[i..i + 4];
                    tangent_vec.push(Vec4::from_slice(slice));
                }
            }
            "TEXCOORD_0" => {
                let values = convert_gltf_buffer_to_f32(buffer_slice, &accessor);
                for i in (0..accessor.count() * 2).step_by(2) {
                    let slice = &values[i..i + 2];
                    texcoord0_vec.push(Vec2::from_slice(slice));
                }
            }
            "TEXCOORD_1" => {
                let values = convert_gltf_buffer_to_f32(buffer_slice, &accessor);
                for i in (0..accessor.count() * 2).step_by(2) {
                    let slice = &values[i..i + 2];
                    texcoord1_vec.push(Vec2::from_slice(slice));
                }
            }
            "COLOR_0" => {
                let values = convert_gltf_buffer_to_f32(buffer_slice, &accessor);
                for i in (0..accessor.count() * 4).step_by(4) {
                    let slice = &values[i..i + 4];
                    colour_vec.push(Vec4::from_slice(slice));
                }
            }
            _ => {}
        }
    }

    // Find indices
    {
        // Get accessor
        let accessor = primitive.indices().unwrap();

        // Get buffer view
        let bufferview = accessor.view().unwrap();

        // Find location in buffer
        let buffer_index = bufferview.buffer().index();
        let buffer_offset = bufferview.offset();
        let buffer_end = bufferview.offset() + bufferview.length();

        // Find location in buffer
        let buffer_base = &mesh_data[buffer_index].0;
        let buffer_slice = buffer_base.get(buffer_offset..buffer_end).unwrap();

        // Convert from raw buffer to f32 vec - this is incredibly cursed but it'll have to do
        let indices_f32 = convert_gltf_buffer_to_f32(buffer_slice, &accessor);
        for index in indices_f32 {
            indices.push(index as u16);
        }
    }

    // Create vertex array
    let mut mesh_out = Mesh {
        verts: Vec::new(),
        vao: 0,
        vbo: 0,
        bvh: None,
    };
    for index in indices {
        let mut vertex = Vertex {
            position: Vec3::new(0., 0., 0.),
            normal: Vec3::new(0., 0., 0.),
            tangent: Vec4::new(0., 0., 0., 0.),
            colour: Vec4::new(1., 1., 1., 1.),
            uv0: Vec2::new(0., 0.),
            uv1: Vec2::new(0., 0.),
        };
        if !position_vec.is_empty() {
            let pos3 = position_vec[index as usize];
            vertex.position = (local_matrix * pos3.extend(1.0)).xyz();
        }
        if !normal_vec.is_empty() {
            vertex.normal = local_matrix.transform_vector3(normal_vec[index as usize]);
        }
        if !tangent_vec.is_empty() {
            let tangent_vec3 = local_matrix.transform_vector3(tangent_vec[index as usize].xyz());
            vertex.tangent.x = tangent_vec3.x;
            vertex.tangent.y = tangent_vec3.y;
            vertex.tangent.z = tangent_vec3.z;
            vertex.tangent.w = tangent_vec[index as usize].w;
        }
        if !texcoord0_vec.is_empty() {
            vertex.uv0 = texcoord0_vec[index as usize];
        }
        if !texcoord1_vec.is_empty() {
            vertex.uv1 = texcoord1_vec[index as usize];
        }
        if !colour_vec.is_empty() {
            vertex.colour.x = f32::powf(colour_vec[index as usize].x, 1.0 / 2.2);
            if vertex.colour.x > 1.0 {
                vertex.colour.x = 1.0
            }
            vertex.colour.y = f32::powf(colour_vec[index as usize].y, 1.0 / 2.2);
            if vertex.colour.y > 1.0 {
                vertex.colour.y = 1.0
            }
            vertex.colour.z = f32::powf(colour_vec[index as usize].z, 1.0 / 2.2);
            if vertex.colour.z > 1.0 {
                vertex.colour.z = 1.0
            }
        }
        mesh_out.verts.push(vertex);
    }
    mesh_out
}

fn traverse_nodes(
    node: &gltf::Node,
    mesh_data: &Vec<Data>,
    local_transform: Mat4,
    primitives_processed: &mut HashMap<String, Mesh>,
) {
    // Convert translation in GLTF model to a Mat4.
    let node_transform = Transform {
        scale: glam::vec3(
            node.transform().decomposed().2[0],
            node.transform().decomposed().2[1],
            node.transform().decomposed().2[2],
        ),
        rotation: glam::quat(
            node.transform().decomposed().1[0],
            node.transform().decomposed().1[1],
            node.transform().decomposed().1[2],
            node.transform().decomposed().1[3],
        ),
        translation: glam::vec3(
            node.transform().decomposed().0[0],
            node.transform().decomposed().0[1],
            node.transform().decomposed().0[2],
        ),
    };

    let new_local_transform = local_transform * node_transform.local_matrix();

    // If it has a mesh, process it
    let mesh = node.mesh();
    if let Some(mesh) = mesh {
        // Get mesh
        let primitives = mesh.primitives();

        for primitive in primitives {
            let mut mesh_buffer_data =
                create_vertex_array(&primitive, mesh_data, new_local_transform);
            let material = String::from(primitive.material().name().unwrap_or("untitled"));
            #[allow(clippy::map_entry)] // This was really annoying and made the code less readable
            if primitives_processed.contains_key(&material) {
                let mesh: &mut Mesh = primitives_processed.get_mut(&material).unwrap();
                mesh.verts.append(&mut mesh_buffer_data.verts);
            } else {
                primitives_processed.insert(material, mesh_buffer_data);
            }
        }
    }

    // If it has children, process those
    for child in node.children() {
        traverse_nodes(&child, mesh_data, new_local_transform, primitives_processed);
    }
}

impl Model {
    pub(crate) fn load_gltf(path: &Path, renderer: &mut Renderer) -> Result<Model, String> {
        let mut model = Model::new();

        // Load GLTF from file
        let gltf_file = gltf::import(path);
        if gltf_file.is_err() {
            return Err("Failed to load GLTF file!".to_string());
        }
        let (gltf_document, mesh_data, image_data) = gltf_file.unwrap();

        let mut meshes = HashMap::<String, Mesh>::new();
        let mut materials = HashMap::<String, Material>::new();

        // Loop over each scene
        let scene = gltf_document.default_scene();
        if let Some(scene) = scene {
            // For each scene, get the nodes
            for node in scene.nodes() {
                traverse_nodes(&node, &mesh_data, Mat4::IDENTITY, &mut meshes);
            }
        }

        // Get all the textures from the GLTF
        for material in gltf_document.materials() {
            let mut new_material = Material::new();

            // Get PBR parameters
            new_material.scl_rgh = material.pbr_metallic_roughness().roughness_factor();
            new_material.scl_mtl = material.pbr_metallic_roughness().metallic_factor();
            new_material.scl_emm = material.emissive_factor().into();

            // Try to find textures
            let tex_info_alb = material.pbr_metallic_roughness().base_color_texture();
            let _tex_info_mtl_rgh = material
                .pbr_metallic_roughness()
                .metallic_roughness_texture();
            let _tex_info_nrm = material.normal_texture();
            let _tex_info_emm = material.emissive_texture();

            // Get the texture data
            if let Some(tex) = tex_info_alb {
                new_material.tex_alb =
                    renderer.upload_texture(&mut Texture::load_texture_from_gltf_image(
                        &image_data[tex.texture().source().index()],
                    )) as i32;
            }

            materials.insert(
                String::from(material.name().unwrap_or("untitled")),
                new_material,
            );
        }

        // Build BVH
        meshes.iter_mut().for_each(|(_name, mesh)| {
            // First, clone all the triangles to a separate vector
            let mut triangles = Vec::new();
            for triangle_vertices in mesh.verts.chunks(3) {
                triangles.push(Triangle {
                    v0: triangle_vertices[0],
                    v1: triangle_vertices[1],
                    v2: triangle_vertices[2],
                })
            }

            // Now we create a new BVH
            mesh.bvh = Some(Arc::new(Bvh::construct(triangles)));
        });

        // Create final vector
        for name in meshes.keys().into_iter() {
            model.meshes.push((name.clone(), meshes.get(name).unwrap().clone(), materials.get(name).unwrap().clone()));
        }

        Ok(model)
    }

    pub(crate) fn new() -> Model {
        Model {
            meshes: Vec::new(),
        }
    }
}
