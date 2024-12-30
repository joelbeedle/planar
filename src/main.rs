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
  aspect_ratio: f32, // aspect = width / height
}

// Generate circle vertex positions in X/Y plane, centered at (0,0).
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

// Index buffer for a line strip: 0,1,2,...,N-1,0
fn generate_circle_indices(segments: usize) -> Vec<u32> {
  let mut indices = Vec::with_capacity(segments + 1);
  for i in 0..segments as u32 {
    indices.push(i);
  }
  indices.push(0);
  indices
}

#[tokio::main]
async fn main() {
  // -------------------------------------
  // 1) Create window + wgpu instance
  // -------------------------------------
  let event_loop = EventLoop::new();
  let window = WindowBuilder::new()
    .with_title("Circle + Triangle (Aspect Ratio, Real Line Topology)")
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
        // We only need POLYGON_MODE_LINE for the triangle wireframe
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
  // 2) Prepare vertex + index data
  // -------------------------------------
  // Circle
  let circle_segments = 64;
  let circle_vertex_data = generate_circle_vertices(0.6, circle_segments);
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

  // Triangle (3 positions, each is [x,y,z])
  let triangle_vertex_data: &[f32] = &[
    // x,    y,    z
    0.0, 0.5, 0.0, // top
    -0.5, -0.5, 0.0, // bottom-left
    0.5, -0.5, 0.0, // bottom-right
  ];
  let triangle_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
    label: Some("Triangle VB"),
    contents: bytemuck::cast_slice(triangle_vertex_data),
    usage: wgpu::BufferUsages::VERTEX,
  });

  // -------------------------------------
  // 3) Uniform buffer for aspect ratio
  // -------------------------------------
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

  // -------------------------------------
  // 4) Shader source
  // -------------------------------------
  let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: Some("Shader Module"),
    source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
  });

  // 1) Define VertexAttribute array as a 'static
  static VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 1] = [wgpu::VertexAttribute {
    offset: 0,
    shader_location: 0,
    format: wgpu::VertexFormat::Float32x3,
  }];

  // 2) A small function returning a one-element array of
  //    wgpu::VertexBufferLayout<'static> so each pipeline call can use it
  fn single_layout() -> [wgpu::VertexBufferLayout<'static>; 1] {
    [wgpu::VertexBufferLayout {
      array_stride: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
      step_mode: wgpu::VertexStepMode::Vertex,
      attributes: &VERTEX_ATTRIBUTES, // <-- references the static
    }]
  }
  // -------------------------------------
  // 5) Pipelines
  // -------------------------------------

  // A) Pipeline for the circle: real line topology
  let circle_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    label: Some("Circle Pipeline Layout"),
    bind_group_layouts: &[&uniform_bind_group_layout],
    push_constant_ranges: &[],
  });

  let circle_pipeline = {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: Some("Circle Pipeline"),
      layout: Some(&circle_pipeline_layout),
      vertex: wgpu::VertexState {
        module: &module,
        entry_point: "vs_main",
        buffers: &single_layout(),
      },
      fragment: Some(wgpu::FragmentState {
        module: &module,
        entry_point: "fs_main",
        targets: &[Some(wgpu::ColorTargetState {
          format: surface_format,
          blend: Some(wgpu::BlendState::REPLACE),
          write_mask: wgpu::ColorWrites::ALL,
        })],
      }),
      primitive: wgpu::PrimitiveState {
        topology: wgpu::PrimitiveTopology::LineStrip, // real line topology
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
    })
  };

  // B) Pipeline for the triangle: wireframe
  let triangle_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    label: Some("Triangle Pipeline Layout"),
    bind_group_layouts: &[&uniform_bind_group_layout],
    push_constant_ranges: &[],
  });

  let triangle_pipeline = {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: Some("Triangle Pipeline"),
      layout: Some(&triangle_pipeline_layout),
      vertex: wgpu::VertexState {
        module: &module,
        entry_point: "vs_main",
        buffers: &single_layout(),
      },
      fragment: Some(wgpu::FragmentState {
        module: &module,
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
        polygon_mode: wgpu::PolygonMode::Line, // wireframe
        unclipped_depth: false,
        conservative: false,
      },
      depth_stencil: None,
      multisample: wgpu::MultisampleState::default(),
      multiview: None,
    })
  };

  // -------------------------------------
  // 6) Run event loop
  // -------------------------------------
  event_loop.run(move |event, _, control_flow| {
    *control_flow = ControlFlow::Poll;

    match event {
      Event::RedrawRequested(_) => {
        let size = window.inner_size();
        if size.width > 0 && size.height > 0 {
          // Re-check aspect ratio in case resized:
          let new_aspect = size.width as f32 / size.height as f32;
          if (new_aspect - uniforms.aspect_ratio).abs() > f32::EPSILON {
            uniforms.aspect_ratio = new_aspect;
            queue.write_buffer(&uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
          }

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

            // 1) Draw the circle as a line strip
            render_pass.set_pipeline(&circle_pipeline);
            render_pass.set_bind_group(0, &uniform_bind_group, &[]);
            render_pass.set_vertex_buffer(0, circle_vertex_buffer.slice(..));
            render_pass.set_index_buffer(circle_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            // circle_index_count includes the repeated '0' at the end
            render_pass.draw_indexed(0..circle_index_count, 0, 0..1);

            // 2) Draw the triangle in wireframe
            render_pass.set_pipeline(&triangle_pipeline);
            render_pass.set_bind_group(0, &uniform_bind_group, &[]);
            render_pass.set_vertex_buffer(0, triangle_vertex_buffer.slice(..));
            render_pass.draw(0..3, 0..1);
          }

          queue.submit(Some(encoder.finish()));
          frame.present();
        }
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
        // Reconfigure surface on resize
        surface_config.width = new_size.width;
        surface_config.height = new_size.height;
        surface.configure(&device, &surface_config);
      }

      Event::MainEventsCleared => {
        // Request redraw
        window.request_redraw();
      }

      _ => {}
    }
  });
}
