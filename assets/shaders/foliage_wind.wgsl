// foliage_wind.wgsl — cross-quad vegetation wind vertex shader ("living grass").
//
// This is the FIRST custom vertex shader in the client: every other surface in
// the game wears bevy_pbr's built-in `StandardMaterial`. It is an *extension*
// vertex shader — the fragment stage is bevy_pbr's own PBR, so alpha-cutout,
// fog and lighting all come for free and only the vertex position is changed.
//
// The four requirements from the Director map onto the code below like this:
//   (1) wind FIELD  = `wind_field()`  — value noise, advected by one whole-scene
//       wind direction, so it moves downwind as time advances.
//   (2) GUST        = `gust()`        — a crest travelling along the wind
//       direction; the sin-of-phase reads as a ripple running through the field.
//   (3) per-plant phase/amp = `plant_phase()` / `plant_amp()` — a deterministic
//       hash of the plant's WORLD cell, so neighbours never sway in lockstep.
//   (4) bend ramp   = `bend = h * h` in `vertex()` — uv.y is the height along
//       the plant (0 at the base, 1 at the tip); the quadratic gives a zero
//       derivative at the base, so the root is ANCHORED and never tears loose.
//
// WIND CORE (the functions below) is portable, self-contained WGSL — it does
// not depend on any bevy import and can be re-homed into any shader.

#import bevy_pbr::mesh_functions::mesh_position_local_to_clip
#import bevy_pbr::mesh_functions::mesh_position_local_to_world
#import bevy_pbr::mesh_vertex_output::MeshVertexOutput
#import bevy_pbr::mesh_vertex_output::Vertex

// One uniform block, bound by the Rust `FoliageWindUniform` (ShaderType) via
// `#[uniform(100)]` on the `MaterialExtension`.
@group(2) @binding(100) var<uniform> wind: WindParams;

struct WindParams {
    time: f32,          // seconds since plugin start, advanced every frame
    wind_speed: f32,    // advection speed of the field (m/s)
    wind_scale: f32,    // spatial frequency of the noise field
    wind_dir: vec2<f32>,// unit vector — the ONE wind direction for the scene
    gust_strength: f32, // ripple amplitude added on top of the field
    gust_freq: f32,     // spatial frequency of the travelling gust
    gust_speed: f32,    // gust crest speed along wind_dir (m/s)
    sway_freq: f32,     // per-plant oscillation frequency (rad/s)
    sway_amp: f32,      // max horizontal displacement at the tip (m)
};

// ---------------------------------------------------------------------------
// WIND CORE
// ---------------------------------------------------------------------------

// Deterministic integer-lattice hash -> [0,1). Same scheme the codebase uses
// elsewhere (`look.rs` integer hash); stable across platforms because it only
// uses u32-safe float ops on the fractional parts.
fn hash22(p: vec2<f32>) -> vec2<f32> {
    let n = sin(p * vec2(127.1, 311.7) + p.yx * vec2(269.5, 183.3));
    return fract(n * 43758.5453);
}

// 2D value noise with a smooth (quintic-ish) interp, in [0,1].
fn value_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash22(i).x;
    let b = hash22(i + vec2(1.0, 0.0)).x;
    let c = hash22(i + vec2(0.0, 1.0)).x;
    let d = hash22(i + vec2(1.0, 1.0)).x;
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// (1) The wind field: two octaves of value noise whose sampling position is
//     advected by wind_dir over time — the pattern *travels downwind*, which is
//     what makes it read as wind rather than as static wobble. Output in [-1,1].
fn wind_field(p: vec2<f32>) -> f32 {
    let advect = p + wind.wind_dir * wind.time * wind.wind_speed;
    let n1 = value_noise(advect * wind.wind_scale);
    let n2 = value_noise(advect * wind.wind_scale * 3.71);
    return (n1 * 0.72 + n2 * 0.28) * 2.0 - 1.0;
}

// (2) A gust: a crest travelling along the wind direction. `dot(p, dir)` is
//     the position along the wind axis, so the sin phase advances uniformly
//     with time — a ripple that visibly runs through the whole field.
fn gust(p: vec2<f32>) -> f32 {
    let phase = dot(p, wind.wind_dir) * wind.gust_freq - wind.time * wind.gust_speed;
    return sin(phase) * wind.gust_strength;
}

// (3) Per-plant phase from the plant's world cell: the same cell always gets
//     the same phase, so a plant keeps its own identity frame-to-frame, and two
//     neighbouring cells differ by construction — no robot synchrony.
fn plant_phase(cell: vec2<f32>) -> f32 {
    return hash22(cell).y * 6.28318;   // [0, 2pi)
}

// (3) Per-plant amplitude, also from the world cell, in [0.55, 1.0] so even a
//     quiet field never collapses to a static line.
fn plant_amp(cell: vec2<f32>) -> f32 {
    return 0.55 + 0.45 * hash22(cell * 3.17).x;
}

// Total sway at the tip, as a horizontal displacement vector (metres).
// The oscillation envelope `0.5 + 0.5*cos(...)` keeps the motion smooth and
// lets each plant breathe at its own phase.
fn sway_metres(world_pos: vec3<f32>) -> vec2<f32> {
    let p = world_pos.xz;
    let cell = floor(p);
    let field = wind_field(p);
    let ripple = gust(p);
    let amount = (field + ripple)
        * wind.sway_amp
        * plant_amp(cell)
        * (0.5 + 0.5 * cos(wind.time * wind.sway_freq + plant_phase(cell)));
    return wind.wind_dir * amount;
}

// ---------------------------------------------------------------------------
// VERTEX (extension over bevy_pbr's mesh vertex stage)
// ---------------------------------------------------------------------------

@vertex
fn vertex(in: Vertex) -> MeshVertexOutput {
    var out: MeshVertexOutput;

    let world = mesh_position_local_to_world(in.instance_index, in.position);

    // (4) Bend ramp: uv.y encodes height along the plant (0 = root, 1 = tip).
    //     `h*h` is zero *and* flat at the root, so the plant is pinned to the
    //     ground and the bend grows toward the tip — exactly the wanted shape.
    let h = in.uv.y;
    let bend = h * h;
    let sway = sway_metres(world.xyz) * bend;
    let displaced = vec3(in.position.x + sway.x, in.position.y, in.position.z + sway.y);

    out.clip_position = mesh_position_local_to_clip(in.instance_index, displaced);
    // The normal is left unchanged: a cross-quad is a thin billboard and the
    // light response of a swaying blade should not fight the wind each frame.
    out.world_position = world;
    out.world_normal = in.normal;
    out.uv = in.uv;
    return out;
}
