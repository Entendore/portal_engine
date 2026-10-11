use portal_engine::Engine;

fn main() {
    env_logger::init();
    pollster::block_on(Engine::run());
}
