use std::sync::mpsc::channel;
use std::{thread, time};

mod gitlab;
mod macos;

use std::sync::mpsc::Sender;
use crate::macos::OSXStatusBar;

pub type NSCallback = Box<dyn Fn(u64, &Sender<String>)>;

fn main() {
//    println!("merge requests: {:?}", gitlab::merge_request_count().unwrap());

    let (tx, rx) = channel::<String>();

    let child = thread::spawn(move || {
        loop {
            thread::sleep(time::Duration::from_millis(1000));
            println!(".");
        }
    });

    let mut status_bar = OSXStatusBar::new(tx);
    loop {
        status_bar.run(false);
    }
    child.join().expect("child panicked");
}
