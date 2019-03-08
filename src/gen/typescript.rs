use serde_derive::Serialize;
use crate::{
    specs::OpenApiSpec,
    gen::Generator,
};

pub struct TypescriptGenerator;
impl Generator for TypescriptGenerator {
    fn generate(spec: &OpenApiSpec) {
    }
}

fn generate_models() -> Vec<ModelFile> {
    vec![]
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type")]
enum ModelFile {
    Alias {
        name: String,
        alias: String,
    },
    Enum {
        name: String,
        variants: Vec<EnumVariant>,
    },
    Struct {
        name: String,
        imports: Vec<Import>,
        filename: String,
        fields: Vec<Field>,
    },
}

#[derive(Clone, Debug, Serialize)]
struct Import {
    pub types: Vec<String>,
    pub file: String,
}

#[derive(Clone, Debug, Serialize)]
struct EnumVariant {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Serialize)]
struct Field {
    pub name: String,
    pub field_type: String,
}
