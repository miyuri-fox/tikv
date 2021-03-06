// Copyright 2019 TiKV Project Authors. Licensed under Apache-2.0.

mod utf8mb4;

pub use self::utf8mb4::*;

use std::cmp::Ordering;
use std::hash::Hasher;
use std::str::Utf8Error;

use codec::prelude::*;

pub macro match_template_collator($t:tt, $($tail:tt)*) {
    match_template::match_template! {
        $t = [
            Utf8Bin => CollatorUtf8Mb4Bin,
            Utf8Mb4Bin => CollatorUtf8Mb4Bin,
            Utf8GeneralCi => CollatorUtf8Mb4GeneralCi,
            Utf8Mb4GeneralCi => CollatorUtf8Mb4GeneralCi,
            Binary => CollatorBinary,
        ],
        $($tail)*
    }
}

pub trait Charset {
    type Char: Copy + Into<u32>;

    fn decode_one(data: &[u8]) -> Option<(Self::Char, usize)>;
}

pub struct CharsetBinary;

impl Charset for CharsetBinary {
    type Char = u8;

    #[inline]
    fn decode_one(data: &[u8]) -> Option<(Self::Char, usize)> {
        if data.is_empty() {
            None
        } else {
            Some((data[0], 1))
        }
    }
}

pub trait Collator {
    type Charset: Charset;

    /// Writes the SortKey of `bstr` into `writer`.
    fn write_sort_key<W: BufferWriter>(
        bstr: &[u8],
        writer: &mut W,
    ) -> Result<usize, DecodeOrWriteError>;

    /// Returns the SortKey of `bstr` as an owned byte vector.
    fn sort_key(bstr: &[u8]) -> Result<Vec<u8>, DecodeError> {
        let mut v = Vec::default();
        match Self::write_sort_key(bstr, &mut v) {
            Ok(_) => Ok(v),
            Err(DecodeOrWriteError::Write(_)) => unreachable!(),
            Err(DecodeOrWriteError::Decode(e)) => Err(e),
        }
    }

    /// Compares `a` and `b` based on their SortKey.
    fn sort_compare(a: &[u8], b: &[u8]) -> Result<Ordering, DecodeError>;

    /// Hashes `bstr` based on its SortKey directly.
    ///
    /// WARN: `sort_hash(str) != hash(sort_key(str))`.
    fn sort_hash<H: Hasher>(bstr: &[u8], state: &mut H) -> Result<(), DecodeError>;
}

pub struct CollatorBinary;

impl Collator for CollatorBinary {
    type Charset = CharsetBinary;

    #[inline]
    fn write_sort_key<W: BufferWriter>(
        bstr: &[u8],
        writer: &mut W,
    ) -> Result<usize, DecodeOrWriteError> {
        writer.write_bytes(bstr)?;
        Ok(bstr.len())
    }

    #[inline]
    fn sort_compare(a: &[u8], b: &[u8]) -> Result<Ordering, DecodeError> {
        Ok(a.cmp(b))
    }

    #[inline]
    fn sort_hash<H: Hasher>(bstr: &[u8], state: &mut H) -> Result<(), DecodeError> {
        use std::hash::Hash;

        bstr.hash(state);
        Ok(())
    }
}

#[derive(Fail, Debug)]
#[fail(display = "Invalid input in charset {}", _0)]
pub struct DecodeError(&'static str);

impl From<Utf8Error> for DecodeError {
    fn from(_: Utf8Error) -> Self {
        DecodeError("UTF8")
    }
}

#[derive(Fail, Debug)]
pub enum DecodeOrWriteError {
    #[fail(display = "{}", _0)]
    Decode(DecodeError),

    #[fail(display = "{}", _0)]
    Write(codec::Error),
}

impl From<codec::Error> for DecodeOrWriteError {
    fn from(e: codec::Error) -> Self {
        DecodeOrWriteError::Write(e)
    }
}

impl From<DecodeError> for DecodeOrWriteError {
    fn from(e: DecodeError) -> Self {
        DecodeOrWriteError::Decode(e)
    }
}

impl From<DecodeError> for crate::error::EvaluateError {
    fn from(e: DecodeError) -> Self {
        crate::error::EvaluateError::InvalidCharacterString {
            charset: e.0.to_string(),
        }
    }
}

impl From<DecodeOrWriteError> for crate::error::EvaluateError {
    fn from(e: DecodeOrWriteError) -> Self {
        crate::error::EvaluateError::Other(e.to_string())
    }
}
