use tokio::sync::mpsc;
use std::collections::HashSet;
use std::sync::Arc;
use redis::AsyncCommands;
use reqwest::Client;
use crate::utils::{QueuedRequest, RequestQueue};

pub(crate) async fn process_requests(
    mut rx: mpsc::Receiver<QueuedRequest>,
    request_queue: Arc<RequestQueue>,
    redis_client: redis::Client,
) {
    let client = Client::new();

    while let Some(mut request) = rx.recv().await {
        let mut used_keys = HashSet::new();

        for key in &request.api_keys {
            if !used_keys.contains(key) && request_queue.can_execute(key) {
                used_keys.insert(key.clone());

                let url = request.url.clone();
                let method = request.method.clone();

                let response = client
                    .request(method.parse().unwrap(), &url)
                    .header("Authorization", format!("Bearer {}", key))
                    .send()
                    .await;

                match response {
                    Ok(resp) => {
                        let body = resp.text().await.unwrap_or_else(|_| "Error".to_string());

                        // Si j'obtiens ce json
                        //{
                        //   "error": "unauthorized.key",
                        //   "message": "The provided API key is unauthorized",
                        //   "type": "error"
                        // }
                        // Je ne veux pas sauvegarder en cache
                        if body.contains("unauthorized.key") {
                            if let Some(channel) = request.response_channel.take() {
                                let _ = channel.send(body);
                            }
                            break;
                        }

                        // Sauvegarde en cache Redis
                        let mut redis_conn = redis_client.get_multiplexed_async_connection().await.unwrap();
                        let cache_key = format!("cache:{}", url);
                        let _: () = redis_conn.set_ex(cache_key, &body, 1800).await.unwrap();

                        // Envoyer la réponse au client
                        if let Some(channel) = request.response_channel.take() {
                            let _ = channel.send(body);
                        }

                        request_queue.update_usage(key.clone()); // Mettre à jour la dernière utilisation de la clé API
                        break;
                    }
                    Err(_) => {
                        if let Some(channel) = request.response_channel.take() {
                            let _ = channel.send("Error calling API".to_string());
                        }
                    }
                }
            }
        }
    }
}
