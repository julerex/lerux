use lerux_ladybird_url::{ParseError, Url};

fn abs(input: &str) -> Url {
    Url::parse(input, None).expect("absolute url")
}

fn rel(input: &str, base: &Url) -> Url {
    Url::parse(input, Some(base)).expect("relative url")
}

fn target(url: &Url) -> String {
    url.request_target().to_string()
}

#[test]
fn absolute_https_stores_default_port_and_omits_it() {
    let page = abs("https://10.0.2.2/fixture.html");
    assert_eq!(page.scheme(), "https");
    assert_eq!(page.host(), "10.0.2.2");
    assert_eq!(page.port(), 443);
    assert_eq!(page.path(), "/fixture.html");
    assert_eq!(page.query(), None);
    assert_eq!(page.fragment(), None);
    assert_eq!(page.to_string(), "https://10.0.2.2/fixture.html");
    assert_eq!(target(&page), "/fixture.html");
}

#[test]
fn explicit_default_port_matches_omitted_port() {
    let omitted = abs("https://10.0.2.2/fixture.html");
    let explicit = abs("https://10.0.2.2:443/fixture.html");
    assert_eq!(explicit.port(), 443);
    assert_eq!(explicit, omitted);
    assert_eq!(explicit.to_string(), "https://10.0.2.2/fixture.html");
}

#[test]
fn fragment_stays_on_the_url_and_off_the_request_target() {
    let page = abs("https://10.0.2.2/fixture.html#top");
    assert_eq!(page.fragment(), Some("top"));
    assert_eq!(page.to_string(), "https://10.0.2.2/fixture.html#top");
    assert_eq!(target(&page), "/fixture.html");
}

#[test]
fn non_default_port_is_stored_and_displayed() {
    let page = abs("https://10.0.2.2:4443/secret");
    assert_eq!(page.port(), 4443);
    assert_eq!(page.to_string(), "https://10.0.2.2:4443/secret");
    assert_eq!(target(&page), "/secret");
}

#[test]
fn bare_host_and_trailing_slash_are_the_root() {
    let bare = abs("https://10.0.2.2");
    let slashed = abs("https://10.0.2.2/");
    assert_eq!(bare.path(), "/");
    assert_eq!(slashed.path(), "/");
    assert_eq!(bare, slashed);
    assert_eq!(bare.to_string(), "https://10.0.2.2/");
    assert_eq!(target(&bare), "/");
}

#[test]
fn scheme_and_host_are_lowercase_and_path_keeps_case() {
    let page = abs("HTTPS://Example.TEST/Fixture.html");
    assert_eq!(page.scheme(), "https");
    assert_eq!(page.host(), "example.test");
    assert_eq!(page.path(), "/Fixture.html");
    assert_eq!(page.to_string(), "https://example.test/Fixture.html");
}

#[test]
fn percent_escapes_stay_literal() {
    let page = abs("https://10.0.2.2/a%2Fb%2Ehtml");
    assert_eq!(page.path(), "/a%2Fb%2Ehtml");
    assert_eq!(page.to_string(), "https://10.0.2.2/a%2Fb%2Ehtml");
}

#[test]
fn backslash_stays_in_the_path() {
    let page = abs(r"https://10.0.2.2/a\b");
    assert_eq!(page.path(), r"/a\b");
}

#[test]
fn query_and_fragment_share_one_path_writer() {
    let page = abs("https://10.0.2.2/fixture.html?q=1#f");
    assert_eq!(page.query(), Some("q=1"));
    assert_eq!(page.fragment(), Some("f"));
    assert_eq!(target(&page), "/fixture.html?q=1");
    assert_eq!(page.to_string(), "https://10.0.2.2/fixture.html?q=1#f");
}

#[test]
fn empty_query_and_empty_fragment_are_present() {
    let query = abs("https://10.0.2.2/fixture.html?");
    assert_eq!(query.query(), Some(""));
    assert_eq!(query.fragment(), None);
    assert_eq!(query.to_string(), "https://10.0.2.2/fixture.html?");
    assert_eq!(target(&query), "/fixture.html?");

    let fragment = abs("https://10.0.2.2/fixture.html#");
    assert_eq!(fragment.fragment(), Some(""));
    assert_eq!(target(&fragment), "/fixture.html");
    assert_ne!(fragment, abs("https://10.0.2.2/fixture.html"));
    assert_eq!(fragment.to_string(), "https://10.0.2.2/fixture.html#");
}

#[test]
fn relative_path_replaces_the_final_segment() {
    let base = abs("https://10.0.2.2/fixture.html");
    let next = rel("other.html", &base);
    assert_eq!(next.scheme(), "https");
    assert_eq!(next.host(), "10.0.2.2");
    assert_eq!(next.port(), 443);
    assert_eq!(next.path(), "/other.html");
    assert_eq!(next.to_string(), "https://10.0.2.2/other.html");
}

#[test]
fn parent_segment_pops_one() {
    let base = abs("https://10.0.2.2/app/index.html");
    let next = rel("../fixture.html", &base);
    assert_eq!(next.to_string(), "https://10.0.2.2/fixture.html");
    assert_eq!(next.path(), "/fixture.html");
}

#[test]
fn absolute_path_replaces_the_base_path() {
    let base = abs("https://10.0.2.2:4443/app/index.html?q=1#top");
    let next = rel("/fixture.html", &base);
    assert_eq!(next.scheme(), "https");
    assert_eq!(next.host(), "10.0.2.2");
    assert_eq!(next.port(), 4443);
    assert_eq!(next.path(), "/fixture.html");
    assert_eq!(next.query(), None);
    assert_eq!(next.fragment(), None);
    assert_eq!(next.to_string(), "https://10.0.2.2:4443/fixture.html");
}

#[test]
fn fragment_reference_keeps_path_and_query() {
    let base = abs("https://10.0.2.2/fixture.html?q=1");
    let next = rel("#top", &base);
    assert_eq!(next.path(), base.path());
    assert_eq!(next.query(), base.query());
    assert_eq!(next.fragment(), Some("top"));
    assert_eq!(next.request_target(), base.request_target());
    assert_eq!(next.to_string(), "https://10.0.2.2/fixture.html?q=1#top");
}

#[test]
fn query_reference_keeps_the_path_and_clears_the_fragment() {
    let base = abs("https://10.0.2.2/app/index.html?old=1#top");
    let next = rel("?q=lerux", &base);
    assert_eq!(next.path(), "/app/index.html");
    assert_eq!(next.query(), Some("q=lerux"));
    assert_eq!(next.fragment(), None);
    assert_eq!(next.to_string(), "https://10.0.2.2/app/index.html?q=lerux");
    assert_eq!(target(&next), "/app/index.html?q=lerux");
}

#[test]
fn empty_input_with_a_base_clones_the_base() {
    let base = abs("https://10.0.2.2/fixture.html?q=1#top");
    assert_eq!(rel("", &base), base);
    assert_eq!(rel("", &base).fragment(), Some("top"));
}

#[test]
fn empty_input_without_a_base_is_an_error() {
    let error: Result<Url, ParseError> = Url::parse("", None);
    assert!(error.is_err());
}

#[test]
fn relative_path_clears_a_missing_query_and_fragment() {
    let base = abs("https://10.0.2.2/fixture.html?q=1#top");
    let next = rel("other.html", &base);
    assert_eq!(next.query(), None);
    assert_eq!(next.fragment(), None);
    assert_eq!(next.to_string(), "https://10.0.2.2/other.html");

    let with_fragment = rel("other.html#section", &base);
    assert_eq!(with_fragment.query(), None);
    assert_eq!(with_fragment.fragment(), Some("section"));
    assert_eq!(
        with_fragment.to_string(),
        "https://10.0.2.2/other.html#section"
    );
}

#[test]
fn dot_segment_is_skipped() {
    let base = abs("https://10.0.2.2/app/index.html");
    assert_eq!(rel("./fixture.html", &base).path(), "/app/fixture.html");
    assert_eq!(rel(".", &base).path(), "/app/");
    assert_eq!(rel("a.b", &base).path(), "/app/a.b");
    assert_eq!(
        abs("https://10.0.2.2/./fixture.html").path(),
        "/fixture.html"
    );
    assert_eq!(
        abs("https://10.0.2.2/app/../fixture.html").path(),
        "/fixture.html"
    );
}

#[test]
fn dot_dot_does_not_pass_the_root() {
    let base = abs("https://10.0.2.2/app/index.html");
    assert_eq!(rel("../../fixture.html", &base).path(), "/fixture.html");
    assert_eq!(rel("../../../fixture.html", &base).path(), "/fixture.html");
    assert_eq!(rel("..", &base).path(), "/");
    assert_eq!(
        abs("https://10.0.2.2/../../fixture.html").path(),
        "/fixture.html"
    );
}

#[test]
fn directory_dot_dot_keeps_the_parent_slash() {
    let base = abs("https://10.0.2.2/app/dir/index.html");
    assert_eq!(rel(".", &base).path(), "/app/dir/");
    assert_eq!(rel("..", &base).path(), "/app/");
}

#[test]
fn port_8443_is_a_normal_port() {
    let page = abs("https://10.0.2.2:8443/fixture.html");
    assert_eq!(page.port(), 8443);
    assert_eq!(page.to_string(), "https://10.0.2.2:8443/fixture.html");
    assert_eq!(target(&page), "/fixture.html");
}

#[test]
fn display_parses_back_to_the_same_url() {
    for input in [
        "https://10.0.2.2/fixture.html",
        "https://10.0.2.2:443/fixture.html",
        "https://10.0.2.2:4443/secret",
        "https://10.0.2.2/fixture.html?q=1#f",
        "HTTPS://Example.TEST/Fixture.html",
    ] {
        let url = abs(input);
        assert_eq!(abs(&url.to_string()), url, "{input}");
    }
}

#[test]
fn absolute_url_ignores_the_base() {
    let base = abs("https://10.0.2.2:4443/app/index.html#top");
    let next = rel("https://example.test/Fixture.html", &base);
    assert_eq!(next.to_string(), "https://example.test/Fixture.html");
}

#[test]
fn rejected_inputs() {
    let base = abs("https://10.0.2.2/fixture.html");
    let rejected = [
        "http://10.0.2.2/fixture.html",
        "HTTP://10.0.2.2/fixture.html",
        "ftp://10.0.2.2/fixture.html",
        "https://user:pass@10.0.2.2/fixture.html",
        "https://user@10.0.2.2/fixture.html",
        "https:///fixture.html",
        "https://",
        "https:///",
        "https://:443/fixture.html",
        "https://10.0.2.2:abc/fixture.html",
        "https://10.0.2.2:/fixture.html",
        "https://10.0.2.2:65536/fixture.html",
        "https://10.0.2.2:-1/fixture.html",
        "https:foo",
        "https://[::1]/fixture.html",
        "other.html",
        "#top",
        "?q=1",
        "",
    ];
    for input in rejected {
        let error: Result<Url, ParseError> = Url::parse(input, None);
        assert!(error.is_err(), "{input}");
    }
    let error: Result<Url, ParseError> = Url::parse("http://10.0.2.2/fixture.html", Some(&base));
    assert!(error.is_err());
}
