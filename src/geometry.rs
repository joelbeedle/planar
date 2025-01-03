pub fn generate_circle_vertices(radius: f32, segments: usize) -> Vec<f32> {
  let mut vertices = Vec::with_capacity(segments * 3);
  let step = std::f32::consts::TAU / segments as f32;

  for i in 0..segments {
    let angle = step * i as f32;
    let x = radius * angle.cos();
    let y = radius * angle.sin();
    vertices.push(x);
    vertices.push(y);
    vertices.push(0.0); // z
  }
  vertices
}

pub fn generate_circle_indices(segments: usize) -> Vec<u32> {
  let mut indices = Vec::with_capacity(segments + 1);
  for i in 0..segments as u32 {
    indices.push(i);
  }
  // Close the loop by going back to the first vertex
  indices.push(0);
  indices
}
