use crate::{search, timer};
use shakmaty::{
    zobrist::{Zobrist64, ZobristHash},
    Chess, Color, Position,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;

const MAX_TRANSPOSITION_TABLE_SIZE_MB: usize = 240;
pub const TRANSPOSITION_TABLE_LENGTH: usize =
    MAX_TRANSPOSITION_TABLE_SIZE_MB * 1_000_000 / size_of::<Option<search::Node>>();

pub struct Engine {
    pub board: Chess,
    debug: Arc<AtomicBool>,
    searching: Arc<AtomicBool>,
    pondering: Arc<AtomicBool>,
    pub plies_since_irreversible_move: u64,
    pub position_history: Vec<Zobrist64>,
    transposition_table: Arc<Mutex<Vec<Option<search::Node>>>>,
}

impl Engine {
    pub fn new() -> Engine {
        println!(
            "{} {}",
            TRANSPOSITION_TABLE_LENGTH,
            size_of::<search::Node>()
        );

        Engine {
            board: Chess::new(),
            debug: Arc::new(AtomicBool::new(true)),
            searching: Arc::new(AtomicBool::new(false)),
            pondering: Arc::new(AtomicBool::new(false)),
            plies_since_irreversible_move: 0,
            position_history: Vec::with_capacity(512),
            transposition_table: Arc::new(Mutex::new(vec![None; TRANSPOSITION_TABLE_LENGTH])),
        }
    }

    pub fn search(
        &self,
        ponder: bool,
        white_time: Option<u64>,
        black_time: Option<u64>,
        white_increment: Option<u64>,
        black_increment: Option<u64>,
        move_time: Option<u64>,
        depth: Option<u8>,
        infinite: bool,
    ) {
        self.searching.store(true, Ordering::Relaxed);

        let board_clone = self.board.clone();

        let debug_clone = Arc::clone(&self.debug);
        let searching_clone = Arc::clone(&self.searching);
        let pondering_clone = Arc::clone(&self.pondering);

        let plies_since_irreversible_clone = self.plies_since_irreversible_move.clone();
        let position_history_clone = self.position_history.clone();

        let transposition_table_clone = Arc::clone(&self.transposition_table);

        thread::spawn(move || {
            search::search(
                board_clone,
                searching_clone,
                pondering_clone,
                debug_clone,
                depth,
                plies_since_irreversible_clone,
                position_history_clone,
                transposition_table_clone,
            )
        });

        let searching_clone = Arc::clone(&self.searching);

        if !infinite {
            if move_time.is_some() {
                let move_time = move_time.unwrap();

                thread::spawn(move || timer::search_for_ms(move_time, searching_clone));
            } else if self.board.turn() == Color::White && white_time.is_some() {
                let remaining = white_time.unwrap();
                let increment = white_increment.unwrap_or(0);

                let move_time = remaining / 20 + increment / 2;

                thread::spawn(move || timer::search_for_ms(move_time, searching_clone));
            } else if self.board.turn() == Color::Black && black_time.is_some() {
                let remaining = black_time.unwrap();
                let increment = black_increment.unwrap_or(0);

                let move_time = remaining / 20 + increment / 2;

                thread::spawn(move || timer::search_for_ms(move_time, searching_clone));
            }
        }
    }

    pub fn debug(&mut self, enable: &bool) {
        self.debug.store(*enable, Ordering::Relaxed);
    }

    pub fn reset(&mut self) {
        self.searching.store(false, Ordering::Relaxed);
        self.pondering.store(false, Ordering::Relaxed);

        self.board = Chess::new();

        self.plies_since_irreversible_move = 0;
        self.position_history.clear();

        self.position_history
            .push(self.board.zobrist_hash(shakmaty::EnPassantMode::Legal));
    }

    pub fn stop(&mut self) {
        self.searching.store(false, Ordering::Relaxed);
        self.pondering.store(false, Ordering::Relaxed);
    }

    pub fn ponder_hit(&mut self) {
        self.pondering.store(false, Ordering::Relaxed);
    }
}
