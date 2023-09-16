use glam::Vec3;

use crate::{aabb::AABB, structs::Triangle};

struct BvhNode {
	bounds: AABB, // 24 bytes
	left_first: u32, // 4 bytes - if leaf, specifies first primitive index, otherwise, specifies node offset
	count: u32, // 4 bytes - if non-zero, this is a leaf node
}

struct Bvh {
	nodes: Vec<BvhNode>, // node 0 is always the root node
	indices: Vec::<u32>,
	triangles: Vec::<Triangle>,
}

enum Axis {
	X, Y, Z
}

impl Bvh {
	pub fn construct(triangles: Vec<Triangle>) -> Self {
		// Create new BVH
		let mut new_bvh = Self {
			nodes: Vec::new(),
			indices: (0..triangles.len() as u32).collect(),
			triangles,
		};

		// Create root node
		new_bvh.nodes.push(BvhNode {
			bounds: AABB::new(),
			left_first: 0,
			count: new_bvh.triangles.len() as _,
		});

		new_bvh.subdivide(0, 0);

		return new_bvh;
	}

	fn subdivide(&mut self, node_index: usize, rec_depth: usize) {
		// Get node
		let node = &mut self.nodes[node_index];

		// Calculate node bounds
		let begin = node.left_first;
		let end = begin + node.count;
		for i in begin..end {
			let triangle = self.triangles.get(i as usize).unwrap();
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
		for i in begin..end {
			let triangle = self.triangles.get(i as usize).unwrap();
			avg += triangle.v0.position;
			avg += triangle.v1.position;
			avg += triangle.v2.position;
		}
		avg /= (node.count * 3) as f32;

		// Determine split axis - choose biggest axis
		let size = node.bounds.max - node.bounds.min;
		let (split_axis, split_pos) = {
			if size.x > size.y && size.x > size.z {
				(Axis::X, avg.x)
			}
			else if size.y > size.x && size.y > size.z {
				(Axis::Y, avg.y)
			}
			else {
				(Axis::Z, avg.z)
			}
		};

		// Partition the index array, and get the split position
		let node_left_first = node.left_first;
		let node_count = node.count;
		node.count = 0; // this is not a leaf node.
		let mut split_index = self.partition(split_axis, split_pos, node_left_first, node_count);

    	// Create 2 child nodes
		let left = node_left_first as usize;
		self.nodes.push(BvhNode{ bounds: AABB::new(), left_first: node_left_first, count: split_index - node_left_first });
		let right = node_left_first as usize + 1;
		self.nodes.push(BvhNode{ bounds: AABB::new(), left_first: split_index, count: node_left_first + node_count - split_index});

		// Subdivide further
		self.subdivide(left, rec_depth + 1);
		self.subdivide(right, rec_depth + 1);

}

	fn partition(&mut self, axis: Axis, pivot: f32, start: u32, count: u32) -> u32 {
		let mut i = start;
		let mut j = start + count - 1;
		while i < j {
			// Get triangle center
			let tri = &self.triangles[self.indices[i as usize] as usize];
			let center = match &axis {
				Axis::X => (tri.v0.position.x + tri.v1.position.x + tri.v2.position.x) / 3.0,
				Axis::Y => (tri.v0.position.y + tri.v1.position.y + tri.v2.position.y) / 3.0,
				Axis::Z => (tri.v0.position.z + tri.v1.position.z + tri.v2.position.z) / 3.0,
			};

	        // If the current primitive's center's <axis>-component is greated than the pivot's <axis>-component
			if center > pivot {
				(self.indices[i as usize], self.indices[j as usize]) = (self.indices[j as usize], self.indices[j as usize]);
				j -= 1;
			}
			i += 1;
		}

		return i;
	}
}