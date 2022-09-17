use std::io;

#[derive(Debug)]
pub enum DecodeError {
    Io(io::Error),
}

impl From<io::Error> for DecodeError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
