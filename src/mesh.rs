use crate::bvh::Bvh;
use crate::graphics::Renderer;
use crate::material::Material;
use crate::structs::{Transform, Triangle};
use crate::texture::Image;
use crate::structs::Vertex;
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
    let mut indices = Vec::<u32>::new();

    let reader = primitive.reader(|buffer| Some(&mesh_data[buffer.index()]));
    
    if let Some(indices_reader) = reader.read_indices() {
        indices_reader.into_u32().for_each(|i| indices.push(i));
    }
    if let Some(positions_reader) = reader.read_positions() {
        positions_reader.for_each(|p| position_vec.push(Vec3::new(p[0], p[1], p[2])));
    }
    if let Some(normals_reader) = reader.read_normals() {
        normals_reader.for_each(|n| normal_vec.push(Vec3::new(n[0], n[1], n[2])));
    }
    if let Some(tangent_reader) = reader.read_tangents() {
        tangent_reader.for_each(|n| tangent_vec.push(Vec4::new(n[0], n[1], n[2], n[3])));
    }
    if let Some(colors_reader) = reader.read_colors(0) {
        colors_reader.into_rgba_f32().for_each(|n| colour_vec.push(Vec4::new(n[0], n[1], n[2], n[3])));
    }
    if let Some(tex_coord_reader) = reader.read_tex_coords(0) {
        tex_coord_reader
            .into_f32()
            .for_each(|tc| texcoord0_vec.push(Vec2::new(tc[0], tc[1])));
    }
    if let Some(tex_coord_reader) = reader.read_tex_coords(1) {
        tex_coord_reader
            .into_f32()
            .for_each(|tc| texcoord1_vec.push(Vec2::new(tc[0], tc[1])));
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

            // Determine material name
            let material = String::from({
                if let Some(material_name) = primitive.material().name() {
                    material_name
                }
                else if let Some(texture) = primitive.material().pbr_metallic_roughness().base_color_texture() {
                    let texture_source = texture.texture().source().source();
                    match texture_source {
                        gltf::image::Source::View { view: _, mime_type: _ } => panic!(),
                        gltf::image::Source::Uri { uri, mime_type: _ } => uri,
                    }
                }
                else {
                    "untitled"
                }
            });

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

            // Determine material name
            let material_name = String::from({
                if let Some(material_name) = material.name() {
                    material_name
                }
                else if let Some(texture) = material.pbr_metallic_roughness().base_color_texture() {
                    let texture_source = texture.texture().source().source();
                    match texture_source {
                        gltf::image::Source::View { view: _, mime_type: _ } => panic!(),
                        gltf::image::Source::Uri { uri, mime_type: _ } => uri,
                    }
                }
                else {
                    "untitled"
                }
            });

            // Get PBR parameters
            new_material.scl_rgh = material.pbr_metallic_roughness().roughness_factor();
            new_material.scl_mtl = material.pbr_metallic_roughness().metallic_factor();
            new_material.scl_emm = material.emissive_factor().into();

            // Try to find textures
            let tex_info_alb = material.pbr_metallic_roughness().base_color_texture();
            let tex_info_mtl_rgh = material
                .pbr_metallic_roughness()
                .metallic_roughness_texture();
            let tex_info_nrm = material.normal_texture();
            let tex_info_emm = material.emissive_texture();

            // Get the texture data
            if let Some(tex) = tex_info_alb {
                // Load image
                let image = Image::load_image_from_gltf(
                    &image_data[tex.texture().source().index()],
                );

                // Allocate in texture atlas
                new_material.tex_alb = renderer.tex_cells.len() as i32;
                let cell = renderer.texture_atlas.allocate_texture(image.width, image.height).unwrap();
                renderer.texture_atlas.upload_image_to_cell(&image, &cell);
                renderer.tex_cells.push(cell);
            }
            if let Some(tex) = tex_info_nrm {
                // Load image
                let image = Image::load_image_from_gltf(
                    &image_data[tex.texture().source().index()],
                );

                // Allocate in texture atlas
                new_material.tex_nrm = renderer.tex_cells.len() as i32;
                let cell = renderer.texture_atlas.allocate_texture(image.width, image.height).unwrap();
                renderer.texture_atlas.upload_image_to_cell(&image, &cell);
                renderer.tex_cells.push(cell);
            }
            if let Some(tex) = tex_info_mtl_rgh {
                // Load image
                let image = Image::load_image_from_gltf(
                    &image_data[tex.texture().source().index()],
                );

                // Allocate in texture atlas
                new_material.tex_mtl_rgh = renderer.tex_cells.len() as i32;
                let cell = renderer.texture_atlas.allocate_texture(image.width, image.height).unwrap();
                renderer.texture_atlas.upload_image_to_cell(&image, &cell);
                renderer.tex_cells.push(cell);
            }
            if let Some(tex) = tex_info_emm {
                // Load image
                let image = Image::load_image_from_gltf(
                    &image_data[tex.texture().source().index()],
                );

                // Allocate in texture atlas
                new_material.tex_emm = renderer.tex_cells.len() as i32;
                let cell = renderer.texture_atlas.allocate_texture(image.width, image.height).unwrap();
                renderer.texture_atlas.upload_image_to_cell(&image, &cell);
                renderer.tex_cells.push(cell);
            }

            materials.insert(
                material_name,
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
