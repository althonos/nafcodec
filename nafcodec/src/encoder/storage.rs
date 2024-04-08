use std::fs::File;
use std::io::Error as IoError;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;

/// A marker type for in-memory storage.
#[derive(Debug, Default, Clone)]
pub struct Memory;

/// An abstract interface for temporary I/O storage.
pub trait Storage: Sized {
    /// The concrete type to which to write temporary data.
    type Buffer: Write;
    /// Create a new buffer.
    fn create_buffer(&self) -> Result<Self::Buffer, IoError>;
    /// Write the contents of the buffer to the given writer.
    fn write_buffer<W: Write>(&self, buffer: Self::Buffer, file: &mut W) -> Result<(), IoError>;
    /// Get the total length of the buffer content.
    fn buffer_length(&self, buffer: &Self::Buffer) -> Result<u64, IoError>;
    /// Close the temporary storage.
    fn close(self) -> Result<(), IoError> {
        Ok(())
    }
}

#[cfg(feature = "tempfile")]
impl Storage for tempfile::TempDir {
    type Buffer = File;
    fn create_buffer(&self) -> Result<Self::Buffer, IoError> {
        tempfile::tempfile_in(self.path())
    }
    fn buffer_length(&self, buffer: &Self::Buffer) -> Result<u64, IoError> {
        buffer.sync_all()?;
        Ok(buffer.metadata()?.len())
    }
    fn write_buffer<W: Write>(
        &self,
        mut buffer: Self::Buffer,
        file: &mut W,
    ) -> Result<(), IoError> {
        buffer.seek(SeekFrom::Start(0))?;
        std::io::copy(&mut buffer, file)?;
        Ok(())
    }
    fn close(self) -> Result<(), IoError> {
        tempfile::TempDir::close(self)
    }
}

impl Storage for Memory {
    type Buffer = Vec<u8>;
    fn create_buffer(&self) -> Result<Self::Buffer, IoError> {
        Ok(Vec::new())
    }
    fn buffer_length(&self, buffer: &Self::Buffer) -> Result<u64, IoError> {
        Ok(buffer.len() as u64)
    }
    fn write_buffer<W: Write>(&self, buffer: Self::Buffer, file: &mut W) -> Result<(), IoError> {
        file.write_all(buffer.as_slice())
    }
}
