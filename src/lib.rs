//! Provides the [`Utf8Output`] type, a UTF-8-decoded variant of [`std::process::Output`] (as
//! produced by [`std::process::Command::output`]).
//!
//! Construct [`Utf8Output`] from [`Output`] via the [`TryInto`] or [`TryFrom`] traits:
//!
//! ```
//! # use std::process::Command;
//! # use std::process::ExitStatus;
//! # use utf8_command::Utf8Output;
//! let output: Utf8Output = Command::new("echo")
//!     .arg("puppy")
//!     .output()
//!     .unwrap()
//!     .try_into()
//!     .unwrap();
//! assert_eq!(
//!     output,
//!     Utf8Output {
//!         status: ExitStatus::default(),
//!         stdout: String::from("puppy\n"),
//!         stderr: String::from(""),
//!     },
//! );
//! ```

#![deny(missing_docs)]

use std::fmt::Debug;
use std::fmt::Display;
use std::process::ExitStatus;
use std::process::Output;
use std::string::FromUtf8Error;

mod context;
use context::FromUtf8ErrorContext;

const ERROR_CONTEXT_BYTES: usize = 1024;

/// A UTF-8-decoded variant of [`std::process::Output`] (as
/// produced by [`std::process::Command::output`]).
///
/// Construct [`Utf8Output`] from [`Output`] via the [`TryInto`] or [`TryFrom`] traits:
///
/// ```
/// # use std::process::Command;
/// # use std::process::ExitStatus;
/// # use utf8_command::Utf8Output;
/// let output: Utf8Output = Command::new("echo")
///     .arg("puppy")
///     .output()
///     .unwrap()
///     .try_into()
///     .unwrap();
/// assert_eq!(
///     output,
///     Utf8Output {
///         status: ExitStatus::default(),
///         stdout: String::from("puppy\n"),
///         stderr: String::from(""),
///     },
/// );
/// ```
///
/// Error messages will include information about the stream that failed to decode, as well as the
/// output (with invalid UTF-8 bytes replaced with U+FFFD REPLACEMENT CHARACTER):
///
/// ```
/// # use std::process::ExitStatus;
/// # use std::process::Output;
/// # use utf8_command::Utf8Output;
/// # use utf8_command::Error;
/// let invalid = Output {
///     status: ExitStatus::default(),
///     stdout: Vec::from(b"\xc3\x28"), // Invalid 2-byte sequence.
///     stderr: Vec::from(b""),
/// };
///
/// let err: Result<Utf8Output, Error> = invalid.try_into();
/// assert_eq!(
///     err.unwrap_err().to_string(),
///     "Stdout contained invalid utf-8 sequence of 1 bytes from index 0: \"�(\""
/// );
/// ```
///
/// If there's a lot of output (currently, more than 1024 bytes), only the portion around the
/// decode error will be shown:
///
/// ```
/// # use std::process::ExitStatus;
/// # use std::process::Output;
/// # use utf8_command::Utf8Output;
/// # use utf8_command::Error;
/// let mut stdout = vec![];
/// for _ in 0..300 {
///     stdout.extend(b"puppy ");
/// }
/// // Add an invalid byte:
/// stdout[690] = 0xc0;
///
/// let invalid = Output {
///     status: ExitStatus::default(),
///     stdout,
///     stderr: Vec::from(b""),
/// };
///
/// let err: Result<Utf8Output, Error> = invalid.try_into();
/// assert_eq!(
///     err.unwrap_err().to_string(),
///     "Stdout contained invalid utf-8 sequence of 1 bytes from index 690: \
///     [178 bytes] \"y puppy puppy puppy puppy puppy puppy puppy puppy puppy \
///     puppy puppy puppy puppy puppy puppy puppy puppy puppy puppy puppy \
///     puppy puppy puppy puppy puppy puppy puppy puppy puppy puppy puppy \
///     puppy puppy puppy puppy puppy puppy puppy puppy puppy puppy puppy \
///     puppy puppy puppy puppy puppy puppy puppy puppy puppy puppy puppy \
///     puppy puppy puppy puppy puppy puppy puppy puppy puppy puppy puppy \
///     puppy puppy puppy puppy puppy puppy puppy puppy puppy puppy puppy \
///     puppy puppy puppy puppy puppy puppy puppy puppy puppy puppy �uppy \
///     puppy puppy puppy puppy puppy puppy puppy puppy puppy puppy puppy \
///     puppy puppy puppy puppy puppy puppy puppy puppy puppy puppy puppy \
///     puppy puppy puppy puppy puppy puppy puppy puppy puppy puppy puppy \
///     puppy puppy puppy puppy puppy puppy puppy puppy puppy puppy puppy \
///     puppy puppy puppy puppy puppy puppy puppy puppy puppy puppy puppy \
///     puppy puppy puppy puppy puppy puppy puppy puppy puppy puppy puppy \
///     puppy puppy puppy puppy puppy puppy puppy puppy puppy puppy puppy \
///     puppy puppy puppy puppy puppy puppy puppy pu\" [598 bytes]"
/// );
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Utf8Output {
    /// The [`std::process::Command`]'s exit status.
    pub status: ExitStatus,
    /// The contents of the [`std::process::Command`]'s [`stdout` stream][stdout], decoded as
    /// UTF-8.
    ///
    /// [stdout]: https://linux.die.net/man/3/stdout
    pub stdout: String,
    /// The contents of the [`std::process::Command`]'s [`stderr` stream][stdout], decoded as
    /// UTF-8.
    ///
    /// [stdout]: https://linux.die.net/man/3/stdout
    pub stderr: String,
}

impl TryFrom<Output> for Utf8Output {
    type Error = Error;

    fn try_from(
        Output {
            status,
            stdout,
            stderr,
        }: Output,
    ) -> Result<Self, Self::Error> {
        let stdout =
            String::from_utf8(stdout).map_err(|err| Error::Stdout(StdoutError { inner: err }))?;
        let stderr =
            String::from_utf8(stderr).map_err(|err| Error::Stderr(StderrError { inner: err }))?;

        Ok(Utf8Output {
            status,
            stdout,
            stderr,
        })
    }
}

impl TryFrom<&Output> for Utf8Output {
    type Error = Error;

    fn try_from(
        Output {
            status,
            stdout,
            stderr,
        }: &Output,
    ) -> Result<Self, Self::Error> {
        let stdout = String::from_utf8(stdout.to_vec())
            .map_err(|err| Error::Stdout(StdoutError { inner: err }))?;
        let stderr = String::from_utf8(stderr.to_vec())
            .map_err(|err| Error::Stderr(StderrError { inner: err }))?;
        let status = *status;

        Ok(Utf8Output {
            status,
            stdout,
            stderr,
        })
    }
}

/// An error produced when converting [`Output`] to [`Utf8Output`], wrapping a [`FromUtf8Error`].
///
/// ```
/// use std::process::ExitStatus;
/// use std::process::Output;
/// use utf8_command::Utf8Output;
/// use utf8_command::Error;
///
/// let invalid = Output {
///     status: ExitStatus::default(),
///     stdout: Vec::from(b""),
///     stderr: Vec::from(b"\xe2\x28\xa1"), // Invalid 3-byte sequence.
/// };
///
/// let result: Result<Utf8Output, Error> = invalid.try_into();
/// assert_eq!(
///     result.unwrap_err().to_string(),
///     "Stderr contained invalid utf-8 sequence of 1 bytes from index 0: \"�(�\""
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The [`Output`]'s stdout field contained invalid UTF-8.
    Stdout(StdoutError),
    /// The [`Output`]'s stderr field contained invalid UTF-8.
    Stderr(StderrError),
}

impl Error {
    /// Get a reference to the inner [`FromUtf8Error`].
    pub fn inner(&self) -> &FromUtf8Error {
        match self {
            Error::Stdout(err) => err.inner(),
            Error::Stderr(err) => err.inner(),
        }
    }
}

impl From<StdoutError> for Error {
    fn from(value: StdoutError) -> Self {
        Self::Stdout(value)
    }
}

impl From<StderrError> for Error {
    fn from(value: StderrError) -> Self {
        Self::Stderr(value)
    }
}

impl From<Error> for FromUtf8Error {
    fn from(value: Error) -> Self {
        match value {
            Error::Stdout(err) => err.inner,
            Error::Stderr(err) => err.inner,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Error::Stdout(err) => write!(f, "{}", err),
            Error::Stderr(err) => write!(f, "{}", err),
        }
    }
}

impl std::error::Error for Error {}

/// The [`Output`]'s `stdout` field contained invalid UTF-8. Wraps a [`FromUtf8Error`].
///
/// ```
/// use utf8_command::StdoutError;
///
/// let invalid_utf8 = Vec::from(b"\x80"); // Invalid single byte.
/// let inner_err = String::from_utf8(invalid_utf8).unwrap_err();
/// let err = StdoutError::from(inner_err);
/// assert_eq!(
///     err.to_string(),
///     "Stdout contained invalid utf-8 sequence of 1 bytes from index 0: \"�\""
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StdoutError {
    inner: FromUtf8Error,
}

impl StdoutError {
    /// Get a reference to the inner [`FromUtf8Error`].
    pub fn inner(&self) -> &FromUtf8Error {
        &self.inner
    }
}

impl From<StdoutError> for FromUtf8Error {
    fn from(value: StdoutError) -> Self {
        value.inner
    }
}

impl From<FromUtf8Error> for StdoutError {
    fn from(inner: FromUtf8Error) -> Self {
        Self { inner }
    }
}

impl Display for StdoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Stdout contained {}: {}",
            self.inner,
            FromUtf8ErrorContext::new(&self.inner, ERROR_CONTEXT_BYTES)
        )
    }
}

impl std::error::Error for StdoutError {}

/// The [`Output`]'s `stderr` field contained invalid UTF-8. Wraps a [`FromUtf8Error`].
///
/// ```
/// use utf8_command::StderrError;
///
/// let invalid_utf8 = Vec::from(b"\xf0\x90"); // Incomplete 4-byte sequence.
/// let inner_err = String::from_utf8(invalid_utf8).unwrap_err();
/// let err = StderrError::from(inner_err);
/// assert_eq!(
///     err.to_string(),
///     "Stderr contained incomplete utf-8 byte sequence from index 0: \"�\""
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StderrError {
    inner: FromUtf8Error,
}

impl StderrError {
    /// Get a reference to the inner [`FromUtf8Error`].
    pub fn inner(&self) -> &FromUtf8Error {
        &self.inner
    }
}

impl From<StderrError> for FromUtf8Error {
    fn from(value: StderrError) -> Self {
        value.inner
    }
}

impl From<FromUtf8Error> for StderrError {
    fn from(inner: FromUtf8Error) -> Self {
        Self { inner }
    }
}

impl Display for StderrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Stderr contained {}: {}",
            self.inner,
            FromUtf8ErrorContext::new(&self.inner, ERROR_CONTEXT_BYTES)
        )
    }
}

impl std::error::Error for StderrError {}
