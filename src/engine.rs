use crate::search;
use shakmaty::{Chess, Move, Position};
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
        search_moves: Option<&[Move]>,
        ponder: bool,
        white_time: Option<u32>,
        black_time: Option<u32>,
        white_increment: Option<u32>,
        black_increment: Option<u32>,
        moves_to_go: Option<u32>,
        depth: Option<u32>,
        nodes: Option<u32>,
        mate: Option<u32>,
        move_time: Option<u32>,
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
                None,
            )
        });
    }

    pub fn print_state(&self) {
        println!(
            "{} {} {} {}",
            self.board.board().to_string(),
            self.debug.load(Ordering::Relaxed),
            self.searching.load(Ordering::Relaxed),
            self.pondering.load(Ordering::Relaxed)
        );
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
