use crate::camera::{Camera, CameraUniform};
use crate::math_util::{portal_link_matrix, Transform};
use crate::portal::Portal;
use crate::world::{Mesh, Vertex, World, Vertex2D};
use glam::{Mat4, Vec4};
use wgpu::util::DeviceExt;

pub const MAX_PORTAL_DEPTH: u32 = 2;

pub struct Pipelines {
    pub world: wgpu::RenderPipeline,
    pub world_stencil_test: wgpu::RenderPipeline,
    pub stencil_inc: wgpu::RenderPipeline,
    pub stencil_dec: wgpu::RenderPipeline,
    pub depth_clear: wgpu::RenderPipeline,
    pub portal_overlay: wgpu::RenderPipeline,
    pub ui_pipeline: wgpu::RenderPipeline,
}

pub struct UploadedMesh {
    pub vertex: wgpu::Buffer,
    pub index: wgpu::Buffer,
    pub num_indices: u32,
}

pub struct Renderer {
    pub surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub pipelines: Pipelines,
    pub depth_format: wgpu::TextureFormat,
    pub stencil_texture: wgpu::TextureView,
    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,
    pub camera_bind_group_layout: wgpu::BindGroupLayout,
    pub scene_meshes: Vec<UploadedMesh>,
    pub portal_meshes: Vec<UploadedMesh>,
    pub cube_mesh: Option<UploadedMesh>,
    pub projectile_meshes: Vec<UploadedMesh>,
    pub particle_meshes: Vec<UploadedMesh>,
    pub tracer_meshes: Vec<UploadedMesh>,
    pub platform_meshes: Vec<UploadedMesh>,
    pub hazard_meshes: Vec<UploadedMesh>,
    pub exit_pad_mesh: Option<UploadedMesh>,
    pub breakable_meshes: Vec<UploadedMesh>,
    pub light_strip_meshes: Vec<UploadedMesh>,
    pub fizzler_meshes: Vec<UploadedMesh>,
    pub pellet_meshes: Vec<UploadedMesh>,
    pub faith_plate_meshes: Vec<UploadedMesh>,
    pub laser_meshes: Vec<UploadedMesh>,
    pub turret_meshes: Vec<UploadedMesh>,
    pub bridge_meshes: Vec<UploadedMesh>,
    pub gel_meshes: Vec<UploadedMesh>,
    pub ghost_mesh: Option<UploadedMesh>,
    pub crosshair_mesh: Option<UploadedMesh>,
    pub crosshair_color: [f32; 3],
    pub win_screen_visible: bool,
    pub show_debug: bool,
    pub debug_speed: f32,
    pub debug_fps: f32,
    pub start_time: std::time::Instant,
}

impl Renderer {
    pub async fn new(window: &winit::window::Window) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::default();
        let surface = unsafe { instance.create_surface(&window) }.unwrap();
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }).await.unwrap();

        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            features: wgpu::Features::empty(),
            limits: wgpu::Limits::default(),
            label: None,
        }, None).await.unwrap();

        let format = surface.get_capabilities(&adapter).formats[0];
        let depth_format = wgpu::TextureFormat::Depth32FloatStencil8;
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format, width: size.width, height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto, view_formats: vec![], 
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shader"), source: wgpu::ShaderSource::Wgsl(SHADER_SRC.into()),
        });

        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0, visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            }],
            label: Some("camera_bgl"),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("layout"), bind_group_layouts: &[&camera_bind_group_layout], push_constant_ranges: &[],
        });

        let stencil_default = wgpu::StencilFaceState { compare: wgpu::CompareFunction::Always, fail_op: wgpu::StencilOperation::Keep, depth_fail_op: wgpu::StencilOperation::Keep, pass_op: wgpu::StencilOperation::Keep };
        let stencil_inc = wgpu::StencilFaceState { compare: wgpu::CompareFunction::Equal, fail_op: wgpu::StencilOperation::Keep, depth_fail_op: wgpu::StencilOperation::Keep, pass_op: wgpu::StencilOperation::IncrementClamp };
        let stencil_dec = wgpu::StencilFaceState { compare: wgpu::CompareFunction::Equal, fail_op: wgpu::StencilOperation::Keep, depth_fail_op: wgpu::StencilOperation::Keep, pass_op: wgpu::StencilOperation::DecrementClamp };
        let stencil_equal = wgpu::StencilFaceState { compare: wgpu::CompareFunction::Equal, fail_op: wgpu::StencilOperation::Keep, depth_fail_op: wgpu::StencilOperation::Keep, pass_op: wgpu::StencilOperation::Keep };

        let world_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("world_pipeline"), layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: "vs_main", buffers: &[Vertex::LAYOUT] },
            fragment: Some(wgpu::FragmentState { module: &shader, entry_point: "fs_main", targets: &[Some(wgpu::ColorTargetState { format, blend: Some(wgpu::BlendState::REPLACE), write_mask: wgpu::ColorWrites::ALL })] }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState { format: depth_format, depth_write_enabled: true, depth_compare: wgpu::CompareFunction::Less, stencil: wgpu::StencilState { front: stencil_default, back: stencil_default, read_mask: 0xFF, write_mask: 0xFF }, bias: wgpu::DepthBiasState::default() }),
            multisample: wgpu::MultisampleState::default(), multiview: None,
        });

        let world_stencil_test_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("world_stencil_pipeline"), layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: "vs_main", buffers: &[Vertex::LAYOUT] },
            fragment: Some(wgpu::FragmentState { module: &shader, entry_point: "fs_main", targets: &[Some(wgpu::ColorTargetState { format, blend: Some(wgpu::BlendState::REPLACE), write_mask: wgpu::ColorWrites::ALL })] }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState { format: depth_format, depth_write_enabled: true, depth_compare: wgpu::CompareFunction::Less, stencil: wgpu::StencilState { front: stencil_equal, back: stencil_equal, read_mask: 0xFF, write_mask: 0xFF }, bias: wgpu::DepthBiasState::default() }),
            multisample: wgpu::MultisampleState::default(), multiview: None,
        });

        let stencil_inc_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("stencil_inc_pipeline"), layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: "vs_main", buffers: &[Vertex::LAYOUT] },
            fragment: None,
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState { format: depth_format, depth_write_enabled: false, depth_compare: wgpu::CompareFunction::Less, stencil: wgpu::StencilState { front: stencil_inc, back: stencil_inc, read_mask: 0xFF, write_mask: 0xFF }, bias: wgpu::DepthBiasState::default() }),
            multisample: wgpu::MultisampleState::default(), multiview: None,
        });

        let stencil_dec_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("stencil_dec_pipeline"), layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: "vs_main", buffers: &[Vertex::LAYOUT] },
            fragment: None,
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState { format: depth_format, depth_write_enabled: false, depth_compare: wgpu::CompareFunction::Always, stencil: wgpu::StencilState { front: stencil_dec, back: stencil_dec, read_mask: 0xFF, write_mask: 0xFF }, bias: wgpu::DepthBiasState::default() }),
            multisample: wgpu::MultisampleState::default(), multiview: None,
        });

        let depth_clear_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("depth_clear_pipeline"), layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: "vs_fullscreen", buffers: &[] },
            fragment: None,
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState { format: depth_format, depth_write_enabled: true, depth_compare: wgpu::CompareFunction::Always, stencil: wgpu::StencilState { front: stencil_equal, back: stencil_equal, read_mask: 0xFF, write_mask: 0xFF }, bias: wgpu::DepthBiasState::default() }),
            multisample: wgpu::MultisampleState::default(), multiview: None,
        });

        let portal_overlay_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("portal_overlay"), layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: "vs_main", buffers: &[Vertex::LAYOUT] },
            fragment: Some(wgpu::FragmentState { module: &shader, entry_point: "fs_overlay", targets: &[Some(wgpu::ColorTargetState { format, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })] }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState { format: depth_format, depth_write_enabled: false, depth_compare: wgpu::CompareFunction::Always, stencil: wgpu::StencilState { front: stencil_default, back: stencil_default, read_mask: 0xFF, write_mask: 0xFF }, bias: wgpu::DepthBiasState::default() }),
            multisample: wgpu::MultisampleState::default(), multiview: None,
        });

        let ui_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ui_pipeline"), layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: "vs_ui", buffers: &[Vertex2D::LAYOUT] },
            fragment: Some(wgpu::FragmentState { module: &shader, entry_point: "fs_ui", targets: &[Some(wgpu::ColorTargetState { format, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })] }),
            primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::LineList, ..Default::default() },
            depth_stencil: None, multisample: wgpu::MultisampleState::default(), multiview: None,
        });

        let pipelines = Pipelines {
            world: world_pipeline, world_stencil_test: world_stencil_test_pipeline,
            stencil_inc: stencil_inc_pipeline, stencil_dec: stencil_dec_pipeline,
            depth_clear: depth_clear_pipeline, portal_overlay: portal_overlay_pipeline, ui_pipeline
        };

        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera_buffer"), size: std::mem::size_of::<CameraUniform>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera_bg"), layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: camera_buffer.as_entire_binding() }],
        });

        let stencil_texture = Self::make_depth_texture(&device, &config, depth_format);

        let ch_verts = [Vertex2D { pos: [-0.02, 0.0] }, Vertex2D { pos: [0.02, 0.0] }, Vertex2D { pos: [0.0, -0.02] }, Vertex2D { pos: [0.0, 0.02] }];
        let cv = device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("ch_vbuf"), contents: bytemuck::cast_slice(&ch_verts), usage: wgpu::BufferUsages::VERTEX });
        let ci = device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("ch_ibuf"), contents: bytemuck::cast_slice(&[0u16, 1, 2, 3]), usage: wgpu::BufferUsages::INDEX });
        let crosshair_mesh = Some(UploadedMesh { vertex: cv, index: ci, num_indices: 4 });

        Self {
            surface, device, queue, config, size, pipelines, depth_format, stencil_texture,
            camera_buffer, camera_bind_group, camera_bind_group_layout,
            scene_meshes: Vec::new(), portal_meshes: Vec::new(), cube_mesh: None,
            projectile_meshes: Vec::new(), particle_meshes: Vec::new(), tracer_meshes: Vec::new(),
            platform_meshes: Vec::new(), hazard_meshes: Vec::new(), exit_pad_mesh: None,
            breakable_meshes: Vec::new(), light_strip_meshes: Vec::new(), fizzler_meshes: Vec::new(),
            pellet_meshes: Vec::new(), faith_plate_meshes: Vec::new(), laser_meshes: Vec::new(),
            turret_meshes: Vec::new(), bridge_meshes: Vec::new(), gel_meshes: Vec::new(),
            ghost_mesh: None, crosshair_mesh, crosshair_color: [1.0, 1.0, 1.0],
            win_screen_visible: false, show_debug: false, debug_speed: 0.0, debug_fps: 0.0,
            start_time: std::time::Instant::now(),
        }
    }

    fn make_depth_texture(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, format: wgpu::TextureFormat) -> wgpu::TextureView {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth/stencil"),
            size: wgpu::Extent3d { width: config.width, height: config.height, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT, view_formats: &[],
        }).create_view(&wgpu::TextureViewDescriptor::default())
    }

    pub fn upload_world(&mut self, world: &World) {
        self.scene_meshes.clear();
        for m in &world.meshes { self.scene_meshes.push(self.upload_mesh(m)); }
        self.upload_portals(world);
        
        let cube_mesh = Mesh::cube(glam::Vec3::ZERO, 1.0, [0.8, 0.6, 0.2]);
        let cube_vertex = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cube_vbuf"), size: std::mem::size_of::<Vertex>() as wgpu::BufferAddress * cube_mesh.vertices.len() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false
        });
        let cube_index = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cube_ibuf"), contents: bytemuck::cast_slice(&cube_mesh.indices), usage: wgpu::BufferUsages::INDEX,
        });
        self.cube_mesh = Some(UploadedMesh { vertex: cube_vertex, index: cube_index, num_indices: cube_mesh.indices.len() as u32 });
        
        let mesh = Mesh::cube(world.exit_pad.pos, world.exit_pad.half_size * 2.0, [0.2, 0.8, 0.4]);
        self.exit_pad_mesh = Some(self.upload_mesh(&mesh));
    }

    pub fn upload_portals(&mut self, world: &World) {
        self.portal_meshes.clear();
        for p in &world.portals {
            let corners = p.corners();
            let ring = Mesh::portal_ring(corners, p.transform.forward(), p.color);
            self.portal_meshes.push(self.upload_mesh(&ring));
        }
    }

    fn upload_mesh(&self, mesh: &Mesh) -> UploadedMesh {
        let vertex = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vbuf"), contents: bytemuck::cast_slice(&mesh.vertices), usage: wgpu::BufferUsages::VERTEX,
        });
        let index = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ibuf"), contents: bytemuck::cast_slice(&mesh.indices), usage: wgpu::BufferUsages::INDEX,
        });
        UploadedMesh { vertex, index, num_indices: mesh.indices.len() as u32 }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 { return; }
        self.size = new_size;
        self.config.width = new_size.width; self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
        self.stencil_texture = Self::make_depth_texture(&self.device, &self.config, self.depth_format);
    }

    pub fn update_cube(&mut self, cube: &crate::cube::Cube) {
        if let Some(uploaded) = &self.cube_mesh {
            let mesh = Mesh::cube(cube.position, cube.half_size * 2.0, [0.8, 0.6, 0.2]);
            self.queue.write_buffer(&uploaded.vertex, 0, bytemuck::cast_slice(&mesh.vertices));
        }
    }

    pub fn update_projectiles(&mut self, projectiles: &[crate::projectile::Projectile]) {
        self.projectile_meshes.clear();
        for p in projectiles {
            let color = if p.is_blue { [0.3, 0.7, 1.0] } else { [1.0, 0.5, 0.3] };
            self.projectile_meshes.push(self.upload_mesh(&Mesh::cube(p.pos, 0.2, color)));
            let dir = p.vel.normalize_or_zero();
            for i in 1..=3 {
                let trail_pos = p.pos - dir * (i as f32 * 0.3);
                let trail_size = 0.2 - (i as f32 * 0.05);
                let trail_color = [color[0] * (1.0 - i as f32 * 0.3), color[1] * (1.0 - i as f32 * 0.3), color[2] * (1.0 - i as f32 * 0.3)];
                self.projectile_meshes.push(self.upload_mesh(&Mesh::cube(trail_pos, trail_size.max(0.05), trail_color)));
            }
        }
    }

    pub fn update_particles(&mut self, particles: &[crate::particle::Particle]) {
        self.particle_meshes.clear();
        for p in particles {
            let life_t = p.life / p.max_life;
            let size = 0.1 * life_t;
            if size > 0.01 { self.particle_meshes.push(self.upload_mesh(&Mesh::cube(p.pos, size, p.color))); }
        }
    }

    pub fn update_tracers(&mut self, tracers: &[(glam::Vec3, glam::Vec3, [f32; 3])]) {
        self.tracer_meshes.clear();
        for (start, end, color) in tracers {
            self.tracer_meshes.push(self.upload_mesh(&Mesh::cube(*end, 0.1, *color)));
        }
    }

    pub fn update_platforms(&mut self, platforms: &[crate::level::Platform], exit_pad: &crate::level::ExitPad) {
        self.platform_meshes.clear();
        for p in platforms { self.platform_meshes.push(self.upload_mesh(&Mesh::cuboid(p.pos, p.half_size, [0.5, 0.5, 0.6]))); }
        self.exit_pad_mesh = Some(self.upload_mesh(&Mesh::cube(exit_pad.pos, exit_pad.half_size * 2.0, [0.2, 0.8, 0.4])));
    }

    pub fn update_hazards(&mut self, hazards: &[crate::level::Hazard]) {
        self.hazard_meshes.clear();
        for h in hazards { self.hazard_meshes.push(self.upload_mesh(&Mesh::cuboid(h.pos, h.half_size, [0.8, 0.2, 0.2]))); }
    }

    pub fn update_dynamic_level_meshes(&mut self, world: &World) {
        self.breakable_meshes.clear();
        for b in &world.breakables {
            if !b.broken {
                let crack_intensity = 1.0 - (b.hp / 30.0).min(1.0);
                let color = [0.5 + crack_intensity * 0.5, 0.5, 0.6];
                self.breakable_meshes.push(self.upload_mesh(&Mesh::cuboid(b.pos, b.half_size, color)));
            }
        }
        self.light_strip_meshes.clear();
        for l in &world.light_strips { self.light_strip_meshes.push(self.upload_mesh(&Mesh::cuboid(l.pos, l.half_size, l.color))); }
        self.fizzler_meshes.clear();
        for f in &world.fizzlers { self.fizzler_meshes.push(self.upload_mesh(&Mesh::cuboid(f.pos, f.half_size, [0.2, 0.5, 1.0]))); }
    }

    pub fn update_pellets(&mut self, pellets: &[crate::pellet::Pellet]) {
        self.pellet_meshes.clear();
        for p in pellets {
            let life_t = p.life / p.max_life;
            self.pellet_meshes.push(self.upload_mesh(&Mesh::cube(p.pos, 0.3 * life_t, [1.0, 0.2, 0.8])));
        }
    }

    pub fn update_faith_plates(&mut self, plates: &[crate::level::FaithPlate]) {
        self.faith_plate_meshes.clear();
        for fp in plates { self.faith_plate_meshes.push(self.upload_mesh(&Mesh::cuboid(fp.pos, fp.half_size, [0.8, 0.6, 0.2]))); }
    }

    pub fn update_lasers(&mut self, segments: &[crate::laser::LaserSegment]) {
        self.laser_meshes.clear();
        for seg in segments {
            let dir = seg.end - seg.start;
            let len = dir.length();
            if len < 0.1 { continue; }
            let steps = (len / 0.5).ceil() as i32;
            for i in 0..steps {
                let pos = seg.start + dir * (i as f32 / steps as f32);
                self.laser_meshes.push(self.upload_mesh(&Mesh::cube(pos, 0.05, [1.0, 0.0, 0.0])));
            }
        }
    }

    pub fn update_turrets(&mut self, turrets: &[crate::turret::Turret]) {
        self.turret_meshes.clear();
        for t in turrets {
            let color = if t.active { [0.9, 0.9, 0.9] } else { [0.3, 0.3, 0.3] };
            self.turret_meshes.push(self.upload_mesh(&Mesh::cube(t.pos, 1.0, color)));
        }
    }

    pub fn update_bridges(&mut self, tiles: &[glam::Vec3]) {
        self.bridge_meshes.clear();
        for tile in tiles { self.bridge_meshes.push(self.upload_mesh(&Mesh::cube(*tile, 2.0, [0.2, 0.5, 1.0]))); }
    }

    pub fn update_gels(&mut self, gels: &[crate::gel::GelPatch]) {
        self.gel_meshes.clear();
        for gel in gels {
            let color = match gel.gel_type {
                crate::gel::GelType::Repulsion => [0.2, 0.4, 1.0],
                crate::gel::GelType::Propulsion => [1.0, 0.5, 0.1],
            };
            self.gel_meshes.push(self.upload_mesh(&Mesh::cuboid(gel.pos, gel.half_size, color)));
        }
    }

    pub fn update_ghost_portal(&mut self, ghost: &Option<(glam::Vec3, glam::Vec3, glam::Vec3, bool)>) {
        if let Some((pos, normal, up, is_valid)) = ghost {
            let color = if *is_valid { [1.0, 1.0, 1.0] } else { [1.0, 0.2, 0.2] };
            let portal = crate::portal::Portal::new(*pos, *normal, *up, 0.9, 1.2);
            let corners = portal.corners();
            let mesh = Mesh::portal_ring(corners, portal.transform.forward(), color);
            self.ghost_mesh = Some(self.upload_mesh(&mesh));
        } else { self.ghost_mesh = None; }
    }

    pub fn update_crosshair(&mut self, color: [f32; 3]) { self.crosshair_color = color; }
    pub fn update_debug(&mut self, show: bool, speed: f32, fps: f32) {
        self.show_debug = show; self.debug_speed = speed; self.debug_fps = fps;
    }

    pub fn render(&mut self, world: &World, camera: &Camera) -> Result<(), wgpu::SurfaceError> {
        let frame = self.surface.get_current_texture()?;
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let time = self.start_time.elapsed().as_secs_f32();

        let cu = CameraUniform::from(camera, time);
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[cu]));
        let mut enc = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut rpass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main_scene"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view, resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.02, g: 0.02, b: 0.03, a: 1.0 }), store: true },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.stencil_texture,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: true }),
                    stencil_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(0), store: true }),
                }),
            });
            rpass.set_pipeline(&self.pipelines.world);
            rpass.set_bind_group(0, &self.camera_bind_group, &[]);
            self.draw_scene_meshes(&mut rpass);
        }
        self.queue.submit(Some(enc.finish()));

        for (i, _) in world.portals.iter().enumerate() {
            if let Some(linked) = world.portals[i].linked {
                self.render_portal_recursive(world, camera, i, linked, &view, 1);
            }
        }

        let cu = CameraUniform::from(camera, time);
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[cu]));
        let mut enc = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut rpass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("overlay"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view, resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: true },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.stencil_texture,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Load, store: true }),
                    stencil_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Load, store: true }),
                }),
            });
            rpass.set_pipeline(&self.pipelines.portal_overlay);
            rpass.set_bind_group(0, &self.camera_bind_group, &[]);
            rpass.set_stencil_reference(0);
            for m in &self.portal_meshes {
                rpass.set_vertex_buffer(0, m.vertex.slice(..));
                rpass.set_index_buffer(m.index.slice(..), wgpu::IndexFormat::Uint16);
                rpass.draw_indexed(0..m.num_indices, 0, 0..1);
            }
        }
        
        {
            let mut rpass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ui_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view, resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: true },
                })],
                depth_stencil_attachment: None,
            });
            rpass.set_pipeline(&self.pipelines.ui_pipeline);
            rpass.set_bind_group(0, &self.camera_bind_group, &[]);
            if let Some(ch) = &self.crosshair_mesh {
                rpass.set_vertex_buffer(0, ch.vertex.slice(..));
                rpass.set_index_buffer(ch.index.slice(..), wgpu::IndexFormat::Uint16);
                rpass.draw_indexed(0..ch.num_indices, 0, 0..1);
            }
        }
        self.queue.submit(Some(enc.finish()));

        if self.win_screen_visible {
            let mut enc = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            {
                let mut rpass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("win_screen"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view, resolve_target: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.5, b: 0.0, a: 1.0 }), store: true },
                    })],
                    depth_stencil_attachment: None,
                });
                let _ = rpass;
            }
            self.queue.submit(Some(enc.finish()));
        }

        frame.present();
        Ok(())
    }

    fn render_portal_recursive(&mut self, world: &World, camera: &Camera, portal_idx: usize, target_idx: usize, view: &wgpu::TextureView, depth: u32) {
        let target_portal = &world.portals[target_idx];
        let source_portal = &world.portals[portal_idx];
        if !camera.is_in_view(source_portal.transform.position, 2.0) { return; }

        if depth > MAX_PORTAL_DEPTH {
            let cu = CameraUniform::from(camera, 0.0);
            self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[cu]));
            let mut enc = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            {
                let mut rpass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("portal_void"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view, resolve_target: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: true },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.stencil_texture,
                        depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Load, store: true }),
                        stencil_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Load, store: true }),
                    }),
                });
                rpass.set_pipeline(&self.pipelines.world_stencil_test);
                rpass.set_stencil_reference(depth);
                let m = &self.portal_meshes[portal_idx];
                rpass.set_vertex_buffer(0, m.vertex.slice(..));
                rpass.set_index_buffer(m.index.slice(..), wgpu::IndexFormat::Uint16);
                rpass.draw_indexed(0..m.num_indices, 0, 0..1);
            }
            self.queue.submit(Some(enc.finish()));
            return;
        }

        let portal = &world.portals[portal_idx];
        let link = portal_link_matrix(&portal.transform, portal.scale, &target_portal.transform, target_portal.scale);
        let virtual_cam = camera.virtual_through(link, &target_portal.transform);

        let cu = CameraUniform::from(camera, 0.0);
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[cu]));
        let mut enc = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut rpass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("stencil_inc"),
                color_attachments: &[None],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.stencil_texture,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Load, store: true }),
                    stencil_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Load, store: true }),
                }),
            });
            rpass.set_pipeline(&self.pipelines.stencil_inc);
            rpass.set_bind_group(0, &self.camera_bind_group, &[]);
            rpass.set_stencil_reference(depth - 1);
            let m = &self.portal_meshes[portal_idx];
            rpass.set_vertex_buffer(0, m.vertex.slice(..));
            rpass.set_index_buffer(m.index.slice(..), wgpu::IndexFormat::Uint16);
            rpass.draw_indexed(0..m.num_indices, 0, 0..1);
        }
        self.queue.submit(Some(enc.finish()));

        let mut enc = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut rpass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("depth_clear"),
                color_attachments: &[None],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.stencil_texture,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Load, store: true }),
                    stencil_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Load, store: true }),
                }),
            });
            rpass.set_pipeline(&self.pipelines.depth_clear);
            rpass.set_stencil_reference(depth);
            rpass.draw(0..3, 0..1);
        }
        self.queue.submit(Some(enc.finish()));

        let cu = CameraUniform::from(&virtual_cam, 0.0);
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[cu]));
        let mut enc = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut rpass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("virtual_scene"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view, resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: true },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.stencil_texture,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Load, store: true }),
                    stencil_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Load, store: true }),
                }),
            });
            rpass.set_pipeline(&self.pipelines.world_stencil_test);
            rpass.set_bind_group(0, &self.camera_bind_group, &[]);
            rpass.set_stencil_reference(depth);
            self.draw_scene_meshes(&mut rpass);
        }
        self.queue.submit(Some(enc.finish()));

        if depth < MAX_PORTAL_DEPTH {
            for (j, _) in world.portals.iter().enumerate() {
                if j == portal_idx { continue; }
                if let Some(nl) = world.portals[j].linked {
                    self.render_portal_recursive(world, &virtual_cam, j, nl, view, depth + 1);
                }
            }
        }

        let cu = CameraUniform::from(camera, 0.0);
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[cu]));
        let mut enc = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut rpass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("stencil_dec"),
                color_attachments: &[None],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.stencil_texture,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Load, store: true }),
                    stencil_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Load, store: true }),
                }),
            });
            rpass.set_pipeline(&self.pipelines.stencil_dec);
            rpass.set_bind_group(0, &self.camera_bind_group, &[]);
            rpass.set_stencil_reference(depth);
            let m = &self.portal_meshes[portal_idx];
            rpass.set_vertex_buffer(0, m.vertex.slice(..));
            rpass.set_index_buffer(m.index.slice(..), wgpu::IndexFormat::Uint16);
            rpass.draw_indexed(0..m.num_indices, 0, 0..1);
        }
        self.queue.submit(Some(enc.finish()));
    }

    fn draw_scene_meshes(&self, rpass: &mut wgpu::RenderPass) {
        for m in &self.scene_meshes {
            rpass.set_vertex_buffer(0, m.vertex.slice(..));
            rpass.set_index_buffer(m.index.slice(..), wgpu::IndexFormat::Uint16);
            rpass.draw_indexed(0..m.num_indices, 0, 0..1);
        }
        if let Some(cube) = &self.cube_mesh {
            rpass.set_vertex_buffer(0, cube.vertex.slice(..));
            rpass.set_index_buffer(cube.index.slice(..), wgpu::IndexFormat::Uint16);
            rpass.draw_indexed(0..cube.num_indices, 0, 0..1);
        }
        for m in &self.projectile_meshes { rpass.set_vertex_buffer(0, m.vertex.slice(..)); rpass.set_index_buffer(m.index.slice(..), wgpu::IndexFormat::Uint16); rpass.draw_indexed(0..m.num_indices, 0, 0..1); }
        for m in &self.particle_meshes { rpass.set_vertex_buffer(0, m.vertex.slice(..)); rpass.set_index_buffer(m.index.slice(..), wgpu::IndexFormat::Uint16); rpass.draw_indexed(0..m.num_indices, 0, 0..1); }
        for m in &self.tracer_meshes { rpass.set_vertex_buffer(0, m.vertex.slice(..)); rpass.set_index_buffer(m.index.slice(..), wgpu::IndexFormat::Uint16); rpass.draw_indexed(0..m.num_indices, 0, 0..1); }
        for m in &self.platform_meshes { rpass.set_vertex_buffer(0, m.vertex.slice(..)); rpass.set_index_buffer(m.index.slice(..), wgpu::IndexFormat::Uint16); rpass.draw_indexed(0..m.num_indices, 0, 0..1); }
        for m in &self.hazard_meshes { rpass.set_vertex_buffer(0, m.vertex.slice(..)); rpass.set_index_buffer(m.index.slice(..), wgpu::IndexFormat::Uint16); rpass.draw_indexed(0..m.num_indices, 0, 0..1); }
        if let Some(m) = &self.exit_pad_mesh { rpass.set_vertex_buffer(0, m.vertex.slice(..)); rpass.set_index_buffer(m.index.slice(..), wgpu::IndexFormat::Uint16); rpass.draw_indexed(0..m.num_indices, 0, 0..1); }
        for m in &self.breakable_meshes { rpass.set_vertex_buffer(0, m.vertex.slice(..)); rpass.set_index_buffer(m.index.slice(..), wgpu::IndexFormat::Uint16); rpass.draw_indexed(0..m.num_indices, 0, 0..1); }
        for m in &self.light_strip_meshes { rpass.set_vertex_buffer(0, m.vertex.slice(..)); rpass.set_index_buffer(m.index.slice(..), wgpu::IndexFormat::Uint16); rpass.draw_indexed(0..m.num_indices, 0, 0..1); }
        for m in &self.fizzler_meshes { rpass.set_vertex_buffer(0, m.vertex.slice(..)); rpass.set_index_buffer(m.index.slice(..), wgpu::IndexFormat::Uint16); rpass.draw_indexed(0..m.num_indices, 0, 0..1); }
        for m in &self.pellet_meshes { rpass.set_vertex_buffer(0, m.vertex.slice(..)); rpass.set_index_buffer(m.index.slice(..), wgpu::IndexFormat::Uint16); rpass.draw_indexed(0..m.num_indices, 0, 0..1); }
        for m in &self.faith_plate_meshes { rpass.set_vertex_buffer(0, m.vertex.slice(..)); rpass.set_index_buffer(m.index.slice(..), wgpu::IndexFormat::Uint16); rpass.draw_indexed(0..m.num_indices, 0, 0..1); }
        for m in &self.laser_meshes { rpass.set_vertex_buffer(0, m.vertex.slice(..)); rpass.set_index_buffer(m.index.slice(..), wgpu::IndexFormat::Uint16); rpass.draw_indexed(0..m.num_indices, 0, 0..1); }
        for m in &self.turret_meshes { rpass.set_vertex_buffer(0, m.vertex.slice(..)); rpass.set_index_buffer(m.index.slice(..), wgpu::IndexFormat::Uint16); rpass.draw_indexed(0..m.num_indices, 0, 0..1); }
        for m in &self.bridge_meshes { rpass.set_vertex_buffer(0, m.vertex.slice(..)); rpass.set_index_buffer(m.index.slice(..), wgpu::IndexFormat::Uint16); rpass.draw_indexed(0..m.num_indices, 0, 0..1); }
        for m in &self.gel_meshes { rpass.set_vertex_buffer(0, m.vertex.slice(..)); rpass.set_index_buffer(m.index.slice(..), wgpu::IndexFormat::Uint16); rpass.draw_indexed(0..m.num_indices, 0, 0..1); }
        if let Some(m) = &self.ghost_mesh { rpass.set_vertex_buffer(0, m.vertex.slice(..)); rpass.set_index_buffer(m.index.slice(..), wgpu::IndexFormat::Uint16); rpass.draw_indexed(0..m.num_indices, 0, 0..1); }
    }
}

const SHADER_SRC: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    pos: vec4<f32>,
};

@group(0) @binding(0) var<uniform> cam: CameraUniform;

struct VsIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
};

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) world_pos: vec3<f32>,
};

@vertex
fn vs_main(in: VsIn) -> VsOut {
    var out: VsOut;
    let world = vec4<f32>(in.pos, 1.0);
    out.clip_pos = cam.view_proj * world;
    out.normal = in.normal;
    out.color = in.color;
    out.world_pos = in.pos;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);
    let l = normalize(vec3<f32>(0.4, 1.0, 0.3));
    let ndotl = max(dot(n, l), 0.0);
    let ambient = 0.35;
    let diffuse = ndotl * 0.65;
    
    let view_dir = normalize(cam.pos.xyz - in.world_pos);
    let half_dir = normalize(l + view_dir);
    let spec = pow(max(dot(n, half_dir), 0.0), 16.0) * 0.3;
    
    let grid_x = abs(fract(in.world_pos.x * 2.0) - 0.5);
    let grid_y = abs(fract(in.world_pos.y * 2.0) - 0.5);
    let grid_z = abs(fract(in.world_pos.z * 2.0) - 0.5);
    let line = min(min(grid_x, grid_y), grid_z);
    let grid_factor = step(0.45, line) * 0.3;
    
    let final_color = in.color * (1.0 - grid_factor);
    return vec4<f32>(final_color * (ambient + diffuse) + spec, 1.0);
}

@fragment
fn fs_overlay(in: VsOut) -> @location(0) vec4<f32> {
    let time = cam.pos.w;
    let pulse = 0.5 + 0.5 * sin(time * 3.0);
    return vec4<f32>(in.color * (1.0 + pulse * 0.5), 1.0);
}

@vertex
fn vs_fullscreen(@builtin(vertex_index) idx: u32) -> @builtin(position) vec4<f32> {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0)
    );
    return vec4<f32>(pos[idx], 1.0, 1.0);
}

@vertex
fn vs_ui(@location(0) pos: vec2<f32>) -> @builtin(position) vec4<f32> {
    return vec4<f32>(pos, 0.0, 1.0);
}

@fragment
fn fs_ui() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 1.0, 1.0, 0.8);
}
"#;
