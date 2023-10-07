use crate::helpers::*;
use std::{path::Path, ffi::c_void, ptr::null};

#[derive(Debug)]
pub struct Image {
    pub width: usize,
    pub height: usize,
    pub depth: usize,
    pub data: Vec<u32>,
}

pub struct Texture {
    pub gl_id: u32,
    pub image: Image,
}


pub struct TextureAtlas {
    pub grid: Vec<u8>,
    pub cell_width: usize,
    pub cell_height: usize,
    pub texture: Texture,
}

#[derive(Debug)]
pub struct TextureAtlasCell {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
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

impl TextureAtlas {
    pub fn new(atlas_width: usize, atlas_height: usize, cell_width: usize, cell_height: usize) -> Self {
        // Sanity check
        assert!(atlas_width > cell_width);
        assert!(atlas_height > cell_height);

        // Create atlas image on CPU
        let image = Image {
            width: atlas_width,
            height: atlas_height,
            depth: 4,
            data: vec![(atlas_width * atlas_height * 4) as u32],
        };

        // Create atlas texture on GPU
        let mut texture = Texture {
            gl_id: 0,
            image,
        };
        unsafe {
            gl::GenTextures(1, &mut texture.gl_id as *mut u32);
            gl::BindTexture(gl::TEXTURE_2D, texture.gl_id);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA8 as _,
                atlas_width as _,
                atlas_height as _,
                0,
                gl::RGBA,
                gl::FLOAT,
                null(),
            );
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as _);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as _);
            gl::BindTexture(gl::TEXTURE_2D, 0);
        };

        // Create atlas grid for allocation
        let grid_w = atlas_width / cell_width;
        let grid_h = atlas_height / cell_height;
        TextureAtlas {
            grid: vec![0; grid_w * grid_h],
            texture,
            cell_width,
            cell_height,
        }
    }

    pub fn upload_image_to_cell(&self, image: &Image, cell: &TextureAtlasCell) {
        unsafe {
            gl::TextureSubImage2D(
                self.texture.gl_id as _, 
                0 as _,
                cell.x as _, cell.y as _,
                cell.w as _, cell.h as _,
                match image.depth {
                    3 => gl::RGB,
                    4 => gl::RGBA,
                    _ => panic!("Unsupported image format!")
                },
                gl::UNSIGNED_BYTE,
                image.data.as_ptr() as *const c_void
            )
        }
    }

    pub fn allocate_texture(&mut self, width: usize, height: usize) -> Option<TextureAtlasCell> {
        // First convert all widths and heights to cell space, rounding to the next power of 2
        let tex_width_cell = 1.max(width / self.cell_width.next_power_of_two());
        let tex_height_cell = 1.max(height / self.cell_height.next_power_of_two());
        let atlas_width_cell = self.texture.image.width / self.cell_width;
        let atlas_height_cell = self.texture.image.height / self.cell_height;

        // Loop over all cells, that are aligned with the texture width
        for cell_y in (0..atlas_height_cell).step_by(tex_height_cell) {
            for cell_x in (0..atlas_width_cell).step_by(tex_width_cell) {
                let mut is_free = true;
                'in_cell_check: for subcell_offset_y in 0..tex_height_cell {
                    for subcell_offset_x in 0..tex_width_cell {
                        let index = (cell_x + subcell_offset_x) + ((cell_y + subcell_offset_y) * atlas_height_cell);
                        if self.grid[index] == 1 {
                            is_free = false;
                            break 'in_cell_check;
                        }
                    }
                }
                if is_free {
                    // Mark it as occupied
                    for subcell_offset_y in 0..tex_height_cell {
                        for subcell_offset_x in 0..tex_width_cell {
                            let index = (cell_x + subcell_offset_x) + ((cell_y + subcell_offset_y) * atlas_height_cell);
                            self.grid[index] = 1;
                        }
                    }

                    return Some(TextureAtlasCell { 
                        x: cell_x * self.cell_width, 
                        y: cell_y * self.cell_height, 
                        w: width, 
                        h: height 
                    });
                }
            }
        }

        None
    }
}

impl Image {
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

    pub fn load_image_from_gltf(image: &gltf::image::Data) -> Image {
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
        Image {
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
