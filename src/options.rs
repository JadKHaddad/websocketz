use crate::http::Header;

#[derive(Debug, thiserror::Error)]
pub enum ConnectOptionsError {
    #[error("path must not be empty")]
    EmptyPath,
}

#[derive(Debug)]
#[non_exhaustive]
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
    /// Creates a new [`ConnectOptions`] with the given path, returning an error if the path is empty.
    pub fn new(path: &'a str) -> Result<Self, ConnectOptionsError> {
        Self::default().with_path(path)
    }

    /// Creates a new [`ConnectOptions`] with the given path without checking if the path is empty.
    pub const fn new_unchecked(path: &'a str) -> Self {
        Self::default().with_path_unchecked(path)
    }

    /// Returns the path
    pub const fn path(&self) -> &str {
        self.path
    }

    /// Sets the path, returning an error if the path is empty.
    pub fn with_path(mut self, path: &'a str) -> Result<Self, ConnectOptionsError> {
        if path.trim().is_empty() {
            return Err(ConnectOptionsError::EmptyPath);
        };

        self.path = path.trim();
        Ok(self)
    }

    /// Sets the path without checking if it is empty.
    pub const fn with_path_unchecked(mut self, path: &'a str) -> Self {
        self.path = path;
        self
    }

    /// Returns the headers
    pub const fn headers(&self) -> &[Header<'b>] {
        self.headers
    }

    /// Sets the headers.
    pub const fn with_headers(mut self, headers: &'a [Header<'b>]) -> Self {
        self.headers = headers;
        self
    }

    /// Creates a new [`ConnectOptions`] with default values.
    ///
    /// This is an internal `const` function alternative to [`Default::default()`].
    const fn default() -> Self {
        Self {
            path: "/",
            headers: &[],
        }
    }
}

#[derive(Debug, Default)]
#[non_exhaustive]
pub struct AcceptOptions<'a, 'b> {
    pub headers: &'a [Header<'b>],
}

impl<'a, 'b> AcceptOptions<'a, 'b> {
    /// Sets the headers.
    pub const fn with_headers(mut self, headers: &'a [Header<'b>]) -> Self {
        self.headers = headers;
        self
    }

    /// Returns the headers.
    pub const fn headers(&self) -> &[Header<'b>] {
        self.headers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_path() {
        let error = ConnectOptions::new("").unwrap_err();
        assert!(matches!(error, ConnectOptionsError::EmptyPath));

        let error = ConnectOptions::new("    ").unwrap_err();
        assert!(matches!(error, ConnectOptionsError::EmptyPath));
    }

    #[test]
    fn path_is_trimmed() {
        let options = ConnectOptions::new("  /test  ").unwrap();
        assert_eq!(options.path(), "/test");

        let options = ConnectOptions::new("/test").unwrap();
        assert_eq!(options.path(), "/test");
    }
}
