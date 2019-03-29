use std::collections::BTreeMap;
use serde_derive::Deserialize;

#[derive(Debug, Deserialize)]
pub struct OpenApiSpec {
    pub openapi: String,

    #[serde(default)]
    pub paths: BTreeMap<String, PathSpec>,

    pub components: Option<ComponentsSpec>,

    #[serde(default)]
    pub tags: Vec<TagSpec>,
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

#[derive(Debug, Deserialize, Clone)]
pub struct ComponentsSpec {
    #[serde(default)]
    pub schemas: BTreeMap<String, RefOr<SchemaSpec>>,

    #[serde(default)]
    pub responses: BTreeMap<String, RefOr<ResponseSpec>>,

    #[serde(default)]
    pub parameters: BTreeMap<String, RefOr<ParameterSpec>>,

    #[serde(rename = "requestBodies")]
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
    description: Option<String>,
    headers: BTreeMap<String, HeaderSpec>,
    content: BTreeMap<String, MediaTypeSpec>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MediaTypeSpec {
    schema: SchemaSpec,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ParameterSpec {}

#[derive(Debug, Deserialize, Clone)]
pub struct RequestBodySpec {}

#[derive(Debug, Deserialize, Clone)]
pub struct HeaderSpec {}

#[derive(Debug, Deserialize, Clone)]
pub struct PathSpec {}

#[derive(Debug, Deserialize, Clone)]
pub struct TagSpec {
    #[serde(default)]
    pub name: String,

    #[serde(default)]
    pub description: String,
}

