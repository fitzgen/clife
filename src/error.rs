use std::error;
use std::fmt;
use std::io;

use glium;

#[derive(Debug)]
pub enum Error {
    WorldBadParts,
    Io(io::Error),
    GliumBufferCreationError(glium::vertex::BufferCreationError),
    GliumProgramCreationError(glium::program::ProgramCreationError),
    GliumDrawError(glium::DrawError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", error::Error::description(self))
    }
}

impl error::Error for Error {
    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::Io(ref e) => Some(e),
            Error::GliumBufferCreationError(ref e) => Some(e),
            Error::GliumProgramCreationError(ref e) => Some(e),
            _ => None,
        }
    }

    fn description(&self) -> &str {
        match *self {
            Error::WorldBadParts => "Could not create World from parts",
            Error::Io(ref e) => e.description(),
            Error::GliumBufferCreationError(ref e) => e.description(),
            Error::GliumProgramCreationError(ref e) => e.description(),
            Error::GliumDrawError(_) => "Draw error",
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::Io(e)
    }
}

impl From<glium::vertex::BufferCreationError> for Error {
    fn from(e: glium::vertex::BufferCreationError) -> Error {
        Error::GliumBufferCreationError(e)
    }
}

impl From<glium::program::ProgramCreationError> for Error {
    fn from(e: glium::program::ProgramCreationError) -> Error {
        Error::GliumProgramCreationError(e)
    }
}

impl From<glium::DrawError> for Error {
    fn from(e: glium::DrawError) -> Error {
        Error::GliumDrawError(e)
    }
}
