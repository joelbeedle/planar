use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct InstanceData {
  pub model_matrix: [[f32; 4]; 4],
}

pub enum ShapeType {
  Circle,
  Triangle,
}

pub struct Shape {
  pub shape_type: ShapeType,
  pub position: glam::Vec3,
  pub scale: f32,
  pub instance_buffer: wgpu::Buffer,
  pub bind_group: wgpu::BindGroup,
}

impl Shape {
  pub fn new_circle(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    position: glam::Vec3,
    scale: f32,
  ) -> Self {
    // Construct the model matrix for the instance
    let instance_data = InstanceData {
      model_matrix: (glam::Mat4::from_translation(position)
        * glam::Mat4::from_scale(glam::Vec3::splat(scale)))
      .to_cols_array_2d(),
    };

    let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("Circle Instance Buffer"),
      contents: bytemuck::cast_slice(&[instance_data]),
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: bind_group_layout,
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: instance_buffer.as_entire_binding(),
      }],
      label: Some("Circle Bind Group"),
    });

    Self {
      shape_type: ShapeType::Circle,
      position,
      scale,
      instance_buffer,
      bind_group,
    }
  }

  pub fn new_triangle(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    position: glam::Vec3,
    scale: f32,
  ) -> Self {
    let instance_data = InstanceData {
      model_matrix: (glam::Mat4::from_translation(position)
        * glam::Mat4::from_scale(glam::Vec3::splat(scale)))
      .to_cols_array_2d(),
    };

    let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("Triangle Instance Buffer"),
      contents: bytemuck::cast_slice(&[instance_data]),
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: bind_group_layout,
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: instance_buffer.as_entire_binding(),
      }],
      label: Some("Triangle Bind Group"),
    });

    Self {
      shape_type: ShapeType::Triangle,
      position,
      scale,
      instance_buffer,
      bind_group,
    }
  }
}
