use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use winit::{
  event::*,
  event_loop::{ControlFlow, EventLoop},
  window::WindowBuilder,
};

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Uniforms {
  aspect_ratio: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct InstanceData {
  model_matrix: [[f32; 4]; 4],
}

enum ShapeType {
  Circle,
  Triangle,
}

struct Shape {
  shape_type: ShapeType,
  position: glam::Vec3,
  scale: f32,
  instance_buffer: wgpu::Buffer,
  bind_group: wgpu::BindGroup,
}

impl Shape {
  fn new_circle(
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

  fn new_triangle(
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

#[tokio::main]
async fn main() {
  // -------------------------------------
  // Setup: Create window, WGPU instance, etc.
  // -------------------------------------
  let event_loop = EventLoop::new();
  let window = WindowBuilder::new()
    .with_title("Shapes Renderer")
    .with_inner_size(winit::dpi::LogicalSize::new(800, 600))
    .build(&event_loop)
    .unwrap();

  let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
    backends: wgpu::Backends::all(),
    dx12_shader_compiler: Default::default(),
  });
  let surface = unsafe { instance.create_surface(&window) }.unwrap();
  let adapter = instance
    .request_adapter(&wgpu::RequestAdapterOptions {
      power_preference: wgpu::PowerPreference::HighPerformance,
      force_fallback_adapter: false,
      compatible_surface: Some(&surface),
    })
    .await
    .expect("Failed to request adapter");

  let (device, queue) = adapter
    .request_device(
      &wgpu::DeviceDescriptor {
        label: Some("Device"),
        features: wgpu::Features::POLYGON_MODE_LINE,
        limits: wgpu::Limits::default(),
      },
      None,
    )
    .await
    .expect("Failed to create device");

  let size = window.inner_size();
  let capabilities = surface.get_capabilities(&adapter);
  let surface_format = capabilities.formats[0];

  let mut surface_config = wgpu::SurfaceConfiguration {
    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
    format: surface_format,
    width: size.width,
    height: size.height,
    present_mode: wgpu::PresentMode::Fifo,
    alpha_mode: wgpu::CompositeAlphaMode::Opaque,
    view_formats: vec![surface_format],
  };
  surface.configure(&device, &surface_config);

  // -------------------------------------
  // Uniform Buffer and Bind Group
  // -------------------------------------
  let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    label: Some("Instance Bind Group Layout"),
    entries: &[wgpu::BindGroupLayoutEntry {
      binding: 0,
      visibility: wgpu::ShaderStages::VERTEX,
      ty: wgpu::BindingType::Buffer {
        ty: wgpu::BufferBindingType::Uniform,
        has_dynamic_offset: false,
        min_binding_size: None,
      },
      count: None,
    }],
  });
  let mut uniforms = Uniforms {
    aspect_ratio: size.width as f32 / size.height as f32,
  };
  let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
    label: Some("Uniform Buffer"),
    contents: bytemuck::bytes_of(&uniforms),
    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
  });

  let uniform_bind_group_layout =
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: Some("Uniform BGL"),
      entries: &[wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::VERTEX,
        ty: wgpu::BindingType::Buffer {
          ty: wgpu::BufferBindingType::Uniform,
          has_dynamic_offset: false,
          min_binding_size: None,
        },
        count: None,
      }],
    });

  let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
    label: Some("Uniform BG"),
    layout: &uniform_bind_group_layout,
    entries: &[wgpu::BindGroupEntry {
      binding: 0,
      resource: uniform_buffer.as_entire_binding(),
    }],
  });
  let circle_segments = 64;
  let circle_vertex_data = generate_circle_vertices(0.5, circle_segments);
  let circle_index_data = generate_circle_indices(circle_segments);
  let circle_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
    label: Some("Circle VB"),
    contents: bytemuck::cast_slice(&circle_vertex_data),
    usage: wgpu::BufferUsages::VERTEX,
  });
  let circle_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
    label: Some("Circle IB"),
    contents: bytemuck::cast_slice(&circle_index_data),
    usage: wgpu::BufferUsages::INDEX,
  });
  let circle_index_count = circle_index_data.len() as u32;

  // Triangle
  let triangle_vertex_data: &[f32] = &[
    0.0, 0.5, 0.0, -0.5, -0.5, 0.0, 0.5, -0.5, 0.0, // x, y, z
  ];
  let triangle_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
    label: Some("Triangle VB"),
    contents: bytemuck::cast_slice(triangle_vertex_data),
    usage: wgpu::BufferUsages::VERTEX,
  });

  // -------------------------------------
  // Create Pipelines
  // -------------------------------------
  let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: Some("Shader Module"),
    source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
  });

  let circle_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    label: Some("Circle Pipeline Layout"),
    bind_group_layouts: &[&uniform_bind_group_layout, &bind_group_layout],
    push_constant_ranges: &[],
  });

  let circle_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("Circle Pipeline"),
    layout: Some(&circle_pipeline_layout),
    vertex: wgpu::VertexState {
      module: &shader,
      entry_point: "vs_main",
      buffers: &[wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[wgpu::VertexAttribute {
          offset: 0,
          shader_location: 0,
          format: wgpu::VertexFormat::Float32x3,
        }],
      }],
    },
    fragment: Some(wgpu::FragmentState {
      module: &shader,
      entry_point: "fs_main",
      targets: &[Some(wgpu::ColorTargetState {
        format: surface_format,
        blend: Some(wgpu::BlendState::REPLACE),
        write_mask: wgpu::ColorWrites::ALL,
      })],
    }),
    primitive: wgpu::PrimitiveState {
      topology: wgpu::PrimitiveTopology::LineStrip,
      strip_index_format: Some(wgpu::IndexFormat::Uint32),
      front_face: wgpu::FrontFace::Ccw,
      cull_mode: None,
      polygon_mode: wgpu::PolygonMode::Fill,
      unclipped_depth: false,
      conservative: false,
    },
    depth_stencil: None,
    multisample: wgpu::MultisampleState::default(),
    multiview: None,
  });

  let triangle_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    label: Some("Triangle Pipeline Layout"),
    bind_group_layouts: &[&uniform_bind_group_layout, &bind_group_layout],
    push_constant_ranges: &[],
  });

  let triangle_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("Triangle Pipeline"),
    layout: Some(&triangle_pipeline_layout),
    vertex: wgpu::VertexState {
      module: &shader,
      entry_point: "vs_main",
      buffers: &[wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[wgpu::VertexAttribute {
          offset: 0,
          shader_location: 0,
          format: wgpu::VertexFormat::Float32x3,
        }],
      }],
    },
    fragment: Some(wgpu::FragmentState {
      module: &shader,
      entry_point: "fs_main",
      targets: &[Some(wgpu::ColorTargetState {
        format: surface_format,
        blend: Some(wgpu::BlendState::REPLACE),
        write_mask: wgpu::ColorWrites::ALL,
      })],
    }),
    primitive: wgpu::PrimitiveState {
      topology: wgpu::PrimitiveTopology::TriangleList,
      strip_index_format: None,
      front_face: wgpu::FrontFace::Ccw,
      cull_mode: None,
      polygon_mode: wgpu::PolygonMode::Line,
      unclipped_depth: false,
      conservative: false,
    },
    depth_stencil: None,
    multisample: wgpu::MultisampleState::default(),
    multiview: None,
  });

  // Event loop would go here for rendering shapes...
  let mut shapes: Vec<Shape> = vec![
    Shape::new_circle(&device, &bind_group_layout, glam::vec3(-0.5, 0.0, 0.0), 1.3),
    Shape::new_triangle(&device, &bind_group_layout, glam::vec3(0.5, 0.5, 0.0), 0.2),
    Shape::new_triangle(&device, &bind_group_layout, glam::vec3(0.2, 0.2, 0.0), 0.4),
  ];

  event_loop.run(move |event, _, control_flow| {
    *control_flow = ControlFlow::Poll;

    match event {
      Event::RedrawRequested(_) => {
        let frame = match surface.get_current_texture() {
          Ok(frame) => frame,
          Err(_) => {
            surface.configure(&device, &surface_config);
            return;
          }
        };
        let view = frame
          .texture
          .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
          label: Some("Render Encoder"),
        });

        // Update aspect ratio in the uniform buffer if window size changes
        let new_aspect_ratio = size.width as f32 / size.height as f32;
        if (new_aspect_ratio - uniforms.aspect_ratio).abs() > f32::EPSILON {
          uniforms.aspect_ratio = new_aspect_ratio;
          queue.write_buffer(&uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
        }

        {
          let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
              view: &view,
              resolve_target: None,
              ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                  r: 0.1,
                  g: 0.1,
                  b: 0.1,
                  a: 1.0,
                }),
                store: true,
              },
            })],
            depth_stencil_attachment: None,
          });

          for shape in &shapes {
            match shape.shape_type {
              ShapeType::Circle => {
                render_pass.set_pipeline(&circle_pipeline);
                render_pass.set_bind_group(0, &uniform_bind_group, &[]);
                render_pass.set_bind_group(1, &shape.bind_group, &[]);
                render_pass.set_vertex_buffer(0, circle_vertex_buffer.slice(..));
                render_pass
                  .set_index_buffer(circle_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..circle_index_count, 0, 0..1);
              }
              ShapeType::Triangle => {
                render_pass.set_pipeline(&triangle_pipeline);
                render_pass.set_bind_group(0, &uniform_bind_group, &[]);
                render_pass.set_bind_group(1, &shape.bind_group, &[]);
                render_pass.set_vertex_buffer(0, triangle_vertex_buffer.slice(..));
                render_pass.draw(0..3, 0..1);
              }
            }
          }
        }

        queue.submit(Some(encoder.finish()));
        frame.present();
      }
      Event::WindowEvent {
        event: WindowEvent::CloseRequested,
        ..
      } => {
        *control_flow = ControlFlow::Exit;
      }
      Event::WindowEvent {
        event: WindowEvent::Resized(new_size),
        ..
      } => {
        surface_config.width = new_size.width;
        surface_config.height = new_size.height;
        surface.configure(&device, &surface_config);

        // Update aspect ratio
        uniforms.aspect_ratio = new_size.width as f32 / new_size.height as f32;
        queue.write_buffer(&uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
      }
      Event::MainEventsCleared => {
        window.request_redraw();
      }
      _ => {}
    }
  });
}
fn generate_circle_vertices(radius: f32, segments: usize) -> Vec<f32> {
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

fn generate_circle_indices(segments: usize) -> Vec<u32> {
  let mut indices = Vec::with_capacity(segments + 1);
  for i in 0..segments as u32 {
    indices.push(i);
  }
  indices.push(0);
  indices
}
