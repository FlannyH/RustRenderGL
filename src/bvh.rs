use std::mem::size_of;

use glam::Vec3;

use crate::{aabb::AABB, structs::Triangle};

pub struct BvhNode {
    pub bounds: AABB,    // 24 bytes
    pub left_first: i32, // 4 bytes - if leaf, specifies first primitive index, otherwise, specifies node offset
    pub count: i32,      // 4 bytes - if non-zero, this is a leaf node
}

pub struct Bvh {
    pub nodes: Vec<BvhNode>, // node 0 is always the root node
    pub indices: Vec<u32>,
    pub triangles: Vec<Triangle>,
    pub gpu_nodes: u32,
    pub gpu_indices: u32,
    pub gpu_triangles: u32,
    pub gpu_counts: u32,
}

enum Axis {
    X,
    Y,
    Z,
}

impl Bvh {
    pub fn construct(triangles: Vec<Triangle>) -> Self {
        // Create new BVH
        let mut new_bvh = Self {
            nodes: Vec::new(),
            indices: (0..triangles.len() as u32).collect(),
            triangles,
            gpu_nodes: 0,
            gpu_indices: 0,
            gpu_triangles: 0,
            gpu_counts: 0,
        };

        // Create root node
        new_bvh.nodes.push(BvhNode {
            bounds: AABB::new(),
            left_first: 0,
            count: new_bvh.triangles.len() as _,
        });

        // Recursively break down into smaller nodes
        new_bvh.subdivide(0, 0);

        // We're done, let's create buffers on the GPU
        let cpu_counts = [new_bvh.nodes.len() as u32, new_bvh.indices.len() as u32];
        unsafe {
            // Nodes
            gl::GenBuffers(1, &mut new_bvh.gpu_nodes);
            gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, new_bvh.gpu_nodes);
            gl::BufferStorage(
                gl::SHADER_STORAGE_BUFFER,
                (new_bvh.nodes.len() * size_of::<BvhNode>()) as isize,
                new_bvh.nodes.as_ptr() as _,
                gl::MAP_READ_BIT,
            );

            // Indices
            gl::GenBuffers(1, &mut new_bvh.gpu_indices);
            gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, new_bvh.gpu_indices);
            gl::BufferStorage(
                gl::SHADER_STORAGE_BUFFER,
                (new_bvh.indices.len() * size_of::<u32>()) as isize,
                new_bvh.indices.as_ptr() as _,
                gl::MAP_READ_BIT,
            );

            // Triangles
            gl::GenBuffers(1, &mut new_bvh.gpu_triangles);
            gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, new_bvh.gpu_triangles);
            gl::BufferStorage(
                gl::SHADER_STORAGE_BUFFER,
                (new_bvh.triangles.len() * size_of::<Triangle>()) as isize,
                new_bvh.triangles.as_ptr() as _,
                gl::MAP_READ_BIT,
            );

            // Counts
            gl::GenBuffers(1, &mut new_bvh.gpu_counts);
            gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, new_bvh.gpu_counts);
            gl::BufferStorage(
                gl::SHADER_STORAGE_BUFFER,
                (cpu_counts.len() * size_of::<u32>()) as isize,
                cpu_counts.as_ptr() as _,
                gl::MAP_READ_BIT,
            );
            gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, 0);
        }

        return new_bvh;
    }

    fn subdivide(&mut self, node_index: usize, rec_depth: usize) {
        // Get node
        let left = self.nodes.len();
        let node = &mut self.nodes[node_index];

        // Calculate node bounds
        let begin = node.left_first;
        let end = begin + node.count;
        for i in begin..end {
            let triangle = self
                .triangles
                .get(*self.indices.get(i as usize).unwrap() as usize)
                .unwrap();
            node.bounds.grow(triangle.v0.position);
            node.bounds.grow(triangle.v1.position);
            node.bounds.grow(triangle.v2.position);
        }

        // Only subdivide if we have more than 2 triangles
        if node.count <= 2 || rec_depth > 30 {
            return;
        }

        // Get the average position of all the primitives
        let mut avg = Vec3::ZERO;
        let mut divide = 0;
        for i in begin..end {
            let triangle = self
                .triangles
                .get(*self.indices.get(i as usize).unwrap() as usize)
                .unwrap();
            avg += triangle.v0.position / 3.0;
            avg += triangle.v1.position / 3.0;
            avg += triangle.v2.position / 3.0;
            divide += 1;
        }
        avg /= divide as f32;

        // Determine split axis - choose biggest axis
        let size = node.bounds.max - node.bounds.min;
        let (split_axis, split_pos) = {
            if size.x > size.y && size.x > size.z {
                (Axis::X, avg.x)
            } else if size.y > size.x && size.y > size.z {
                (Axis::Y, avg.y)
            } else {
                (Axis::Z, avg.z)
            }
        };

        // Partition the index array, and get the split position
        let start_index = node.left_first;
        let node_count = node.count;
        node.count = -1; // this is not a leaf node.
        node.left_first = left as _; // this node has to point to the 2 child nodes
        let split_index = self.partition(split_axis, split_pos, start_index, node_count);
        let node = &mut self.nodes[node_index];

        // Abort if one of the sides is empty
        if split_index - start_index == 0 || split_index - start_index == node_count {
            node.count = node_count;
            return;
        }

        // Create 2 child nodes
        self.nodes.push(BvhNode {
            bounds: AABB::new(),
            left_first: start_index,
            count: split_index - start_index,
        });
        let right = self.nodes.len();
        self.nodes.push(BvhNode {
            bounds: AABB::new(),
            left_first: split_index,
            count: start_index + node_count - split_index,
        });

        // Subdivide further
        self.subdivide(left, rec_depth + 1);
        self.subdivide(right, rec_depth + 1);
    }

    fn partition(&mut self, axis: Axis, pivot: f32, start: i32, count: i32) -> i32 {
        let mut i = start;
        let mut j = start + count - 1;
        while i <= j {
            // Get triangle center
            let tri = &self.triangles[self.indices[i as usize] as usize];
            let center = match &axis {
                Axis::X => (tri.v0.position.x + tri.v1.position.x + tri.v2.position.x) / 3.0,
                Axis::Y => (tri.v0.position.y + tri.v1.position.y + tri.v2.position.y) / 3.0,
                Axis::Z => (tri.v0.position.z + tri.v1.position.z + tri.v2.position.z) / 3.0,
            };

            // If the current primitive's center's <axis>-component is greated than the pivot's <axis>-component
            if center > pivot {
                (self.indices[i as usize], self.indices[j as usize]) =
                    (self.indices[j as usize], self.indices[i as usize]);
                j -= 1;
            } else {
                i += 1;
            }
        }

        return i;
    }
}
