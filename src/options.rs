use crate::http::Header;

// TODO: add the error type
// TODO: maybe move the check if the path is empty to the codec. This makes the api clearer. We would have a new function with the path and that is it.

#[derive(Debug)]
pub struct ConnectOptions<'a, 'b> {
    /// Must not be empty
    pub path: &'a str,
    pub headers: &'a [Header<'b>],
}

impl<'a, 'b> Default for ConnectOptions<'a, 'b> {
    fn default() -> Self {
        Self::default()
    }
}

impl<'a, 'b> ConnectOptions<'a, 'b> {
    pub fn new(path: &'a str) -> Result<Self, ()> {
        Self::default().with_path(path)
    }

    pub const fn new_unchecked(path: &'a str) -> Self {
        Self::default().with_path_unchecked(path)
    }

    pub const fn path(&self) -> &str {
        self.path
    }

    pub fn with_path(mut self, path: &'a str) -> Result<Self, ()> {
        // TODO: test empty str
        self.path = path;
        Ok(self)
    }

    pub const fn with_path_unchecked(mut self, path: &'a str) -> Self {
        self.path = path;
        self
    }

    pub const fn headers(&self) -> &[Header<'b>] {
        self.headers
    }

    pub const fn with_headers(mut self, headers: &'a [Header<'b>]) -> Self {
        self.headers = headers;
        self
    }

    const fn default() -> Self {
        Self {
            path: "/",
            headers: &[],
        }
    }
}

#[derive(Debug, Default)]
pub struct AcceptOptions<'a, 'b> {
    pub headers: &'a [Header<'b>],
}

impl<'a, 'b> AcceptOptions<'a, 'b> {
    pub const fn with_headers(mut self, headers: &'a [Header<'b>]) -> Self {
        self.headers = headers;
        self
    }

    pub const fn headers(&self) -> &[Header<'b>] {
        self.headers
    }
}
