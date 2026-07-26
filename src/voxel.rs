//! Voxel chunk generation, greedy meshing, and a procedurally-built texture atlas.
//!
//! Kept deliberately small — this is a Phase-0 spike whose job is to prove the
//! render pipeline (greedy mesh -> atlas-sampled StandardMaterial), not to be a
//! finished engine.

use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Cubic chunk edge (32^3 voxels).
pub const CHUNK: i32 = 32;

// Block ids. 0 == air.
const AIR: u8 = 0;
const GRASS: u8 = 1;
const DIRT: u8 = 2;
const STONE: u8 = 3;
const SAND: u8 = 4;

/// Number of tiles in the (horizontal) atlas.
const N_TILES: usize = 4;
const TILE_PX: usize = 16;

/// A single chunk's dense voxel volume.
pub struct Chunk {
    blocks: Vec<u8>,
}

#[inline]
fn idx(x: i32, y: i32, z: i32) -> usize {
    (x + CHUNK * (z + CHUNK * y)) as usize
}

/// Deterministic terrain height (in world space) so neighbouring chunks tile seamlessly.
fn terrain_height(wx: f32, wz: f32) -> i32 {
    let h = 14.0
        + 7.0 * (wx * 0.08).sin() * (wz * 0.06).cos()
        + 3.0 * ((wx + wz) * 0.15).sin();
    (h.round() as i32).clamp(1, CHUNK - 2)
}

impl Chunk {
    /// Generate a chunk whose origin sits at world (chunk_x*CHUNK, 0, chunk_z*CHUNK).
    pub fn generate(chunk_x: i32, chunk_z: i32) -> Self {
        let mut blocks = vec![AIR; (CHUNK * CHUNK * CHUNK) as usize];
        for z in 0..CHUNK {
            for x in 0..CHUNK {
                let wx = (chunk_x * CHUNK + x) as f32;
                let wz = (chunk_z * CHUNK + z) as f32;
                let h = terrain_height(wx, wz);
                for y in 0..=h.min(CHUNK - 1) {
                    let b = if y == h {
                        if h < 11 { SAND } else { GRASS }
                    } else if y > h - 4 {
                        DIRT
                    } else {
                        STONE
                    };
                    blocks[idx(x, y, z)] = b;
                }
            }
        }
        Self { blocks }
    }

    #[inline]
    fn get(&self, x: i32, y: i32, z: i32) -> u8 {
        if x < 0 || y < 0 || z < 0 || x >= CHUNK || y >= CHUNK || z >= CHUNK {
            AIR
        } else {
            self.blocks[idx(x, y, z)]
        }
    }

    /// Greedy-mesh the chunk into a single Bevy mesh.
    ///
    /// Classic Lysenko sweep: for each of the 3 axes, build a per-slice mask of
    /// exposed faces, then merge co-planar same-block faces into the largest
    /// possible quads. Returns the mesh plus the emitted quad count (for stats).
    pub fn greedy_mesh(&self) -> (Mesh, usize) {
        let dims = [CHUNK, CHUNK, CHUNK];
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
                        let a = self.get(x[0], x[1], x[2]);
                        let b = self.get(x[0] + q[0], x[1] + q[1], x[2] + q[2]);
                        let sa = a != AIR;
                        let sb = b != AIR;
                        mask[n] = if sa == sb {
                            0
                        } else if sa {
                            a as i32 // front face, normal +q
                        } else {
                            -(b as i32) // back face, normal -q
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

                            // atlas UVs for this block's tile (single tile stretched
                            // across the merged quad — the standard greedy-mesh
                            // limitation; production fix = a texture array / custom
                            // material with per-voxel tile indices).
                            let t = (block - 1) as f32;
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
}

/// Build the 4-tile texture atlas procedurally (no shipped PNG => identical on
/// native and web, no asset-path headaches).
pub fn build_atlas() -> Image {
    let w = N_TILES * TILE_PX;
    let h = TILE_PX;
    let mut data = vec![0u8; w * h * 4];

    // base colour per tile: grass, dirt, stone, sand
    let bases: [[u8; 3]; N_TILES] = [
        [70, 160, 66],
        [124, 88, 56],
        [128, 128, 138],
        [214, 202, 148],
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
