use crate::camera::Camera;
use crate::cube::Cube;
use crate::player::{InputState, Player};
use crate::projectile::Projectile;
use crate::particle::Particle;
use crate::renderer::Renderer;
use crate::world::World;
use crate::level::LevelData;
use glam::{Quat, Vec3};
use rodio::{OutputStream, Sink, Source};
use winit::{
    event::{DeviceEvent, ElementState, Event, KeyboardInput, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub struct Engine {
    pub renderer: Renderer,
    pub world: World,
    pub camera: Camera,
    pub player: Player,
    pub cube: Cube,
    pub projectiles: Vec<Projectile>,
    pub particles: Vec<Particle>,
    pub tracers: Vec<(Vec3, Vec3, [f32; 3], f32)>,
    pub pellets: Vec<crate::pellet::Pellet>,
    pub input: InputState,
    pub yaw: f32,
    pub pitch: f32,
    pub current_level: u32,
    pub level_timer: f32,
    pub death_count: u32,
    pub won: bool,
    pub turret_hit: bool,
    pub audio_stream: OutputStream,
    pub audio_sink: Sink,
    pub fps_timer: f32,
    pub fps_frames: u32,
    pub current_fps: f32,
}

fn gen_beep(freq: u32, dur_ms: u64) -> rodio::buffer::SamplesBuffer<f32> {
    let sample_rate = 44100;
    let mut samples = Vec::new();
    for i in 0..(sample_rate * dur_ms / 1000) {
        let t = i as f32 / sample_rate as f32;
        let amp = 0.3 * (1.0 - (i as f32 / (sample_rate * dur_ms / 1000) as f32));
        samples.push((t * freq as f32 * 2.0 * std::f32::consts::PI).sin() * amp);
    }
    rodio::buffer::SamplesBuffer::new(1, sample_rate, samples)
}

impl Engine {
    pub async fn run() {
        env_logger::init();
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_title("Rust Portal Engine")
            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720))
            .build(&event_loop)
            .unwrap();

        if window.set_cursor_grab(winit::window::CursorGrabMode::Locked).is_err() {
            let _ = window.set_cursor_grab(winit::window::CursorGrabMode::Confined);
        }
        window.set_cursor_visible(false);

        let mut renderer = Renderer::new(&window).await;
        let mut world = World::new();
        world.load_level(LevelData::chamber_01());
        renderer.upload_world(&world);

        let (stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();
        sink.set_volume(0.5);

        let mut engine = Engine {
            renderer, world,
            camera: Camera::new(std::f32::consts::FRAC_PI_2, 1280.0 / 720.0),
            player: Player::new(),
            cube: Cube::new(Vec3::new(0.0, 0.4, 3.0)),
            projectiles: Vec::new(), particles: Vec::new(), tracers: Vec::new(), pellets: Vec::new(),
            input: InputState::default(),
            yaw: 0.0, pitch: 0.0,
            current_level: 1, level_timer: 0.0, death_count: 0, won: false, turret_hit: false,
            audio_stream: stream, audio_sink: sink,
            fps_timer: 0.0, fps_frames: 0, current_fps: 0.0,
        };

        let mut last = std::time::Instant::now();

        event_loop.run(move |event, _, control| match event {
            Event::WindowEvent { ref event, .. } => match event {
                WindowEvent::Resized(s) => {
                    engine.renderer.resize(*s);
                    engine.camera.aspect = s.width as f32 / s.height as f32;
                }
                WindowEvent::CloseRequested => *control = ControlFlow::Exit,
                WindowEvent::KeyboardInput { input, .. } => {
                    let pressed = input.state == ElementState::Pressed;
                    if let Some(k) = input.virtual_keycode {
                        match k {
                            winit::event::VirtualKeyCode::W => engine.input.forward = pressed,
                            winit::event::VirtualKeyCode::S => engine.input.backward = pressed,
                            winit::event::VirtualKeyCode::A => engine.input.left = pressed,
                            winit::event::VirtualKeyCode::D => engine.input.right = pressed,
                            winit::event::VirtualKeyCode::LShift => engine.input.sprint = pressed,
                            winit::event::VirtualKeyCode::Space => engine.input.jump = pressed,
                            winit::event::VirtualKeyCode::LControl => engine.input.crouch = pressed,
                            winit::event::VirtualKeyCode::E => { if pressed { engine.input.toggle_grab = true; } }
                            winit::event::VirtualKeyCode::Q => {
                                if pressed {
                                    engine.player.reset_portals(&mut engine.world);
                                    engine.audio_sink.append(gen_beep(200, 300));
                                }
                            }
                            winit::event::VirtualKeyCode::R => {
                                if pressed {
                                    let next_level = LevelData::get_level(engine.current_level);
                                    engine.world.load_level(next_level);
                                    engine.renderer.upload_world(&engine.world);
                                    engine.player.respawn(&mut engine.camera, engine.world.player_spawn);
                                    engine.cube.respawn(engine.world.cube_spawn);
                                    engine.pellets.clear();
                                    engine.audio_sink.append(gen_beep(400, 100));
                                }
                            }
                            winit::event::VirtualKeyCode::F3 => {
                                if pressed { engine.renderer.show_debug = !engine.renderer.show_debug; }
                            }
                            winit::event::VirtualKeyCode::Escape => *control = ControlFlow::Exit,
                            _ => {}
                        }
                    }
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    if *state == ElementState::Pressed {
                        match button {
                            MouseButton::Left => {
                                if engine.cube.grabbed {
                                    engine.cube.grabbed = false;
                                    engine.cube.velocity = engine.camera.transform.forward() * 20.0;
                                } else {
                                    if let Some((pos, normal, up, center, hw, hh, _)) = engine.player.shoot_portal_hitscan(&engine.camera, &engine.world, true) {
                                        let start = engine.camera.transform.position;
                                        let color = [0.3, 0.7, 1.0];
                                        engine.tracers.push((start, pos, color, 0.1));
                                        engine.player.spawn_portal_at(pos, normal, up, center, hw, hh, true, &mut engine.world);
                                        engine.audio_sink.append(gen_beep(800, 100));
                                    }
                                }
                            }
                            MouseButton::Right => {
                                if let Some((pos, normal, up, center, hw, hh, _)) = engine.player.shoot_portal_hitscan(&engine.camera, &engine.world, false) {
                                    let start = engine.camera.transform.position;
                                    let color = [1.0, 0.5, 0.3];
                                    engine.tracers.push((start, pos, color, 0.1));
                                    engine.player.spawn_portal_at(pos, normal, up, center, hw, hh, false, &mut engine.world);
                                    engine.audio_sink.append(gen_beep(600, 100));
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            },
            Event::DeviceEvent { event, .. } => {
                if let DeviceEvent::MouseMotion { delta } = event {
                    engine.yaw -= (delta.0 as f32) * 0.0025;
                    engine.pitch -= (delta.1 as f32) * 0.0025;
                    engine.pitch = engine.pitch.clamp(-1.55, 1.55);
                    let q = Quat::from_rotation_y(engine.yaw) * Quat::from_rotation_x(engine.pitch);
                    engine.camera.transform.orientation = q;
                }
            }
            Event::MainEventsCleared => {
                if engine.won { return; }
                let now = std::time::Instant::now();
                let dt = (now - last).as_secs_f32().min(0.05);
                last = now;
                
                if engine.input.toggle_grab {
                    engine.input.toggle_grab = false;
                    if engine.cube.grabbed {
                        engine.cube.grabbed = false;
                        engine.cube.velocity = engine.camera.transform.forward() * 10.0;
                    } else {
                        if engine.player.try_grab_cube(&engine.camera, &engine.cube, &engine.world.portals) {
                            engine.cube.grabbed = true;
                            engine.audio_sink.append(gen_beep(400, 50));
                        }
                    }
                }

                for t in &mut engine.tracers { t.3 -= dt; }
                engine.tracers.retain(|t| t.3 > 0.0);
                let active_tracers: Vec<(Vec3, Vec3, [f32; 3])> = engine.tracers.iter().map(|t| (t.0, t.1, t.2)).collect();
                engine.renderer.update_tracers(&active_tracers);

                engine.world.update_hazards(dt);
                engine.world.update_platforms(dt);
                engine.renderer.update_hazards(&engine.world.hazards);
                engine.renderer.update_platforms(&engine.world.platforms, &engine.world.exit_pad);
                engine.renderer.update_dynamic_level_meshes(&engine.world);

                let mut new_segments = Vec::new();
                for emitter in &engine.world.laser_emitters {
                    let segs = crate::laser::calculate_laser_path(emitter.pos, emitter.dir, &engine.world, &engine.cube, 3);
                    new_segments.extend(segs);
                }
                engine.world.laser_segments = new_segments;
                engine.renderer.update_lasers(&engine.world.laser_segments);

                for c in &mut engine.world.laser_catchers { c.active = false; }
                for seg in &engine.world.laser_segments {
                    for c in &mut engine.world.laser_catchers {
                        if let Some(t) = crate::math_util::ray_plane_intersect(seg.start, seg.end - seg.start, c.pos, c.normal) {
                            if t > 0.0 && t < (seg.end - seg.start).length() {
                                c.active = true;
                            }
                        }
                    }
                }
                for i in 0..engine.world.doors.len() {
                    let mut should_open = false;
                    for c in &engine.world.laser_catchers { if c.linked_door == i && c.active { should_open = true; } }
                    for b in &engine.world.buttons { if b.linked_door == i && b.is_pressed { should_open = true; } }
                    engine.world.doors[i].is_open = should_open;
                    if should_open {
                        engine.world.doors[i].pos.y = crate::math_util::lerp(engine.world.doors[i].pos.y, engine.world.doors[i].open_y, 10.0 * dt);
                    } else {
                        engine.world.doors[i].pos.y = crate::math_util::lerp(engine.world.doors[i].pos.y, engine.world.doors[i].closed_y, 10.0 * dt);
                    }
                }

                let mut all_tiles = Vec::new();
                for emitter in &engine.world.bridge_emitters {
                    let bridge = crate::bridge::calculate_bridge_path(emitter.pos, emitter.dir, &engine.world);
                    all_tiles.extend(bridge.tiles);
                }
                engine.world.bridge_tiles = all_tiles;
                engine.renderer.update_bridges(&engine.world.bridge_tiles);

                let mut pellet_hits = Vec::new();
                for emitter in &mut engine.world.pellet_emitters {
                    emitter.timer += dt;
                    if emitter.timer >= emitter.interval {
                        emitter.timer = 0.0;
                        engine.pellets.push(crate::pellet::Pellet::new(emitter.pos, emitter.dir));
                    }
                }
                for p in &mut engine.pellets {
                    let hit = p.update(dt, &engine.world);
                    if hit {
                        let dist_to_player = (p.pos - engine.camera.transform.position).length();
                        let dist_to_cube = (p.pos - engine.cube.position).length();
                        if dist_to_player < 1.0 { engine.player.dead = true; }
                        if dist_to_cube < 1.0 { engine.cube.dead = true; }
                    }
                }
                engine.pellets.retain(|p| p.life > 0.0);
                engine.renderer.update_pellets(&engine.pellets);
                engine.renderer.update_gels(&engine.world.gel_patches);
                engine.renderer.update_faith_plates(&engine.world.faith_plates);
                engine.renderer.update_turrets(&engine.world.turrets);

                engine.turret_hit = false;
                for t in &mut engine.world.turrets {
                    if t.active {
                        if t.update(dt, engine.camera.transform.position, &engine.world) {
                            engine.turret_hit = true;
                        }
                    }
                }
                if engine.turret_hit { engine.player.dead = true; }
                for t in &mut engine.world.turrets {
                    if (engine.cube.position - t.pos).length() < 1.0 && engine.cube.velocity.length() > 2.0 {
                        t.active = false;
                    }
                }

                if engine.player.teleported {
                    let normal = engine.camera.transform.forward() * -1.0;
                    for _ in 0..15 { engine.particles.push(Particle::new(engine.camera.transform.position, normal, [1.0, 1.0, 1.0])); }
                    engine.audio_sink.append(gen_beep(150, 150));
                }
                if engine.cube.teleported {
                    let normal = engine.cube.velocity.normalize_or_zero() * -1.0;
                    for _ in 0..15 { engine.particles.push(Particle::new(engine.cube.position, normal, [0.8, 0.6, 0.2])); }
                    engine.audio_sink.append(gen_beep(150, 150));
                }

                for p in &mut engine.particles { p.update(dt); }
                engine.particles.retain(|p| p.life > 0.0);
                engine.renderer.update_particles(&engine.particles);

                if engine.player.dead {
                    engine.death_count += 1;
                    engine.player.respawn(&mut engine.camera, engine.world.player_spawn);
                    engine.audio_sink.append(gen_beep(100, 500));
                }
                if engine.cube.dead { engine.cube.respawn(engine.world.cube_spawn); }

                let dist_to_exit = (engine.camera.transform.position - engine.world.exit_pad.pos).length();
                if dist_to_exit < 1.5 {
                    engine.current_level += 1;
                    if engine.current_level > 9 {
                        engine.won = true;
                        engine.renderer.win_screen_visible = true;
                        engine.audio_sink.append(gen_beep(1500, 1000));
                    } else {
                        engine.level_timer = 0.0;
                        let next_level = LevelData::get_level(engine.current_level);
                        engine.world.load_level(next_level);
                        engine.renderer.upload_world(&engine.world);
                        engine.player.respawn(&mut engine.camera, engine.world.player_spawn);
                        engine.cube.respawn(engine.world.cube_spawn);
                        engine.pellets.clear();
                        engine.audio_sink.append(gen_beep(1200, 200));
                    }
                }

                if engine.cube.grabbed {
                    engine.cube.position = engine.camera.transform.position + engine.camera.transform.forward() * 2.5 - engine.camera.transform.up() * 0.5;
                    engine.cube.velocity = Vec3::ZERO;
                } else {
                    engine.cube.update(dt, &engine.world);
                }

                engine.player.update(&mut engine.camera, &engine.input, dt, &engine.world);
                engine.renderer.update_ghost_portal(&engine.player.ghost_portal);
                
                let bob_y = engine.player.bob_time.sin() * 0.05;
                let bob_roll = engine.player.bob_time.cos() * 0.01;
                engine.camera.bob_y = bob_y;
                engine.camera.bob_roll = bob_roll;

                engine.renderer.update_cube(&engine.cube);
                engine.renderer.upload_portals(&engine.world);
                
                engine.fps_timer += dt;
                engine.fps_frames += 1;
                if engine.fps_timer >= 0.5 {
                    engine.current_fps = engine.fps_frames as f32 / engine.fps_timer;
                    engine.fps_timer = 0.0;
                    engine.fps_frames = 0;
                }
                let speed = Vec3::new(engine.player.velocity.x, 0.0, engine.player.velocity.z).length();
                engine.renderer.update_debug(engine.renderer.show_debug, speed, engine.current_fps);
                
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                match engine.renderer.render(&engine.world, &engine.camera) {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => engine.renderer.resize(engine.renderer.size),
                    Err(wgpu::SurfaceError::OutOfMemory) => *control = ControlFlow::Exit,
                    Err(e) => log::error!("render error: {:?}", e),
                }
            }
            _ => {}
        });
    }
}
