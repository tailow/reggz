use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, SystemTime},
};

pub fn timer(remaining: u64, increment: u64, searching: Arc<AtomicBool>) {
    let start_time = SystemTime::now();

    let duration = Duration::from_millis(remaining / 20 + increment / 2);

    while searching.load(Ordering::Relaxed) {
        match start_time.elapsed() {
            Ok(elapsed) => {
                if elapsed > duration {
                    searching.store(false, Ordering::Relaxed);

                    return;
                }

                thread::sleep(Duration::from_millis(10));
            }
            _ => (),
        }
    }
}
