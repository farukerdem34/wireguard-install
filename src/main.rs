#![recursion_limit = "256"]

mod checks;
mod client;
mod enums;
mod initialization;
mod install;
mod models;
mod os_detection;
mod uninstall;
mod utils;

use crate::initialization::initial_check;

#[tokio::main]
async fn main() {
    let _ = initial_check().await.expect("Initial checks failed");
}
