use std::collections::{BTreeMap, HashMap};
use serde_derive::Serialize;
use voca_rs::case;
use std::fmt::Debug;
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


        let model_files: Vec<String> = models.iter().map(|it| it.filename()).collect();
        let model_index = IndexFile {
            exports: model_files.iter()
                .map(|it| it.trim_end_matches(".ts").to_owned())
                .collect(),
        };

        util::write_templates(&templates, &models, Some("components")).unwrap();
        util::write_templates(&templates, &vec![model_index], Some("components")).unwrap();

        let server = spec.servers.iter().next().unwrap();
        let services = generate_services(
            &case::kebab_case(&spec.info.title),
            &server.url,
            &spec.tags, &spec.paths,
        );
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
                let (models, imports) = generate_model(&name, spec, None);

                let root = models[0].clone().1;
                let mut nested: HashMap<String, Vec<Model>> = HashMap::new();
                for (namespace, _) in models.iter().skip(1) {
                    if let Some(namespace) = namespace.clone() {
                        let children = models.iter()
                            .filter(|(namespace, _)| namespace == namespace)
                            .cloned()
                            .map(|(_, model)| model)
                            .collect();

                        nested.insert(namespace, children);
                    }
                }

                let mut imports_map: HashMap<String, Vec<String>> = HashMap::new();
                for import in imports {
                    let mut entry = imports_map.entry(import.file.clone()).or_insert(vec![]);
                    if (!entry.iter().any(|it| it == &import.import_type)) {
                        entry.push(String::from(import.import_type));
                    }
                }

                let imports = imports_map
                    .into_iter()
                    .map(|(file, types)| GroupedImport { file, types })
                    .collect();

                result.push(ModelFile {
                    imports, root, nested,
                });
            },
            RefOr::Ref { .. } => println!("skipping {}, ref at base", name),
        };
    }

    result
}

fn generate_model(
    name: &str,
    spec: &SchemaSpec,
    namespace: Option<String>,
) -> (Vec<(Option<String>, Model)>, Vec<Import>) {
    let mut models = Vec::new();
    let mut imports = Vec::new();

    let child_namespace = namespace.clone().map_or(
        case::pascal_case(name),
        |parent| format!("{}.{}", parent, case::pascal_case(name)),
    );

    if !spec.schema_enum.is_empty() {
        let (_, enum_model, _) = generate_field(name, spec, None);
        return (enum_model, imports);
    }

    match spec.schema_type.as_ref().map(|it| it.as_str()) {
        // base case
        Some("string") | Some("number") => {
            models.push((namespace, Model::Alias {
                name: String::from(name),
                alias: spec.schema_type.clone().unwrap(),
                is_array: false,
            }));

            (models, imports)
        },
        Some("array") => {
            match spec.items {
                Some(RefOr::Object(ref spec)) => {
                    let (item_models, items_imports) = generate_model(
                        "Item", spec, Some(child_namespace.clone()),
                    );

                    let item_model = item_models[0].clone();

                    models.push((namespace, Model::Alias {
                        name: String::from(name),
                        alias: format!("{}.Item", child_namespace),
                        is_array: true,
                    }));

                    imports.extend(items_imports.into_iter());
                },
                Some(RefOr::Ref { ref ref_path }) => {
                    let parts = ref_path
                        .split("/")
                        .map(|it| String::from(it))
                        .collect::<Vec<String>>();

                    imports.push(Import {
                        import_type: parts[3].clone(),
                        file: parts[1].clone(),
                    });

                    models.push((namespace, Model::Alias {
                        name: String::from(name),
                        alias: String::from(name),
                        is_array: true,
                    }));
                },
                None => {
                    println!(
                        "skipping array {:?}:{} since items field was not present!",
                        namespace, name,
                    );
                },
            }

            (models, imports)
        },
        Some("object") => {
            let mut fields = vec![];

            for (f_name, f_spec) in spec.properties.iter() {
                match f_spec {
                    RefOr::Object(spec) => {
                        let (field, field_models, field_imports) =
                            generate_field(&f_name, spec, namespace.clone());

                        if let Some(field) = field {
                            fields.push(field);
                            models.extend(field_models.into_iter());
                            imports.extend(field_imports.into_iter());
                        }
                    },
                    RefOr::Ref { ref_path } => {
                        let parts = ref_path
                            .split("/")
                            .map(|it| String::from(it))
                            .collect::<Vec<String>>();

                        imports.push(Import {
                            import_type: parts[3].clone(),
                            file: parts[1].clone(),
                        });

                        fields.push(Field {
                            name: f_name.clone(),
                            required: spec.required.iter().any(|it| it == f_name),
                            field_type: parts[3].clone(),
                            is_array: false,
                        });
                    },
                }
            }

            models.push((namespace, Model::Struct {
                name: String::from(name),
                fields,
            }));

            (models, imports)
        },
        _ => (models, imports),
    }
}

fn generate_field(
    name: &str,
    spec: &SchemaSpec,
    namespace: Option<String>,
) -> (Option<Field>, Vec<(Option<String>, Model)>, Vec<Import>) {
    let mut models = Vec::new();
    let mut imports = Vec::new();

    if !spec.schema_enum.is_empty() {
        match spec.schema_type.as_ref().map(|it| it.as_str()) {
            Some("string") | None => {
                models.push((namespace.clone(), Model::Enum {
                    name: String::from(name),
                    variants: spec.schema_enum.iter().map(|it| {
                        EnumVariant {
                            name: it.clone(),
                            value: it.clone(),
                        }
                    }).collect()
                }));

                let result = Field {
                    name: String::from(name),
                    field_type: namespace.clone().map_or_else(
                        || case::pascal_case(name),
                        |ref parent| format!(
                            "{}.{}",
                            case::pascal_case(parent),
                            case::pascal_case(name)
                        ),
                    ),
                    required: false,
                    is_array: false,
                };

                return (Some(result), models, imports);
            },
            Some("number") => {
                println!("skipping {}, number enums are not supported", name);
            },
            Some(other) => {
                println!(
                    "skipping {}, invalid model type for enum: {}",
                    name, other,
                );
            },
        }

        return (None, models, imports);
    }

    let result = match spec.schema_type.as_ref().map(|it| it.as_str()) {
        Some("number") | Some("string") => {
            if !spec.schema_enum.is_empty() {
                let (field, enum_model, _) = generate_field(name, spec, namespace.clone());
                models.extend(enum_model.into_iter());
                field
            } else {
                Some(Field {
                    name: String::from(name),
                    required: false,
                    field_type: spec.schema_type.clone().unwrap(),
                    is_array: false,
                })
            }
        },
        Some("array") => {
            let (item_models, items_imports) = generate_model(
                "Item", spec, namespace.clone(),
            );

            let item_model = item_models[0].clone();
            imports.extend(items_imports.into_iter());

            Some(Field {
                name: String::from(name),
                required: false,
                field_type: item_model.1.name(),
                is_array: true,
            })
        },
        Some("object") => {
            Some(Field {
                name: String::from(name),
                required: false,
                // TODO(hilmar): Generate nested object
                field_type: String::from("any"),
                is_array: false,
            })
        },
        _ => None,
    };

    (result, models, imports)
}

#[derive(Clone, Debug, Serialize)]
struct ModelFile {
    imports: Vec<GroupedImport>,
    root: Model,
    nested: HashMap<String, Vec<Model>>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type")]
enum Model {
    Alias {
        name: String,
        alias: String,
        is_array: bool,
    },
    Enum {
        name: String,
        variants: Vec<EnumVariant>,
    },
    Struct {
        name: String,
        fields: Vec<Field>,
    },
}

impl Model {
    fn name(&self) -> String {
        match self {
            Model::Alias { ref name, .. } => name.clone(),
            Model::Enum { ref name, .. } => name.clone(),
            Model::Struct { ref name, .. } => name.clone(),
        }
    }
}

impl TemplateContext for ModelFile {
    fn template(&self) -> &'static str { "model.tera" }
    fn filename(&self) -> String {
        match self.root {
            Model::Alias { ref name, .. } => format!("{}.ts", case::kebab_case(name)),
            Model::Enum { ref name, .. } => format!("{}.ts", case::kebab_case(name)),
            Model::Struct { ref name, .. } => format!("{}.ts", case::kebab_case(name)),
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
    pub import_type: String,
    pub file: String,
}

#[derive(Clone, Debug, Serialize)]
struct GroupedImport {
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

