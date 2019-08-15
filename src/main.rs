use std::{thread, time::Duration};
use std::process::exit;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;

use crate::config::read_config;
use crate::gitlab::{Gitlab, PipelineStatus};
use crate::macos::OSXStatusBar;

mod gitlab;
mod macos;
mod config;

pub type NSCallback = Box<dyn Fn(u64, &Sender<String>)>;

fn main() {
    let config = read_config().expect("error reading config.toml");

    let mut status_bar = {
        let (tx, rx) = channel::<String>();
        let mut status_bar = OSXStatusBar::new(&config.title, tx);
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

                let status = gl.pipeline_status("master")
                    .map(status_icon)
                    .unwrap_or_else(|e| {
                        println!("error: {:?}", e);
                        "?"
                    });

                let merge_requests: String = gl.user_merge_requests(&config.branch_users)
                    .map(|v| v.iter()
                        .map(|mrs| format!("{:}{:}", mrs.branch, status_icon(mrs.status)))
                        .collect())
                    .unwrap_or_else(|e| {
                        println!("error: {:?}", e);
                        "⨳".to_string()
                    });

                let mut title = format!("{:} {:}", config.title, status);
                if requires_merge != "0" {
                    title.push(' ');
                    title.push_str(&*requires_merge);
                }
                if merge_requests != "" {
                    title.push(' ');
                    title.push_str(&*merge_requests);
                }
                tx.send(title);
                stopper.stop();
                thread::sleep(Duration::from_millis(60_000));
            }
        });
        rx
    };

    loop {
        status_bar.run(true);
        while let Ok(title) = rx.try_recv() {
            status_bar.set_title(title.as_str());
        }
    }
}

fn status_icon(status: PipelineStatus) -> &'static str {
    match status {
        PipelineStatus::Running => "🏃",
        PipelineStatus::Pending => "🕗",
        PipelineStatus::Success => "👍",
        PipelineStatus::Failed => "💩",
        PipelineStatus::Canceled => "✋",
        PipelineStatus::Skipped => "⦳",
        PipelineStatus::Manual => "👉",
    }
}
