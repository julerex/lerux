use core::mem::{align_of, offset_of, size_of};

use lerux_ladybird_http::{
    request_page_zeroed, RequestBody, RequestPage, SubmitError, HTTPS_PORT, REQUEST_BODY_SIZE,
    REQUEST_REGION_SIZE, REQUEST_STATUS_ERR, REQUEST_STATUS_IDLE, REQUEST_STATUS_OK,
    REQUEST_STATUS_PENDING, REQUEST_URL_CAP,
};

const FIXTURE: &str = "https://10.0.2.2:8443/fixture.html";

fn submitted(url: &str) -> RequestPage {
    let mut page = request_page_zeroed();
    page.submit(url).expect("url");
    page
}

fn mapped_body() -> Vec<u8> {
    vec![0; REQUEST_BODY_SIZE]
}

fn pending() -> (RequestPage, Vec<u8>) {
    (submitted(FIXTURE), mapped_body())
}

#[test]
fn zeroed_header_has_a_fixed_layout() {
    assert_eq!(offset_of!(RequestPage, status), 0);
    assert_eq!(offset_of!(RequestPage, url_len), 4);
    assert_eq!(offset_of!(RequestPage, body_len), 8);
    assert_eq!(offset_of!(RequestPage, url), 12);
    assert!(size_of::<RequestPage>() <= REQUEST_REGION_SIZE);
    assert!(align_of::<RequestPage>() <= 8);
    assert_eq!(size_of::<RequestBody>(), REQUEST_BODY_SIZE);
    assert_eq!(size_of::<RequestPage>(), 4 + 4 + 4 + REQUEST_URL_CAP);

    let page = request_page_zeroed();
    assert_eq!(page.status, REQUEST_STATUS_IDLE);
    assert_eq!(page.url_len, 0);
    assert_eq!(page.body_len, 0);
    assert!(page.url.iter().all(|byte| *byte == 0));
}

#[test]
fn absolute_https_url_uses_its_path_as_the_target() {
    let page = submitted("https://10.0.2.2/fixture.html");
    assert_eq!(page.status, REQUEST_STATUS_PENDING);
    assert_eq!(page.host(), Some("10.0.2.2"));
    assert_eq!(page.port(), Some(HTTPS_PORT));
    assert_eq!(page.request_target(), Some("/fixture.html"));

    let explicit = submitted(FIXTURE);
    assert_eq!(explicit.port(), Some(8443));
    assert_eq!(explicit.request_target(), Some("/fixture.html"));

    let bare = submitted("https://10.0.2.2");
    assert_eq!(bare.request_target(), Some("/"));
    let slashed = submitted("https://10.0.2.2/");
    assert_eq!(slashed.request_target(), Some("/"));
}

#[test]
fn query_stays_on_the_target_and_the_fragment_does_not() {
    let page = submitted("https://10.0.2.2/fixture.html?q=1#f");
    assert_eq!(page.url(), Some("https://10.0.2.2/fixture.html?q=1#f"));
    assert_eq!(page.request_target(), Some("/fixture.html?q=1"));

    let empty_query = submitted("https://10.0.2.2/fixture.html?");
    assert_eq!(empty_query.request_target(), Some("/fixture.html?"));

    let cased = submitted("HTTPS://Example.Test/Fixture.html");
    assert_eq!(cased.host(), Some("Example.Test"));
    assert_eq!(cased.request_target(), Some("/Fixture.html"));
}

#[test]
fn rejects_a_url_that_is_not_one_absolute_https_get() {
    let mut page = request_page_zeroed();
    for url in [
        "",
        "/fixture.html",
        "http://10.0.2.2/fixture.html",
        "https:fixture.html",
        "https:///fixture.html",
        "https://user@10.0.2.2/fixture.html",
        "https://10.0.2.2/fixture.html ",
        "https://10.0.2.2/a b",
        "https://[::1]/fixture.html",
        "https://10.0.2.2?q=1",
    ] {
        assert_eq!(page.submit(url), Err(SubmitError::Url), "{url}");
        assert_eq!(page.status, REQUEST_STATUS_IDLE);
    }
}

#[test]
fn a_second_submit_waits_until_the_first_finishes() {
    let mut page = submitted(FIXTURE);
    assert_eq!(
        page.submit("https://10.0.2.2/other.html"),
        Err(SubmitError::Busy)
    );
    assert_eq!(page.status, REQUEST_STATUS_PENDING);
    assert_eq!(page.url(), Some(FIXTURE));
    assert_eq!(page.request_target(), Some("/fixture.html"));
}

#[test]
fn url_longer_than_the_header_is_rejected() {
    let mut url = String::from("https://10.0.2.2/");
    while url.len() < REQUEST_URL_CAP {
        url.push('a');
    }
    assert!(submitted(&url).request_target().is_some());

    url.push('b');
    let mut page = request_page_zeroed();
    assert_eq!(page.submit(&url), Err(SubmitError::Url));
    assert_eq!(page.status, REQUEST_STATUS_IDLE);
    assert_eq!(page.url_len, 0);
}

#[test]
fn fitted_body_sets_ok_status_and_length() {
    let (mut page, mut region) = pending();
    region.fill(0xAB);
    let body = b"fixture-bytes";
    assert!(page.write_body(&mut region, body));
    assert_eq!(page.status, REQUEST_STATUS_OK);
    assert_eq!(page.body_len, u32::try_from(body.len()).unwrap());
    assert_eq!(page.body(&region), Some(body.as_slice()));
    assert!(region[body.len()..].iter().all(|byte| *byte == 0));

    assert!(!page.write_body(&mut region, b"second"));
    assert_eq!(page.body(&region), Some(body.as_slice()));
    assert_eq!(page.status, REQUEST_STATUS_OK);
}

#[test]
fn empty_body_is_ok_with_length_zero() {
    let (mut page, mut region) = pending();
    region.fill(0xAB);
    assert!(page.write_body(&mut region, b""));
    assert_eq!(page.status, REQUEST_STATUS_OK);
    assert_eq!(page.body_len, 0);
    assert_eq!(page.body(&region), Some(b"".as_slice()));
    assert!(region.iter().all(|byte| *byte == 0));
}

#[test]
fn body_of_exactly_64_kib_fits() {
    let (mut page, mut region) = pending();
    let body = vec![0x22_u8; REQUEST_BODY_SIZE];
    assert!(page.write_body(&mut region, &body));
    assert_eq!(page.status, REQUEST_STATUS_OK);
    assert_eq!(page.body_len, u32::try_from(REQUEST_BODY_SIZE).unwrap());
    assert_eq!(page.body(&region), Some(body.as_slice()));
}

#[test]
fn overflow_sets_error_status_and_clears_the_body() {
    let (mut page, mut region) = pending();
    region.fill(0xAB);
    let body = vec![0x11_u8; REQUEST_BODY_SIZE + 1];
    assert!(page.write_body(&mut region, &body));
    assert_eq!(page.status, REQUEST_STATUS_ERR);
    assert_eq!(page.body_len, 0);
    assert!(region.iter().all(|byte| *byte == 0));
    assert!(page.body(&region).is_none());
    assert_eq!(page.request_target(), Some("/fixture.html"));
    assert_ne!(region.first(), Some(&0x11));
}

#[test]
fn write_before_submit_does_not_touch_the_region() {
    let mut page = request_page_zeroed();
    let mut region = mapped_body();
    region.fill(0xAB);
    assert!(!page.write_body(&mut region, b"nope"));
    assert!(region.iter().all(|byte| *byte == 0xAB));
    assert_eq!(page.status, REQUEST_STATUS_IDLE);
    assert_eq!(page.body_len, 0);
}

#[test]
fn slot_accepts_another_request_after_the_first_finishes() {
    let (mut page, mut region) = pending();
    let body = vec![0_u8; REQUEST_BODY_SIZE + 1];
    assert!(page.write_body(&mut region, &body));
    assert_eq!(page.status, REQUEST_STATUS_ERR);
    page.submit("https://10.0.2.2/other.html").expect("next");
    assert_eq!(page.status, REQUEST_STATUS_PENDING);
    assert_eq!(page.body_len, 0);
    assert_eq!(page.request_target(), Some("/other.html"));
    let len = usize::try_from(page.url_len).unwrap();
    assert!(page.url[len..].iter().all(|byte| *byte == 0));
}
