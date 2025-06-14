use framez::{decode::Decoder, encode::Encoder};
use httparse::{Header, Status};

#[derive(Debug)]
pub struct Response<'buf, const N: usize> {
    version: Option<u8>,
    code: Option<u16>,
    reason: Option<&'buf str>,
    headers: [Header<'buf>; N],
}

impl<'buf, const N: usize> Response<'buf, N> {
    pub fn new(
        version: Option<u8>,
        code: Option<u16>,
        reason: Option<&'buf str>,
        headers: [Header<'buf>; N],
    ) -> Self {
        Response {
            version,
            code,
            reason,
            headers,
        }
    }

    pub const fn version(&self) -> Option<u8> {
        self.version
    }

    pub const fn code(&self) -> Option<u16> {
        self.code
    }

    pub const fn reason(&self) -> Option<&'buf str> {
        self.reason
    }

    pub const fn headers(&self) -> &[Header<'buf>; N] {
        &self.headers
    }
}

#[derive(Debug)]
pub struct ResponseCodec<const N: usize> {}

impl<const N: usize> ResponseCodec<N> {
    pub fn new() -> Self {
        ResponseCodec {}
    }
}

impl<const N: usize> framez::decode::DecodeError for ResponseCodec<N> {
    type Error = httparse::Error;
}

impl<'buf, const N: usize> Decoder<'buf> for ResponseCodec<N> {
    type Item = Response<'buf, N>;

    fn decode(&mut self, src: &'buf mut [u8]) -> Result<Option<(Self::Item, usize)>, Self::Error> {
        let mut headers = [httparse::EMPTY_HEADER; N];
        let mut response = httparse::Response::new(&mut headers);

        match response.parse(src) {
            Ok(Status::Complete(len)) => Ok(Some((
                Response::new(response.version, response.code, response.reason, headers),
                len,
            ))),
            Ok(Status::Partial) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

#[derive(Debug)]
pub struct Request<'headers, 'buf> {
    method: &'buf str,
    path: &'buf str,
    headers: &'headers [Header<'buf>],
    additional_headers: &'headers [Header<'buf>],
}

impl<'headers, 'buf> Request<'headers, 'buf> {
    pub fn new(
        method: &'buf str,
        path: &'buf str,
        headers: &'headers [Header<'buf>],
        additional_headers: &'headers [Header<'buf>],
    ) -> Self {
        Request {
            method,
            path,
            headers,
            additional_headers,
        }
    }
}

#[derive(Debug)]
pub struct RequestCodec {}

impl RequestCodec {
    pub fn new() -> Self {
        RequestCodec {}
    }
}

impl Encoder<Request<'_, '_>> for RequestCodec {
    type Error = ();

    fn encode(&mut self, item: Request<'_, '_>, dst: &mut [u8]) -> Result<usize, Self::Error> {
        let mut pos = 0;

        fn write_bytes(dst: &mut [u8], pos: &mut usize, data: &[u8]) -> Result<(), ()> {
            if *pos + data.len() > dst.len() {
                return Err(());
            }

            dst[*pos..*pos + data.len()].copy_from_slice(data);

            *pos += data.len();

            Ok(())
        }

        // Request line: METHOD PATH HTTP/1.1\r\n
        write_bytes(dst, &mut pos, item.method.as_bytes())?;
        write_bytes(dst, &mut pos, b" ")?;
        write_bytes(dst, &mut pos, item.path.as_bytes())?;
        write_bytes(dst, &mut pos, b" HTTP/1.1\r\n")?;

        // Headers
        for header in item.headers.iter() {
            write_bytes(dst, &mut pos, header.name.as_bytes())?;
            write_bytes(dst, &mut pos, b": ")?;
            write_bytes(dst, &mut pos, header.value)?;
            write_bytes(dst, &mut pos, b"\r\n")?;
        }

        for header in item.additional_headers.iter() {
            write_bytes(dst, &mut pos, header.name.as_bytes())?;
            write_bytes(dst, &mut pos, b": ")?;
            write_bytes(dst, &mut pos, header.value)?;
            write_bytes(dst, &mut pos, b"\r\n")?;
        }

        // End of headers
        write_bytes(dst, &mut pos, b"\r\n")?;

        Ok(pos)
    }
}
