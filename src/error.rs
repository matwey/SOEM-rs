use std::error;
use std::ffi::NulError;
use std::fmt;
use std::io;
use std::iter::Iterator;
use std::option::Option;
use std::result;

pub trait ErrorGenerator: fmt::Debug {
    fn iserror(&mut self) -> bool;
    fn next(&mut self) -> Option<String>;
}

#[derive(Debug)]
pub struct ErrorIterator<'a>(&'a mut dyn ErrorGenerator);

impl<'a> ErrorIterator<'a> {
    pub fn new(generator: &'a mut dyn ErrorGenerator) -> Self {
        ErrorIterator(generator)
    }

    pub fn fmt(&mut self, f: &mut fmt::Formatter) -> fmt::Result {
        self.try_for_each(|x| writeln!(f, "{}", x))
    }
}

impl<'a> Iterator for ErrorIterator<'a> {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        self.0.next()
    }
}

impl<'a> Drop for ErrorIterator<'a> {
    fn drop(&mut self) {
        while let Some(_) = self.0.next() {}
    }
}

impl<'a> fmt::Display for ErrorIterator<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error iterator")
    }
}

impl<'a> error::Error for ErrorIterator<'a> {}

#[derive(Debug)]
pub enum InitError {
    CStringError(NulError),
    IOError(io::Error),
}

impl fmt::Display for InitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            InitError::CStringError(ref err) => write!(f, "CString error: {}", err),
            InitError::IOError(ref err) => write!(f, "I/O error: {}", err),
        }
    }
}

impl error::Error for InitError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            InitError::CStringError(ref err) => Some(err),
            InitError::IOError(ref err) => Some(err),
        }
    }
}

#[derive(Debug)]
pub enum EtherCatError {
    NoFrame,
    OtherFrame,
    Error,
}

impl EtherCatError {
    pub fn from_code(x: i32) -> result::Result<EtherCatError, i32> {
        match x {
            -1 => Ok(EtherCatError::NoFrame),
            -2 => Ok(EtherCatError::OtherFrame),
            -3 => Ok(EtherCatError::Error),
            x => Err(x),
        }
    }
}

impl fmt::Display for EtherCatError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            EtherCatError::NoFrame => write!(f, "No frame received"),
            EtherCatError::OtherFrame => write!(f, "Unkown frame received"),
            EtherCatError::Error => write!(f, "General EtherCat error"),
        }
    }
}

impl error::Error for EtherCatError {}
