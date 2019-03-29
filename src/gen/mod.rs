use serde::Serialize;

use crate::specs::OpenApiSpec;
mod typescript;

pub use typescript::TypescriptGenerator as Typescript;

pub trait Generator {
    fn generate(spec: &OpenApiSpec);
}

pub trait TemplateContext : Serialize {
    fn template(&self) -> &'static str;
    fn filename(&self) -> String;
}

