use crate::portal::Portal;
use crate::level::{LevelData, Platform, Hazard, Fizzler, BreakableWall, LightStrip, FaithPlate, PelletEmitter, LaserEmitter, LaserCatcher, PortalBlocker};
use crate::turret::Turret;
use crate::gel::GelPatch;
use crate::bridge::BridgeSegment;
use crate::laser::LaserSegment;
use glam::Vec3;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos:    [f32; 3],
    pub normal: [f32; 3],
    pub color:  [f32; 3],
}

impl Vertex {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3],
    };
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex2D {
    pub pos: [f32; 2],
}

impl Vertex2D {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Vertex2D>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x2],
    };
}

pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
}

impl Mesh {
    pub fn cube(pos: Vec3, size: f32, color: [f32; 3]) -> Self {
        let s = size / 2.0;
        Self::cuboid(pos, Vec3::splat(s), color)
    }

    pub fn cuboid(pos: Vec3, half_size: Vec3, color: [f32; 3]) -> Self {
        let s = half_size;
        let p = pos;
        let faces = [
            (Vec3::new( s.x, s.y, s.z), Vec3::new( s.x, s.y,-s.z), Vec3::new( s.x,-s.y,-s.z), Vec3::new( s.x,-s.y, s.z), Vec3::new( 1.0, 0.0, 0.0)),
            (Vec3::new(-s.x, s.y,-s.z), Vec3::new(-s.x, s.y, s.z), Vec3::new(-s.x,-s.y, s.z), Vec3::new(-s.x,-s.y,-s.z), Vec3::new(-1.0, 0.0, 0.0)),
            (Vec3::new(-s.x, s.y, s.z), Vec3::new( s.x, s.y, s.z), Vec3::new( s.x, s.y,-s.z), Vec3::new(-s.x, s.y,-s.z), Vec3::new( 0.0, 1.0, 0.0)),
            (Vec3::new(-s.x,-s.y,-s.z), Vec3::new( s.x,-s.y,-s.z), Vec3::new( s.x,-s.y, s.z), Vec3::new(-s.x,-s.y, s.z), Vec3::new( 0.0,-1.0, 0.0)),
            (Vec3::new(-s.x, s.y, s.z), Vec3::new(-s.x,-s.y, s.z), Vec3::new( s.x,-s.y, s.z), Vec3::new( s.x, s.y, s.z), Vec3::new( 0.0, 0.0, 1.0)),
            (Vec3::new( s.x, s.y,-s.z), Vec3::new( s.x,-s.y,-s.z), Vec3::new(-s.x,-s.y,-s.z), Vec3::new(-s.x, s.y,-s.z), Vec3::new( 0.0, 0.0,-1.0)),
        ];
        let mut verts = Vec::new();
        let mut idx = Vec::new();
        for (i, (a, b, c, d, n)) in faces.iter().enumerate() {
            let base = (i * 4) as u16;
            for v in [a, b, c, d] {
                verts.push(Vertex { pos: [p.x + v.x, p.y + v.y, p.z + v.z], normal: [n.x, n.y, n.z], color });
            }
            idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        Self { vertices: verts, indices: idx }
    }

    pub fn portal_ring(corners: [Vec3; 4], normal: Vec3, color: [f32; 3]) -> Self {
        let center = (corners[0] + corners[1] + corners[2] + corners[3]) / 4.0;
        let outer: [Vec3; 4] = [
            corners[0] + (corners[0] - center) * 0.2,
            corners[1] + (corners[1] - center) * 0.2,
            corners[2] + (corners[2] - center) * 0.2,
            corners[3] + (corners[3] - center) * 0.2,
        ];
        let n = [normal.x, normal.y, normal.z];
        let mut verts = Vec::new();
        for v in corners.iter().chain(outer.iter()) {
            verts.push(Vertex { pos: [v.x, v.y, v.z], normal: n, color });
        }
        let idx = vec![0, 4, 5, 0, 5, 1, 1, 5, 6, 1, 6, 2, 2, 6, 7, 2, 7, 3, 3, 7, 4, 3, 4, 0];
        Mesh { vertices: verts, indices: idx }
    }
    
    pub fn quad(corners: [Vec3; 4], normal: Vec3, color: [f32; 3]) -> Self {
        let n = [normal.x, normal.y, normal.z];
        let mut verts = Vec::new();
        for c in corners.iter() {
            verts.push(Vertex { pos: [c.x, c.y, c.z], normal: n, color });
        }
        Self { vertices: verts, indices: vec![0, 1, 2, 0, 2, 3] }
    }
}

#[derive(Clone)]
pub struct Wall {
    pub center: Vec3,
    pub normal: Vec3,
    pub up: Vec3,
    pub half_width: f32,
    pub half_height: f32,
    pub portalable: bool,
    pub is_glass: bool,
}

#[derive(Clone)]
pub struct Button {
    pub pos: Vec3,
    pub is_pressed: bool,
    pub linked_door: usize,
}

#[derive(Clone)]
pub struct Door {
    pub pos: Vec3,
    pub closed_y: f32,
    pub open_y: f32,
    pub is_open: bool,
    pub half_size: Vec3,
}

pub struct World {
    pub meshes: Vec<Mesh>,
    pub walls: Vec<Wall>,
    pub pillars: Vec<(Vec3, f32)>,
    pub portals: Vec<Portal>,
    pub buttons: Vec<Button>,
    pub doors: Vec<Door>,
    pub platforms: Vec<Platform>,
    pub hazards: Vec<Hazard>,
    pub fizzlers: Vec<Fizzler>,
    pub breakables: Vec<BreakableWall>,
    pub light_strips: Vec<LightStrip>,
    pub faith_plates: Vec<FaithPlate>,
    pub pellet_emitters: Vec<PelletEmitter>,
    pub laser_emitters: Vec<LaserEmitter>,
    pub laser_catchers: Vec<LaserCatcher>,
    pub turrets: Vec<Turret>,
    pub bridge_emitters: Vec<crate::level::LightBridgeEmitter>,
    pub gel_patches: Vec<GelPatch>,
    pub portal_blockers: Vec<PortalBlocker>,
    pub laser_segments: Vec<LaserSegment>,
    pub bridge_tiles: Vec<Vec3>,
    pub current_level: u32,
    pub player_spawn: Vec3,
    pub cube_spawn: Vec3,
    pub exit_pad: crate::level::ExitPad,
}

impl World {
    pub fn new() -> Self {
        Self {
            meshes: Vec::new(), walls: Vec::new(), pillars: Vec::new(),
            portals: Vec::new(), buttons: Vec::new(), doors: Vec::new(),
            platforms: Vec::new(), hazards: Vec::new(), fizzlers: Vec::new(),
            breakables: Vec::new(), light_strips: Vec::new(), faith_plates: Vec::new(),
            pellet_emitters: Vec::new(), laser_emitters: Vec::new(), laser_catchers: Vec::new(),
            turrets: Vec::new(), bridge_emitters: Vec::new(), gel_patches: Vec::new(),
            portal_blockers: Vec::new(), laser_segments: Vec::new(), bridge_tiles: Vec::new(),
            current_level: 1, player_spawn: Vec3::ZERO, cube_spawn: Vec3::ZERO,
            exit_pad: crate::level::ExitPad { pos: Vec3::ZERO, half_size: 1.0 },
        }
    }

    pub fn load_level(&mut self, level_data: LevelData) {
        self.meshes = level_data.meshes;
        self.walls = level_data.walls;
        self.pillars = level_data.pillars;
        self.platforms = level_data.platforms;
        self.hazards = level_data.hazards;
        self.fizzlers = level_data.fizzlers;
        self.breakables = level_data.breakables;
        self.light_strips = level_data.light_strips;
        self.faith_plates = level_data.faith_plates;
        self.pellet_emitters = level_data.pellet_emitters;
        self.laser_emitters = level_data.laser_emitters;
        self.laser_catchers = level_data.laser_catchers;
        self.turrets = level_data.turrets;
        self.bridge_emitters = level_data.bridge_emitters;
        self.gel_patches = level_data.gel_patches;
        self.portal_blockers = level_data.portal_blockers;
        self.player_spawn = level_data.player_spawn;
        self.cube_spawn = level_data.cube_spawn;
        self.exit_pad = level_data.exit_pad;
        
        self.portals.clear();
        self.portals.push(Portal::new(Vec3::new(0.0, -100.0, -9.9), Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, 1.0, 0.0), 0.9, 1.2));
        self.portals.push(Portal::new(Vec3::new(0.0, -100.0, 9.9), Vec3::new(0.0, 0.0, -1.0), Vec3::new(0.0, 1.0, 0.0), 0.9, 1.2));
        self.portals[0].linked = Some(1);
        self.portals[1].linked = Some(0);
        self.portals[0].color = [0.3, 0.7, 1.0];
        self.portals[1].color = [1.0, 0.5, 0.3];
        self.laser_segments.clear();
        self.bridge_tiles.clear();
    }

    pub fn update_platforms(&mut self, dt: f32) {
        for p in &mut self.platforms {
            let target = if p.going_forward { p.end_pos } else { p.start_pos };
            let dir = target - p.pos;
            let dist = dir.length();
            if dist < 0.1 {
                p.going_forward = !p.going_forward;
                p.vel = Vec3::ZERO;
            } else {
                let move_vel = dir.normalize() * p.speed;
                p.pos += move_vel * dt;
                p.vel = move_vel;
            }
        }
    }

    pub fn update_hazards(&mut self, dt: f32) {
        for h in &mut self.hazards {
            let target = if h.going_forward { h.end_pos } else { h.start_pos };
            let dir = target - h.pos;
            let dist = dir.length();
            if dist < 0.1 {
                h.going_forward = !h.going_forward;
            } else {
                h.pos += dir.normalize() * h.speed * dt;
            }
        }
    }
}
