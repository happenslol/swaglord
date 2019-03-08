#![allow(unused)]

mod specs;
mod gen;
mod util;

use std::{
    env,
    path::Path,
    fs::File,
    io::Read,
};

use specs::OpenApiSpec;
use gen::{Generator, Typescript};
use tera;

#[derive(Debug)]
pub enum Error {
    TeraError(tera::Error),
    IoError(std::io::Error),
}

impl From<tera::Error> for Error {
    fn from(error: tera::Error) -> Self {
        Error::TeraError(error)
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::IoError(error)
    }
}

fn main() -> Result<(), Error> {
    let args = env::args().collect::<Vec<String>>();
    if args.len() < 2 {
        panic!("please supply a file name");
    }

    let path = Path::new(&args[1]);
    if !path.is_file() {
        panic!("path does not point to a file");
    }

    let mut f = File::open(path).expect("file not found");

    let mut contents = String::new();
    f.read_to_string(&mut contents).expect("error reading file");

    let spec = serde_yaml::from_str::<OpenApiSpec>(&contents).unwrap();
    Typescript::generate(&spec);

    Ok(())
}

