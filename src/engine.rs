use crate::search;
use shakmaty::{Chess, Position};
use std::thread;

pub struct Engine {
    pub board: Chess,
    debug: bool,
    searching: bool,
    pondering: bool,
}

impl Engine {
    pub fn new() -> Engine {
        Engine {
            board: Chess::new(),
            debug: false,
            searching: false,
            pondering: false,
        }
    }

    pub fn search(&self) {
        thread::spawn(search::search);
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

    pub fn set_option(&self, name: &str, value: Option<&str>) {
        if let Some(optional_value) = value {
            println!("Set option {} with value {}", name, optional_value);
        } else {
            println!("Set option {}", name);
        }
    }

    pub fn print_info(&self) {}
}
