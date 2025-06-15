use httparse::Header;

#[derive(Debug)]
pub struct Options<'headers, 'a> {
    pub(crate) path: &'a str,
    pub(crate) headers: &'headers [Header<'a>],
}

impl<'headers, 'a> Options<'headers, 'a> {
    /// Creates a new [`Options`].
    pub fn new(path: &'a str, headers: &'headers [Header<'a>]) -> Self {
        Options { path, headers }
    }
}
