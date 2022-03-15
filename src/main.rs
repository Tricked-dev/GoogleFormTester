use crate::{client::CLIENT, forms::GoogleFormSpammer};
use clap::Parser;
use comfy_table::presets::UTF8_FULL;
use comfy_table::*;
use futures::{stream, StreamExt};
use tracing::debug;

use std::{
    sync::{Arc, Mutex},
    time::Instant,
    vec,
};

mod client;
mod forms;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(short, long)]
    /// url to test on
    url: String,

    #[clap(short, long, default_value_t = 5000)]
    /// Amount of times to test
    times: usize,

    #[clap(short, long)]
    /// Weather or not this is a google form
    google: bool,

    #[clap(short, long)]
    /// Only do required parts with google forms.
    required: bool,

    #[clap(short, long, default_value_t = 8)]
    // Thread count
    parallel: usize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let begin = Instant::now();
    let cli = Cli::parse();
    println!("[-] fetching {} for {} times", cli.url, cli.times);
    tracing_subscriber::fmt::init();

    let spammer = if cli.google {
        let mut spammer = GoogleFormSpammer::new(cli.url.clone(), cli.required);
        spammer._scrape_form().await;
        Some(Arc::new(spammer))
    } else {
        None
    };

    let started = Instant::now();

    let (success, failed) = (Arc::new(Mutex::new(0)), Arc::new(Mutex::new(0)));

    let urls = vec![cli.url; cli.times];
    let bodies = stream::iter(urls)
        .map(|url| {
            let (success_clone, failed_clone) = (Arc::clone(&success), Arc::clone(&failed));
            let spammer = spammer.as_ref().map(Arc::clone);
            async move {
                if cli.google {
                    let r = spammer.unwrap().post_data().await;
                    match r {
                        true => {
                            let mut suc = success_clone.lock().unwrap();
                            *suc += 1;
                        }
                        false => {
                            let mut fail = failed_clone.lock().unwrap();
                            *fail += 1;
                        }
                    }
                } else {
                    let r = CLIENT.get(url).send().await;
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
            }
        })
        .buffer_unordered(cli.parallel);

    bodies.for_each(|_| async {}).await;

    let (f, s) = (*failed.lock().unwrap(), *success.lock().unwrap());
    let now = Instant::now();
    let elapsed = now.duration_since(started).as_millis();
    let mut table = Table::new();

    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .add_row(vec![
            Cell::new("Time Elapsed"),
            Cell::new(format!(
                "{} total: {}",
                elapsed as f64 / 1000.0,
                now.duration_since(begin).as_millis() as f64 / 1000.0
            )),
        ])
        .add_row(vec![
            Cell::new("Speed"),
            Cell::new(format!("{} req/s", s as f64 / (elapsed as f64 / 1000.0))),
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
