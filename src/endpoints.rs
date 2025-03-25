use crate::utils::{api_request, ApiKeys, QueuedRequest, RequestResponse};
use rocket::{get, State};
use rocket::serde::json::Json;
use serde_json::{json, Value};
use tokio::sync::{broadcast, mpsc};

#[get("/notations?<week>&<server>&<country>")]
pub async fn get_notations(
    queue: &State<mpsc::Sender<QueuedRequest>>,
    response_broadcast_tx: &State<broadcast::Sender<RequestResponse>>,
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
        api_keys: api_keys.0,
    };

    let response = api_request(queue, redis_client, request, response_broadcast_tx).await;

    if country.is_some() {
        let country = country.unwrap();
        if let Ok(response) = response {
            if let Some(data) = response.get("data") {
                if let Some(notation) = data.as_array() {
                    let filtered_notations = notation
                        .into_iter()
                        .filter(|n| n["pays"].as_str().unwrap().to_lowercase() == country)
                        .collect::<Vec<_>>();
                    let mut response = response.as_object().unwrap().clone();
                    response.insert("data".to_string(), serde_json::to_value(filtered_notations).unwrap());
                    return Ok(Json(json!(response)));
                }
            }
            return Ok(response); // Si la réponse n'est pas un tableau, on la renvoie telle quelle (c'est que le json est inattendu)
        }
    }
    response // Soit si country est None, soit si la requête à échouer (Err)
}

#[get("/country/<server>/<country>")]
pub async fn get_country(
    queue: &State<mpsc::Sender<QueuedRequest>>,
    response_broadcast_tx: &State<broadcast::Sender<RequestResponse>>,
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
        api_keys: api_keys.0,
    };

    api_request(queue, redis_client, request, response_broadcast_tx).await
}
