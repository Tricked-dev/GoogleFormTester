use futures::future;
use humantime::format_duration;
use hyper::Client;
use hyper_tls::HttpsConnector;
use std::iter;
use std::{sync::Arc, time::Instant};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    start().await
}

async fn start() -> Result<(), Box<dyn std::error::Error>> {
    let url = "https://ascella.wtf/v2/ascella/view/zBNwf9q";
    let times = 500;
    let threads = 50;
    let started = Instant::now();
    // let https = HttpsConnector::new();
    // let client = Arc::new(Client::builder().build::<_, hyper::Body>(https));

    let r = future::join_all(iter::repeat(0).take(threads).map(|thread| {
        // let times = times;
        tokio::spawn(async move {
            let https = HttpsConnector::new();
            let client_arc = Arc::new(Client::builder().build::<_, hyper::Body>(https));
            let client = client_arc.clone();
            async move {
                future::join_all(iter::repeat(0).take(times / threads).map(|_| {
                    let client = client.clone();
                    async move {
                        println!("Starting req thread {thread}");
                        let r = &client.get(url.parse().unwrap()).await.unwrap();
                        println!("Ending req {}", r.status());
                    }
                }))
                .await;
            }
        })
    }))
    .await;
    for handle in r {
        handle.unwrap().await;
    }

    let now = Instant::now();
    let elapsed = now.duration_since(started);
    println!(
        "elapsed: {} reqs/s {}",
        format_duration(elapsed).to_string(),
        elapsed.as_secs() / times as u64
    );
    Ok(())
}
