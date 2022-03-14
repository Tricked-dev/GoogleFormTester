use clap::Parser;
use comfy_table::presets::UTF8_FULL;
use comfy_table::*;
use futures::{stream, StreamExt};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
use tracing::debug;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(short, long)]
    /// url to test on
    url: String,

    #[clap(short, long, default_value_t = 5000)]
    /// Amount of times to test
    times: usize,

    #[clap(short, long, default_value_t = 8)]
    // Thread count
    threads: usize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    println!("[-] fetching {} for {} times", cli.url, cli.times);
    tracing_subscriber::fmt::init();
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/60.0.3112.113 Safari/537.36")
        .build()
        .unwrap();

    let started = Instant::now();

    let (success, failed) = (Arc::new(Mutex::new(0)), Arc::new(Mutex::new(0)));

    let urls = vec![cli.url; cli.times];
    let bodies = stream::iter(urls)
        .map(|url| {
            let client = &client;
            let (success_clone, failed_clone) = (Arc::clone(&success), Arc::clone(&failed));
            async move {
                let r = client.get(url).send().await;
                match r {
                    Ok(r) => {
                        let mut suc = success_clone.lock().unwrap();
                        *suc += 1;
                        debug!("req s: {}", r.status().as_u16())
                    }
                    Err(e) => {
                        let mut fail = failed_clone.lock().unwrap();
                        *fail += 1;
                        debug!("{e:?}")
                    }
                };
            }
        })
        .buffer_unordered(cli.threads);

    bodies.for_each(|_| async {}).await;

    let (f, s) = (*failed.lock().unwrap(), *success.lock().unwrap());
    let now = Instant::now();
    let elapsed = now.duration_since(started);
    let mut table = Table::new();

    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .add_row(vec![
            Cell::new("Time Elapsed"),
            Cell::new(format!("{}", elapsed.as_millis() as f64 / 1000.0)),
        ])
        .add_row(vec![
            Cell::new("Speed"),
            Cell::new(format!(
                "{} req/s",
                s as f64 / (elapsed.as_millis() as f64 / 1000.0)
            )),
        ])
        .add_row(vec![
            Cell::new("Amount of requests"),
            Cell::new(format!(
                "total: {} succes % {}",
                s,
                if f == 0 || s == 0 { 100 } else { s / f * 100 }
            )),
        ]);
    println!("{table}");
    Ok(())
}
