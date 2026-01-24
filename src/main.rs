mod enums;
mod models;
mod utils;
mod install;
mod checks;
mod os_detection;
mod initialization;

use crate::initialization::initial_check;

#[tokio::main]
async fn main() {
    let _ = initial_check().await.expect("Initial checks failed");
}