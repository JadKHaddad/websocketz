// https://github.com/denoland/fastwebsockets/blob/d45101044613c47e7a44a14a59bde02285503c6d/src/mask.rs

#[inline]
fn unmask_easy(payload: &mut [u8], mask: [u8; 4]) {
    payload.iter_mut().enumerate().for_each(|(i, v)| {
        *v ^= mask[i & 3];
    });
}

// Faster version of `unmask_easy()` which operates on 4-byte blocks.
// https://github.com/snapview/tungstenite-rs/blob/e5efe537b87a6705467043fe44bb220ddf7c1ce8/src/protocol/frame/mask.rs#L23
//
// https://godbolt.org/z/EPTYo5jK8
#[inline]
fn unmask_fallback(buf: &mut [u8], mask: [u8; 4]) {
    let mask_u32 = u32::from_ne_bytes(mask);

    let (prefix, words, suffix) = unsafe { buf.align_to_mut::<u32>() };
    unmask_easy(prefix, mask);
    let head = prefix.len() & 3;
    let mask_u32 = if head > 0 {
        if cfg!(target_endian = "big") {
            mask_u32.rotate_left(8 * head as u32)
        } else {
            mask_u32.rotate_right(8 * head as u32)
        }
    } else {
        mask_u32
    };
    for word in words.iter_mut() {
        *word ^= mask_u32;
    }
    unmask_easy(suffix, mask_u32.to_ne_bytes());
}

/// Unmask a payload using the given 4-byte mask.
#[inline]
pub fn unmask(payload: &mut [u8], mask: [u8; 4]) {
    unmask_fallback(payload, mask)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unmask() {
        let mut payload = [0u8; 33];
        let mask = [1, 2, 3, 4];
        unmask(&mut payload, mask);
        assert_eq!(
            &payload,
            &[
                1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4,
                1, 2, 3, 4, 1
            ]
        );
    }

    #[test]
    fn length_variation_unmask() {
        for len in &[0, 2, 3, 8, 16, 18, 31, 32, 40] {
            let mut payload = std::vec![0u8; *len];
            let mask = [1, 2, 3, 4];
            unmask(&mut payload, mask);

            let expected = (0..*len)
                .map(|i| (i & 3) as u8 + 1)
                .collect::<std::vec::Vec<_>>();
            assert_eq!(payload, expected);
        }
    }

    #[test]
    fn length_variation_unmask_2() {
        for len in &[0, 2, 3, 8, 16, 18, 31, 32, 40] {
            let mut payload = std::vec![0u8; *len];
            let mask = rand::random::<[u8; 4]>();
            unmask(&mut payload, mask);

            let expected = (0..*len).map(|i| mask[i & 3]).collect::<std::vec::Vec<_>>();
            assert_eq!(payload, expected);
        }
    }
}
