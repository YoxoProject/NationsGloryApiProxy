use crate::utils::{ApiKeyUsage, QueuedRequest, RequestResponse};
use redis::AsyncCommands;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Mutex};

pub async fn process_requests_v2(
    mut queue_rx: mpsc::Receiver<QueuedRequest>,
    response_broadcast_tx: broadcast::Sender<RequestResponse>,
    api_key_usage: Arc<ApiKeyUsage>,
    redis_client: redis::Client,
) {
    let client = reqwest::Client::new();
    let mut waiting_requests: Vec<QueuedRequest> = Vec::new();
    let used_keys = Arc::new(Mutex::new(HashSet::new()));

    loop {
        // On commence par traiter les requêtes en attente
        received_queue(&mut queue_rx, &mut waiting_requests);

        // On traite les requêtes en attente: on vérifie lequel peuvent être executé puis on les exécuter dans un nouveau thread.
        // On se doit de veiller à ce que nous sélectionnons qu'une clé API par requête
        {
            let mut remaining_requests = Vec::new(); // Liste temporaire pour stocker les requêtes non exécutées

            for request in waiting_requests.drain(..) {
                let mut executed = false;
                for api_key in request.api_keys.clone() {
                    if !used_keys.lock().await.contains(&api_key)
                        && api_key_usage.can_execute(&api_key)
                    {
                        used_keys.lock().await.insert(api_key.clone());
                        executed = true;
                        // On exécute la requête dans un thread séparé
                        tokio::spawn({
                            let used_keys = used_keys.clone();
                            let request = request.clone();
                            let api_key = api_key.clone();
                            let redis_client = redis_client.clone();
                            let client = client.clone();
                            let response_broadcast_tx = response_broadcast_tx.clone();
                            let api_key_usage = api_key_usage.clone();
                            let used_keys = used_keys.clone();
                            async move {
                                execute_request(
                                    request,
                                    api_key.clone(),
                                    redis_client,
                                    client,
                                    response_broadcast_tx,
                                    api_key_usage,
                                    used_keys,
                                )
                                .await;
                            }
                        });
                        break;
                    }
                }
                if !executed {
                    remaining_requests.push(request);
                }
            }
            waiting_requests = remaining_requests;
        }

        // On attend 10ms avant de continuer (valeur arbitraire)
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
}

pub fn received_queue(
    queue_rx: &mut mpsc::Receiver<QueuedRequest>,
    waiting_requests: &mut Vec<QueuedRequest>,
) {
    while let Ok(request) = queue_rx.try_recv() {
        QueuedRequest::insert_request_to_queue(waiting_requests, request);
    }
}

// Attention ! Le fonctionnement actuel fait que si quelqu'un envoie une requête avec une clé API invalide, la requête retournera une erreur pour tout le monde !
// TODO: Ajouter un système pour remettre la requête dans la file d'attente si une clé API est invalide et qu'il reste des clés API à essayer
pub async fn execute_request(
    request: QueuedRequest,
    api_key: String,
    redis_client: redis::Client,
    request_client: reqwest::Client,
    response_broadcast_tx: broadcast::Sender<RequestResponse>,
    api_key_usage: Arc<ApiKeyUsage>,
    used_key: Arc<Mutex<HashSet<String>>>,
) {
    let url = request.url.clone();
    let method = request.method.clone();

    let response = request_client
        .request(method.parse().unwrap(), &url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await;

    api_key_usage.update_usage(api_key.clone());
    used_key.lock().await.remove(&api_key);

    match response {
        Ok(resp) => {
            let body_text = resp
                .text()
                .await
                .unwrap_or_else(|_| json!({"error": "Failed to read response"}).to_string());
            let body: Value = serde_json::from_str(&body_text).unwrap_or_else(
                |_| json!({"error": "Failed to parse response", "message": body_text}),
            );

            {
                let body = json!({"cached": false, "data": body});
                response_broadcast_tx
                    .send(RequestResponse {
                        url: url.clone(),
                        method: method.clone(),
                        body: body.clone(),
                    })
                    .expect("TODO: panic message");
            }

            if body.get("error").is_some() {
                // Si l'API a renvoyé une erreur, on ne cache pas la réponse
                return;
            }
            let mut redis_conn = redis_client
                .get_multiplexed_async_connection()
                .await
                .unwrap();
            let cache_key = format!("cache:{}", url);
            {
                let actual_time = chrono::Utc::now().to_rfc3339();
                let body = json!({"cached": true, "cached_time": actual_time,"data": body});
                let _: () = redis_conn
                    .set_ex(cache_key, body.to_string(), 1800)
                    .await
                    .unwrap();
            }
        }
        Err(_) => {
            response_broadcast_tx
                .send(RequestResponse {
                    url: url.clone(),
                    method: method.clone(),
                    body: json!({"error": "API request failed"}),
                })
                .expect("TODO: panic message");
        }
    }
}
