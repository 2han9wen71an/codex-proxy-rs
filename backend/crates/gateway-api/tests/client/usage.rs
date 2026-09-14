use super::*;

const RECORDS_URI: &str = "/api/client/usage/records?range=7d&endsAt=2026-09-14T00:00:00Z";

async fn login_cookie(app: &axum::Router) -> String {
    let response = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/auth/login",
            json!({ "type": "key", "apiKey": RAW_KEY }),
        ))
        .await
        .expect("login response");
    response.headers()[header::SET_COOKIE]
        .to_str()
        .expect("cookie")
        .split(';')
        .next()
        .expect("cookie pair")
        .to_owned()
}

#[tokio::test]
async fn records_should_require_client_session_not_admin_cookie() {
    let (app, _) = client_app().await;
    for cookie in ["", "cpr_session=admin-session"] {
        let response = app
            .clone()
            .oneshot(cookie_request(Method::GET, RECORDS_URI, cookie))
            .await
            .expect("records response");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(response_json(response).await["code"], 40101);
    }
}

#[tokio::test]
async fn records_should_force_session_key_and_only_serialize_safe_facts() {
    let (app, store) = client_app().await;
    let cookie = login_cookie(&app).await;
    let response = app
        .oneshot(cookie_request(Method::GET, RECORDS_URI, &cookie))
        .await
        .expect("records response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    assert_eq!(
        response_json(response).await["data"],
        json!({
            "items": [{ "id": "request-42", "startedAt": "2026-09-07T00:00:00+00:00", "model": "gpt-5", "outcome": "failed", "totalTokens": null, "latencyMs": 1234, "cost": null }],
            "nextCursor": null,
        })
    );
    assert_eq!(
        store.usage_filters(),
        vec![
            UsageFilter {
                client_api_key_ref: Some("key-42".to_owned()),
                ..UsageFilter::default()
            };
            1
        ]
    );
}

#[tokio::test]
async fn records_should_reject_scope_overrides_and_invalid_pagination() {
    let (app, _) = client_app().await;
    let cookie = login_cookie(&app).await;
    for suffix in [
        "&clientApiKeyRef=other-key",
        "&accountId=other-account",
        "&limit=0",
        "&limit=101",
        "&beforeId=request-42",
        "&beforeAt=2026-09-13T00:00:00Z",
        "&beforeAt=2026-09-13T00:00:00Z&beforeId=",
    ] {
        let response = app
            .clone()
            .oneshot(cookie_request(
                Method::GET,
                &format!("{RECORDS_URI}{suffix}"),
                &cookie,
            ))
            .await
            .expect("invalid query response");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{suffix}");
    }
}

#[tokio::test]
async fn usage_should_reject_unsupported_ranges() {
    let (app, _) = client_app().await;
    let cookie = login_cookie(&app).await;
    let response = app
        .oneshot(cookie_request(
            Method::GET,
            "/api/client/usage?range=31d",
            &cookie,
        ))
        .await
        .expect("invalid range response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn pagination_should_require_an_anchored_end_time() {
    let (app, _) = client_app().await;
    let cookie = login_cookie(&app).await;
    let response = app
        .oneshot(cookie_request(
            Method::GET,
            "/api/client/usage/records?range=7d&beforeAt=2026-09-13T00:00:00Z&beforeId=request-42",
            &cookie,
        ))
        .await
        .expect("missing anchor response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
