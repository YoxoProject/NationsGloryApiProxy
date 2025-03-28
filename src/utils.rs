use dashmap::DashMap;
use redis::AsyncCommands;
use rocket::request::{FromRequest, Outcome};
use rocket::serde::json::Json;
use rocket::{Request, State};
use serde_json::Value;
use std::time::{Duration, Instant};
use chrono::NaiveDate;
use tokio::sync::{broadcast, mpsc};

#[derive(Debug, Clone)]
pub struct QueuedRequest {
    pub url: String,
    pub method: String,
    pub api_keys: Vec<String>,
    pub cache_time: Option<u64>
}

impl QueuedRequest {
    // Fonction pour insérer une requête dans la file d'attente
    // Si une requête avec la même URL et le même verbe HTTP existe déjà, on ajoute des clés API à la requête existante afin de lui donner plus de chances d'être exécutée
    // Sinon, on ajoute la nouvelle requête à la file d'attente tout simplement
    pub fn insert_request_to_queue(list: &mut Vec<QueuedRequest>, new_request: QueuedRequest) {
        if let Some(existing) = list
            .iter_mut()
            .find(|req| req.url == new_request.url && req.method == new_request.method)
        {
            for key in new_request.api_keys {
                if !existing.api_keys.contains(&key) {
                    existing.api_keys.push(key);
                }
            }
        } else {
            list.push(new_request);
        }
    }
}

#[derive(Debug, Clone)]
pub struct RequestResponse {
    pub url: String,
    pub method: String,
    pub body: Value,
}

pub struct ApiKeyUsage {
    last_usage: DashMap<String, Instant>, // Associe une clé API à son dernier usage
}

impl ApiKeyUsage {
    pub fn new() -> Self {
        Self {
            last_usage: DashMap::new(),
        }
    }

    pub fn can_execute(&self, api_key: &String) -> bool {
        if let Some(last_time) = self.last_usage.get(api_key) {
            return last_time.elapsed() >= Duration::from_millis(500);
        }
        true
    }

    pub fn update_usage(&self, api_key: String) {
        self.last_usage.insert(api_key, Instant::now());
    }
}

pub struct ApiKeys(pub Vec<String>);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ApiKeys {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match req.headers().get_one("Authorization") {
            Some(keys) if !keys.is_empty() => {
                let keys_vec = keys.split(',').map(String::from).collect();
                Outcome::Success(ApiKeys(keys_vec))
            }
            _ => Outcome::Error((rocket::http::Status::BadRequest, ())),
        }
    }
}

pub async fn api_request(
    queue: &State<mpsc::Sender<QueuedRequest>>,
    redis_client: &State<redis::Client>,
    request: QueuedRequest,
    response_broadcast_tx: &State<broadcast::Sender<RequestResponse>>,
) -> Result<Json<Value>, rocket::http::Status> {
    // Vérification du cache Redis
    let mut redis_conn = redis_client
        .get_multiplexed_async_connection()
        .await
        .unwrap();
    let cache_key = format!("cache:{}", request.url);
    if let Ok(cached_response) = redis_conn.get::<_, String>(&cache_key).await {
        if let Ok(json_value) = serde_json::from_str::<Value>(&cached_response) {
            return Ok(Json(json_value));
        }
    }

    let url = request.url.clone();
    let method = request.method.clone();

    let mut rx = response_broadcast_tx.subscribe();

    queue.send(request).await.unwrap();

    while let Ok(response) = rx.recv().await {
        if response.url == url && response.method == method {
            return Ok(Json(response.body));
        }
    }
    Err(rocket::http::Status::InternalServerError)
}

pub fn get_week_number_from_date(date: NaiveDate) -> i64 {
    let ref_date = NaiveDate::from_ymd_opt(1970, 1, 12).unwrap();
    let diff_in_days = date.signed_duration_since(ref_date).num_days();
    (diff_in_days / 7) + 1
}

pub fn get_current_week_number() -> i64 {
    let today = chrono::Utc::now().naive_utc().date();
    get_week_number_from_date(today)
}

pub fn get_cache_time_from_week_number(week_number: i64) -> Option<u64> {
    let current_week = get_current_week_number();
    if (week_number < current_week) && week_number != -1 {
        Some(60 * 60 * 24 * 30 * 2) // 2 mois
    } else {
        None
    }
}