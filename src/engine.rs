use crate::search;
use shakmaty::{Chess, Position};

pub struct Engine {
    pub board: Chess,
    debug: bool,
    searching: bool,
    pondering: bool,
}

impl Engine {
    pub fn new() -> Engine {
        Engine {
            board: Chess::default(),
            debug: false,
            searching: false,
            pondering: false,
        }
    }

    pub fn search(&self) {
        search::search()
    }

    pub fn print_state(&self) {
        println!(
            "{} {} {} {}",
            self.board.board().to_string(),
            self.debug,
            self.searching,
            self.pondering
        );
    }

    pub fn debug(&mut self, enable: &bool) {
        self.debug = *enable;
    }

    pub fn reset(&mut self) {
        self.searching = false;
        self.pondering = false;

        self.board = Chess::default()
    }

    pub fn stop(&mut self) {
        self.searching = false;
        self.pondering = false;
    }

    pub fn ponder_hit(&mut self) {}

    pub fn set_option(&self) {}
}
