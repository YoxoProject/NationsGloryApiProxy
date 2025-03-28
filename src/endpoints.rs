use crate::utils::{api_request, get_cache_time_from_week_number, ApiKeys, QueuedRequest, RequestResponse};
use rocket::serde::json::Json;
use rocket::{get, State};
use serde_json::{json, Value};
use tokio::sync::{broadcast, mpsc};

#[get("/planning?<server>&<month>&<year>")]
pub async fn get_planning(
    queue: &State<mpsc::Sender<QueuedRequest>>,
    response_broadcast_tx: &State<broadcast::Sender<RequestResponse>>,
    redis_client: &State<redis::Client>,
    api_keys: ApiKeys,
    server: &str,
    month: &str,
    year: &str,
) -> Result<Json<Value>, rocket::http::Status> {
    if api_keys.0.is_empty() {
        return Err(rocket::http::Status::BadRequest);
    }

    let server = server.to_lowercase();
    let month = month.to_lowercase();
    let year = year.to_lowercase();

    let url = format!(
        "https://publicapi.nationsglory.fr/planning?server={}&month={}&year={}",
        server, month, year
    );

    let request = QueuedRequest {
        url,
        method: "GET".to_string(),
        api_keys: api_keys.0,
        cache_time: None,
    };

    api_request(queue, redis_client, request, response_broadcast_tx).await
}

#[get("/playercount")]
pub async fn get_playercount(
    queue: &State<mpsc::Sender<QueuedRequest>>,
    response_broadcast_tx: &State<broadcast::Sender<RequestResponse>>,
    redis_client: &State<redis::Client>,
    api_keys: ApiKeys,
) -> Result<Json<Value>, rocket::http::Status> {
    if api_keys.0.is_empty() {
        return Err(rocket::http::Status::BadRequest);
    }

    let url = "https://publicapi.nationsglory.fr/playercount".to_string();

    let request = QueuedRequest {
        url,
        method: "GET".to_string(),
        api_keys: api_keys.0,
        cache_time: Some(60),
    };

    api_request(queue, redis_client, request, response_broadcast_tx).await
}

#[get("/hdv/<server>/list")]
pub async fn get_hdv(
    queue: &State<mpsc::Sender<QueuedRequest>>,
    response_broadcast_tx: &State<broadcast::Sender<RequestResponse>>,
    redis_client: &State<redis::Client>,
    api_keys: ApiKeys,
    server: &str,
) -> Result<Json<Value>, rocket::http::Status> {
    if api_keys.0.is_empty() {
        return Err(rocket::http::Status::BadRequest);
    }

    let server = server.to_lowercase();

    let url = format!("https://publicapi.nationsglory.fr/hdv/{}/list", server);

    let request = QueuedRequest {
        url,
        method: "GET".to_string(),
        api_keys: api_keys.0,
        cache_time: None,
    };

    api_request(queue, redis_client, request, response_broadcast_tx).await
}

#[get("/notation?<week>", rank = 1)]
pub async fn get_all_notations(
    queue: &State<mpsc::Sender<QueuedRequest>>,
    response_broadcast_tx: &State<broadcast::Sender<RequestResponse>>,
    redis_client: &State<redis::Client>,
    api_keys: ApiKeys,
    week: &str,
) -> Result<Json<Value>, rocket::http::Status> {
    if api_keys.0.is_empty() {
        return Err(rocket::http::Status::BadRequest);
    }

    let week = week.to_lowercase();

    let url = format!(
        "https://publicapi.nationsglory.fr/notations?week={}",
        week
    );

    let week_number = week.parse::<i64>();
    let cache_time = get_cache_time_from_week_number(week_number.unwrap_or(-1));

    let request = QueuedRequest {
        url,
        method: "GET".to_string(),
        api_keys: api_keys.0,
        cache_time
    };

    api_request(queue, redis_client, request, response_broadcast_tx).await
}

#[get("/notations?<week>&<server>&<country>", rank = 2)]
pub async fn get_notations(
    queue: &State<mpsc::Sender<QueuedRequest>>,
    response_broadcast_tx: &State<broadcast::Sender<RequestResponse>>,
    redis_client: &State<redis::Client>,
    api_keys: ApiKeys,
    week: &str,
    server: &str,
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

    let week_number = week.parse::<i64>();
    let cache_time = get_cache_time_from_week_number(week_number.unwrap_or(-1));

    let request = QueuedRequest {
        url,
        method: "GET".to_string(),
        api_keys: api_keys.0,
        cache_time,
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
                    response.insert(
                        "data".to_string(),
                        serde_json::to_value(filtered_notations).unwrap(),
                    );
                    return Ok(Json(json!(response)));
                }
            }
            return Ok(response); // Si la réponse n'est pas un tableau, on la renvoie telle quelle (c'est que le json est inattendu)
        }
    }
    response // Soit si country est None, soit si la requête à échouer (Err)
}

#[get("/country/<server>/<country>", rank = 2)]
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
        server, country
    );

    let request = QueuedRequest {
        url,
        method: "GET".to_string(),
        api_keys: api_keys.0,
        cache_time: None,
    };

    api_request(queue, redis_client, request, response_broadcast_tx).await
}

#[get("/country/list/<server>", rank = 1)]
pub async fn get_country_list(
    queue: &State<mpsc::Sender<QueuedRequest>>,
    response_broadcast_tx: &State<broadcast::Sender<RequestResponse>>,
    redis_client: &State<redis::Client>,
    api_keys: ApiKeys,
    server: &str,
) -> Result<Json<Value>, rocket::http::Status> {
    if api_keys.0.is_empty() {
        return Err(rocket::http::Status::BadRequest);
    }

    let server = server.to_lowercase();

    let url = format!("https://publicapi.nationsglory.fr/country/list/{}", server);

    let request = QueuedRequest {
        url,
        method: "GET".to_string(),
        api_keys: api_keys.0,
        cache_time: None,
    };

    api_request(queue, redis_client, request, response_broadcast_tx).await
}

#[get("/user/<username>")]
pub async fn get_user(
    queue: &State<mpsc::Sender<QueuedRequest>>,
    response_broadcast_tx: &State<broadcast::Sender<RequestResponse>>,
    redis_client: &State<redis::Client>,
    api_keys: ApiKeys,
    username: &str,
) -> Result<Json<Value>, rocket::http::Status> {
    if api_keys.0.is_empty() {
        return Err(rocket::http::Status::BadRequest);
    }

    let username = username.to_lowercase();

    let url = format!("https://publicapi.nationsglory.fr/user/{}", username);

    let request = QueuedRequest {
        url,
        method: "GET".to_string(),
        api_keys: api_keys.0,
        cache_time: None,
    };

    api_request(queue, redis_client, request, response_broadcast_tx).await
}

#[get("/ngisland/list?<page>")]
pub async fn get_ngisland_list(
    queue: &State<mpsc::Sender<QueuedRequest>>,
    response_broadcast_tx: &State<broadcast::Sender<RequestResponse>>,
    redis_client: &State<redis::Client>,
    api_keys: ApiKeys,
    page: &str,
) -> Result<Json<Value>, rocket::http::Status> {
    if api_keys.0.is_empty() {
        return Err(rocket::http::Status::BadRequest);
    }

    let url = format!(
        "https://publicapi.nationsglory.fr/ngisland/list?page={}",
        page
    );

    let request = QueuedRequest {
        url,
        method: "GET".to_string(),
        api_keys: api_keys.0,
        cache_time: None,
    };

    api_request(queue, redis_client, request, response_broadcast_tx).await
}
