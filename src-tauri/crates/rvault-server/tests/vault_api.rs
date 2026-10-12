use std::time::Duration;

use rvault_server::{serve, ServerConfig};
use rvault_storage::VaultSession;
use tokio::sync::oneshot;

#[tokio::test]
async fn unlocked_vault_hides_passwords_until_revealed() {
    let path = std::env::temp_dir().join(format!("rvault-api-{}.vault", std::process::id()));
    cleanup(&path);
    {
        VaultSession::create(&path, "correct horse").unwrap();
    }

    let (bound_tx, bound_rx) = oneshot::channel();
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let server = tokio::spawn(async move {
        serve(
            ServerConfig {
                bind: "127.0.0.1:0".into(),
                vault_path: path.clone(),
                cors_origins: Vec::new(),
                idle_lock_secs: 900,
            },
            shutdown_rx,
            Some(bound_tx),
        )
        .await
        .unwrap();
    });
    let addr = tokio::time::timeout(Duration::from_secs(10), bound_rx)
        .await
        .expect("bind timeout")
        .unwrap();
    let http = reqwest::Client::new();
    let base = format!("http://{addr}");

    let extension = "moz-extension://4b1c2d3e-0000-4000-8000-000000000000";
    let preflight = http
        .request(reqwest::Method::OPTIONS, format!("{base}/v1/vault/unlock"))
        .header("Origin", extension)
        .header("Access-Control-Request-Method", "POST")
        .header("Access-Control-Request-Headers", "content-type")
        .send()
        .await
        .unwrap();
    assert_eq!(
        preflight.headers().get("access-control-allow-origin").and_then(|v| v.to_str().ok()),
        Some(extension)
    );
    let from_extension = http
        .get(format!("{base}/v1/vault/status"))
        .header("Origin", extension)
        .send()
        .await
        .unwrap();
    assert_eq!(
        from_extension.headers().get("access-control-allow-origin").and_then(|v| v.to_str().ok()),
        Some(extension)
    );

    let rejected = http
        .post(format!("{base}/v1/vault/unlock"))
        .json(&serde_json::json!({ "password": "nope" }))
        .send()
        .await
        .unwrap();
    assert_eq!(rejected.status(), 401);

    let unlocked = http
        .post(format!("{base}/v1/vault/unlock"))
        .json(&serde_json::json!({ "password": "correct horse" }))
        .send()
        .await
        .unwrap();
    assert!(unlocked.status().is_success(), "{}", unlocked.text().await.unwrap());
    let session: serde_json::Value = unlocked.json().await.unwrap();
    let token = session["token"].as_str().unwrap().to_string();

    let created = http
        .post(format!("{base}/v1/entries"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "title": "Demo",
            "password": "pw-secret-xyz"
        }))
        .send()
        .await
        .unwrap();
    let created_body = created.text().await.unwrap();
    assert!(created_body.contains("Demo"), "{created_body}");
    assert!(!created_body.contains("pw-secret-xyz"), "{created_body}");
    let created_json: serde_json::Value = serde_json::from_str(&created_body).unwrap();
    let id = created_json["id"].as_str().unwrap();

    let listed = http
        .get(format!("{base}/v1/entries"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(listed.contains("Demo"));
    assert!(!listed.contains("pw-secret-xyz"));

    let revealed = http
        .get(format!("{base}/v1/entries/{id}/password"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let revealed: serde_json::Value = revealed.json().await.unwrap();
    assert_eq!(revealed["password"], "pw-secret-xyz");

    let renewed = http
        .post(format!("{base}/v1/vault/unlock"))
        .json(&serde_json::json!({ "password": "correct horse" }))
        .send()
        .await
        .unwrap();
    let renewed: serde_json::Value = renewed.json().await.unwrap();
    let next_token = renewed["token"].as_str().unwrap();
    assert_ne!(next_token, token);

    let stale = http
        .get(format!("{base}/v1/entries"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(stale.status(), 401);

    let locked = http
        .post(format!("{base}/v1/vault/lock"))
        .bearer_auth(next_token)
        .send()
        .await
        .unwrap();
    assert_eq!(locked.status(), 204);

    let after = http
        .get(format!("{base}/v1/entries"))
        .bearer_auth(next_token)
        .send()
        .await
        .unwrap();
    assert_eq!(after.status(), 401);

    let _ = shutdown_tx.send(());
    server.await.unwrap();
    cleanup(&std::env::temp_dir().join(format!("rvault-api-{}.vault", std::process::id())));
}

fn cleanup(path: &std::path::Path) {
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(path.with_extension("vault.tmp.db"));
    let _ = std::fs::remove_file(path.with_extension("vault.bak.1"));
    let _ = std::fs::remove_file(path.with_extension("vault.bak.2"));
    let _ = std::fs::remove_file(path.with_extension("vault.bak.3"));
    let _ = std::fs::remove_file(path.with_extension("vault.tmp"));
}
