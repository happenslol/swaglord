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

        let server = spec.servers.iter().next().expect("No servers");
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

    let client_name_kebab = case::kebab_case(client_name);
    let client_name_pascal = case::pascal_case(client_name);

    tag_map
        .into_iter()
        .map(|(tag_name, (endpoints, imports))| {
            ServiceFile {
                name: format!("{}Service", case::pascal_case(&tag_name)),
                client_name_kebab: client_name_kebab.clone(),
                client_name_pascal: client_name_pascal.clone(),
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
        let (models, imports) = generate_model(&name, &spec, None);
        println!("result: {:?}", models);

        let root = models[0].clone().1;
        let mut nested: HashMap<String, Vec<Model>> = HashMap::new();
        for (namespace, model) in models.into_iter().skip(1) {
            let namespace = namespace.expect("child should have namespace!");
            nested.entry(namespace).or_insert(vec![]).push(model);
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
    }

    result
}

fn get_ref(path: &String) -> (String, Import) {
    let parts = path
        .split("/")
        .map(|it| String::from(it))
        .collect::<Vec<String>>();

    let import = Import {
        import_type: parts[3].clone(),
        file: parts[1].clone(),
    };

    (parts[3].clone(), import)
}

fn generate_model(
    name: &str,
    spec: &RefOr<SchemaSpec>,
    namespace: Option<String>,
) -> (Vec<(Option<String>, Model)>, Vec<Import>) {
    println!("generating {} in namespace {:?}", name, namespace);

    let mut models = Vec::new();
    let mut imports = Vec::new();

    let child_namespace = namespace.clone().map_or(
        case::pascal_case(name),
        |parent| format!("{}.{}", parent, case::pascal_case(name)),
    );

    match spec {
        RefOr::Ref { ref ref_path } => {
            let (ref_type, import) = get_ref(ref_path);
            println!("\tfound ref {}", ref_type);
            imports.push(import);
            models.push((namespace, Model::Alias {
                name: String::from(name),
                alias: ref_type,
                is_array: false,
            }));

            (models, imports)
        },
        RefOr::Object(ref spec) => {
            // base case: enum at current level
            if !spec.schema_enum.is_empty() {
                println!("\tfound enum");
                match spec.schema_type.as_ref().map(|it| it.as_str()) {
                    Some("string") | None => {
                        models.push((namespace, Model::Enum {
                            name: String::from(name),
                            variants: spec.schema_enum.iter().map(|it| {
                                EnumVariant {
                                    name: it.clone(),
                                    value: it.clone(),
                                }
                            }).collect()
                        }));

                        return (models, imports);
                    },
                    Some("number") => {
                        println!(
                            "skipping {}, number enums are not supported",
                            name
                        );
                    },
                    Some(other) => {
                        println!(
                            "skipping {}, invalid model type for enum: {}",
                            name, other,
                        );
                    },
                }

                return (models, imports);
            }

            match spec.schema_type.as_ref().map(|it| it.as_str()) {
                // base case
                // TODO(hilmar): Respect format!
                Some("string") | Some("number") => {
                    println!("\tfound primitive {:?}", spec.schema_type);
                    models.push((namespace, Model::Alias {
                        name: String::from(name),
                        alias: spec.schema_type.clone().unwrap(),
                        is_array: false,
                    }));

                    (models, imports)
                },
                Some("array") => {
                    println!("\tfound array");
                    match spec.items {
                        Some(ref spec) => {
                            // this is needed due to the box
                            let spec = spec.clone();
                            let (item_models, item_imports) = match spec {
                                RefOr::Object(obj) => generate_model(
                                    "Item",
                                    &RefOr::Object(*obj),
                                    Some(child_namespace.clone()),
                                ),
                                RefOr::Ref { ref_path } => generate_model(
                                    "Item",
                                    &RefOr::Ref { ref_path },
                                    Some(child_namespace.clone()),
                                ),
                            };

                            let (_, item_model) = item_models[0].clone();

                            match item_model {
                                // this means a base case happened
                                Model::Alias { ref alias, is_array, .. } => {
                                    if (is_array) {
                                        panic!("can't have nested array fields at the moment");
                                    }

                                    models.push((namespace, Model::Alias {
                                        name: String::from(name),
                                        alias: alias.clone(),
                                        is_array: true,
                                    }));

                                    models.extend(item_models.into_iter().skip(1));
                                    imports.extend(item_imports.into_iter());
                                },
                                // this means we generated children
                                _ => {
                                    models.push((namespace, Model::Alias {
                                        name: String::from(name),
                                        alias: format!(
                                            "{}.{}",
                                            child_namespace.clone(),
                                            item_model.name()
                                        ),
                                        is_array: true,
                                    }));

                                    models.extend(item_models.into_iter());
                                    imports.extend(item_imports.into_iter());
                                },
                            }
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
                    println!("\tfound object");
                    let mut fields = vec![];
                    let mut sub_models = vec![];

                    for (field_name, field_spec) in spec.properties.iter() {
                        println!("\tprocessing field {}", field_name);
                        let (field_models, field_imports) = generate_model(
                            &case::pascal_case(field_name), field_spec, 
                            Some(child_namespace.clone()),
                        );

                        // ignore namespace
                        let (_, field_model) = field_models.iter().cloned().nth(0)
                            .expect("no first model");

                        match field_model {
                            // this means a base case happened
                            Model::Alias { ref alias, is_array, .. } => {
                                fields.push(Field {
                                    name: field_name.clone(),
                                    field_type: alias.clone(),
                                    required: spec.required.iter()
                                        .any(|r| r == field_name),
                                    is_array,
                                });

                                sub_models.extend(field_models.into_iter().skip(1));
                                imports.extend(field_imports.into_iter());
                            },
                            // this means we generated children
                            _ => {
                                fields.push(Field {
                                    name: field_name.clone(),
                                    field_type: format!(
                                        "{}.{}",
                                        child_namespace.clone(),
                                        field_model.name()
                                    ),
                                    required: spec.required.iter()
                                        .any(|r| r == field_name),
                                    is_array: false,
                                });

                                sub_models.extend(field_models.into_iter());
                                imports.extend(field_imports.into_iter());
                            },
                        }
                    }

                    models.push((namespace, Model::Struct {
                        name: String::from(name),
                        fields,
                    }));
                    models.extend(sub_models.into_iter());

                    (models, imports)
                },
                _ => (models, imports),
            }
        },
    }
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

