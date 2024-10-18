use crate::search;
use shakmaty::{Chess, Move};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;

pub struct Engine {
    pub board: Chess,
    debug: Arc<AtomicBool>,
    searching: Arc<AtomicBool>,
    pondering: Arc<AtomicBool>,
}

impl Engine {
    pub fn new() -> Engine {
        Engine {
            board: Chess::new(),
            debug: Arc::new(AtomicBool::new(false)),
            searching: Arc::new(AtomicBool::new(false)),
            pondering: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn search(
        &self,
        ponder: bool,
        white_time: Option<u64>,
        black_time: Option<u64>,
        white_increment: Option<u64>,
        black_increment: Option<u64>,
        depth: Option<u64>,
        move_time: Option<u64>,
        infinite: bool,
    ) {
        self.searching.store(true, Ordering::Relaxed);

        let board_clone = self.board.clone();

        let debug_clone = Arc::clone(&self.debug);
        let searching_clone = Arc::clone(&self.searching);
        let pondering_clone = Arc::clone(&self.pondering);

        thread::spawn(move || {
            search::search(
                board_clone,
                searching_clone,
                pondering_clone,
                debug_clone,
                depth,
            )
        });
    }

    pub fn debug(&mut self, enable: &bool) {
        self.debug.store(*enable, Ordering::Relaxed);
    }

    pub fn reset(&mut self) {
        self.searching.store(false, Ordering::Relaxed);
        self.pondering.store(false, Ordering::Relaxed);

        self.board = Chess::default()
    }

    pub fn stop(&mut self) {
        self.searching.store(false, Ordering::Relaxed);
        self.pondering.store(false, Ordering::Relaxed);
    }

    pub fn ponder_hit(&mut self) {
        self.pondering.store(false, Ordering::Relaxed);
    }

    pub fn set_option(&self, name: &str, value: Option<&str>) {
        if let Some(optional_value) = value {
            println!("Set option {} with value {}", name, optional_value);
        } else {
            println!("Set option {}", name);
        }
    }
}
