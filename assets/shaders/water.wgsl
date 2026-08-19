// Voxelforge water surface — the first custom material in this crate.
//
// WHAT IT ADDS OVER `StandardMaterial`. The shipped water was a flat blue tile
// with a baked `water_n.png`: it never moved, it never took the sky, and a
// shallow puddle read exactly like open sea. Three things separate a river in a
// shader mod from a blue quad, and all three are here:
//
//   1. MOVING WAVE NORMALS. Three octaves of directional sine over world XZ,
//      animated off `globals.time`, differentiated analytically into a normal.
//      Procedural, NOT a scrolled normal-map sample — Bevy silently drops a
//      normal map on a mesh without `ATTRIBUTE_TANGENT`, and a procedural
//      gradient can never be dropped. The octaves travel on three different
//      headings so the interference pattern never tiles visibly.
//   2. FRESNEL SKY REFLECTION. Water is ~2% reflective head-on and ~100% at a
//      grazing angle; that ramp IS the look. Schlick against the perturbed
//      normal, mixed toward a two-stop sky gradient passed in from `look.rs`'s
//      hour palette, plus a specular sun glint so the low sun lays a broken
//      highlight down the water instead of one dead sheen.
//   3. DEPTH-GRADED COLOUR. With a depth prepass the surface reads the opaque
//      scene behind it, turns the gap into a thickness in metres, and grades
//      shallow -> deep colour AND opacity by Beer's law. Shorelines go clear
//      and green over sand; the channel goes dark blue and hides its bed.
//      `#ifdef DEPTH_PREPASS` — with no prepass the term folds to the shallow
//      colour and the other two effects still stand on their own.

#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
    forward_io::{VertexOutput, FragmentOutput},
    mesh_view_bindings::{view, globals},
    view_transformations::depth_ndc_to_view_z,
}
#import bevy_pbr::prepass_utils

// Every field is a vec4 on purpose: std140 pads a vec3 to 16 bytes anyway, and a
// mixed scalar/vec3 layout is the classic silent-corruption bug in a hand-written
// uniform. Rust-side mirror: `WaterExtension` in `client/src/water.rs`.
struct WaterMaterial {
    // rgb = the colour a 0 m film of water takes; a = unused.
    shallow_color: vec4<f32>,
    // rgb = the colour water converges on at `depth_scale` metres; a = unused.
    deep_color: vec4<f32>,
    // rgb = sky at the horizon; a = unused.
    horizon_color: vec4<f32>,
    // rgb = sky at the zenith; a = unused.
    zenith_color: vec4<f32>,
    // rgb = sun tint for the glint; a = glint gain.
    sun_color: vec4<f32>,
    // xyz = unit vector pointing AT the sun (same convention as `look.rs`).
    sun_dir: vec4<f32>,
    // x = wave amplitude (normal strength), y = wave spatial frequency,
    // z = wave speed, w = depth_scale in metres (Beer's-law falloff distance).
    wave: vec4<f32>,
    // x = Fresnel F0, y = Fresnel exponent, z = glint sharpness (specular power),
    // w = maximum opacity at full depth (0..1).
    optics: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100)
var<uniform> water: WaterMaterial;

/// One travelling wave octave. Returns the XZ gradient of a sine ridge running
/// along `dir`, which is exactly the slope the surface normal needs — no finite
/// differencing, no sampling, no tiling.
fn wave_grad(p: vec2<f32>, dir: vec2<f32>, freq: f32, speed: f32, amp: f32, t: f32) -> vec2<f32> {
    let phase = dot(p, dir) * freq + t * speed;
    // d/dp of (amp * sin(phase)) = amp * cos(phase) * freq * dir
    return dir * (amp * freq * cos(phase));
}

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    let world_pos = in.world_position.xyz;
    let t = globals.time;

    // ---- 1. moving wave normals ------------------------------------------
    //
    // Three headings that are not multiples of each other, so the sum has no
    // short common period. Each successive octave is finer, faster and weaker —
    // the standard ocean falloff, which keeps the big swell readable while the
    // chop only breaks up the highlight.
    let amp = water.wave.x;
    let freq = water.wave.y;
    let speed = water.wave.z;
    let p = world_pos.xz;
    var g = vec2<f32>(0.0);
    g += wave_grad(p, normalize(vec2<f32>(1.0, 0.35)), freq, speed, amp, t);
    g += wave_grad(p, normalize(vec2<f32>(-0.55, 1.0)), freq * 2.17, speed * 1.4, amp * 0.55, t);
    g += wave_grad(p, normalize(vec2<f32>(0.8, -0.9)), freq * 4.31, speed * 2.1, amp * 0.28, t);

    // The surface is a horizontal quad, so its geometric normal is +Y and the
    // perturbed normal is just (-dh/dx, 1, -dh/dz) renormalised. Only tilt a
    // face that actually points up: a water column's SIDE quad keeps its own
    // normal, or the bank would light as if it were lying flat.
    let up_facing = clamp(pbr_input.world_normal.y, 0.0, 1.0);
    let wave_n = normalize(vec3<f32>(-g.x, 1.0, -g.y));
    let n = normalize(mix(pbr_input.N, wave_n, up_facing));
    pbr_input.N = n;

    let v = normalize(view.world_position.xyz - world_pos);

    // ---- 3. depth-graded colour ------------------------------------------
    //
    // Done before lighting so the graded colour is what actually gets lit.
    // No prepass => 0.0, i.e. everything reads as the SHALLOW stop. An honest
    // no-op floor: the effect is absent rather than faked from a proxy.
    var thickness = 0.0;
#ifdef DEPTH_PREPASS
    // `prepass_depth` is the opaque scene. Water is alpha-blended and therefore
    // absent from the prepass, so this is the BED behind the surface, never the
    // surface itself.
    let bed_ndc = prepass_utils::prepass_depth(in.position, 0u);
    let bed_z = depth_ndc_to_view_z(bed_ndc);
    let surf_z = depth_ndc_to_view_z(in.position.z);
    // View-space z is negative in front of the camera, so the gap is the
    // absolute difference — and because it is measured ALONG the view ray it
    // already lengthens at a glancing angle, which is the physically right
    // behaviour: look flat across a river and you see less of its bed.
    thickness = abs(bed_z - surf_z);
#endif
    let depth_t = 1.0 - exp(-thickness / max(water.wave.w, 0.001)); // Beer's law
    let body = mix(water.shallow_color.rgb, water.deep_color.rgb, depth_t);
    pbr_input.material.base_color = vec4<f32>(
        pbr_input.material.base_color.rgb * body,
        // Clear at the shoreline, opaque over the channel — the single strongest
        // cue that this is a body of water and not a blue lid.
        mix(0.10, water.optics.w, depth_t),
    );

    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

    var out: FragmentOutput;
    if (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
        out.color = apply_pbr_lighting(pbr_input);
    } else {
        out.color = pbr_input.material.base_color;
    }

    // ---- 2. Fresnel sky + sun glint --------------------------------------
    let f0 = water.optics.x;
    let cos_nv = clamp(dot(n, v), 0.0, 1.0);
    let fresnel = clamp(f0 + (1.0 - f0) * pow(1.0 - cos_nv, water.optics.y), 0.0, 1.0);

    // Reflected view ray, coloured by a two-stop sky gradient. Cheap stand-in
    // for a reflection probe, and at a low sun it is the read that matters: the
    // far water takes the horizon band, the near water takes the zenith.
    let r = reflect(-v, n);
    let sky = mix(water.horizon_color.rgb, water.zenith_color.rgb, clamp(r.y, 0.0, 1.0));

    // Specular glint off the wave facets. Blinn-Phong against the perturbed
    // normal, so the chop shatters the highlight into a broken path instead of
    // one mirror blob.
    let h = normalize(v + water.sun_dir.xyz);
    let glint = pow(clamp(dot(n, h), 0.0, 1.0), water.optics.z) * water.sun_color.a;

    out.color = vec4<f32>(
        mix(out.color.rgb, sky, fresnel) + water.sun_color.rgb * glint,
        // A grazing surface is a mirror, and a mirror is not see-through: push
        // opacity up with Fresnel or the reflection reads as a ghost.
        clamp(out.color.a + fresnel * 0.6 + glint, 0.0, 1.0),
    );

    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
