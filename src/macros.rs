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

#[macro_export]
macro_rules! send_fragmented {
    ($websocketz:expr, $message:expr, $fragment_size:expr) => {{
        $crate::functions::send_fragmented(
            &mut $websocketz.core.framed.core.codec,
            &mut $websocketz.core.framed.core.inner,
            &mut $websocketz.core.framed.core.state.write,
            $message,
            $fragment_size,
        )
        .await
    }};
}
