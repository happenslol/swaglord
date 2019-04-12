use std::collections::BTreeMap;
use serde_derive::Deserialize;

#[derive(Debug, Deserialize)]
pub struct OpenApiSpec {
    pub openapi: String,

    pub info: OpenApiInfoSpec,

    #[serde(default)]
    pub paths: BTreeMap<String, PathSpec>,

    pub components: Option<ComponentsSpec>,

    #[serde(default)]
    pub tags: Vec<TagSpec>,

    #[serde(default)]
    pub servers: Vec<ServerSpec>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OpenApiInfoSpec {
    pub title: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerSpec {
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum RefOr<T> where T: Clone {
    Ref {
        #[serde(rename = "$ref")]
        ref_path: String,
    },
    Object(T),
}

impl<T> RefOr<T> where T: Clone {
    pub fn map_cloned<U, F: FnOnce(T) -> RefOr<U>>(&self, f: F) -> RefOr<U> where U: Clone {
        match self {
            RefOr::Ref { ref ref_path } => RefOr::Ref { ref_path: ref_path.clone() },
            RefOr::Object(x) => f(x.clone()),
        }
    }

    pub fn maybe_map_cloned<U, F: FnOnce(T) -> Option<RefOr<U>>>(
        &self, f: F,
    ) -> Option<RefOr<U>> where U: Clone {
        match self {
            RefOr::Ref { ref ref_path } => Some(RefOr::Ref { ref_path: ref_path.clone() }),
            RefOr::Object(x) => f(x.clone()),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ComponentsSpec {
    #[serde(default)]
    pub schemas: BTreeMap<String, RefOr<SchemaSpec>>,

    #[serde(default)]
    pub responses: BTreeMap<String, RefOr<ResponseSpec>>,

    #[serde(default)]
    pub parameters: BTreeMap<String, RefOr<ParameterSpec>>,

    #[serde(rename = "requestBodies")]
    #[serde(default)]
    pub request_bodies: BTreeMap<String, RefOr<RequestBodySpec>>,

    #[serde(default)]
    pub headers: BTreeMap<String, RefOr<HeaderSpec>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SchemaSpec {
    #[serde(rename = "type")]
    pub schema_type: Option<String>,

    #[serde(default)]
    pub description: String,

    #[serde(default)]
    pub required: Vec<String>,

    #[serde(default,rename = "enum")]
    pub schema_enum: Vec<String>,

    pub format: Option<String>,

    pub items: Option<RefOr<Box<SchemaSpec>>>,

    #[serde(default)]
    pub properties: BTreeMap<String, RefOr<SchemaSpec>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ResponseSpec {
    pub description: Option<String>,

    #[serde(default)]
    pub headers: BTreeMap<String, HeaderSpec>,

    #[serde(default)]
    pub content: BTreeMap<String, MediaTypeSpec>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MediaTypeSpec {
    pub schema: RefOr<SchemaSpec>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ParameterSpec {}

#[derive(Debug, Deserialize, Clone)]
pub struct RequestBodySpec {
    pub description: Option<String>,

    #[serde(default)]
    pub content: BTreeMap<String, MediaTypeSpec>,

    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HeaderSpec {}

#[derive(Debug, Deserialize, Clone)]
pub struct PathSpec {
    pub summary: Option<String>,
    pub description: Option<String>,

    pub get: Option<OperationSpec>,
    pub post: Option<OperationSpec>,
    pub put: Option<OperationSpec>,
    pub patch: Option<OperationSpec>,
    pub delete: Option<OperationSpec>,
    pub head: Option<OperationSpec>,
    pub trace: Option<OperationSpec>,
    pub options: Option<OperationSpec>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OperationSpec {
    #[serde(default)]
    pub tags: Vec<String>,

    pub summary: Option<String>,
    #[serde(rename = "operationId")]
    pub operation_id: String,

    #[serde(default)]
    pub parameters: Vec<ParamSpec>,

    #[serde(rename = "requestBody")]
    pub request_body: Option<RefOr<RequestBodySpec>>,
    pub responses: BTreeMap<String, ResponseSpec>,

    #[serde(default)]
    pub deprecated: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ParamSpec {
    pub name: String,

    #[serde(rename = "in")]
    pub location: String,

    pub description: Option<String>,

    #[serde(default)]
    pub required: bool,

    #[serde(default)]
    pub deprecated: bool,

    pub schema: RefOr<SchemaSpec>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TagSpec {
    #[serde(default)]
    pub name: String,

    #[serde(default)]
    pub description: String,
}

