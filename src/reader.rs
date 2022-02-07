
use std::io::{Result, Read, Seek, ErrorKind, Error, Cursor, SeekFrom};

pub struct BufStreamReader<R> where R: Read {
    reader: R,
    offset: u64,
    buffer_size: usize,
    bytes_in_buffer: usize,
    cursor: Cursor<Vec<u8>>,
    current_in_buffer: u64,
}

impl<R> BufStreamReader<R> where R: Read {
    /// Creates a new BufStreamReader with a specified `buffer_size`.
    /// This newly created object wraps another object which is [Read](std::io::Read).
    /// 
    ///  - `buffer_size` - Size of the read buffer. [BufStreamReader] always tries to read `buffer_size` bytes from ` reader, but it is not guaranteed that the buffer actually holds that number of bytes (e.g. at the end of the stream)
    ///  - `reader` - Reader which has to be wrapped
    pub fn new(mut reader: R, buffer_size: usize) -> Result<Self> {
        // already read the first buffer:
        let (bytes, cursor) = Self::initialize_buffer(&mut reader, buffer_size)?;

        Ok(Self {
            reader,
            cursor,
            buffer_size,
            bytes_in_buffer: bytes,
            offset: 0,
            current_in_buffer: 0
        })
    }

    /// Returns the offset of the current buffer in the wrapped stream.
    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn current(&self) -> u64 {
        self.current_in_buffer
    }

    fn read_next_buffer(&mut self) -> Result<()> {
        let (bytes, cursor) = Self::initialize_buffer(&mut self.reader, self.buffer_size)?;
        self.offset += self.bytes_in_buffer as u64;
        self.cursor = cursor;
        self.bytes_in_buffer = bytes;
        self.current_in_buffer = 0;
        Ok(())
    }

    fn initialize_buffer(reader: &mut R, buffer_size: usize) -> Result<(usize, Cursor<Vec<u8>>)> {
        let mut buffer = vec![0; buffer_size];
        let bytes = reader.read(&mut buffer[..])?;
        if bytes == 0 {
            return Err(Error::new(ErrorKind::UnexpectedEof, "read 0 bytes"));
        }
        Ok((bytes, Cursor::new(buffer)))
    }

    /// jump a certain number of blocks forward
    fn seek_until_position(&mut self, position_in_buffer: u64) -> Result<u64> {
        if (position_in_buffer as usize) < self.bytes_in_buffer {
            Ok(position_in_buffer)
        } else {
            let offset_in_buffer = position_in_buffer % self.buffer_size as u64;
             
            // One of the buffers to skip has already been read, so this can be subtracted.
            let skip_buffers = ((position_in_buffer - offset_in_buffer) / self.buffer_size as u64) - 1;

            let mut skip = vec![0; self.buffer_size];
            for _ in 0..skip_buffers {
                let bytes_skipped = self.reader.read(&mut skip[..])?;
                self.offset += bytes_skipped as u64;
            }
            self.read_next_buffer()?;
            Ok(offset_in_buffer)
        }
    }
}

impl<R> Read for BufStreamReader<R> where R: Read {
    fn read(&mut self, dst: &mut [u8]) -> Result<usize> {
        let mut bytes_read = 0;
        loop {
            match self.cursor.read(&mut dst[bytes_read..]) {
                Ok(bytes) => {
                    bytes_read += bytes;
                    if bytes_read == dst.len() {
                        self.current_in_buffer += bytes as u64;
                        return Ok(bytes_read)
                    }
                    assert!(bytes_read < dst.len());
                    self.read_next_buffer()?;
                }
                Err(why) => match why.kind() {
                    ErrorKind::UnexpectedEof => {
                        self.read_next_buffer()?;
                    }
                    _ => {
                        return Err(why);
                    }
                }
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
                let mut position_in_buffer = pos - self.offset;
                if position_in_buffer as usize >= self.bytes_in_buffer {
                    position_in_buffer = self.seek_until_position(position_in_buffer)?;
                }
                Ok(self.cursor.seek(SeekFrom::Start(position_in_buffer))? + self.offset)
            }

            SeekFrom::Current(pos) => {
                if pos < 0 {
                    let pos = -pos as u64;
                    if pos > self.current_in_buffer {
                        return Err(Error::new(ErrorKind::InvalidData, "cannot seek before current buffer"))
                    }
                }
                let mut position_in_buffer = (pos + (self.current_in_buffer as i64)) as u64;

                if position_in_buffer as usize >= self.bytes_in_buffer {
                    position_in_buffer = self.seek_until_position(position_in_buffer)?;
                }
                Ok(self.cursor.seek(SeekFrom::Start(position_in_buffer))? + self.offset)
            }

            // We don't know where the end of a stream is, so this cannot be implemented
            SeekFrom::End(_) => {
                unimplemented!();
            }
        }
    }
}