use std::io;

pub fn input_loop() {
    let mut input: String = String::new();

    loop {
        input.clear();

        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line.");

        if input.trim() == "uci" {
            println!("uciok")
        }
    }
}
