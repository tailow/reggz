use engine::Engine;

mod engine;
mod evaluate;
mod search;
mod timer;
mod uci;

fn main() {
    println!("Seggz II UCI Chess engine by tailow");

    let mut engine: Engine = Engine::new();

    uci::input_loop(&mut engine);
}
