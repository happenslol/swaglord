#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;

mod specs;

use std::env;
use std::path::Path;
use std::fs::File;
use std::io::Read;

use specs::OpenApiSpec;

fn main() {
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

    let parsed = serde_yaml::from_str::<OpenApiSpec>(&contents).unwrap();
}

