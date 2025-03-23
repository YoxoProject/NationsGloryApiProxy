use crate::utils::{api_request, ApiKeys, QueuedRequest};
use rocket::{get, State};
use tokio::sync::mpsc;

#[get("/notations?<week>&<server>&<country>")]
pub async fn get_notations(
    queue: &State<mpsc::Sender<QueuedRequest>>,
    redis_client: &State<redis::Client>,
    api_keys: ApiKeys,
    week: String,
    server: String,
    country: Option<String>,
) -> Result<String, rocket::http::Status> {
    if api_keys.0.is_empty() {
        return Err(rocket::http::Status::BadRequest);
    }

    let week = week.to_lowercase();
    let server = server.to_lowercase();
    let country = country.map(|c| c.to_lowercase());

    let url = format!(
        "https://publicapi.nationsglory.fr/notations?week={}&server={}",
        week, server,
    );

    let request = QueuedRequest {
        url,
        method: "GET".to_string(),
        body: None,
        api_keys: api_keys.0,
        response_channel: None,
    };

    let response = api_request(queue, redis_client, request).await;

    if country.is_some() {
        let country = country.unwrap();
        let notations: Vec<serde_json::Value> = serde_json::from_str(&response?).unwrap();
        let filtered_notations: Vec<serde_json::Value> = notations
            .into_iter()
            .filter(|n| n["pays"].as_str().unwrap().to_lowercase() == country.to_lowercase())
            .collect();
        Ok(serde_json::to_string(&filtered_notations).unwrap())
    } else {
        response
    }
}

#[get("/country/<server>/<country>")]
pub async fn get_country(
    queue: &State<mpsc::Sender<QueuedRequest>>,
    redis_client: &State<redis::Client>,
    api_keys: ApiKeys,
    server: &str,
    country: &str,
) -> Result<String, rocket::http::Status> {
    if api_keys.0.is_empty() {
        return Err(rocket::http::Status::BadRequest);
    }

    let server = server.to_lowercase();
    let country = country.to_lowercase();

    let url = format!(
        "https://publicapi.nationsglory.fr/country/{}/{}",
        server,
        country
    );

    let request = QueuedRequest {
        url,
        method: "GET".to_string(),
        body: None,
        api_keys: api_keys.0,
        response_channel: None,
    };

    api_request(queue, redis_client, request).await
}
