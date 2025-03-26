use crate::endpoints::{get_country, get_country_list, get_hdv, get_ngisland_list, get_notations, get_planning, get_playercount, get_user};
use crate::utils::ApiKeyUsage;
use crate::worker::process_requests_v2;
use dotenv::dotenv;
use rocket::{get, routes};
use std::env;
use std::path::Path;
use std::sync::Arc;
use rocket::fs::NamedFile;
use tokio::sync::{broadcast, mpsc};

mod endpoints;
mod utils;
mod worker;

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    dotenv().ok(); // Charge le fichier .env
    let redis_url = env::var("REDIS_URL").expect("REDIS_URL must be set");

    let (queue_tx, queue_rx) = mpsc::channel(100);
    let (response_broadcast_tx, _) = broadcast::channel(100);
    let api_key_usage = Arc::new(ApiKeyUsage::new());
    let redis_client = redis::Client::open(redis_url).unwrap();

    // Lancer la tâche de worker dans un contexte async
    let worker_redis = redis_client.clone();
    let worker_response_broadcast_tx = response_broadcast_tx.clone();
    tokio::spawn(async move {
        process_requests_v2(
            queue_rx,
            worker_response_broadcast_tx,
            api_key_usage,
            worker_redis,
        )
        .await;
    });

    rocket::build()
        .manage(queue_tx)
        .manage(response_broadcast_tx)
        .manage(redis_client)
        .mount("/", routes![home_page, get_planning, get_playercount, get_hdv, get_notations, get_country, get_country_list, get_user, get_ngisland_list])
        .launch()
        .await?;

    Ok(())
}
#[get("/")]
async fn home_page() -> Option<NamedFile> {
    NamedFile::open(Path::new("static/index.html")).await.ok()
}
