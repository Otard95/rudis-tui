use std::{
    thread,
    time::{Duration, Instant},
    sync::mpsc::{Sender, Receiver}
};
use crossterm::event::{self, Event as CEvent, KeyEvent};

use crate::app::App;

pub enum Event<I> {
    Input(I),
    Tick,
}

pub fn start_input_thread(tx: Sender<Event<KeyEvent>>, tick_rate: Duration) {
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout).expect("poll works") {
                if let CEvent::Key(key) = event::read().expect("can real events") {
                    tx.send(Event::Input(key)).expect("can send events");
                }
            }

            if last_tick.elapsed() >= tick_rate {
                if let Ok(_) = tx.send(Event::Tick) {
                    last_tick = Instant::now();
                }
            }
        }
    });
}

pub fn handle_input(app: &mut App, rx: &Receiver<Event<KeyEvent>>) {
        match rx.recv() {
            Ok(rec) => match rec {
                Event::Input(event) => app.handle_input(event),
                Event::Tick => {}
            },
            Err(e) => {
                println!("rx.recv() failed: {}", e);
            }
        }
}
