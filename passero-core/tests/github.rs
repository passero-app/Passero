use mockito::Matcher;
use passero_core::github::{poll_once, refresh, request_device_code, PollResult};

fn device_grant_body() -> Matcher {
    Matcher::AllOf(vec![
        Matcher::UrlEncoded("client_id".into(), "client123".into()),
        Matcher::UrlEncoded("device_code".into(), "dc123".into()),
        Matcher::UrlEncoded(
            "grant_type".into(),
            "urn:ietf:params:oauth:grant-type:device_code".into(),
        ),
    ])
}

#[test]
fn device_code_request_parses() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("POST", "/login/device/code")
        .match_body(Matcher::UrlEncoded("client_id".into(), "client123".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"device_code":"dc123","user_code":"ABCD-1234","verification_uri":"https://github.com/login/device","expires_in":900,"interval":5}"#)
        .create();
    let dc = request_device_code(&server.url(), "client123").unwrap();
    assert_eq!(dc.user_code, "ABCD-1234");
    assert_eq!(dc.device_code, "dc123");
    assert_eq!(dc.interval, 5);
}

#[test]
fn poll_pending_then_token() {
    let mut server = mockito::Server::new();
    let m = server
        .mock("POST", "/login/oauth/access_token")
        .match_body(device_grant_body())
        .with_status(200)
        .with_body(r#"{"error":"authorization_pending"}"#)
        .expect(1)
        .create();
    assert!(matches!(
        poll_once(&server.url(), "client123", "dc123").unwrap(),
        PollResult::Pending
    ));
    m.assert();

    let _m2 = server
        .mock("POST", "/login/oauth/access_token")
        .match_body(device_grant_body())
        .with_status(200)
        .with_body(r#"{"access_token":"ghu_tok","expires_in":28800,"refresh_token":"ghr_ref","token_type":"bearer"}"#)
        .create();
    match poll_once(&server.url(), "client123", "dc123").unwrap() {
        PollResult::Token(t) => {
            assert_eq!(t.access_token, "ghu_tok");
            assert_eq!(t.refresh_token.as_deref(), Some("ghr_ref"));
            assert_eq!(t.expires_in, Some(28800));
        }
        other => panic!("expected token, got {other:?}"),
    }
}

#[test]
fn poll_slow_down() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("POST", "/login/oauth/access_token")
        .with_status(200)
        .with_body(r#"{"error":"slow_down"}"#)
        .create();
    assert!(matches!(
        poll_once(&server.url(), "client123", "dc123").unwrap(),
        PollResult::SlowDown
    ));
}

#[test]
fn poll_denied_is_error() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("POST", "/login/oauth/access_token")
        .with_status(200)
        .with_body(r#"{"error":"access_denied"}"#)
        .create();
    assert!(poll_once(&server.url(), "client123", "dc123").is_err());
}

#[test]
fn refresh_parses() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("POST", "/login/oauth/access_token")
        .match_body(Matcher::AllOf(vec![
            Matcher::UrlEncoded("client_id".into(), "client123".into()),
            Matcher::UrlEncoded("refresh_token".into(), "ghr_old".into()),
            Matcher::UrlEncoded("grant_type".into(), "refresh_token".into()),
        ]))
        .with_status(200)
        .with_body(r#"{"access_token":"ghu_new","expires_in":28800,"refresh_token":"ghr_new","token_type":"bearer"}"#)
        .create();
    let t = refresh(&server.url(), "client123", "ghr_old").unwrap();
    assert_eq!(t.access_token, "ghu_new");
}
