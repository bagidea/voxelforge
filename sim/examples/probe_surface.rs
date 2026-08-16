// Scratch probe (yamamoto, audio lane) — scan world columns for surface
// blocks that map to FootstepSurface::Grass / Wood, to find real walkable
// coordinates for the audio proof script. Not part of the shipped crate.
use voxelforge_sim::block::BlockId;
use voxelforge_sim::worldgen::{terrain_block, terrain_height};

fn surface_label(b: BlockId) -> &'static str {
    match b {
        BlockId::GRASS | BlockId::DIRT | BlockId::MOSS => "Grass",
        BlockId::STONE | BlockId::COBBLESTONE | BlockId::OBSIDIAN | BlockId::BRICK
        | BlockId::LIMESTONE | BlockId::CLAY | BlockId::GRAVEL => "Stone",
        BlockId::WOOD => "Wood",
        BlockId::SAND | BlockId::RED_SAND | BlockId::SNOW => "Sand",
        _ => "Grass",
    }
}

fn check(label: &str, x: f32, z: f32, want: &str) {
    let h = terrain_height(x, z);
    let b = terrain_block(x, z, h, h);
    let got = surface_label(b);
    let verdict = if got == want { "PASS" } else { "FAIL" };
    println!(
        "{label}: surface_at({x},{z}) h={h} block={b:?} => {got} (want {want}) [{verdict}]"
    );
}

fn main() {
    // Scan strictly inside the EXISTING campfire_square bounds (28..52,28..52)
    // so the walk-detour doesn't need to touch the region fixture or the
    // prove_audio.sh gate's hardcoded zone-crossing assertions.
    let mut counts: std::collections::HashMap<&str, i32> = std::collections::HashMap::new();
    let mut grass_pts = vec![];
    let mut wood_pts = vec![];
    for x in 28..=52 {
        for z in 28..=52 {
            let wx = x as f32;
            let wz = z as f32;
            let h = terrain_height(wx, wz);
            let b = terrain_block(wx, wz, h, h);
            let label = surface_label(b);
            *counts.entry(label).or_insert(0) += 1;
            if label == "Grass" {
                grass_pts.push((x, z));
            }
            if label == "Wood" {
                wood_pts.push((x, z));
            }
        }
    }
    println!("counts in campfire_square (28..=52,28..=52): {:?}", counts);
    println!("GRASS pts ({}): {:?}", grass_pts.len(), grass_pts);
    println!("WOOD pts ({}): {:?}", wood_pts.len(), wood_pts);

    println!();
    println!("== audio_proof_main.rs WalkLeg targets — direct check ==");
    check("leg3 target (wood)", 36.0, 46.0, "Wood");
    check("leg4 target (grass)", 44.0, 49.0, "Grass");
    println!();
    let in_bounds = |x: f32, z: f32| (28.0..=52.0).contains(&x) && (28.0..=52.0).contains(&z);
    println!(
        "leg3 (36,46) in campfire_square bounds 28..52,28..52 => {}",
        in_bounds(36.0, 46.0)
    );
    println!(
        "leg4 (44,49) in campfire_square bounds 28..52,28..52 => {}",
        in_bounds(44.0, 49.0)
    );
}
