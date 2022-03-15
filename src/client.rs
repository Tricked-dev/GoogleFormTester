use reqwest::Client;

lazy_static::lazy_static! {
   pub static ref CLIENT: Client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/60.0.3112.113 Safari/537.36")
        .build()
        .unwrap();
}
