use httparse::Header;

#[derive(Debug)]
pub struct ConnectOptions<'a, 'b> {
    pub path: &'a str,
    pub headers: &'a [Header<'b>],
}

impl<'a, 'b> ConnectOptions<'a, 'b> {
    pub const fn new(path: &'a str, headers: &'a [Header<'b>]) -> Self {
        Self { path, headers }
    }

    pub const fn path(&self) -> &str {
        self.path
    }

    pub const fn headers(&self) -> &[Header<'b>] {
        self.headers
    }
}

#[derive(Debug)]
pub struct AcceptOptions<'a, 'b> {
    pub headers: &'a [Header<'b>],
}

impl<'a, 'b> AcceptOptions<'a, 'b> {
    pub const fn new(headers: &'a [Header<'b>]) -> Self {
        Self { headers }
    }

    pub const fn headers(&self) -> &[Header<'b>] {
        self.headers
    }
}
