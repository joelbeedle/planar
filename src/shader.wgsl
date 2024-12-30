struct Uniforms {
    aspect_ratio: f32,
}
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(@location(0) position : vec3<f32>) -> @builtin(position) vec4<f32> {
    // Adjust x by aspect ratio so circle remains truly circular.
    let corrected_x = position.x / uniforms.aspect_ratio;
    return vec4<f32>(corrected_x, position.y, position.z, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    // Just draw white lines
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}
