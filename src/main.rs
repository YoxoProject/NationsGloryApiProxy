use std::env;
use crate::endpoints::{get_country, get_notations};
use crate::utils::RequestQueue;
use crate::worker::process_requests;
use rocket::routes;
use std::sync::Arc;
use dotenv::dotenv;
use tokio::sync::mpsc;

mod endpoints;
mod utils;
mod worker;

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    dotenv().ok(); // Charge le fichier .env
    let redis_url = env::var("REDIS_URL").expect("REDIS_URL must be set");

    let (tx, rx) = mpsc::channel(100);
    let queue = Arc::new(RequestQueue::new());
    let redis_client = redis::Client::open(redis_url).unwrap();

    // Lancer la tâche de worker dans un contexte async
    let worker_rx = rx;
    let worker_queue = queue.clone();
    let worker_redis = redis_client.clone();
    tokio::spawn(async move {
        process_requests(worker_rx, worker_queue, worker_redis).await;
    });

    rocket::build()
        .manage(tx)
        .manage(redis_client)
        .mount("/", routes![get_notations, get_country])
        .launch()
        .await?;

    Ok(())
}
