use surf::{Client, Config};

use std::time::Duration;

lazy_static::lazy_static! {
   pub static ref CLIENT: Client = Config::new()
    .set_timeout(Some(Duration::from_secs(5)))
    .try_into().unwrap();
}
