//! Client-side voxel helpers: greedy mesh builder and procedural texture atlas.
//!
//! All block-type and chunk-data definitions live in `voxelforge_sim`; this
//! module only contains the Bevy-coupled surface: Mesh construction and the
//! atlas Image. Keeping the mesher here (not in sim) prevents a Bevy dependency
//! from leaking into the shared crate.

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use voxelforge_sim::block::BlockId;
use voxelforge_sim::chunk::{ChunkData, CHUNK_SIZE};

/// Number of tiles in the (horizontal) atlas — one per BlockId (0-15).
const N_TILES: usize = 16;
const TILE_PX: usize = 16;

/// Greedy-mesh a chunk into a single Bevy Mesh.
///
/// Classic Lysenko sweep: for each of the 3 axes, build a per-slice mask of
/// exposed faces, then merge co-planar same-block faces into the largest
/// possible quads. Returns the mesh plus the emitted quad count (for stats).
pub fn greedy_mesh_chunk(chunk: &ChunkData) -> (Mesh, usize) {
    let dims = [CHUNK_SIZE, CHUNK_SIZE, CHUNK_SIZE];
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut quads = 0usize;

    for d in 0..3usize {
        let u = (d + 1) % 3;
        let v = (d + 2) % 3;
        let mut x = [0i32; 3];
        let mut q = [0i32; 3];
        q[d] = 1;

        // mask[i + j*du_len]: 0 = no face, +id = front face (+q), -id = back face (-q)
        let mut mask = vec![0i32; (dims[u] * dims[v]) as usize];

        x[d] = -1;
        while x[d] < dims[d] {
            // ---- build mask for the plane between slice x[d] and x[d]+1 ----
            let mut n = 0usize;
            for j in 0..dims[v] {
                for i in 0..dims[u] {
                    x[u] = i;
                    x[v] = j;
                    let a = chunk.get(x[0], x[1], x[2]);
                    let b = chunk.get(x[0] + q[0], x[1] + q[1], x[2] + q[2]);
                    let sa = a.is_opaque();
                    let sb = b.is_opaque();
                    mask[n] = if sa == sb {
                        0
                    } else if sa {
                        a.0 as i32
                    } else {
                        -(b.0 as i32)
                    };
                    n += 1;
                }
            }

            x[d] += 1;

            // ---- greedily merge the mask into quads ----
            let mut n = 0usize;
            for j in 0..dims[v] {
                let mut i = 0i32;
                while i < dims[u] {
                    let c = mask[n];
                    if c != 0 {
                        // width run along u
                        let mut w = 1i32;
                        while i + w < dims[u] && mask[n + w as usize] == c {
                            w += 1;
                        }
                        // height run along v
                        let mut h = 1i32;
                        'grow: while j + h < dims[v] {
                            for k in 0..w {
                                if mask[n + k as usize + (h * dims[u]) as usize] != c {
                                    break 'grow;
                                }
                            }
                            h += 1;
                        }

                        // emit the quad
                        x[u] = i;
                        x[v] = j;
                        let mut du = [0f32; 3];
                        du[u] = w as f32;
                        let mut dv = [0f32; 3];
                        dv[v] = h as f32;
                        let p = [x[0] as f32, x[1] as f32, x[2] as f32];

                        let front = c > 0;
                        let block = c.unsigned_abs() as u8;
                        let mut nrm = [0f32; 3];
                        nrm[d] = if front { 1.0 } else { -1.0 };

                        let base = positions.len() as u32;
                        let v0 = p;
                        let v1 = [p[0] + du[0], p[1] + du[1], p[2] + du[2]];
                        let v2 = [
                            p[0] + du[0] + dv[0],
                            p[1] + du[1] + dv[1],
                            p[2] + du[2] + dv[2],
                        ];
                        let v3 = [p[0] + dv[0], p[1] + dv[1], p[2] + dv[2]];
                        positions.extend_from_slice(&[v0, v1, v2, v3]);
                        normals.extend_from_slice(&[nrm; 4]);

                        // Atlas UVs for this block's tile (single tile stretched
                        // across the merged quad — the standard greedy-mesh
                        // limitation; production fix = a texture array / custom
                        // material with per-voxel tile indices).
                        let t = block as f32;
                        let inset = 0.5 / (N_TILES * TILE_PX) as f32;
                        let u0 = t / N_TILES as f32 + inset;
                        let u1 = (t + 1.0) / N_TILES as f32 - inset;
                        uvs.extend_from_slice(&[[u0, 0.0], [u1, 0.0], [u1, 1.0], [u0, 1.0]]);

                        if front {
                            indices.extend_from_slice(&[
                                base, base + 1, base + 2, base, base + 2, base + 3,
                            ]);
                        } else {
                            indices.extend_from_slice(&[
                                base, base + 2, base + 1, base, base + 3, base + 2,
                            ]);
                        }
                        quads += 1;

                        // zero the consumed mask cells
                        for l in 0..h {
                            for k in 0..w {
                                mask[n + k as usize + (l * dims[u]) as usize] = 0;
                            }
                        }
                        i += w;
                        n += w as usize;
                    } else {
                        i += 1;
                        n += 1;
                    }
                }
            }
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    (mesh, quads)
}

/// Build the 4-tile texture atlas procedurally (no shipped PNG => identical on
/// native and web, no asset-path headaches).
pub fn build_atlas() -> Image {
    let w = N_TILES * TILE_PX;
    let h = TILE_PX;
    let mut data = vec![0u8; w * h * 4];

    // base colour per tile — sourced from BlockId so the atlas stays in sync
    // with the sim palette automatically.  Tile index == block ID.
    let bases: [[u8; 3]; N_TILES] = [
        BlockId::AIR.base_color(),
        BlockId::GRASS.base_color(),
        BlockId::DIRT.base_color(),
        BlockId::STONE.base_color(),
        BlockId::SAND.base_color(),
        BlockId::WOOD.base_color(),
        BlockId::LEAVES.base_color(),
        BlockId::SNOW.base_color(),
        BlockId::RED_SAND.base_color(),
        BlockId::CLAY.base_color(),
        BlockId::GRAVEL.base_color(),
        BlockId::COBBLESTONE.base_color(),
        BlockId::OBSIDIAN.base_color(),
        BlockId::BRICK.base_color(),
        BlockId::MOSS.base_color(),
        BlockId::LIMESTONE.base_color(),
    ];

    for ty in 0..h {
        for tx in 0..w {
            let tile = tx / TILE_PX;
            let lx = tx % TILE_PX;
            let base = bases[tile];
            // cheap deterministic per-texel dither so the surface reads as textured
            let hsh = ((tx as u32).wrapping_mul(374761393)
                ^ (ty as u32).wrapping_mul(668265263))
                .wrapping_mul(1274126177);
            let n = (hsh >> 24) as i32 % 26 - 13;
            // darker tile border grid
            let border = lx == 0 || ty == 0 || lx == TILE_PX - 1 || ty == TILE_PX - 1;
            let shade = if border { -40 } else { n };
            let px = (ty * w + tx) * 4;
            for c in 0..3 {
                data[px + c] = (base[c] as i32 + shade).clamp(0, 255) as u8;
            }
            data[px + 3] = 255;
        }
    }

    let mut image = Image::new(
        Extent3d {
            width: w as u32,
            height: h as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    // crisp voxel look
    image.sampler = ImageSampler::nearest();
    image
}
