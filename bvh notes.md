# BVH 
Optimizations:
- left node index should be aligned to grid of 2 (index % 2 == 0), makes nodes sit together on 1 cache line
- bvh node pool needs to be aligned to 64 bytes (cpu cache line)
- early out via `if (!intersects_bounds()) return;`
- might be possible to early out after a bounding box check if the distance to that bounding box is bigger than the current lowest primitive hit distance

## Extra notes
TLAS: 
- Is a tree that contains BLAS's in the leaf nodes, or maybe primitives like spheres, if wanted.
- Each BLAS node also has a local matrix, so when entering a BLAS, we can transform the ray from world-space to model-space, and continue from there. Then when doing shadow or reflection rays, we convert back to world-space and cast from there.

BLAS:
- Is a BVH for a single mesh
- Is traversed in model-space, so rays have to be converted from world-space to model-space.
- If a TLAS has multiple BLAS's, the BLAS uses the same node pool, and then each BLAS just has a unique root index. So in a sense the node pool can just be all the BVH's node pools appended to each other, and then just offset.
- If I just make one global node pool, one global primitive pool, and one global index pool, shared across all BVH's, and then when creating a BVH for a mesh, we use that pool, and then just store the root index into the node pool in the mesh.