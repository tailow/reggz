use std::io;

pub fn input_loop() {
    let mut input: String = String::new();

    loop {
        input.clear();

        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line.");

        let input = input.trim();

        let mut split_input = input.split_whitespace();

        let command = match split_input.next() {
            Some(cmd) => cmd,
            None => continue,
        };

        let args: Vec<&str> = split_input.collect();

        match command {
            "uci" => uci(),
            "debug" => debug(&args),
            "isready" => isready(),
            "setoption" => setoption(&args),
            "position" => position(&args),
            "go" => go(&args),
            "stop" => stop(),
            "ponderhit" => ponderhit(),
            "quit" => quit(),
            _ => continue,
        }
    }
}

fn uci() {
    println!("id name Seggz\nid author tailow\nuciok")
}

fn debug(_args: &[&str]) {
    match _args.get(0) {
        Some(&"on") => {}
        Some(&"off") => {}
        _ => return,
    }
}

fn isready() {
    println!("readyok")
}

fn setoption(_args: &[&str]) {}

fn position(_args: &[&str]) {}

fn go(_args: &[&str]) {}

fn stop() {}

fn ponderhit() {}

fn quit() {
    std::process::exit(0);
}
