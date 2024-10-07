use engine::Engine;

mod engine;
mod evaluate;
mod search;
mod uci;

fn main() {
    let mut engine: Engine = Engine::new();

    uci::input_loop(&mut engine);
}
