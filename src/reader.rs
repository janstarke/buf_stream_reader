
use std::io::{Result, Read, Seek, ErrorKind, Error, SeekFrom};

pub struct BufStreamReader<R> where R: Read {
    reader: R,
    offset: u64,
    bytes_in_buffer: usize,
    buffer: Vec<u8>,
    current_in_buffer: u64,
}

impl<R> BufStreamReader<R> where R: Read {
    /// Creates a new BufStreamReader with a specified `buffer_size`.
    /// This newly created object wraps another object which is [Read](std::io::Read).
    /// 
    ///  - `buffer_size` - Size of the read buffer. [BufStreamReader] always tries to read `buffer_size` bytes from ` reader, but it is not guaranteed that the buffer actually holds that number of bytes (e.g. at the end of the stream)
    ///  - `reader` - Reader which has to be wrapped
    pub fn new(reader: R, buffer_size: usize) -> Self {
        let buffer = vec![0; buffer_size];
        Self {
            reader,
            buffer,
            bytes_in_buffer: 0,
            offset: 0,
            current_in_buffer: 0
        }
    }

    /// Returns the offset of the current buffer in the wrapped stream.
    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn current(&self) -> u64 {
        self.current_in_buffer
    }

    fn read_next_buffer(&mut self) -> Result<()> {

        let bytes = self.reader.read(&mut self.buffer[..])?;
        if bytes == 0 {
            return Err(Error::new(ErrorKind::UnexpectedEof, "read 0 bytes"));
        }

        self.offset += self.bytes_in_buffer as u64;
        self.bytes_in_buffer = bytes;
        self.current_in_buffer = 0;
        Ok(())
    }

    /// jump a certain number of blocks forward
    fn seek_until_position(&mut self, mut position_in_buffer: u64) -> Result<u64> {
        while position_in_buffer >= self.bytes_in_buffer as u64 {
            position_in_buffer -= self.bytes_in_buffer as u64;
            self.read_next_buffer()?;
        }

        self.current_in_buffer = position_in_buffer;
        assert!(self.current_in_buffer < self.bytes_in_buffer as u64);

        Ok(self.offset + self.current_in_buffer)
    }
}

impl<R> Read for BufStreamReader<R> where R: Read {
    fn read(&mut self, dst: &mut [u8]) -> Result<usize> {
        let mut bytes_read = 0;
        loop {
            let can_read = self.bytes_in_buffer - self.current_in_buffer as usize;

            // the current buffer contains no more data to return, we must obtain
            // no data from the wrapped reader
            if can_read == 0 {
                if let Err(why) = self.read_next_buffer() {

                    // the wrapped reader encountered an EOF, so we are stuck
                    // with what we have read so far
                    if why.kind() == ErrorKind::UnexpectedEof {
                        if bytes_read > 0 {
                            return Ok(bytes_read)
                        } else {
                            return Err(why)
                        }
                    }
                }

                // at this position, we have successfully refilled the buffer and
                // continue with the next iteration
            } else {
                // do_read contains the number of bytes we'll really read
                let do_read = std::cmp::min(can_read, dst.len()-bytes_read);

                // the range where we'll write what we read
                let src_begin = self.current_in_buffer as usize;
                let src_end = src_begin + do_read as usize;
                let dst_begin = bytes_read;
                let dst_end = dst_begin + do_read as usize;

                // reading...
                dst[dst_begin..dst_end].copy_from_slice(&self.buffer[src_begin..src_end]);

                // update internal variables and bounds check
                bytes_read += do_read;
                self.current_in_buffer += do_read as u64;
                assert!(self.current_in_buffer <= self.bytes_in_buffer as u64);

                if bytes_read == dst.len() {
                    return Ok(bytes_read);
                }

                // we need to read more bytes
                assert!(bytes_read < dst.len());
            }
        }
    }
}

impl<R> Seek for BufStreamReader<R> where R: Read {
    fn seek(&mut self, seek_from: SeekFrom) -> Result<u64> {
        match seek_from {
            SeekFrom::Start(pos) => {
                // don't seek befor the end of the current buffer
                if pos < self.offset {
                    return Err(Error::new(ErrorKind::InvalidData, "cannot seek before current buffer"));
                }

                // We can seek behind the end of the current buffer,
                // but this requires discarding the current buffer
                // and reloading a new buffer.
                self.seek_until_position(pos - self.offset)
            }

            SeekFrom::Current(pos) => {
                if pos < 0 {
                    let pos = -pos as u64;
                    if pos > self.current_in_buffer {
                        return Err(Error::new(ErrorKind::InvalidData, "cannot seek before current buffer"))
                    }
                }
                self.seek_until_position((pos + (self.current_in_buffer as i64)) as u64)
            }

            // We don't know where the end of a stream is, so this cannot be implemented
            SeekFrom::End(_) => {
                unimplemented!();
            }
        }
    }
}