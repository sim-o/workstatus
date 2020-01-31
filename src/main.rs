use std::{thread, time::Duration};
use std::process::exit;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;

use crate::config::{Config, read_config};
use crate::gitlab::{Gitlab, PipelineStatus};
use crate::macos::OSXStatusBar;

mod config;
mod gitlab;
mod macos;

pub type NSCallback = Box<dyn Fn(u64, &Sender<String>)>;

fn main() {
    let config = read_config().expect("error reading config.toml");

    let (tx_query, rx_query) = channel::<String>();

    let mut status_bar = {
        let mut status_bar = OSXStatusBar::new(&config.title, tx_query.clone());
        {
            let cb: NSCallback = Box::new(move |_sender, tx| {
                tx.send("manual".to_string()).expect("manual refresh send failed");
            });
            let _ = status_bar.add_item(None, "Refresh", cb, false);
        }
        {
            let cb: NSCallback = Box::new(move |_sender, _tx| {
                exit(0);
            });
            let _ = status_bar.add_item(None, "Quit", cb, false);
        }

        let tx_query_manual = tx_query.clone();
        thread::spawn(move || {
            loop {
                tx_query_manual.send("interval".to_string()).expect("interval send failed");
                thread::sleep(Duration::from_millis(60_000));
            }
        });
        status_bar
    };

    let rx = {
        let (tx, rx) = channel::<String>();
        let stopper = status_bar.stopper();
        thread::spawn(move || {
            let gl = &mut Gitlab::new(
                config.gitlab_url.as_str(),
                config.token.as_str(),
            );

            for reason in rx_query.iter() {
                println!("refreshing: {:}", reason);
                let title = make_title(&config, gl);
                tx.send(title).expect("worker send failed");
                stopper.stop();
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

fn make_title(config: &Config, gl: &mut Gitlab) -> String {
    let projects = &config.project;
    let title: String = projects
        .iter()
        .map(|p| {
            let requires_merge = gl
                .merge_request_count(&p.name, &p.ignore_users)
                .map(|i| format!("{:}", i))
                .unwrap_or("⨳".to_string());

            let status = gl
                .pipeline_status(&p.name, "master")
                .map(status_icon)
                .unwrap_or_else(|e| {
                    println!("error: {:?}", e);
                    "?"
                });

            let merge_requests: String = gl
                .user_merge_requests(&p.name, &p.branch_users)
                .map(|v| {
                    v.iter()
                        .map(|mrs| format!("{:}{:}", mrs.branch, status_icon(mrs.status)))
                        .collect()
                })
                .unwrap_or_else(|e| {
                    println!("error: {:?}", e);
                    "⨳".to_string()
                });

            let mut title = String::new();
            if status != "" || requires_merge != "0" || merge_requests != "" {
                title.push_str(&p.title);
                if status != "" {
                    title.push_str(status);
                }
                if requires_merge != "0" {
                    title.push_str(&*requires_merge);
                }
                if merge_requests != "" {
                    title.push_str(&*merge_requests);
                }
            }
            title
        })
        .collect();

    if title == "" {
        config.title.to_string()
    } else {
        title
    }
}

fn status_icon(status: PipelineStatus) -> &'static str {
    match status {
        PipelineStatus::Running => "🏃",
        PipelineStatus::Pending => "🕗",
        PipelineStatus::Success => "",
        PipelineStatus::Failed => "💩",
        PipelineStatus::Canceled => "✋",
        PipelineStatus::Skipped => "⦳",
        PipelineStatus::Manual => "",
    }
}
