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

        engine.print_state();
    }
}

fn uci() {
    println!("id name Seggz\nid author tailow\nuciok")
}

fn debug(args: &[&str], engine: &mut Engine) {
    match args[0] {
        "on" => (*engine).debug(&true),
        "off" => (*engine).debug(&false),
        _ => return,
    }
}

fn isready() {
    println!("readyok")
}

fn setoption(_args: &[&str], engine: &mut Engine) {
    engine.set_option();
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
                    eprintln!("Failed to parse FEN string.");

                    return;
                }
            };

            (*engine).board = match fen.into_position(shakmaty::CastlingMode::Standard) {
                Ok(board) => board,
                Err(_) => {
                    eprintln!("Failed to convert to position.");

                    return;
                }
            };
        }
        Some(&"startpos") => (*engine).board = Chess::default(),
        _ => {
            eprintln!("Invalid argument.");

            return;
        }
    }

    if args.get(moves_index) == Some(&"moves") {
        for &new_move_string in args[moves_index + 1..].iter() {
            let uci_move: UciMove = match UciMove::from_ascii(new_move_string.as_bytes()) {
                Ok(m) => m,
                Err(_) => {
                    eprintln!("Failed to parse move.");

                    return;
                }
            };

            let new_move: Move = match uci_move.to_move(&(*engine).board) {
                Ok(m) => m,
                Err(_) => {
                    eprintln!("Invalid UCI move.");

                    return;
                }
            };

            (*engine).board.play_unchecked(&new_move);
        }
    }
}

fn go(args: &[&str], engine: &mut Engine) {
    engine.search();
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
