use std::sync::mpsc::channel;
use std::{thread, time::Duration};

mod gitlab;
mod macos;
mod config;

use std::sync::mpsc::Sender;
use crate::macos::OSXStatusBar;
use std::process::exit;
use crate::config::read_config;
use crate::gitlab::{Gitlab, PipelineStatus};

pub type NSCallback = Box<dyn Fn(u64, &Sender<String>)>;

fn main() {
    let config = read_config().expect("error reading config.toml");

    let mut status_bar = {
        let (tx, rx) = channel::<String>();
        let mut status_bar = OSXStatusBar::new(tx);
        let cb: NSCallback = Box::new(move |_sender, tx| {
            tx.send("quit".to_string());
        });
        let _ = status_bar.add_item(None, "Quit", cb, false);

        thread::spawn(move || {
            for msg in rx.iter() {
                match msg.as_str() {
                    "quit" => exit(0),
                    _ => panic!("unexpected message"),
                }
            }
        });
        status_bar
    };

    let rx = {
        let (tx, rx) = channel::<String>();
        let stopper = status_bar.stopper();
        thread::spawn(move || {
            let mut gl = Gitlab::new(
                config.gitlab_url.as_str(),
                config.token.as_str(),
                config.project_name.as_str());

            loop {
                let requires_merge = gl.merge_request_count(&config.ignore_users)
                    .map(|i| format!("{:}", i))
                    .unwrap_or("⨳".to_string());

                let master_status = gl.pipeline_status("master")
                    .unwrap_or(PipelineStatus::Success);
                let status_char = match master_status {
                    PipelineStatus::Running => "🏃",
                    PipelineStatus::Pending => "🕗",
                    PipelineStatus::Success => "👍",
                    PipelineStatus::Failed => "💩",
                    PipelineStatus::Canceled => "⏹",
                    PipelineStatus::Skipped => "⦳",
                };

                tx.send(format!("{:}: M:{:} A:{:}", config.project_name, status_char, requires_merge));
                stopper.stop();
                thread::sleep(Duration::from_millis(60_000));
            }
        });
        rx
    };

    loop {
        status_bar.run(true);
        if let Ok(title) = rx.try_recv() {
            status_bar.set_title(title.as_str());
        }
    }
}
