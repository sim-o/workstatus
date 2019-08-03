use std::sync::mpsc::channel;

mod gitlab;
mod macos;

fn main() {
    println!("merge requests: {:?}", gitlab::merge_request_count().unwrap());

    let (tx, rx) = channel::<String>();
    let mut status = connectr::StatusBar::new(tx.clone());
}
