use comfy_table::presets::UTF8_FULL;
use comfy_table::*;
use rayon::prelude::*;
use reqwest::blocking::Client;
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
use tracing::debug;

lazy_static::lazy_static! {
    static ref CLIENT:Client = reqwest::blocking::Client::builder()
    .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/60.0.3112.113 Safari/537.36")
    .build().unwrap();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // let url = "http://192.168.178.11:8083/";
    let url = "https://ascella.wtf/v2/ascella/view/zBNwf9q";
    let times = 5000;
    let started = Instant::now();

    let (success, failed) = (Arc::new(Mutex::new(0)), Arc::new(Mutex::new(0)));

    (0..times).into_par_iter().for_each(|_| {
        let (success_clone, failed_clone) = (Arc::clone(&success), Arc::clone(&failed));
        let r = CLIENT.get(url).send();
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
    });
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
