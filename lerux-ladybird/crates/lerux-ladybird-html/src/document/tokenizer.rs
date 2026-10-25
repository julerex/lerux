pub(super) struct Tokenizer<'a> {
    input: &'a str,
    pos: usize,
}

pub(super) enum DataToken<'a> {
    Doctype,
    StartTag { name: &'a str },
    EndTag { name: &'a str },
    Characters(&'a str),
}

pub(super) struct RawSpan<'a> {
    pub(super) text: &'a str,
    pub(super) closed: bool,
}

impl<'a> Tokenizer<'a> {
    pub(super) fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    pub(super) fn next_data(&mut self) -> Option<DataToken<'a>> {
        loop {
            if self.pos >= self.input.len() {
                return None;
            }
            if self.input.as_bytes()[self.pos] != b'<' {
                let text = self.take_until(b'<');
                return Some(DataToken::Characters(text));
            }
            if self.starts_with_doctype() {
                self.pos += "<!DOCTYPE".len();
                self.skip_through(b'>');
                return Some(DataToken::Doctype);
            }
            let next = self.input.as_bytes().get(self.pos + 1).copied();
            if matches!(next, Some(b'!' | b'?')) {
                self.pos += 2;
                self.skip_through(b'>');
                continue;
            }
            return Some(self.take_tag());
        }
    }

    pub(super) fn take_raw_text(&mut self, end_tag: &'static str) -> RawSpan<'a> {
        let start = self.pos;
        let bytes = self.input.as_bytes();
        let mut index = start;
        while index < bytes.len() {
            if self.matches_end_tag(index, end_tag) {
                let text = &self.input[start..index];
                self.pos = index + 2 + end_tag.len();
                self.skip_through(b'>');
                return RawSpan { text, closed: true };
            }
            index += 1;
        }
        let text = &self.input[start..];
        self.pos = self.input.len();
        RawSpan {
            text,
            closed: false,
        }
    }

    fn starts_with_doctype(&self) -> bool {
        let bytes = self.input.as_bytes();
        let marker = b"<!DOCTYPE";
        let end = self.pos + marker.len();
        if end > bytes.len() || !bytes[self.pos..end].eq_ignore_ascii_case(marker) {
            return false;
        }
        end == bytes.len() || ends_name(bytes[end])
    }

    fn matches_end_tag(&self, lt: usize, end_tag: &str) -> bool {
        let bytes = self.input.as_bytes();
        if lt + 1 >= bytes.len() || bytes[lt] != b'<' || bytes[lt + 1] != b'/' {
            return false;
        }
        let name = end_tag.as_bytes();
        let start = lt + 2;
        let end = start + name.len();
        if end > bytes.len() || !bytes[start..end].eq_ignore_ascii_case(name) {
            return false;
        }
        end < bytes.len() && ends_name(bytes[end])
    }

    fn take_tag(&mut self) -> DataToken<'a> {
        let bytes = self.input.as_bytes();
        let end_tag = bytes.get(self.pos + 1) == Some(&b'/');
        let name_at = self.pos + usize::from(end_tag) + 1;
        let mut name_end = name_at;
        while name_end < bytes.len() && !ends_name(bytes[name_end]) {
            name_end += 1;
        }
        let name = &self.input[name_at..name_end];
        self.pos = name_end;
        self.skip_through(b'>');
        if end_tag {
            DataToken::EndTag { name }
        } else {
            DataToken::StartTag { name }
        }
    }

    fn take_until(&mut self, byte: u8) -> &'a str {
        let rest = &self.input[self.pos..];
        let end = rest
            .bytes()
            .position(|item| item == byte)
            .unwrap_or(rest.len());
        let text = &rest[..end];
        self.pos += end;
        text
    }

    fn skip_through(&mut self, byte: u8) {
        let rest = &self.input[self.pos..];
        if let Some(index) = rest.bytes().position(|item| item == byte) {
            self.pos += index + 1;
        } else {
            self.pos = self.input.len();
        }
    }
}

fn ends_name(byte: u8) -> bool {
    byte.is_ascii_whitespace() || byte == b'/' || byte == b'>'
}
