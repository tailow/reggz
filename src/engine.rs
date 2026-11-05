use crate::{search, timer};
use shakmaty::{zobrist::Zobrist64, Chess, Color, Position};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;

const MAX_TRANSPOSITION_TABLE_SIZE_MB: usize = 256;
const MOVE_OVERHEAD_MS: u64 = 50;
pub const TRANSPOSITION_TABLE_LENGTH: usize =
    MAX_TRANSPOSITION_TABLE_SIZE_MB * 1_000_000 / size_of::<Option<search::Node>>();

pub struct Engine {
    pub board: Chess,
    debug: Arc<AtomicBool>,
    searching: Arc<AtomicBool>,
    pub position_history: Vec<Zobrist64>,
    transposition_table: Arc<Mutex<Vec<Option<search::Node>>>>,
    white_time: Option<u64>,
    black_time: Option<u64>,
    white_increment: Option<u64>,
    black_increment: Option<u64>,
}

impl Engine {
    pub fn new() -> Engine {
        Engine {
            board: Chess::new(),
            debug: Arc::new(AtomicBool::new(true)),
            searching: Arc::new(AtomicBool::new(false)),
            position_history: Vec::with_capacity(512),
            transposition_table: Arc::new(Mutex::new(vec![None; TRANSPOSITION_TABLE_LENGTH])),
            white_time: None,
            black_time: None,
            white_increment: None,
            black_increment: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn search(
        &mut self,
        white_time: Option<u64>,
        black_time: Option<u64>,
        white_increment: Option<u64>,
        black_increment: Option<u64>,
        move_time: Option<u64>,
        depth: Option<i16>,
        infinite: bool,
    ) {
        self.searching.store(true, Ordering::Relaxed);

        self.white_time = white_time;
        self.black_time = black_time;
        self.white_increment = white_increment;
        self.black_increment = black_increment;

        let board_clone = self.board.clone();

        let debug_clone = Arc::clone(&self.debug);
        let searching_clone = Arc::clone(&self.searching);

        let mut position_history_clone = self.position_history.clone();

        let mut transposition_table_clone = Arc::clone(&self.transposition_table);

        let mut searcher = search::Searcher {
            nodes: 0,
            searching: searching_clone.clone(),
            debug: debug_clone,
            max_depth: depth,
            best_root_move: None,
        };

        thread::spawn(move || {
            searcher.search(
                board_clone,
                &mut position_history_clone,
                &mut transposition_table_clone,
            )
        });

        // Start timer if not infinite
        if !infinite {
            let move_time = move_time.or_else(|| self.allocate_time());

            if let Some(move_time) = move_time {
                thread::spawn(move || timer::search_for_ms(move_time, searching_clone));
            }
        }
    }

    // Time to spend on this move from the remaining clock time and increment, never more than
    // the remaining time minus a safety margin for communication overhead
    fn allocate_time(&self) -> Option<u64> {
        let (remaining, increment) = if self.board.turn() == Color::White {
            (self.white_time?, self.white_increment.unwrap_or(0))
        } else {
            (self.black_time?, self.black_increment.unwrap_or(0))
        };

        let budget = remaining / 20 + increment / 2;
        let limit = remaining.saturating_sub(MOVE_OVERHEAD_MS);

        Some(budget.min(limit))
    }

    pub fn debug(&mut self, enable: &bool) {
        self.debug.store(*enable, Ordering::Relaxed);
    }

    // Start a new game: reset the position and clear the transposition table
    pub fn new_game(&mut self) {
        self.searching.store(false, Ordering::Relaxed);

        self.set_position(Chess::new());

        for node in self.transposition_table.lock().unwrap().iter_mut() {
            *node = None;
        }

        self.white_time = None;
        self.black_time = None;
        self.white_increment = None;
        self.black_increment = None;
    }

    // Set the root position and start the position history from it
    pub fn set_position(&mut self, board: Chess) {
        self.board = board;

        self.position_history.clear();

        self.position_history
            .push(self.board.zobrist_hash(shakmaty::EnPassantMode::Legal));
    }

    pub fn stop(&mut self) {
        self.searching.store(false, Ordering::Relaxed);
    }

    // Start timer
    pub fn ponder_hit(&mut self) {
        let searching_clone = Arc::clone(&self.searching);

        if let Some(move_time) = self.allocate_time() {
            thread::spawn(move || timer::search_for_ms(move_time, searching_clone));
        }
    }
}
