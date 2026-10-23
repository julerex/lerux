//! Address line for the desk browser, and the firmware framebuffer mode.
//!
//! [`Chrome`] is the state machine. The page area is blank, one failure line,
//! or one page URL. A load is idle or one in-flight id.

#![no_std]

#[cfg(test)]
extern crate std;

mod mode;
mod url;

use lerux_input::{Key, KeyEvent};

pub use mode::{mode_from_mb2_tag, ScanoutMode};
pub use url::{HttpsUrl, NamePolicy};

const LINE_CAP: usize = 256;

/// Non-zero load id. Zero is the cleared shared page and matches nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoadId(u64);

impl LoadId {
    /// Parse an id from outside this crate. Zero is rejected.
    pub fn from_raw(raw: u64) -> Option<Self> {
        if raw == 0 {
            None
        } else {
            Some(Self(raw))
        }
    }

    pub fn raw(self) -> u64 {
        self.0
    }
}

/// Why the page area shows a failure line.
///
/// [`FailLine::NotPublicHttps`] comes from the address line.
/// A fetch or a paint is [`FailLine::Fetch`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailLine {
    Fetch(FetchFail),
    NotPublicHttps,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FetchFail {
    NameLookup,
    Certificate,
    Connection,
    CouldNotPaint,
}

impl FailLine {
    pub fn text(self) -> &'static str {
        match self {
            Self::Fetch(FetchFail::NameLookup) => "name lookup failed",
            Self::Fetch(FetchFail::Certificate) => "certificate rejected",
            Self::Fetch(FetchFail::Connection) => "connection failed",
            Self::Fetch(FetchFail::CouldNotPaint) => "page could not be painted",
            Self::NotPublicHttps => "not a public https address",
        }
    }
}

#[expect(
    clippy::large_enum_variant,
    reason = "a parsed URL owns its bytes and a rejection does not"
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Classify {
    Empty,
    Url(HttpsUrl),
    Rejected(FailLine),
}

/// Parse `text` under `policy`.
///
/// A missing scheme becomes `https`. An empty string is [`Classify::Empty`].
/// `http`, a literal address, an empty host, or a name this policy does not
/// treat as public is [`FailLine::NotPublicHttps`].
pub fn classify(text: &str, policy: NamePolicy) -> Classify {
    if text.is_empty() {
        return Classify::Empty;
    }
    match url::normalize(text, policy) {
        Ok(url) => Classify::Url(url),
        Err(()) => Classify::Rejected(FailLine::NotPublicHttps),
    }
}

/// Text of the 16 address rows.
///
/// `cursor` is `None` while the page is focused. `selected` is true only when
/// the whole line is selected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineView<'a> {
    pub text: &'a str,
    pub cursor: Option<u16>,
    pub selected: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageView<'a> {
    Blank,
    Failure(FailLine),
    Page(&'a HttpsUrl),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Selection {
    Caret,
    All,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LineEdit {
    bytes: [u8; LINE_CAP],
    len: u16,
    cursor: u16,
    selection: Selection,
}

impl LineEdit {
    const fn empty() -> Self {
        Self {
            bytes: [0; LINE_CAP],
            len: 0,
            cursor: 0,
            selection: Selection::Caret,
        }
    }

    fn from_url(url: HttpsUrl, selection: Selection) -> Self {
        let mut edit = Self::empty();
        let bytes = url.as_str().as_bytes();
        let len = bytes.len().min(LINE_CAP);
        edit.bytes[..len].copy_from_slice(&bytes[..len]);
        edit.len = u16::try_from(len).unwrap_or(0);
        edit.selection = selection;
        edit.cursor = match selection {
            Selection::All => 0,
            Selection::Caret => edit.len,
        };
        edit
    }

    fn set_text(&mut self, text: &str) {
        let bytes = text.as_bytes();
        if bytes.len() > LINE_CAP {
            return;
        }
        self.bytes[..bytes.len()].copy_from_slice(bytes);
        self.len = u16::try_from(bytes.len()).unwrap_or(0);
        self.cursor = self.len;
        self.selection = Selection::Caret;
    }

    fn text(&self) -> &str {
        let slice = &self.bytes[..usize::from(self.len)];
        core::str::from_utf8(slice).unwrap_or_default()
    }

    fn insert(&mut self, ch: u8) {
        if matches!(self.selection, Selection::All) {
            self.bytes[0] = ch;
            self.len = 1;
            self.cursor = 1;
            self.selection = Selection::Caret;
            return;
        }
        let len = usize::from(self.len);
        if len >= LINE_CAP {
            return;
        }
        let at = usize::from(self.cursor).min(len);
        self.bytes.copy_within(at..len, at + 1);
        self.bytes[at] = ch;
        self.len += 1;
        self.cursor = u16::try_from(at + 1).unwrap_or(self.cursor);
    }

    fn backspace(&mut self) {
        if matches!(self.selection, Selection::All) {
            self.len = 0;
            self.cursor = 0;
            self.selection = Selection::Caret;
            return;
        }
        if self.cursor == 0 {
            return;
        }
        let at = usize::from(self.cursor);
        let len = usize::from(self.len);
        self.bytes.copy_within(at..len, at - 1);
        self.len -= 1;
        self.cursor -= 1;
    }

    fn left(&mut self) {
        if matches!(self.selection, Selection::All) {
            self.cursor = 0;
            self.selection = Selection::Caret;
            return;
        }
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    fn right(&mut self) {
        if matches!(self.selection, Selection::All) {
            self.cursor = self.len;
            self.selection = Selection::Caret;
            return;
        }
        if self.cursor < self.len {
            self.cursor += 1;
        }
    }
}

#[expect(
    clippy::large_enum_variant,
    reason = "the page area holds one URL or one failure, never both"
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shown {
    Blank,
    Failed(FailLine),
    Page(HttpsUrl),
}

#[expect(clippy::large_enum_variant, reason = "an idle load has no URL bytes")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Flight {
    Idle,
    InFlight { url: HttpsUrl, id: LoadId },
}

#[expect(
    clippy::large_enum_variant,
    reason = "each state owns the URL bytes it shows"
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChromeInner {
    Line {
        edit: LineEdit,
        shown: Shown,
        flight: Flight,
    },
    Loading {
        target: HttpsUrl,
        id: LoadId,
        shown: Shown,
    },
    Reading {
        page: HttpsUrl,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chrome {
    inner: ChromeInner,
    next_id: u64,
}

/// What [`Chrome::on_key`] and [`Chrome::boot_load`] ask the caller to publish.
#[expect(
    clippy::large_enum_variant,
    reason = "a commit carries the URL the caller publishes"
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    None,
    Commit {
        url: HttpsUrl,
        id: LoadId,
    },
    /// Abandon `id` and do not start another load.
    Drop {
        id: LoadId,
    },
    /// The page is focused and this key is not Ctrl+L or Escape.
    PageKey(KeyEvent),
}

/// What [`Chrome::accept`] changed in the page area.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Present {
    Page,
    Failure(FailLine),
    Hold,
}

/// Bitmap ready, or one of the four fetch failures.
#[expect(
    clippy::large_enum_variant,
    reason = "a painted outcome carries the final URL"
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaintOutcome {
    Painted(HttpsUrl),
    Failed(FetchFail),
}

impl Chrome {
    /// Empty line, blank page, line focused, nothing in flight.
    pub const fn fresh() -> Self {
        Self {
            inner: ChromeInner::Line {
                edit: LineEdit::empty(),
                shown: Shown::Blank,
                flight: Flight::Idle,
            },
            next_id: 1,
        }
    }

    /// Enter `text` on [`Self::fresh`] under `policy`.
    ///
    /// A committed URL focuses the page. [`Self::loaded_address`] stays empty
    /// until [`Self::accept`] of that id.
    pub fn boot_load(text: &str, policy: NamePolicy) -> (Self, Effect) {
        match classify(text, policy) {
            Classify::Empty => (Self::fresh(), Effect::None),
            Classify::Rejected(_) => {
                let mut chrome = Self::fresh();
                if let ChromeInner::Line { edit, .. } = &mut chrome.inner {
                    edit.set_text(text);
                }
                chrome.reject_line()
            }
            Classify::Url(url) => Self::fresh().start_load(url, Shown::Blank),
        }
    }

    /// Address-line and focus transitions.
    ///
    /// Ctrl+L focuses the line and selects its text. Escape abandons an
    /// in-flight load. Any other key while the page is focused is
    /// [`Effect::PageKey`].
    pub fn on_key(self, event: KeyEvent, policy: NamePolicy) -> (Self, Effect) {
        if event.ctrl && matches!(event.key, Key::Char(b'l' | b'L')) {
            return (self.focus_line(), Effect::None);
        }
        if matches!(event.key, Key::Escape) {
            return self.escape();
        }
        if matches!(self.inner, ChromeInner::Line { .. }) {
            self.on_line_key(event, policy)
        } else {
            (self, Effect::PageKey(event))
        }
    }

    /// Apply a paint for `id`. A different id leaves this value unchanged.
    pub fn accept(self, id: LoadId, outcome: PaintOutcome) -> (Self, Present) {
        match self.inner {
            ChromeInner::Loading {
                target,
                id: inflight,
                ..
            } if inflight == id => self.land(None, target, outcome),
            ChromeInner::Line {
                edit,
                flight: Flight::InFlight { url, id: inflight },
                ..
            } if inflight == id => self.land(Some(edit), url, outcome),
            _ => (self, Present::Hold),
        }
    }

    /// Text the address rows paint.
    ///
    /// During a load the row shows that load's URL. The page area keeps the
    /// previous paint until [`Self::accept`].
    pub fn line(&self) -> LineView<'_> {
        match &self.inner {
            ChromeInner::Line { edit, .. } => LineView {
                text: edit.text(),
                cursor: Some(edit.cursor),
                selected: matches!(edit.selection, Selection::All),
            },
            ChromeInner::Loading { target, .. } => LineView {
                text: target.as_str(),
                cursor: None,
                selected: false,
            },
            ChromeInner::Reading { page } => LineView {
                text: page.as_str(),
                cursor: None,
                selected: false,
            },
        }
    }

    pub fn page(&self) -> PageView<'_> {
        match &self.inner {
            ChromeInner::Reading { page } => PageView::Page(page),
            ChromeInner::Loading { shown, .. } | ChromeInner::Line { shown, .. } => match shown {
                Shown::Blank => PageView::Blank,
                Shown::Failed(reason) => PageView::Failure(*reason),
                Shown::Page(url) => PageView::Page(url),
            },
        }
    }

    /// Final URL of the page on screen, after a paint has landed.
    pub fn loaded_address(&self) -> Option<&HttpsUrl> {
        match self.page() {
            PageView::Page(url) => Some(url),
            PageView::Blank | PageView::Failure(_) => None,
        }
    }

    fn alloc_id(&mut self) -> LoadId {
        let raw = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        if self.next_id == 0 {
            self.next_id = 1;
        }
        LoadId(raw)
    }

    fn start_load(mut self, url: HttpsUrl, shown: Shown) -> (Self, Effect) {
        let id = self.alloc_id();
        self.inner = ChromeInner::Loading {
            target: url,
            id,
            shown,
        };
        (self, Effect::Commit { url, id })
    }

    fn reject_line(mut self) -> (Self, Effect) {
        let ChromeInner::Line { edit, flight, .. } = self.inner else {
            return (self, Effect::None);
        };
        let effect = match flight {
            Flight::InFlight { id, .. } => Effect::Drop { id },
            Flight::Idle => Effect::None,
        };
        self.inner = ChromeInner::Line {
            edit,
            shown: Shown::Failed(FailLine::NotPublicHttps),
            flight: Flight::Idle,
        };
        (self, effect)
    }

    fn enter(self, policy: NamePolicy) -> (Self, Effect) {
        let owned = {
            let ChromeInner::Line { edit, .. } = &self.inner else {
                return (self, Effect::None);
            };
            let mut buf = [0u8; LINE_CAP];
            let len = usize::from(edit.len).min(LINE_CAP);
            buf[..len].copy_from_slice(&edit.bytes[..len]);
            (buf, len)
        };
        let text = core::str::from_utf8(&owned.0[..owned.1]).unwrap_or_default();
        match classify(text, policy) {
            Classify::Empty => (self, Effect::None),
            Classify::Rejected(_) => self.reject_line(),
            Classify::Url(url) => {
                let shown = match self.inner {
                    ChromeInner::Line { shown, .. } => shown,
                    _ => Shown::Blank,
                };
                self.start_load(url, shown)
            }
        }
    }

    fn on_line_key(mut self, event: KeyEvent, policy: NamePolicy) -> (Self, Effect) {
        if event.ctrl {
            return (self, Effect::None);
        }
        if matches!(event.key, Key::Enter) {
            return self.enter(policy);
        }
        if let ChromeInner::Line { edit, .. } = &mut self.inner {
            match event.key {
                Key::Backspace => edit.backspace(),
                Key::Left => edit.left(),
                Key::Right => edit.right(),
                Key::Char(ch) if (0x20..=0x7e).contains(&ch) => edit.insert(ch),
                _ => {}
            }
        }
        (self, Effect::None)
    }

    fn focus_line(mut self) -> Self {
        self.inner = match self.inner {
            ChromeInner::Line {
                mut edit,
                shown,
                flight,
            } => {
                edit.selection = Selection::All;
                edit.cursor = 0;
                ChromeInner::Line {
                    edit,
                    shown,
                    flight,
                }
            }
            ChromeInner::Loading { target, id, shown } => ChromeInner::Line {
                edit: LineEdit::from_url(target, Selection::All),
                shown,
                flight: Flight::InFlight { url: target, id },
            },
            ChromeInner::Reading { page } => ChromeInner::Line {
                edit: LineEdit::from_url(page, Selection::All),
                shown: Shown::Page(page),
                flight: Flight::Idle,
            },
        };
        self
    }

    fn escape(mut self) -> (Self, Effect) {
        let (next, effect) = match self.inner {
            ChromeInner::Reading { .. } => return (self, Effect::None),
            ChromeInner::Loading { target, id, shown } => (
                after_escape(shown, LineEdit::from_url(target, Selection::Caret)),
                Effect::Drop { id },
            ),
            ChromeInner::Line {
                edit,
                shown,
                flight: Flight::InFlight { id, .. },
            } => (after_escape(shown, edit), Effect::Drop { id }),
            ChromeInner::Line {
                edit,
                shown,
                flight: Flight::Idle,
            } => (after_escape(shown, edit), Effect::None),
        };
        self.inner = next;
        (self, effect)
    }

    fn land(
        mut self,
        edit: Option<LineEdit>,
        flight_url: HttpsUrl,
        outcome: PaintOutcome,
    ) -> (Self, Present) {
        match outcome {
            PaintOutcome::Painted(url) => {
                self.inner = if let Some(edit) = edit {
                    ChromeInner::Line {
                        edit,
                        shown: Shown::Page(url),
                        flight: Flight::Idle,
                    }
                } else {
                    ChromeInner::Reading { page: url }
                };
                (self, Present::Page)
            }
            PaintOutcome::Failed(fail) => {
                let reason = FailLine::Fetch(fail);
                let edit = match edit {
                    Some(mut edit) => {
                        edit.set_text(flight_url.as_str());
                        edit
                    }
                    None => LineEdit::from_url(flight_url, Selection::All),
                };
                self.inner = ChromeInner::Line {
                    edit,
                    shown: Shown::Failed(reason),
                    flight: Flight::Idle,
                };
                (self, Present::Failure(reason))
            }
        }
    }
}

fn after_escape(shown: Shown, edit: LineEdit) -> ChromeInner {
    match shown {
        Shown::Page(page) => ChromeInner::Reading { page },
        other => ChromeInner::Line {
            edit,
            shown: other,
            flight: Flight::Idle,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lerux_input::{Key, KeyEvent};

    const FIXTURE: &str = "https://host:8443/fixture.html";

    fn boot_fixture() -> (Chrome, HttpsUrl, LoadId) {
        let (chrome, effect) = Chrome::boot_load(FIXTURE, NamePolicy::DemoHost);
        let Effect::Commit { url, id } = effect else {
            panic!("boot load commits the fixture");
        };
        assert_eq!(url.as_str(), FIXTURE, "the commit is that url");
        assert_eq!(chrome.line().text, FIXTURE, "the line text is that url");
        assert!(
            chrome.loaded_address().is_none(),
            "loaded_address is empty until accept"
        );
        (chrome, url, id)
    }

    fn type_and_enter(text: &str, policy: NamePolicy) -> (Chrome, Effect) {
        let mut chrome = Chrome::fresh();
        for byte in text.bytes() {
            let (next, effect) = chrome.on_key(
                KeyEvent {
                    key: Key::Char(byte),
                    ctrl: false,
                },
                policy,
            );
            assert!(matches!(effect, Effect::None), "typing does not commit");
            chrome = next;
        }
        chrome.on_key(
            KeyEvent {
                key: Key::Enter,
                ctrl: false,
            },
            policy,
        )
    }

    #[test]
    fn boot_load_commits_the_fixture() {
        boot_fixture();
    }

    #[test]
    fn stale_accept_leaves_the_line() {
        let (chrome, url, id) = boot_fixture();
        let stale = LoadId::from_raw(id.raw().wrapping_add(1)).expect("a different id is non-zero");
        let (next, present) = chrome.accept(stale, PaintOutcome::Painted(url));
        assert_eq!(present, Present::Hold);
        assert!(next.loaded_address().is_none(), "a stale id loads no page");
        assert_eq!(next.line().text, chrome.line().text);
    }

    #[test]
    fn accept_painted_reports_the_loaded_url() {
        let (chrome, url, id) = boot_fixture();
        let (chrome, present) = chrome.accept(id, PaintOutcome::Painted(url));
        assert_eq!(present, Present::Page);
        assert_eq!(
            chrome
                .loaded_address()
                .expect("a painted outcome reports the loaded url")
                .as_str(),
            FIXTURE
        );
        assert_eq!(chrome.line().text, FIXTURE);
        assert!(chrome.line().cursor.is_none());
    }

    #[test]
    fn accept_fetch_failure_keeps_the_failed_url() {
        let (chrome, _url, id) = boot_fixture();
        let (chrome, present) = chrome.accept(id, PaintOutcome::Failed(FetchFail::Connection));
        assert_eq!(
            present,
            Present::Failure(FailLine::Fetch(FetchFail::Connection))
        );
        assert!(
            chrome.loaded_address().is_none(),
            "a fetch failure reports no loaded page"
        );
        assert_eq!(
            chrome.line().text,
            FIXTURE,
            "the line text is the failed url"
        );
        assert!(chrome.line().selected);
        match chrome.page() {
            PageView::Failure(line) => assert_eq!(line.text(), "connection failed"),
            _ => panic!("the page area shows the fetch failure"),
        }
    }

    #[test]
    fn http_and_other_host_do_not_commit() {
        for typed in ["http://host/a", "https://example.com/"] {
            let (chrome, effect) = type_and_enter(typed, NamePolicy::DemoHost);
            assert!(
                !matches!(effect, Effect::Commit { .. }),
                "a rejected address does not commit"
            );
            assert!(chrome.loaded_address().is_none());
            assert_eq!(chrome.line().text, typed);
            match chrome.page() {
                PageView::Failure(line) => {
                    assert_eq!(line.text(), "not a public https address");
                }
                _ => panic!("the page area shows the rejection"),
            }
        }
    }

    #[test]
    fn enter_on_an_empty_line_does_not_commit() {
        let chrome = Chrome::fresh();
        let (chrome, effect) = chrome.on_key(
            KeyEvent {
                key: Key::Enter,
                ctrl: false,
            },
            NamePolicy::DemoHost,
        );
        assert!(matches!(effect, Effect::None));
        assert_eq!(chrome.line().text, "");
        assert!(chrome.loaded_address().is_none());
        assert!(matches!(chrome.page(), PageView::Blank));
    }

    #[test]
    fn missing_scheme_commits_as_https() {
        let (chrome, effect) = type_and_enter("host/a", NamePolicy::DemoHost);
        let Effect::Commit { url, .. } = effect else {
            panic!("a name with no scheme commits as https");
        };
        assert_eq!(url.as_str(), "https://host/a");
        assert_eq!(chrome.line().text, "https://host/a");
        assert!(chrome.loaded_address().is_none());
    }

    #[test]
    fn public_policy_rejects_host_and_literals() {
        let (_chrome, effect) = type_and_enter("https://example.com/a", NamePolicy::Public);
        assert!(
            matches!(effect, Effect::Commit { .. }),
            "a public name commits without a suffix list"
        );
        let (chrome, effect) = type_and_enter("https://host/a", NamePolicy::Public);
        assert!(!matches!(effect, Effect::Commit { .. }));
        assert!(matches!(
            chrome.page(),
            PageView::Failure(FailLine::NotPublicHttps)
        ));
        let (chrome, effect) = type_and_enter("https://10.0.2.2/", NamePolicy::Public);
        assert!(!matches!(effect, Effect::Commit { .. }));
        assert!(matches!(
            chrome.page(),
            PageView::Failure(FailLine::NotPublicHttps)
        ));
    }

    #[test]
    fn ctrl_l_replaces_the_selected_text() {
        let (chrome, url, id) = boot_fixture();
        let (chrome, _) = chrome.accept(id, PaintOutcome::Painted(url));
        let (chrome, effect) = chrome.on_key(
            KeyEvent {
                key: Key::Char(b'l'),
                ctrl: true,
            },
            NamePolicy::DemoHost,
        );
        assert!(matches!(effect, Effect::None));
        assert!(chrome.line().cursor.is_some(), "ctrl+l focuses the line");
        assert!(chrome.line().selected, "ctrl+l selects the address");
        assert_eq!(chrome.line().text, FIXTURE);
        let (chrome, effect) = chrome.on_key(
            KeyEvent {
                key: Key::Char(b'a'),
                ctrl: false,
            },
            NamePolicy::DemoHost,
        );
        assert!(
            matches!(effect, Effect::None),
            "the character does not commit"
        );
        assert_eq!(chrome.line().text, "a");
        assert!(!chrome.line().selected);
    }

    #[test]
    fn failure_while_the_line_is_focused_shows_the_failed_url() {
        let (chrome, _url, id) = boot_fixture();
        let (chrome, effect) = chrome.on_key(
            KeyEvent {
                key: Key::Char(b'l'),
                ctrl: true,
            },
            NamePolicy::DemoHost,
        );
        assert!(matches!(effect, Effect::None));
        let (chrome, effect) = chrome.on_key(
            KeyEvent {
                key: Key::Char(b'x'),
                ctrl: false,
            },
            NamePolicy::DemoHost,
        );
        assert!(matches!(effect, Effect::None));
        assert_eq!(chrome.line().text, "x");
        let (chrome, present) = chrome.accept(id, PaintOutcome::Failed(FetchFail::NameLookup));
        assert_eq!(
            present,
            Present::Failure(FailLine::Fetch(FetchFail::NameLookup))
        );
        assert_eq!(chrome.line().text, FIXTURE);
        assert!(
            !chrome.line().selected,
            "a line that was already focused is not selected"
        );
        assert!(chrome.line().cursor.is_some());
        assert!(chrome.loaded_address().is_none());
    }

    #[test]
    fn ctrl_letter_on_the_line_does_not_insert() {
        let mut chrome = Chrome::fresh();
        for byte in b"host" {
            let (next, _) = chrome.on_key(
                KeyEvent {
                    key: Key::Char(*byte),
                    ctrl: false,
                },
                NamePolicy::DemoHost,
            );
            chrome = next;
        }
        let (chrome, effect) = chrome.on_key(
            KeyEvent {
                key: Key::Char(b'a'),
                ctrl: true,
            },
            NamePolicy::DemoHost,
        );
        assert!(matches!(effect, Effect::None));
        assert_eq!(chrome.line().text, "host");
    }

    #[test]
    fn tab_on_a_page_is_a_page_key() {
        let (chrome, url, id) = boot_fixture();
        let (chrome, _) = chrome.accept(id, PaintOutcome::Painted(url));
        let (chrome, effect) = chrome.on_key(
            KeyEvent {
                key: Key::Tab,
                ctrl: false,
            },
            NamePolicy::DemoHost,
        );
        match effect {
            Effect::PageKey(event) => {
                assert_eq!(event.key, Key::Tab);
                assert!(!event.ctrl);
            }
            _ => panic!("tab on the page is a page key and does not commit"),
        }
        assert_eq!(chrome.loaded_address().map(HttpsUrl::as_str), Some(FIXTURE));
        assert_eq!(chrome.line().text, FIXTURE);
    }

    #[test]
    fn escape_on_a_page_is_not_a_page_key() {
        let (chrome, url, id) = boot_fixture();
        let (chrome, _) = chrome.accept(id, PaintOutcome::Painted(url));
        let (next, effect) = chrome.on_key(
            KeyEvent {
                key: Key::Escape,
                ctrl: false,
            },
            NamePolicy::DemoHost,
        );
        assert!(matches!(effect, Effect::None));
        assert_eq!(next.loaded_address().map(HttpsUrl::as_str), Some(FIXTURE));
        assert_eq!(next.line().text, chrome.line().text);
    }

    #[test]
    fn failure_lines_use_the_spec_spellings() {
        assert_eq!(
            FailLine::Fetch(FetchFail::NameLookup).text(),
            "name lookup failed"
        );
        assert_eq!(
            FailLine::Fetch(FetchFail::Certificate).text(),
            "certificate rejected"
        );
        assert_eq!(
            FailLine::Fetch(FetchFail::Connection).text(),
            "connection failed"
        );
        assert_eq!(
            FailLine::Fetch(FetchFail::CouldNotPaint).text(),
            "page could not be painted"
        );
        assert_eq!(
            FailLine::NotPublicHttps.text(),
            "not a public https address"
        );
    }
}
