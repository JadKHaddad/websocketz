use framez::{decode::Decoder, encode::Encoder};
use httparse::{Header, Status};

use crate::error::{HttpDecodeError, HttpEncodeError};

#[derive(Debug)]
pub struct Response<'buf, const N: usize> {
    code: Option<u16>,
    headers: [Header<'buf>; N],
}

impl<'buf, const N: usize> Response<'buf, N> {
    pub const fn new(code: Option<u16>, headers: [Header<'buf>; N]) -> Self {
        Response { code, headers }
    }

    pub const fn code(&self) -> Option<u16> {
        self.code
    }

    pub fn header(&self, name: &str) -> Option<&Header<'buf>> {
        self.headers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case(name))
    }

    pub fn header_value(&self, name: &str) -> Option<&'buf [u8]> {
        self.header(name).map(|h| h.value)
    }

    pub fn header_value_str(&self, name: &str) -> Option<&'buf str> {
        self.header_value(name)
            .and_then(|v| core::str::from_utf8(v).ok())
    }
}

#[derive(Debug)]
pub struct ResponseCodec<const N: usize> {}

impl<const N: usize> ResponseCodec<N> {
    pub const fn new() -> Self {
        ResponseCodec {}
    }
}

impl<const N: usize> framez::decode::DecodeError for ResponseCodec<N> {
    type Error = HttpDecodeError;
}

impl<'buf, const N: usize> Decoder<'buf> for ResponseCodec<N> {
    type Item = Response<'buf, N>;

    fn decode(&mut self, src: &'buf mut [u8]) -> Result<Option<(Self::Item, usize)>, Self::Error> {
        let mut headers = [httparse::EMPTY_HEADER; N];
        let mut response = httparse::Response::new(&mut headers);

        match response.parse(src)? {
            Status::Complete(len) => Ok(Some((Response::new(response.code, headers), len))),
            Status::Partial => Ok(None),
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
    const fn new(
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

    pub const fn get(
        path: &'buf str,
        headers: &'headers [Header<'buf>],
        additional_headers: &'headers [Header<'buf>],
    ) -> Self {
        Self::new("GET", path, headers, additional_headers)
    }
}

#[derive(Debug)]
pub struct RequestCodec {}

impl RequestCodec {
    pub const fn new() -> Self {
        RequestCodec {}
    }
}

impl Encoder<Request<'_, '_>> for RequestCodec {
    type Error = HttpEncodeError;

    fn encode(&mut self, item: Request<'_, '_>, dst: &mut [u8]) -> Result<usize, Self::Error> {
        let mut pos = 0;

        fn write_bytes(
            dst: &mut [u8],
            pos: &mut usize,
            data: &[u8],
        ) -> Result<(), HttpEncodeError> {
            if *pos + data.len() > dst.len() {
                return Err(HttpEncodeError::BufferTooSmall);
            }

            dst[*pos..*pos + data.len()].copy_from_slice(data);

            *pos += data.len();

            Ok(())
        }

        write_bytes(dst, &mut pos, item.method.as_bytes())?;
        write_bytes(dst, &mut pos, b" ")?;
        write_bytes(dst, &mut pos, item.path.as_bytes())?;
        write_bytes(dst, &mut pos, b" HTTP/1.1\r\n")?;

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

        write_bytes(dst, &mut pos, b"\r\n")?;

        Ok(pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod decode {
        use std::vec::Vec;

        use super::*;

        const OK_RESPONSE: &[u8] =
            b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\n\0\0\0\0\0\0";

        fn ok_response() -> Vec<u8> {
            OK_RESPONSE.to_vec()
        }

        fn partial_response() -> Vec<u8> {
            OK_RESPONSE[..16].to_vec()
        }

        #[test]
        fn ok() {
            let mut response = ok_response();
            let mut codec = ResponseCodec::<2>::new();

            let (response, len) = codec.decode(&mut response).unwrap().unwrap();

            assert_eq!(response.code(), Some(200));
            assert_eq!(
                response.header_value_str("content-type"),
                Some("text/plain")
            );
            assert_eq!(response.header_value_str("Connection"), Some("close"));

            assert_eq!(len, 64);
        }

        #[test]
        fn too_many_headers() {
            let mut response = ok_response();
            let mut codec = ResponseCodec::<1>::new();

            let error = codec.decode(&mut response).unwrap_err();

            assert!(matches!(
                error,
                HttpDecodeError::Parse(httparse::Error::TooManyHeaders)
            ));
        }

        #[test]
        fn partial() {
            let mut response = partial_response();
            let mut codec = ResponseCodec::<2>::new();

            let result = codec.decode(&mut response).unwrap();

            assert!(result.is_none());
        }
    }

    mod encode {
        use super::*;

        const OK_REQUEST: &[u8] =
            b"GET /index.html HTTP/1.1\r\nHost: example.com\r\nUser-Agent: test-agent\r\nAccept: text/html\r\n\r\n";

        const HEADERS: &[Header] = &[
            Header {
                name: "Host",
                value: b"example.com",
            },
            Header {
                name: "User-Agent",
                value: b"test-agent",
            },
        ];

        const ADDITIONAL_HEADERS: &[Header] = &[Header {
            name: "Accept",
            value: b"text/html",
        }];

        #[test]
        fn ok() {
            let request = Request::get("/index.html", HEADERS, ADDITIONAL_HEADERS);

            let mut codec = RequestCodec::new();

            let mut buf = std::vec![0; 1024];

            let len = codec.encode(request, &mut buf).unwrap();

            assert!(len == OK_REQUEST.len());
            assert_eq!(&buf[..len], OK_REQUEST);
        }

        #[test]
        fn buffer_too_small() {
            let request = Request::get("/index.html", HEADERS, ADDITIONAL_HEADERS);

            let mut codec = RequestCodec::new();

            let mut buf = std::vec![0; 10];

            let error = codec.encode(request, &mut buf).unwrap_err();

            assert!(matches!(error, HttpEncodeError::BufferTooSmall));
        }
    }
}
