//! This struct provides a buffered access to a [`Read`](std::io::Read) object
//! with a limited [`Seek`](std::io::Seek) implementation. In other words, [`BufStreamReader`] turns a
//! [`Read`](std::io::Read) into a [`Read`](std::io::Read)+[`Seek`](std::io::Seek), which can be used
//! together with binary parsers such as [`binread`](https://crates.io/crates/binread)
//! (which is the reason why I created this crate).
//! 
//! Seeking is limited by the following constraints:
//! 
//!  - Only a small bunch of bytes is buffered (defined by the `buffer_size` parameter of [`BufStreamReader::new()`])
//!  - We don't know the end of the stream, using [`SeekFrom::End`](std::io::SeekFrom::End) is not supported
//!  - Seeking backward is allowed only if the targeted position is within the current buffer. 
//! 
//! # Seeking backward as possible as far as there are data in the current buffer
//! ```rust
//! use std::io::{Cursor, Read, Seek, SeekFrom};
//! use buf_stream_reader::BufStreamReader;
//! # let mut arr: [u8; 256] = [0; 256];  
//! # for (elem, val) in arr.iter_mut().zip(0..=255) { *elem = val; }
//! let cursor = Cursor::new(&arr); // points to array with values from \x00 .. \xff
//! let mut reader = BufStreamReader::new(cursor, 16).unwrap();
//! 
//! let mut buffer: [u8; 7] = [0; 7];
//! 
//! /* straightly reading 7 bytes works */
//! assert_eq!(reader.read(&mut buffer).unwrap(), buffer.len());
//! assert_eq!(&buffer, &arr[0..7]);
//! 
//! /* seeking backwards inside the current buffer */
//! assert!(reader.seek(SeekFrom::Current(-4)).is_ok());
//! assert_eq!(reader.read(&mut buffer).unwrap(), 7);
//! assert_eq!(&buffer, &arr[3..10]);
//! ```
//! 
//! # Seeking backwards is not possible if the destination is not within of behind the current buffer
//! ```rust
//! # use std::io::{Cursor, Read, Seek, SeekFrom};
//! # use buf_stream_reader::BufStreamReader;
//! # let mut arr: [u8; 256] = [0; 256];  
//! # for (elem, val) in arr.iter_mut().zip(0..=255) { *elem = val; }
//! let cursor = Cursor::new(&arr); // points to array with values from \x00 .. \xff
//! let mut reader = BufStreamReader::new(cursor, 16).unwrap();
//! 
//! let mut buffer: [u8; 7] = [0; 7];
//! assert!(reader.seek(SeekFrom::Start(96)).is_ok());
//! assert!(reader.seek(SeekFrom::Start(95)).is_err());
//! assert!(reader.seek(SeekFrom::Current(-1)).is_err());
//! ```
//! 
//! # Seeking forward is not limited, as well as reading beyond buffer limits (as far as data is available, of course)
//! ```rust
//! # use std::io::{Cursor, Read, Seek, SeekFrom};
//! # use buf_stream_reader::BufStreamReader;
//! # let mut arr: [u8; 256] = [0; 256];  
//! # for (elem, val) in arr.iter_mut().zip(0..=255) { *elem = val; }
//! let cursor = Cursor::new(&arr); // points to array with values from \x00 .. \xff
//! let mut reader = BufStreamReader::new(cursor, 16).unwrap();
//! 
//! let mut buffer: [u8; 7] = [0; 7];
//! assert!(reader.seek(SeekFrom::Start(10)).is_ok());
//! assert_eq!(reader.read(&mut buffer).unwrap(), buffer.len());
//! assert_eq!(&buffer, &arr[10..17]);
//! 
//! assert!(reader.seek(SeekFrom::Current(122)).is_ok());
//! assert_eq!(reader.read(&mut buffer).unwrap(), buffer.len());
//! assert_eq!(&buffer, &arr[139..146]);
//! ```
mod reader;
pub use crate::reader::BufStreamReader;