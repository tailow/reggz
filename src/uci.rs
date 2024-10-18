use crate::engine::Engine;
use shakmaty::{fen::Fen, uci::UciMove, Chess, Move, Position};
use std::io;

pub fn input_loop(mut engine: &mut Engine) {
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
            "debug" => debug(&args, &mut engine),
            "isready" => isready(),
            "setoption" => setoption(&args, &mut engine),
            "position" => position(&args, &mut engine),
            "ucinewgame" => ucinewgame(&mut engine),
            "go" => go(&args, &mut engine),
            "stop" => stop(&mut engine),
            "ponderhit" => ponderhit(&mut engine),
            "quit" => quit(),
            _ => continue,
        }
    }
}

fn uci() {
    println!("id name Seggz II\nid author tailow\nuciok")
}

fn debug(args: &[&str], engine: &mut Engine) {
    match args.get(0) {
        Some(&"on") => (*engine).debug(&true),
        Some(&"off") => (*engine).debug(&false),
        _ => return,
    }
}

fn isready() {
    println!("readyok")
}

fn setoption(args: &[&str], engine: &mut Engine) {
    if args.get(0) == Some(&"name") {
        let mut name: String = String::new();
        let mut value: String = String::new();
        let mut first_value_index: usize = 0;

        let tokens = match args.get(1..) {
            Some(t) => t,
            _ => return,
        };

        for (i, token) in tokens.iter().enumerate() {
            if *token == "value" {
                first_value_index = i + 1;

                break;
            }

            name.push_str(token);
            name.push(' ');
        }

        if tokens.contains(&"value") && tokens.len() > first_value_index {
            for value_token in tokens[first_value_index..].iter() {
                value.push_str(value_token);
                value.push(' ');
            }

            (*engine).set_option(name.trim(), Some(value.trim()));
        } else {
            (*engine).set_option(name.trim(), None);
        }
    }
}

fn ucinewgame(engine: &mut Engine) {
    engine.reset();
}

fn position(args: &[&str], engine: &mut Engine) {
    let mut moves_index: usize = 1;

    match args.get(0) {
        Some(&"fen") => {
            let mut fen_string: String = String::new();

            for (i, token) in args[1..].iter().enumerate() {
                if token == &"moves" {
                    moves_index = i + 1;

                    break;
                }

                fen_string.push_str(token);
                fen_string.push(' ');
            }

            let fen: Fen = match Fen::from_ascii(fen_string.as_bytes()) {
                Ok(fen) => fen,
                Err(_) => {
                    eprintln!("Failed to parse FEN string: {}", fen_string);

                    return;
                }
            };

            (*engine).board = match fen.into_position(shakmaty::CastlingMode::Standard) {
                Ok(board) => board,
                Err(_) => {
                    eprintln!("Failed to convert to fen into position.");

                    return;
                }
            };
        }
        Some(&"startpos") => (*engine).board = Chess::default(),
        None => {
            eprintln!("No arguments given with position.");

            return;
        }
        _ => {
            eprintln!("Invalid argument: {}", args[0]);

            return;
        }
    }

    if args.get(moves_index) == Some(&"moves") {
        for &new_move_string in args[moves_index + 1..].iter() {
            let uci_move: UciMove = match UciMove::from_ascii(new_move_string.as_bytes()) {
                Ok(m) => m,
                Err(_) => {
                    eprintln!("Failed to parse move: {}", new_move_string);

                    return;
                }
            };

            let new_move: Move = match uci_move.to_move(&(*engine).board) {
                Ok(m) => m,
                Err(_) => {
                    eprintln!("Illegal UCI move: {}", uci_move.to_string());

                    return;
                }
            };

            (*engine).board.play_unchecked(&new_move);
        }
    }
}

fn go(args: &[&str], engine: &mut Engine) {
    let mut ponder = false;
    let mut wtime: Option<u64> = None;
    let mut btime: Option<u64> = None;
    let mut winc: Option<u64> = None;
    let mut binc: Option<u64> = None;
    let mut depth: Option<u64> = None;
    let mut infinite = false;

    let mut iter = args.iter();

    while let Some(arg) = iter.next() {
        match *arg {
            "ponder" => ponder = true,
            "wtime" => wtime = iter.next().and_then(|v| v.parse().ok()),
            "btime" => btime = iter.next().and_then(|v| v.parse().ok()),
            "winc" => winc = iter.next().and_then(|v| v.parse().ok()),
            "binc" => binc = iter.next().and_then(|v| v.parse().ok()),
            "depth" => depth = iter.next().and_then(|v| v.parse().ok()),
            "infinite" => infinite = true,
            _ => {}
        }
    }

    engine.search(ponder, wtime, btime, winc, binc, depth, infinite);
}

fn stop(engine: &mut Engine) {
    engine.stop();
}

fn ponderhit(engine: &mut Engine) {
    engine.ponder_hit();
}

fn quit() {
    std::process::exit(0);
}
