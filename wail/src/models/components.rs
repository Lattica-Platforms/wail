/// Models for source components
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComponentInfo {
    pub imports: Vec<InterfaceInfo>,
    pub exports: Vec<InterfaceInfo>,
    pub package: Option<PackageInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InterfaceInfo {
    pub name: String,
    pub namespace: String,
    pub package: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PackageInfo {
    pub namespace: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComponentsConfig {
    pub entities: Vec<Entity>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Entity {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Source {
    File {
        #[serde(with = "source_file_format")]
        path: PathBuf,
    },
    OCI {
        #[serde(with = "source_oci_format")]
        reference: String,
    },
}

// Custom serialization for file:// prefix
mod source_file_format {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::path::PathBuf;

    pub fn serialize<S>(path: &PathBuf, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("file://{}", path.display()))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<PathBuf, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if let Some(path) = s.strip_prefix("file://") {
            Ok(PathBuf::from(path))
        } else {
            Err(serde::de::Error::custom("path must start with file://"))
        }
    }
}

// Custom serialization for oci:// prefix
mod source_oci_format {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(reference: &str, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("oci://{}", reference))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<String, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if let Some(reference) = s.strip_prefix("oci://") {
            Ok(reference.to_string())
        } else {
            Err(serde::de::Error::custom("reference must start with oci://"))
        }
    }
}

impl Entity {
    pub fn get_source(&self) -> PathBuf {
        match &self.source {
            Some(Source::File { path }) => path.clone(),
            Some(Source::OCI { .. }) => PathBuf::new(), // Handle OCI references separately
            None => PathBuf::from(format!("./{}/build/*.wasm", self.name)),
        }
    }
}
