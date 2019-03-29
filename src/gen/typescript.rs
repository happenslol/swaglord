use std::collections::{BTreeMap, HashMap};
use serde_derive::Serialize;
use voca_rs::case;
use crate::{
    specs::{OpenApiSpec, SchemaSpec, RefOr, PathSpec, TagSpec},
    gen::{Generator, TemplateContext},
    util,
};

pub struct TypescriptGenerator;
impl Generator for TypescriptGenerator {
    fn generate(spec: &OpenApiSpec) {
        let models = spec.components.as_ref()
            .map_or_else(|| vec![], |it| generate_models(&it.schemas));

        let templates = util::load_templates("angular-client").unwrap();


        let model_files = models.iter().map(|it| it.filename()).collect();
        let model_index = IndexFile { exports: model_files };

        util::write_templates(&templates, &models, Some("components")).unwrap();
        util::write_templates(&templates, &vec![model_index], Some("components")).unwrap();

        let services = generate_services("Lightning", "api.com", &spec.tags, &spec.paths);
        let service_files = services.iter().map(|it| it.filename()).collect();
        let service_index = IndexFile { exports: service_files };

        util::write_templates(&templates, &services, Some("services")).unwrap();
        util::write_templates(&templates, &vec![service_index], Some("services")).unwrap();
    }
}

fn generate_services(
    client_name: &str,
    base_path: &str,
    tags: &Vec<TagSpec>,
    paths: &BTreeMap<String, PathSpec>,
) -> Vec<ServiceFile> {
    let mut tag_map: HashMap<String, (Vec<Endpoint>, Vec<Import>)> = HashMap::new();

    for tag in tags.iter() {
        tag_map.insert(tag.name.clone(), (vec![], vec![]));
    }

    for (name, spec) in paths.iter() {}

    tag_map
        .into_iter()
        .map(|(tag_name, (endpoints, imports))| {
            ServiceFile {
                name: format!("{}Service", case::pascal_case(&tag_name)),
                client_name_kebab: case::kebab_case(client_name),
                client_name_pascal: case::pascal_case(client_name),
                base_path: String::from(base_path),
                endpoints, imports,
            }
        })
        .collect()
}

fn generate_models(
    model_specs: &BTreeMap<String, RefOr<SchemaSpec>>,
) -> Vec<ModelFile> {
    let mut result = vec![];

    for (name, spec) in model_specs {
        match spec {
            RefOr::Object(ref spec) => {
                if let Some(model) = generate_model(&name, spec) {
                    result.push(model);
                }
            },
            RefOr::Ref { .. } => println!("skipping {}, ref at base", name),
        };
    }

    result
}

fn generate_model(name: &str, spec: &SchemaSpec) -> Option<ModelFile> {
    if !spec.schema_enum.is_empty() {
        match spec.schema_type.as_ref().map(|it| it.as_str()) {
            Some("string") | None => {
                return Some(ModelFile::Enum {
                    name: String::from(name),
                    variants: spec.schema_enum.iter().map(|it| {
                        EnumVariant {
                            name: it.clone(),
                            value: it.clone(),
                        }
                    }).collect()
                });
            },
            Some("number") => {
                println!("skipping {}, number enums are not supported", name);
            },
            Some(other) => {
                println!(
                    "skipping {}, invalid model type for enum: {}",
                    name, other,
                );
                return None;
            },
        };
    }

    match spec.schema_type.as_ref().map(|it| it.as_str()) {
        Some("string") | Some("number") => Some(ModelFile::Alias {
            name: String::from(name),
            alias: spec.schema_type.clone().unwrap(),
        }),
        Some("array") => {
            None
        },
        Some("object") => {
            let mut fields = vec![];
            let mut imports = HashMap::new();

            for (f_name, f_spec) in spec.properties.iter() {
                match f_spec {
                    RefOr::Object(spec) => {
                        match spec.schema_type.as_ref().map(|it| it.as_str()) {
                            Some("number") | Some("string") => {
                                // TODO(hilmar): Handle nested enums
                                fields.push(Field {
                                    name: f_name.clone(),
                                    required: spec.required.iter().any(|it| it == f_name),
                                    field_type: spec.schema_type.clone().unwrap(),
                                    is_array: false,
                                });
                            },
                            Some("array") => {
                                fields.push(Field {
                                    name: f_name.clone(),
                                    required: spec.required.iter().any(|it| it == f_name),
                                    // TODO(hilmar): Generate actual type
                                    field_type: String::from("any"),
                                    is_array: true,
                                });
                            },
                            Some("object") => {
                                fields.push(Field {
                                    name: f_name.clone(),
                                    required: spec.required.iter().any(|it| it == f_name),
                                    // TODO(hilmar): Generate nested object
                                    field_type: String::from("any"),
                                    is_array: false,
                                });
                            },
                            _ => {},
                        }
                    },
                    RefOr::Ref { ref_path } => {
                        let parts = ref_path
                            .split("/")
                            .map(|it| String::from(it))
                            .collect::<Vec<String>>();

                        let mut entry = imports.entry(parts[1].clone()).or_insert(vec![]);
                        entry.push(parts[3].clone());

                        fields.push(Field {
                            name: f_name.clone(),
                            required: spec.required.iter().any(|it| it == f_name),
                            field_type: parts[3].clone(),
                            is_array: false,
                        });
                    },
                }
            }

            let imports = imports
                .into_iter()
                .map(|(file, types)| Import { file, types })
                .collect::<Vec<Import>>();

            Some(ModelFile::Struct {
                name: String::from(name),
                imports,
                fields,
            })
        },
        _ => None,
    }
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
        fields: Vec<Field>,
    },
}

impl TemplateContext for ModelFile {
    fn template(&self) -> &'static str { "model.tera" }
    fn filename(&self) -> String {
        match self {
            ModelFile::Alias { name, .. } => format!("{}.ts", case::kebab_case(name)),
            ModelFile::Enum { name, .. } => format!("{}.ts", case::kebab_case(name)),
            ModelFile::Struct { name, .. } => format!("{}.ts", case::kebab_case(name)),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct IndexFile {
    exports: Vec<String>,
}

impl TemplateContext for IndexFile {
    fn template(&self) -> &'static str { "index.tera" }
    fn filename(&self) -> String { String::from("index.ts") }
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
    pub required: bool,
    pub is_array: bool,
}

#[derive(Clone, Debug, Serialize)]
struct ServiceFile {
    pub client_name_pascal: String,
    pub client_name_kebab: String,
    pub imports: Vec<Import>,
    pub name: String,
    pub base_path: String,
    pub endpoints: Vec<Endpoint>,
}

impl TemplateContext for ServiceFile {
    fn template(&self) -> &'static str { "service.tera" }
    fn filename(&self) -> String { format!("{}.ts", case::kebab_case(&self.name)) }
}

#[derive(Clone, Debug, Serialize)]
struct Endpoint {
    pub name: String,
    pub body_param: Option<Field>,
    pub query_params: Vec<Field>,
    pub header_params: Vec<Field>,
    pub return_type: String,
    pub method: String,
    pub path: String,
}

