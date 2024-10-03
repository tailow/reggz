pub mod debug;
pub mod engine;
pub mod uci;

fn main() {
    uci::input_loop();
}
