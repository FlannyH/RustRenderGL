use crate::helpers::*;
use std::path::Path;

pub struct Texture {
    pub gl_id: u32,
    pub width: usize,
    pub height: usize,
    pub depth: usize,
    pub data: Vec<u32>,
}

#[derive(PartialEq)]
pub enum FilterMode {
    Point,
    Linear,
}

pub enum WrapMode {
    Repeat,
    Mirror,
    Clamp,
}

pub struct Sampler {
    pub filter_mode_mag: FilterMode,
    pub filter_mode_min: FilterMode,
    pub filter_mode_mipmap: FilterMode,
    pub wrap_mode_s: WrapMode,
    pub wrap_mode_t: WrapMode,
    pub mipmap_enabled: bool,
}

#[derive(Clone)]
enum PixelComp {
    Skip,
    Red,
    Green,
    Blue,
    Alpha,
}

impl Texture {
    pub fn load(path: &Path) -> Self {
        //Load image
        let loaded_image = stb_image::image::load(path);

        //Map the image data to argb8 format
        if let stb_image::image::LoadResult::ImageU8(image) = loaded_image {
            if image.depth == 4 {
                let data = (0..image.data.len() / 4)
                    .map(|id| {
                        colour_rgba(
                            image.data[id * 4 + 3],
                            image.data[id * 4],
                            image.data[id * 4 + 1],
                            image.data[id * 4 + 2],
                        )
                    })
                    .collect();
                Self {
                    gl_id: 0,
                    width: image.width,
                    height: image.height,
                    depth: image.depth,
                    data,
                }
            } else if image.depth == 3 {
                let data = (0..image.data.len() / 3)
                    .map(|id| {
                        colour_rgba(
                            255,
                            image.data[id * 3],
                            image.data[id * 3 + 1],
                            image.data[id * 3 + 2],
                        )
                    })
                    .collect();
                Self {
                    gl_id: 0,
                    width: image.width,
                    height: image.height,
                    depth: image.depth,
                    data,
                }
            } else {
                panic!("Unsupported texture type");
            }
        } else {
            panic!("Unsupported texture type");
        }
    }

    pub fn load_texture_from_gltf_image(image: &gltf::image::Data) -> Texture {
        // Get pixel swizzle pattern
        let swizzle_pattern = match image.format {
            gltf::image::Format::R8 => vec![PixelComp::Red],
            gltf::image::Format::R8G8 => vec![PixelComp::Red, PixelComp::Green],
            gltf::image::Format::R8G8B8 => vec![PixelComp::Red, PixelComp::Green, PixelComp::Blue],
            gltf::image::Format::R8G8B8A8 => vec![
                PixelComp::Red,
                PixelComp::Green,
                PixelComp::Blue,
                PixelComp::Alpha,
            ],
            gltf::image::Format::R16 => vec![PixelComp::Skip, PixelComp::Red],
            gltf::image::Format::R16G16 => vec![
                PixelComp::Skip,
                PixelComp::Red,
                PixelComp::Skip,
                PixelComp::Green,
            ],
            gltf::image::Format::R16G16B16 => vec![
                PixelComp::Skip,
                PixelComp::Red,
                PixelComp::Skip,
                PixelComp::Green,
                PixelComp::Skip,
                PixelComp::Blue,
            ],
            gltf::image::Format::R16G16B16A16 => vec![
                PixelComp::Skip,
                PixelComp::Red,
                PixelComp::Skip,
                PixelComp::Green,
                PixelComp::Skip,
                PixelComp::Blue,
                PixelComp::Skip,
                PixelComp::Alpha,
            ],
            _ => panic!("Texture format unsupported!"),
        };
        Texture {
            gl_id: 0,
            width: image.width as usize,
            height: image.height as usize,
            depth: 4,
            data: {
                let mut data = Vec::<u32>::new();
                for i in (0..image.pixels.len()).step_by(swizzle_pattern.len()) {
                    let mut new_pixel = 0xFFFFFFFFu32;
                    for (comp, entry) in swizzle_pattern.iter().enumerate() {
                        match entry {
                            PixelComp::Skip => {}
                            PixelComp::Red => {
                                new_pixel = new_pixel & 0xFFFFFF00 | image.pixels[i + comp] as u32
                            }
                            PixelComp::Green => {
                                new_pixel =
                                    new_pixel & 0xFFFF00FF | (image.pixels[i + comp] as u32) << 8
                            }
                            PixelComp::Blue => {
                                new_pixel =
                                    new_pixel & 0xFF00FFFF | (image.pixels[i + comp] as u32) << 16
                            }
                            PixelComp::Alpha => {
                                new_pixel =
                                    new_pixel & 0x00FFFFFF | (image.pixels[i + comp] as u32) << 24
                            }
                        }
                    }
                    data.push(new_pixel);
                }
                data
            },
        }
    }
}
