use std::fmt::Display;
use std::ops::Range;
use std::string::FromUtf8Error;

/// A [`std::string::FromUtf8Error`] formatted with the text decoded in a best-effort manner.
pub(crate) struct FromUtf8ErrorContext<'inner> {
    inner: &'inner FromUtf8Error,
    max_size: usize,
}

impl<'a> FromUtf8ErrorContext<'a> {
    pub(crate) fn new(inner: &'a FromUtf8Error, max_size: usize) -> Self {
        Self { inner, max_size }
    }

    /// Get a 'window' of bytes to display in the error message.
    ///
    /// This is a range of (at most) `max_size` that the input can be sliced on to display the
    /// portion of input around the encoding error.
    fn window(&self) -> Range<usize> {
        let bytes = self.inner.as_bytes();
        let mut range = self.window_unadjusted();

        if range.start != 0 && !is_codepoint_boundary(bytes[range.start]) {
            // Note: I think this will always be adjusted up because the input up to the error
            // index is valid UTF-8, and the lower bound is always before the error index.
            range.start = self
                .adjust_index_up(range.start)
                .or_else(|| self.adjust_index_down(range.start))
                .unwrap_or(range.start);
        }

        if range.end != bytes.len() && !is_codepoint_boundary(bytes[range.end]) {
            range.end = self
                .adjust_index_down(range.end)
                .or_else(|| self.adjust_index_up(range.end))
                .unwrap_or(range.end);
        }

        range
    }

    /// Get an unadjusted 'window' of bytes to display in the error message.
    ///
    /// The indexes in this range have not been checked to make sure they lie on UTF-8 boundaries.
    fn window_unadjusted(&self) -> Range<usize> {
        let bytes = self.inner.as_bytes();
        if bytes.len() <= self.max_size {
            return 0..bytes.len();
        }

        // Half the length of the window.
        let half_window = self.max_size / 2;
        let error_index = self.inner.utf8_error().valid_up_to();

        let upper_bound = error_index + half_window;
        if upper_bound >= bytes.len() {
            // The natural window centered on `error_index` extends past the end of the input. Use
            // the end of the input as the right endpoint.
            return bytes.len() - self.max_size..bytes.len();
        }

        let lower_bound = error_index.checked_sub(half_window);
        if lower_bound.is_none() {
            // The natural window centered on `error_index` extends before the start of the input. Use
            // the start of the input as the left endpoint.
            return 0..self.max_size;
        }

        // The natural window is contained entirely within the input.
        error_index - half_window..error_index + half_window
    }

    /// Adjust the given index so that it lies on a UTF-8 boundary in the input, if possible.
    ///
    /// This is done by adjusting the index up to 3 bytes downwards.
    fn adjust_index_down(&self, index: usize) -> Option<usize> {
        // Logic adapted from unstable `std` method:
        // https://github.com/rust-lang/rust/blob/a7e4de13c1785819f4d61da41f6704ed69d5f203/library/core/src/str/mod.rs#L264-L276
        let bytes = self.inner.as_bytes();
        let lower_bound = index.saturating_sub(3);
        bytes[lower_bound..=index]
            .iter()
            .rposition(|&b| is_codepoint_boundary(b))
            .map(|i| i + lower_bound)
    }

    /// Adjust the given index so that it lies on a UTF-8 boundary in the input, if possible.
    ///
    /// This is done by adjusting the index up to 3 bytes upwards.
    fn adjust_index_up(&self, index: usize) -> Option<usize> {
        // Logic adapted from unstable `std` method:
        // https://github.com/rust-lang/rust/blob/a7e4de13c1785819f4d61da41f6704ed69d5f203/library/core/src/str/mod.rs#L302-L311
        let bytes = self.inner.as_bytes();
        let upper_bound = Ord::min(index + 4, bytes.len());
        bytes[index..upper_bound]
            .iter()
            .position(|&b| is_codepoint_boundary(b))
            .map(|i| i + index)
    }
}

impl<'a> Display for FromUtf8ErrorContext<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let bytes = self.inner.as_bytes();
        if bytes.len() <= self.max_size {
            write!(f, "{:?}", String::from_utf8_lossy(bytes))
        } else {
            let range = self.window();
            let before = range.start;
            let after = bytes.len() - range.end;

            if before != 0 {
                write!(f, "{} ", ByteCount(before))?;
            }

            // TODO: It might be nice to print the hex values of the bytes like `\x62` instead of
            // just `�` U+FFFD REPLACEMENT CHARACTER.
            write!(f, "{:?}", String::from_utf8_lossy(&bytes[range]))?;

            if after != 0 {
                write!(f, " {}", ByteCount(after))?;
            }

            Ok(())
        }
    }
}

fn is_codepoint_boundary(byte: u8) -> bool {
    // Stolen from a private `std` method:
    // https://github.com/rust-lang/rust/blob/a7e4de13c1785819f4d61da41f6704ed69d5f203/library/core/src/num/mod.rs#L1101-L1104
    // This is bit magic equivalent to: b < 128 || b >= 192
    (byte as i8) >= -0x40
}

struct ByteCount(usize);

impl Display for ByteCount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 == 1 {
            write!(f, "[1 byte]")
        } else {
            write!(f, "[{} bytes]", self.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn err(bytes: &[u8]) -> FromUtf8Error {
        String::from_utf8(bytes.to_vec()).unwrap_err()
    }

    #[test]
    fn test_simple() {
        assert_eq!(
            FromUtf8ErrorContext {
                inner: &err(b"puppy\xc0doggy"),
                max_size: 32,
            }
            .to_string(),
            "\"puppy�doggy\""
        );
    }

    #[test]
    fn test_truncation() {
        // Adjusts the lower bound up (3->4) and the upper bound down (35->32).
        assert_eq!(
            FromUtf8ErrorContext {
                inner: &err(b"\xf0\x9f\x98\x8a\
                    \xf0\x9f\x98\x8a\
                    \xe2\x9c\x93\
                    \xf0\x9f\x98\x8a\
                    \xf0\x9f\x98\x8a\
                    \xc0\
                    \xf0\x9f\x98\x8a\
                    \xf0\x9f\x98\x8a\
                    \xf0\x9f\x98\x8a\
                    \xf0\x9f\x98\x8a\
                    \xf0\x9f\x98\x8a"),
                max_size: 32,
            }
            .to_string(),
            "[4 bytes] \"😊✓😊😊�😊😊😊\" [8 bytes]"
        );
    }

    #[test]
    fn test_truncation_up() {
        // Adjusts the lower bound up (3->4) and the upper bound up (35->37).
        assert_eq!(
            FromUtf8ErrorContext {
                inner: &err(b"\xf0\x9f\x98\x8a\
                    \xf0\x9f\x98\x8a\
                    \xe2\x9c\x93\
                    \xf0\x9f\x98\x8a\
                    \xf0\x9f\x98\x8a\
                    \xc0\
                    \xf0\x9f\x98\x8a\
                    \xf0\x9f\x98\x8a\
                    \xf0\x9f\x98\x8a\
                    \x80\x80\x80\x80\
                    \x80\x62\x80\x80"),
                max_size: 32,
            }
            .to_string(),
            "[4 bytes] \"😊✓😊😊�😊😊😊�����\" [3 bytes]"
        );
    }

    #[test]
    fn test_truncation_near_end() {
        assert_eq!(
            FromUtf8ErrorContext {
                inner: &err(b"puppy puppy puppy puppy puppy \
                doggy doggy doggy doggy\xc0doggy"),
                max_size: 32,
            }
            .to_string(),
            "[27 bytes] \"py doggy doggy doggy doggy�doggy\""
        );

        assert_eq!(
            FromUtf8ErrorContext {
                inner: &err(b"puppy puppy puppy puppy puppy \
                doggy doggy doggy doggy\xc0"),
                max_size: 32,
            }
            .to_string(),
            "[22 bytes] \"y puppy doggy doggy doggy doggy�\""
        );
    }

    #[test]
    fn test_truncation_near_start() {
        assert_eq!(
            FromUtf8ErrorContext {
                inner: &err(b"puppy\xc0puppy puppy puppy puppy \
                doggy doggy doggy doggy doggy"),
                max_size: 32,
            }
            .to_string(),
            "\"puppy�puppy puppy puppy puppy do\" [27 bytes]"
        );

        assert_eq!(
            FromUtf8ErrorContext {
                inner: &err(b"\xc0puppy puppy puppy puppy puppy \
                doggy doggy doggy doggy"),
                max_size: 32,
            }
            .to_string(),
            "\"�puppy puppy puppy puppy puppy d\" [22 bytes]"
        );
    }
}
