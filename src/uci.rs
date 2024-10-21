use crate::engine::Engine;
use shakmaty::{fen::Fen, uci::UciMove, zobrist::ZobristHash, EnPassantMode, Move, Position};
use std::{io, str::SplitWhitespace};

pub fn input_loop(mut engine: &mut Engine) {
    let mut input: String = String::new();

    loop {
        input.clear();

        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line.");

        let input = input.trim();

        let mut tokens = input.split_whitespace();

        while let Some(token) = tokens.next() {
            match token {
                "uci" => uci(),
                "debug" => debug(&mut tokens, &mut engine),
                "isready" => isready(),
                "position" => position(&mut tokens, &mut engine),
                "ucinewgame" => ucinewgame(&mut engine),
                "go" => go(&mut tokens, &mut engine),
                "stop" => stop(&mut engine),
                "ponderhit" => ponderhit(&mut engine),
                "quit" => quit(),
                _ => {}
            }
        }
    }
}

fn uci() {
    println!("id name Reggz\nid author tailow\nuciok")
}

fn debug(tokens: &mut SplitWhitespace<'_>, engine: &mut Engine) {
    match tokens.next() {
        Some("on") => engine.debug(&true),
        Some("off") => engine.debug(&false),
        _ => return,
    }
}

fn isready() {
    println!("readyok")
}

fn ucinewgame(engine: &mut Engine) {
    engine.reset();
}

fn position(tokens: &mut SplitWhitespace<'_>, engine: &mut Engine) {
    match tokens.next() {
        Some("startpos") => engine.reset(),
        Some("fen") => {
            engine.reset();

            let fen_parts: Vec<&str> = tokens.by_ref().take(6).collect();

            let fen_string = fen_parts.join(" ");

            let fen: Fen = match Fen::from_ascii(fen_string.as_bytes()) {
                Ok(fen) => fen,
                Err(_) => return,
            };

            engine.board = match fen.into_position(shakmaty::CastlingMode::Standard) {
                Ok(board) => board,
                Err(_) => return,
            };
        }
        _ => return,
    }

    if let Some("moves") = tokens.next() {
        let moves: Vec<&str> = tokens.collect();

        for uci_move_string in moves {
            let uci_move: UciMove = match UciMove::from_ascii(uci_move_string.as_bytes()) {
                Ok(m) => m,
                Err(_) => return,
            };

            let new_move: Move = match uci_move.to_move(&engine.board) {
                Ok(m) => m,
                Err(_) => return,
            };

            engine.board.play_unchecked(&new_move);

            engine
                .position_history
                .push(engine.board.zobrist_hash(EnPassantMode::Legal));

            if engine.board.is_irreversible(&new_move) {
                engine.plies_since_irreversible_move = 0;
            } else {
                engine.plies_since_irreversible_move += 1;
            }
        }
    }
}

fn go(tokens: &mut SplitWhitespace<'_>, engine: &mut Engine) {
    let mut ponder = false;
    let mut wtime: Option<u64> = None;
    let mut btime: Option<u64> = None;
    let mut winc: Option<u64> = None;
    let mut binc: Option<u64> = None;
    let mut movetime: Option<u64> = None;
    let mut depth: Option<u8> = None;
    let mut infinite = false;

    while let Some(token) = tokens.next() {
        match token {
            "ponder" => ponder = true,
            "wtime" => wtime = tokens.next().and_then(|v| v.parse().ok()),
            "btime" => btime = tokens.next().and_then(|v| v.parse().ok()),
            "winc" => winc = tokens.next().and_then(|v| v.parse().ok()),
            "binc" => binc = tokens.next().and_then(|v| v.parse().ok()),
            "movetime" => movetime = tokens.next().and_then(|v| v.parse().ok()),
            "depth" => depth = tokens.next().and_then(|v| v.parse().ok()),
            "infinite" => infinite = true,
            _ => {}
        }
    }

    engine.search(ponder, wtime, btime, winc, binc, movetime, depth, infinite);
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
