// Uniform data for instance transformations
struct InstanceData {
    model_matrix: mat4x4<f32>,
};

struct Uniforms {
    aspect_ratio: f32,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(1) @binding(0)
var<uniform> instance: InstanceData;

// Input from vertex buffer
struct VertexInput {
    @location(0) position: vec3<f32>,
};

// Output to the fragment shader
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;

    // Apply aspect ratio correction to the x-coordinate
    let corrected_model_matrix = instance.model_matrix * mat4x4<f32>(
        vec4<f32>(1.0 / uniforms.aspect_ratio, 0.0, 0.0, 0.0),
        vec4<f32>(0.0, 1.0, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, 1.0, 0.0),
        vec4<f32>(0.0, 0.0, 0.0, 1.0),
    );

    // Transform the vertex position
    let world_position = corrected_model_matrix * vec4(input.position, 1.0);
    output.world_position = world_position.xyz;
    output.clip_position = world_position;

    return output;
}

// Input from vertex shader
@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Simple coloring based on position (example: a gradient effect)
    let color = vec3(0.5 + input.world_position.x, 0.5 + input.world_position.y, 1.0);
    return vec4(color, 1.0); // RGBA color
}
