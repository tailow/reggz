use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

pub fn search_for_ms(move_time: u64, searching: Arc<AtomicBool>) {
    let start_time = Instant::now();

    let duration = Duration::from_millis(move_time);

    while searching.load(Ordering::Relaxed) {
        if start_time.elapsed() > duration {
            searching.store(false, Ordering::Relaxed);

            return;
        }

        thread::sleep(Duration::from_millis(10));
    }
}
