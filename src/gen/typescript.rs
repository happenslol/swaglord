use std::collections::{BTreeMap, HashMap};
use serde_derive::Serialize;
use voca_rs::case;
use std::fmt::Debug;
use crate::{
    specs::{OpenApiSpec, SchemaSpec, RefOr, PathSpec, TagSpec, OperationSpec},
    gen::{Generator, TemplateContext},
    util,
};

pub struct TypescriptGenerator;
impl Generator for TypescriptGenerator {
    fn generate(spec: &OpenApiSpec) {
        let templates = util::load_templates("angular-client").unwrap();

        let schema_models = spec.components.as_ref()
            .map_or_else(|| vec![], |it| generate_models(&it.schemas));
        let schema_files: Vec<String> = schema_models.iter()
            .map(|it| it.filename()).collect();

        if !schema_files.is_empty() {
            let schema_index = IndexFile {
                exports: schema_files.iter()
                    .map(|it| it.trim_end_matches(".ts").to_owned())
                    .collect(),
            };

            util::write_templates(&templates, &schema_models, Some("schemas")).unwrap();
            util::write_templates(&templates, &vec![schema_index], Some("schemas")).unwrap();
        }

        let response_models = spec.components.as_ref().map_or_else(|| vec![], |it| {
            // discard everything that doesn't contain a json body
            let mut response_specs = BTreeMap::new();
            it.responses.iter()
                .filter_map(|(name, spec)| {
                    if let Some(spec) = spec.maybe_map_cloned(|it| {
                        it.content.get("application/json").map(|it| it.schema.clone())
                    }) { Some((name.clone(), spec)) } else { None }
                })
                .for_each(|(name, spec)| { response_specs.insert(name, spec); });

            generate_models(&response_specs)
        });
        let response_files: Vec<String> = response_models.iter()
            .map(|it| it.filename()).collect();

        if !response_files.is_empty() {
            let response_index = IndexFile {
                exports: response_files.iter()
                    .map(|it| it.trim_end_matches(".ts").to_owned())
                    .collect(),
            };

            util::write_templates(&templates, &response_models, Some("responses")).unwrap();
            util::write_templates(&templates, &vec![response_index], Some("responses")).unwrap();
        }

        let request_models = spec.components.as_ref().map_or_else(|| vec![], |it| {
            // discard everything that doesn't contain a json body
            let mut request_specs = BTreeMap::new();
            it.request_bodies.iter()
                .filter_map(|(name, spec)| {
                    if let Some(spec) = spec.maybe_map_cloned(|it| {
                        it.content.get("application/json").map(|it| it.schema.clone())
                    }) { Some((name.clone(), spec)) } else { None }
                })
                .for_each(|(name, spec)| { request_specs.insert(name, spec); });

            generate_models(&request_specs)
        });
        let request_files: Vec<String> = request_models.iter()
            .map(|it| it.filename()).collect();

        if !request_files.is_empty() {
            let request_index = IndexFile {
                exports: request_files.iter()
                    .map(|it| it.trim_end_matches(".ts").to_owned())
                    .collect(),
            };

            util::write_templates(&templates, &request_models, Some("request-bodies")).unwrap();
            util::write_templates(&templates, &vec![request_index], Some("request-bodies")).unwrap();
        }

        let server = spec.servers.iter().nth(0).expect("no servers");
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
    let mut tag_map = HashMap::new();

    for tag in tags.iter() {
        tag_map.insert(tag.name.clone(), (vec![], vec![], vec![]));
    }

    for (path, spec) in paths.iter() {
        let mut ops = Vec::new();
        if let Some(ref get_op) = spec.get { ops.push(("get", get_op.clone())); }
        if let Some(ref post_op) = spec.post { ops.push(("post", post_op.clone())); }
        if let Some(ref put_op) = spec.put { ops.push(("put", put_op.clone())); }
        if let Some(ref patch_op) = spec.patch { ops.push(("patch", patch_op.clone())); }
        if let Some(ref delete_op) = spec.delete { ops.push(("delete", delete_op.clone())); }
        if let Some(ref head_op) = spec.head { ops.push(("head", head_op.clone())); }
        if let Some(ref trace_op) = spec.trace { ops.push(("trace", trace_op.clone())); }
        if let Some(ref options_op) = spec.options { ops.push(("options", options_op.clone())); }

        insert_endpoints(&mut tag_map, &path, &ops);
    }

    let client_name_kebab = case::kebab_case(client_name);
    let client_name_pascal = case::pascal_case(client_name);

    tag_map
        .into_iter()
        .map(|(tag_name, (endpoints, models, imports))| {
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

            ServiceFile {
                name: format!("{}Service", case::pascal_case(&tag_name)),
                client_name_kebab: client_name_kebab.clone(),
                client_name_pascal: client_name_pascal.clone(),
                base_path: String::from(base_path),
                endpoints, imports,
                models,
                nested: HashMap::new(),
            }
        })
        .collect()
}

fn insert_endpoints(
    tags: &mut HashMap<String, (Vec<Endpoint>, Vec<Model>, Vec<Import>)>,
    path: &str,
    ops: &Vec<(&str, OperationSpec)>,
) {
    for (method, spec) in ops {
        if spec.tags.is_empty() {
            println!("\tskipping untagged operation: {} ({})", spec.operation_id, method);
            continue;
        }

        let (ref mut endpoints, ref mut models, ref mut imports) = tags
            .get_mut(&spec.tags[0]).expect("tag not found");

        let request_body = spec.request_body.clone().map(|request_spec| {
            let (request_models, request_imports) = generate_model(
                &format!("{}Request", case::pascal_case(&spec.operation_id)),
                &request_spec.maybe_map_cloned(|it| {
                    it.content.get("application/json").map(|it| it.schema.clone())
                }).expect("no json body!"),
                None,
            );

            let root = if !request_models.is_empty() {
                request_models[0].clone().1
            } else {
                panic!("no models generated for request body");
            };

            imports.extend(request_imports.into_iter());
            match root {
                Model::Alias { ref alias, is_array, .. } => {
                    // TODO
                    // models.extend(request_models.into_iter().skip(1));

                    Field {
                        name: alias.clone(),
                        field_type: alias.clone(),
                        required: true, // TODO
                        is_array,
                    }
                },
                _ => {
                    // TODO
                    // models.extend(request_models.into_iter());

                    Field {
                        name: root.name(),
                        field_type: root.name(),
                        required: true, // TODO
                        is_array: false,
                    }
                },
            }
        });

        let mut query_params = Vec::new();
        let mut path_params = Vec::new();
        let mut header_params = Vec::new();

        for param in spec.parameters.iter() {
            let (param_models, param_imports) = generate_model(
                &case::pascal_case(&param.name),
                &param.schema,
                Some(case::pascal_case(&spec.operation_id)),
            );

            let root = if !param_models.is_empty() {
                param_models[0].clone().1
            } else {
                panic!("no models generated for param");
            };

            imports.extend(param_imports.into_iter());
            let root_field = match root {
                Model::Alias { ref alias, is_array, .. } => {
                    // TODO
                    // models.extend(param_models.into_iter().skip(1));

                    Field {
                        name: param.name.clone(),
                        field_type: alias.clone(),
                        required: param.required,
                        is_array,
                    }
                },
                _ => {
                    // TODO
                    // models.extend(param_models.into_iter());

                    Field {
                        name: param.name.clone(),
                        field_type: root.name(),
                        required: param.required,
                        is_array: false,
                    }
                },
            };

            match param.location.as_ref() {
                "query" => query_params.push(root_field),
                "header" => header_params.push(root_field),
                "path" => path_params.push(root_field),
                _ => panic!("unknown param location"),
            }
        }

        endpoints.push(Endpoint {
            name: spec.operation_id.clone(),
            body_param: request_body,
            path_params, query_params, header_params,
            return_type: String::from("undefined"),
            method: String::from(*method),
            path: String::from(path),
        });
    }
}

fn generate_models(model_specs: &BTreeMap<String, RefOr<SchemaSpec>>) -> Vec<ModelFile> {
    let mut result = vec![];

    for (name, spec) in model_specs {
        let (models, imports) = generate_model(&name, &spec, None);

        let root = if !models.is_empty() {
            models[0].clone().1
        } else {
            println!("no models generated from {}", name);
            continue;
        };

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
        // requestBodies -> request-bodies, everything else is the same
        import_type: case::pascal_case(&parts[3]),
        file: case::kebab_case(&parts[2]),
    };

    (parts[3].clone(), import)
}

fn generate_model(
    name: &str,
    spec: &RefOr<SchemaSpec>,
    namespace: Option<String>,
) -> (Vec<(Option<String>, Model)>, Vec<Import>) {
    let mut models = Vec::new();
    let mut imports = Vec::new();

    let child_namespace = namespace.clone().map_or(
        case::pascal_case(name),
        |parent| format!("{}.{}", parent, case::pascal_case(name)),
    );

    match spec {
        RefOr::Ref { ref ref_path } => {
            let (ref_type, import) = get_ref(ref_path);
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
                    models.push((namespace, Model::Alias {
                        name: String::from(name),
                        alias: spec.schema_type.clone().unwrap(),
                        is_array: false,
                    }));

                    (models, imports)
                },
                Some("array") => {
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
                    let mut fields = vec![];
                    let mut sub_models = vec![];

                    for (field_name, field_spec) in spec.properties.iter() {
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
    client_name_pascal: String,
    client_name_kebab: String,
    imports: Vec<GroupedImport>,
    name: String,
    base_path: String,
    endpoints: Vec<Endpoint>,
    models: Vec<Model>,
    nested: HashMap<String, Vec<Model>>,
}

impl TemplateContext for ServiceFile {
    fn template(&self) -> &'static str { "service.tera" }
    fn filename(&self) -> String { format!("{}.ts", case::kebab_case(&self.name)) }
}

#[derive(Clone, Debug, Serialize)]
struct Endpoint {
    pub name: String,
    pub body_param: Option<Field>,
    pub path_params: Vec<Field>,
    pub query_params: Vec<Field>,
    pub header_params: Vec<Field>,
    pub return_type: String,
    pub method: String,
    pub path: String,
}

#[derive(Serialize)]
struct ClientConfigFile(String);
impl TemplateContext for ClientConfigFile {
    fn template(&self) -> &'static str { "config.tera" }
    fn filename(&self) -> String { format!("{}.config.ts", case::kebab_case(&self.0)) }
}

#[derive(Serialize)]
struct ClientModuleFile(String);
impl TemplateContext for ClientModuleFile {
    fn template(&self) -> &'static str { "module.tera" }
    fn filename(&self) -> String { format!("{}.module.ts", case::kebab_case(&self.0)) }
}

#[derive(Serialize)]
struct PackageFile {
    name: String,
    version: String,
    description: String
}
impl TemplateContext for PackageFile {
    fn template(&self) -> &'static str { "package.tera" }
    fn filename(&self) -> String { String::from("package.json") }
}

#[derive(Serialize)]
struct ReadmeFile;
impl TemplateContext for ReadmeFile {
    fn template(&self) -> &'static str { "module.tera" }
    fn filename(&self) -> String { String::from("README.md") }
}

#[derive(Serialize)]
struct GitIgnoreFile;
impl TemplateContext for GitIgnoreFile {
    fn template(&self) -> &'static str { "gitignore.tera" }
    fn filename(&self) -> String { String::from("gitignore") }
}

#[derive(Serialize)]
struct TsConfigFile;
impl TemplateContext for TsConfigFile {
    fn template(&self) -> &'static str { "tsconfig.tera" }
    fn filename(&self) -> String { String::from("tsconfig.json") }
}

#[derive(Serialize)]
struct LicenseFile;
impl TemplateContext for LicenseFile {
    fn template(&self) -> &'static str { "license.tera" }
    fn filename(&self) -> String { String::from("LICENSE.md" )}
}

#[derive(Serialize)]
struct TypingsFile;
impl TemplateContext for TypingsFile {
    fn template(&self) -> &'static str { "typings.tera" }
    fn filename(&self) -> String { String::from("typings.d.ts") }
}

#[derive(Serialize)]
struct UtilFile;
impl TemplateContext for UtilFile {
    fn template(&self) -> &'static str { "util.tera" }
    fn filename(&self) -> String { String::from("util.ts") }
}

#[derive(Serialize)]
struct VariablesFile;
impl TemplateContext for VariablesFile {
    fn template(&self) -> &'static str { "variables.tera" }
    fn filename(&self) -> String { String::from("variables.ts") }
}

