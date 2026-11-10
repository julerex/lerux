use lerux_ladybird_http::{
    request_page_zeroed, response_body, Load, RequestPage, ResponseBody, BODY_CAP,
    REQUEST_STATE_IDLE, TLS_NAME, URL_CAP,
};

#[test]
fn fixture_url_is_one_get_for_the_smoke_host() {
    let load = Load::parse("https://host:8443/fixture.html").expect("fixture url");
    assert_eq!(load.port(), 8443);
    assert_eq!(load.target(), "/fixture.html");
    assert_eq!(TLS_NAME, "host");
    let mut buf = [0u8; 128];
    let n = load.write_get(&mut buf).expect("get");
    assert_eq!(
        &buf[..n],
        b"GET /fixture.html HTTP/1.1\r\nHost: host\r\nConnection: close\r\n\r\n"
    );
}

#[test]
fn another_host_is_refused() {
    assert!(Load::parse("https://example.com/fixture.html").is_err());
    assert!(Load::parse("https://10.0.2.2:8443/fixture.html").is_err());
}

#[test]
fn set_url_leaves_the_state_idle() {
    let mut page = request_page_zeroed();
    assert!(page.set_url(b"https://host:8443/fixture.html"));
    assert_eq!(page.state, REQUEST_STATE_IDLE);
    assert_eq!(page.url(), b"https://host:8443/fixture.html");
    assert_eq!(page.body_len(), 0);
    assert!(!page.set_url(b""));
    assert!(!page.set_url(&[b'a'; URL_CAP + 1]));
}

#[test]
fn a_body_past_the_region_clears_the_length() {
    let mut page = request_page_zeroed();
    assert!(!page.accept_body(BODY_CAP + 1));
    assert_eq!(page.body_len(), 0);
    assert!(page.accept_body(BODY_CAP));
    assert_eq!(page.body_len(), BODY_CAP);
}

#[test]
fn a_redirect_or_an_oversized_length_is_an_error() {
    let redirect = b"HTTP/1.1 302 Found\r\nLocation: /fixture.html\r\nContent-Length: 0\r\n\r\n";
    assert!(matches!(
        response_body(redirect, BODY_CAP),
        ResponseBody::Error
    ));
    let oversized = b"HTTP/1.1 200 OK\r\nContent-Length: 65537\r\n\r\n";
    assert!(matches!(
        response_body(oversized, BODY_CAP),
        ResponseBody::Error
    ));
    assert!(matches!(
        response_body(b"HTTP/1.1 200 OK\r\n", BODY_CAP),
        ResponseBody::Pending
    ));
}

#[test]
fn the_fixture_file_is_the_response_body() {
    let file = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../support/browser/fixture.html"
    ))
    .expect("fixture file");
    let mut raw = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        file.len()
    )
    .into_bytes();
    raw.extend_from_slice(&file);
    match response_body(&raw, BODY_CAP) {
        ResponseBody::Ready(body) => assert_eq!(body, file.as_slice()),
        ResponseBody::Pending | ResponseBody::Error => panic!("fixture response"),
    }
}

#[test]
fn the_page_fits_in_its_region() {
    assert!(core::mem::size_of::<RequestPage>() <= 0x1000);
}
