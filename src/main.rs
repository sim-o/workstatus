use std::sync::mpsc::channel;
use crate::macos::OSXStatusBar;

mod gitlab;
mod macos;

fn main() {
    println!("merge requests: {:?}", gitlab::merge_request_count().unwrap());

    let (tx, rx) = channel::<String>();
    let mut status = OSXStatusBar::new(tx.clone());
}
