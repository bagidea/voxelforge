// Standalone proof that `water::tests::the_wgsl_uniform_mirrors_the_rust_struct`
// actually parses the shader — same parser body, no bevy, no cargo.
//
//   rustc --test _flamingo_wgsl_abi_check.rs -o _flamingo_wgsl_abi_check.exe
//   ./_flamingo_wgsl_abi_check.exe
//
// The in-crate test gets the Rust field order from reflection; here it is the
// literal below, because there is no bevy to reflect with. That is the ONLY
// difference — the WGSL side is parsed from the real file both times.

const WGSL: &str = include_str!("assets/shaders/water.wgsl");

/// Declaration order of `WaterExtension` in `client/src/water.rs`.
const RUST_FIELDS: [&str; 8] = [
    "shallow_color",
    "deep_color",
    "horizon_color",
    "zenith_color",
    "sun_color",
    "sun_dir",
    "wave",
    "optics",
];

fn wgsl_uniform_fields() -> Vec<(String, String)> {
    let body = WGSL
        .split_once("struct WaterMaterial {")
        .expect("water.wgsl must declare `struct WaterMaterial {`")
        .1
        .split_once('}')
        .expect("unterminated struct WaterMaterial")
        .0;
    body.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with("//"))
        .map(|l| {
            let (name, ty) = l
                .trim_end_matches(',')
                .split_once(':')
                .unwrap_or_else(|| panic!("unparsable field line in water.wgsl: {l:?}"));
            (name.trim().to_string(), ty.trim().to_string())
        })
        .collect()
}

#[test]
fn wgsl_matches_rust() {
    let wgsl = wgsl_uniform_fields();
    let names: Vec<&str> = wgsl.iter().map(|(n, _)| n.as_str()).collect();
    println!("parsed from water.wgsl: {names:?}");
    assert_eq!(names, RUST_FIELDS);
    for (n, t) in &wgsl {
        assert_eq!(t, "vec4<f32>", "{n}");
    }
}

#[test]
fn the_gate_can_fail() {
    let wgsl = wgsl_uniform_fields();
    assert!(!wgsl.is_empty(), "parser read nothing");
    let mut planted: Vec<&str> = wgsl.iter().map(|(n, _)| n.as_str()).collect();
    planted.swap(0, 1);
    assert_ne!(planted, RUST_FIELDS, "a swapped pair still compared equal");
}

#[test]
fn depth_term_is_guarded() {
    let guard = WGSL
        .split_once("#ifdef DEPTH_PREPASS")
        .expect("guard missing")
        .1
        .split_once("#endif")
        .expect("unterminated guard")
        .0;
    assert!(guard.contains("pbr_input.material.base_color = vec4<f32>("));
    let before = WGSL.split_once("#ifdef DEPTH_PREPASS").unwrap().0;
    assert!(!before.contains("pbr_input.material.base_color ="));
}

#[test]
fn no_hardcoded_opacity_literal_survives() {
    // The reviewer's bug was the literal 0.10 in the alpha mix.
    assert!(
        !WGSL.contains("mix(0.10,"),
        "the hardcoded shore-opacity literal is still in the shader"
    );
    assert!(WGSL.contains("mix(water.shallow_color.a, water.deep_color.a, depth_t)"));
}
