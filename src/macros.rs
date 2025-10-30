/// Read a [`Message`](crate::Message) from a [`WebSocket`](crate::WebSocket) or [`WebSocketRead`](crate::WebSocketRead).
///
/// # Parameters
///
/// - `$websocketz`: The WebSocket instance to read from.
///
/// # Return
/// - `Some(Ok(Message))`: A message was successfully read.
/// - `Some(Err(Error))`: An error occurred while reading a message. The caller should stop reading.
/// - `None`: The WebSocket connection has been closed (EOF). The caller should stop reading.
#[macro_export]
macro_rules! next {
    ($websocketz:expr) => {{
        'next: loop {
            match $websocketz
                .caller()
                .call(
                    $websocketz.auto(),
                    &mut $websocketz.core.framed.core.codec,
                    &mut $websocketz.core.framed.core.inner,
                    &mut $websocketz.core.framed.core.state.read,
                    &mut $websocketz.core.framed.core.state.write,
                    &mut $websocketz.core.fragments_state,
                    &mut $websocketz.core.state,
                )
                .await
            {
                Some(Ok(None)) => continue 'next,
                Some(Ok(Some(item))) => break 'next Some(Ok(item)),
                Some(Err(err)) => break 'next Some(Err(err)),
                None => break 'next None,
            }
        }
    }};
}

/// Send a [`Message`](crate::Message) through a [`WebSocket`](crate::WebSocket) or [`WebSocketWrite`](crate::WebSocketWrite).
///
/// # Parameters
/// - `$websocketz`: The WebSocket instance to send the message through.
/// - `$message`: The message to send.
#[macro_export]
macro_rules! send {
    ($websocketz:expr, $message:expr) => {{
        $crate::functions::send(
            &mut $websocketz.core.framed.core.codec,
            &mut $websocketz.core.framed.core.inner,
            &mut $websocketz.core.framed.core.state.write,
            &mut $websocketz.core.state,
            $message,
        )
        .await
    }};
}

/// Send a fragmented [`Message`](crate::Message) through a [`WebSocket`](crate::WebSocket) or [`WebSocketWrite`](crate::WebSocketWrite).
///
/// # Parameters
/// - `$websocketz`: The WebSocket instance to send the message through.
/// - `$message`: The message to send.
/// - `$fragment_size`: The size of each fragment.
#[macro_export]
macro_rules! send_fragmented {
    ($websocketz:expr, $message:expr, $fragment_size:expr) => {{
        $crate::functions::send_fragmented(
            &mut $websocketz.core.framed.core.codec,
            &mut $websocketz.core.framed.core.inner,
            &mut $websocketz.core.framed.core.state.write,
            &mut $websocketz.core.state,
            $message,
            $fragment_size,
        )
        .await
    }};
}
