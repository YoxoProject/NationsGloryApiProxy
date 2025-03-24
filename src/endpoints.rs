use crate::utils::{api_request, ApiKeys, QueuedRequest};
use rocket::{get, State};
use rocket::serde::json::Json;
use serde_json::Value;
use tokio::sync::mpsc;

#[get("/notations?<week>&<server>&<country>")]
pub async fn get_notations(
    queue: &State<mpsc::Sender<QueuedRequest>>,
    redis_client: &State<redis::Client>,
    api_keys: ApiKeys,
    week: String,
    server: String,
    country: Option<String>,
) -> Result<Json<Value>, rocket::http::Status> {
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
        if let Ok(response) = response {
            let filtered_notations = response
                .as_array().unwrap()
                .into_iter()
                .filter(|n| n["pays"].as_str().unwrap().to_lowercase() == country.to_lowercase())
                .collect::<Vec<_>>();
            return Ok(Json(serde_json::to_value(filtered_notations).unwrap()));
        }
    }
    response // Soit si country est None, soit si la requête à échouer (Err)
}

#[get("/country/<server>/<country>")]
pub async fn get_country(
    queue: &State<mpsc::Sender<QueuedRequest>>,
    redis_client: &State<redis::Client>,
    api_keys: ApiKeys,
    server: &str,
    country: &str,
) -> Result<Json<Value>, rocket::http::Status> {
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
