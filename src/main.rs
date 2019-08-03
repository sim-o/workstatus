use std::sync::mpsc::channel;
use std::{thread, time::Duration};

mod gitlab;
mod macos;

use std::sync::mpsc::Sender;
use crate::macos::OSXStatusBar;
use std::process::exit;

pub type NSCallback = Box<dyn Fn(u64, &Sender<String>)>;

fn main() {
//    println!("merge requests: {:?}", gitlab::merge_request_count().unwrap());

    let (tx, rx) = channel::<String>();

    let mut status_bar = OSXStatusBar::new(tx);
    let cb: NSCallback = Box::new(move |sender, tx| {
        tx.send("quit".to_string());
    });
    let _ = status_bar.add_item(None, "Quit", cb, false);

    let stopper = status_bar.app.stopper();

    let child = thread::spawn(move || {
        for msg in rx.iter() {
            match msg.as_str() {
                "quit" => exit(0),
                _ => panic!("unexpected message"),
            }
        }
    });

    status_bar.run(true);
}
