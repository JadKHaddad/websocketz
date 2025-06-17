use crate::CloseCode;

#[derive(Debug)]
pub struct CloseFrame<'a> {
    /// The reason as a code.
    code: CloseCode,
    /// The reason as text string.
    reason: &'a str,
}

impl<'a> CloseFrame<'a> {
    /// Creates a new [`CloseFrame`].
    pub const fn new(code: CloseCode, reason: &'a str) -> Self {
        Self { code, reason }
    }

    pub const fn no_reason(code: CloseCode) -> Self {
        Self::new(code, "")
    }

    /// Returns the close code.
    pub const fn code(&self) -> CloseCode {
        self.code
    }

    /// Returns the reason as a string slice.
    pub const fn reason(&self) -> &'a str {
        self.reason
    }
}
