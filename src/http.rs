use framez::{decode::Decoder, encode::Encoder};
use httparse::{Header, Status};

use crate::error::{HttpDecodeError, HttpEncodeError};

pub trait HeaderExt {
    fn header(&self, name: &str) -> Option<&Header<'_>>;

    fn header_value(&self, name: &str) -> Option<&'_ [u8]> {
        self.header(name).map(|h| h.value)
    }

    fn header_value_str(&self, name: &str) -> Option<&'_ str> {
        self.header_value(name)
            .and_then(|v| core::str::from_utf8(v).ok())
    }
}

impl HeaderExt for [Header<'_>] {
    fn header(&self, name: &str) -> Option<&Header<'_>> {
        self.iter().find(|h| h.name.eq_ignore_ascii_case(name))
    }
}

#[derive(Debug)]
pub struct OutResponse<'headers, 'buf> {
    code: &'buf str,
    status: &'buf str,
    headers: &'headers [Header<'buf>],
    additional_headers: &'headers [Header<'buf>],
}

impl<'headers, 'buf> OutResponse<'headers, 'buf> {
    const fn new(
        code: &'buf str,
        status: &'buf str,
        headers: &'headers [Header<'buf>],
        additional_headers: &'headers [Header<'buf>],
    ) -> Self {
        OutResponse {
            code,
            status,
            headers,
            additional_headers,
        }
    }

    pub const fn switching_protocols(
        headers: &'headers [Header<'buf>],
        additional_headers: &'headers [Header<'buf>],
    ) -> Self {
        Self::new("101", "Switching Protocols", headers, additional_headers)
    }
}

#[derive(Debug)]
pub struct OutResponseCodec {}

impl OutResponseCodec {
    pub const fn new() -> Self {
        OutResponseCodec {}
    }
}

impl Encoder<OutResponse<'_, '_>> for OutResponseCodec {
    type Error = HttpEncodeError;

    fn encode(&mut self, item: OutResponse<'_, '_>, dst: &mut [u8]) -> Result<usize, Self::Error> {
        let mut pos = 0;

        write(dst, &mut pos, b"HTTP/1.1 ")?;
        write(dst, &mut pos, item.code.as_bytes())?;
        write(dst, &mut pos, b" ")?;
        write(dst, &mut pos, item.status.as_bytes())?;
        write(dst, &mut pos, b"\r\n")?;

        for header in item.headers.iter() {
            write(dst, &mut pos, header.name.as_bytes())?;
            write(dst, &mut pos, b": ")?;
            write(dst, &mut pos, header.value)?;
            write(dst, &mut pos, b"\r\n")?;
        }

        for header in item.additional_headers.iter() {
            write(dst, &mut pos, header.name.as_bytes())?;
            write(dst, &mut pos, b": ")?;
            write(dst, &mut pos, header.value)?;
            write(dst, &mut pos, b"\r\n")?;
        }

        write(dst, &mut pos, b"\r\n")?;

        Ok(pos)
    }
}

#[derive(Debug)]
pub struct InResponse<'buf, const N: usize> {
    code: Option<u16>,
    headers: [Header<'buf>; N],
}

impl<'buf, const N: usize> InResponse<'buf, N> {
    pub const fn new(code: Option<u16>, headers: [Header<'buf>; N]) -> Self {
        InResponse { code, headers }
    }

    pub const fn code(&self) -> Option<u16> {
        self.code
    }

    pub const fn headers(&self) -> &[Header<'buf>] {
        &self.headers
    }
}

#[derive(Debug)]
pub struct InResponseCodec<const N: usize> {}

impl<const N: usize> InResponseCodec<N> {
    pub const fn new() -> Self {
        InResponseCodec {}
    }
}

impl<const N: usize> framez::decode::DecodeError for InResponseCodec<N> {
    type Error = HttpDecodeError;
}

impl<'buf, const N: usize> Decoder<'buf> for InResponseCodec<N> {
    type Item = InResponse<'buf, N>;

    fn decode(&mut self, src: &'buf mut [u8]) -> Result<Option<(Self::Item, usize)>, Self::Error> {
        let mut headers = [httparse::EMPTY_HEADER; N];
        let mut response = httparse::Response::new(&mut headers);

        match response.parse(src)? {
            Status::Complete(len) => Ok(Some((InResponse::new(response.code, headers), len))),
            Status::Partial => Ok(None),
        }
    }
}

#[derive(Debug)]
pub struct OutRequest<'headers, 'buf> {
    method: &'buf str,
    path: &'buf str,
    headers: &'headers [Header<'buf>],
    additional_headers: &'headers [Header<'buf>],
}

impl<'headers, 'buf> OutRequest<'headers, 'buf> {
    const fn new(
        method: &'buf str,
        path: &'buf str,
        headers: &'headers [Header<'buf>],
        additional_headers: &'headers [Header<'buf>],
    ) -> Self {
        OutRequest {
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
pub struct OutRequestCodec {}

impl OutRequestCodec {
    pub const fn new() -> Self {
        OutRequestCodec {}
    }
}

impl Encoder<OutRequest<'_, '_>> for OutRequestCodec {
    type Error = HttpEncodeError;

    fn encode(&mut self, item: OutRequest<'_, '_>, dst: &mut [u8]) -> Result<usize, Self::Error> {
        let mut pos = 0;

        write(dst, &mut pos, item.method.as_bytes())?;
        write(dst, &mut pos, b" ")?;
        write(dst, &mut pos, item.path.as_bytes())?;
        write(dst, &mut pos, b" HTTP/1.1\r\n")?;

        for header in item.headers.iter() {
            write(dst, &mut pos, header.name.as_bytes())?;
            write(dst, &mut pos, b": ")?;
            write(dst, &mut pos, header.value)?;
            write(dst, &mut pos, b"\r\n")?;
        }

        for header in item.additional_headers.iter() {
            write(dst, &mut pos, header.name.as_bytes())?;
            write(dst, &mut pos, b": ")?;
            write(dst, &mut pos, header.value)?;
            write(dst, &mut pos, b"\r\n")?;
        }

        write(dst, &mut pos, b"\r\n")?;

        Ok(pos)
    }
}

#[derive(Debug)]
pub struct InRequest<'buf, const N: usize> {
    headers: [Header<'buf>; N],
}

impl<'buf, const N: usize> InRequest<'buf, N> {
    pub const fn new(headers: [Header<'buf>; N]) -> Self {
        InRequest { headers }
    }

    pub const fn headers(&self) -> &[Header<'buf>] {
        &self.headers
    }
}

#[derive(Debug)]
pub struct InRequestCodec<const N: usize> {}

impl<const N: usize> InRequestCodec<N> {
    pub const fn new() -> Self {
        InRequestCodec {}
    }
}

impl<const N: usize> framez::decode::DecodeError for InRequestCodec<N> {
    type Error = HttpDecodeError;
}

impl<'buf, const N: usize> Decoder<'buf> for InRequestCodec<N> {
    type Item = InRequest<'buf, N>;

    fn decode(&mut self, src: &'buf mut [u8]) -> Result<Option<(Self::Item, usize)>, Self::Error> {
        let mut headers = [httparse::EMPTY_HEADER; N];
        let mut request = httparse::Request::new(&mut headers);

        match request.parse(src)? {
            Status::Complete(len) => Ok(Some((InRequest::new(headers), len))),
            Status::Partial => Ok(None),
        }
    }
}

fn write(dst: &mut [u8], pos: &mut usize, data: &[u8]) -> Result<(), HttpEncodeError> {
    if *pos + data.len() > dst.len() {
        return Err(HttpEncodeError::BufferTooSmall);
    }

    dst[*pos..*pos + data.len()].copy_from_slice(data);

    *pos += data.len();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    mod decode {
        use std::vec::Vec;

        use super::*;

        mod response {
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
                let mut codec = InResponseCodec::<2>::new();

                let (response, len) = codec.decode(&mut response).unwrap().unwrap();

                assert_eq!(response.code(), Some(200));
                assert_eq!(
                    response.headers().header_value_str("content-type"),
                    Some("text/plain")
                );
                assert_eq!(
                    response.headers().header_value_str("Connection"),
                    Some("close")
                );
                assert_eq!(response.headers().len(), 2);

                assert_eq!(len, 64);
            }

            #[test]
            fn too_many_headers() {
                let mut response = ok_response();
                let mut codec = InResponseCodec::<1>::new();

                let error = codec.decode(&mut response).unwrap_err();

                assert!(matches!(
                    error,
                    HttpDecodeError::Parse(httparse::Error::TooManyHeaders)
                ));
            }

            #[test]
            fn partial() {
                let mut response = partial_response();
                let mut codec = InResponseCodec::<2>::new();

                let result = codec.decode(&mut response).unwrap();

                assert!(result.is_none());
            }
        }

        mod request {
            use super::*;

            const OK_REQUEST: &[u8] =
            b"GET /index.html HTTP/1.1\r\nHost: example.com\r\nUser-Agent: test-agent\r\nAccept: text/html\r\n\r\n\0\0\0\0\0\0";

            fn ok_request() -> Vec<u8> {
                OK_REQUEST.to_vec()
            }

            fn partial_request() -> Vec<u8> {
                OK_REQUEST[..16].to_vec()
            }

            #[test]
            fn ok() {
                let mut request = ok_request();
                let mut codec = InRequestCodec::<3>::new();

                let (request, len) = codec.decode(&mut request).unwrap().unwrap();

                assert_eq!(
                    request.headers().header_value_str("Host"),
                    Some("example.com")
                );
                assert_eq!(
                    request.headers().header_value_str("User-agent"),
                    Some("test-agent")
                );
                assert_eq!(
                    request.headers().header_value_str("accept"),
                    Some("text/html")
                );
                assert_eq!(len, 90);

                assert_eq!(request.headers().len(), 3);
            }

            #[test]
            fn too_many_headers() {
                let mut request = ok_request();
                let mut codec = InRequestCodec::<2>::new();

                let error = codec.decode(&mut request).unwrap_err();

                assert!(matches!(
                    error,
                    HttpDecodeError::Parse(httparse::Error::TooManyHeaders)
                ));
            }

            #[test]
            fn partial() {
                let mut request = partial_request();
                let mut codec = InRequestCodec::<3>::new();

                let result = codec.decode(&mut request).unwrap();

                assert!(result.is_none());
            }
        }
    }

    mod encode {
        use super::*;

        mod request {
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
                let request = OutRequest::get("/index.html", HEADERS, ADDITIONAL_HEADERS);

                let mut codec = OutRequestCodec::new();

                let mut buf = std::vec![0; 1024];

                let len = codec.encode(request, &mut buf).unwrap();

                assert!(len == OK_REQUEST.len());
                assert_eq!(&buf[..len], OK_REQUEST);
            }

            #[test]
            fn buffer_too_small() {
                let request = OutRequest::get("/index.html", HEADERS, ADDITIONAL_HEADERS);

                let mut codec = OutRequestCodec::new();

                let mut buf = std::vec![0; 10];

                let error = codec.encode(request, &mut buf).unwrap_err();

                assert!(matches!(error, HttpEncodeError::BufferTooSmall));
            }
        }

        mod response {
            use super::*;

            const OK_RESPONSE: &[u8] =
                b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\n";

            const SWITCHING_PROTOCOLS_RESPONSE: &[u8] =
                b"HTTP/1.1 101 Switching Protocols\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\n";

            const HEADERS: &[Header] = &[Header {
                name: "Content-Type",
                value: b"text/plain",
            }];

            const ADDITIONAL_HEADERS: &[Header] = &[Header {
                name: "Connection",
                value: b"close",
            }];

            #[test]
            fn ok() {
                let response = OutResponse::new("200", "OK", HEADERS, ADDITIONAL_HEADERS);

                let mut codec = OutResponseCodec::new();

                let mut buf = std::vec![0; 1024];

                let len = codec.encode(response, &mut buf).unwrap();

                assert!(len == OK_RESPONSE.len());
                assert_eq!(&buf[..len], OK_RESPONSE);
            }

            #[test]
            fn ok_switching_protocols() {
                let response = OutResponse::switching_protocols(HEADERS, ADDITIONAL_HEADERS);

                let mut codec = OutResponseCodec::new();

                let mut buf = std::vec![0; 1024];

                let len = codec.encode(response, &mut buf).unwrap();

                assert!(len == SWITCHING_PROTOCOLS_RESPONSE.len());
                assert_eq!(&buf[..len], SWITCHING_PROTOCOLS_RESPONSE);
            }

            #[test]
            fn buffer_too_small() {
                let response = OutResponse::new("200", "OK", HEADERS, ADDITIONAL_HEADERS);

                let mut codec = OutResponseCodec::new();

                let mut buf = std::vec![0; 10];

                let error = codec.encode(response, &mut buf).unwrap_err();

                assert!(matches!(error, HttpEncodeError::BufferTooSmall));
            }
        }
    }
}
