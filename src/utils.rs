use dashmap::DashMap;
use redis::AsyncCommands;
use rocket::{Request, State};
use std::time::{Duration, Instant};
use rocket::request::{FromRequest, Outcome};
use tokio::sync::{mpsc, oneshot};

#[derive(Debug)]
pub struct QueuedRequest {
    pub url: String,
    pub method: String,
    pub body: Option<String>,
    pub api_keys: Vec<String>,
    pub response_channel: Option<oneshot::Sender<String>>,
}

pub struct RequestQueue {
    last_usage: DashMap<String, Instant>, // Associe une clé API à son dernier usage
}

impl RequestQueue {
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
) -> Result<String, rocket::http::Status> {
    // Vérification du cache Redis
    let mut redis_conn = redis_client.get_multiplexed_async_connection().await.unwrap();
    let cache_key = format!("cache:{}", request.url);
    if let Ok(cached_response) = redis_conn.get(&cache_key).await {
        return Ok(cached_response);
    }

    // Envoi au worker et attente de la réponse
    let (tx, rx) = oneshot::channel();
    let queued_request = QueuedRequest {
        response_channel: Some(tx),
        ..request
    };

    queue.send(queued_request).await.unwrap();

    match rx.await {
        Ok(response) => Ok(response),
        Err(_) => Err(rocket::http::Status::InternalServerError),
    }
}
