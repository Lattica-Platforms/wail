use serde::Deserialize;
use serde::Serialize;
use wadm_types::LinkProperty;
use wadm_types::Trait;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LinkConstructor {
    /// Has to be provided. where this link is coming from.
    pub pre_component_id: String,
    /// If provided, the post component id linked to has to be equivalent to this.
    /// If not provided, linking can be made to any component id aslong as interfaces are met
    pub post_component_id: Option<String>,
    /// The interfaces a post component must export for this link to be satisfied
    pub interfaces: Vec<String>,
    /// The namespace for the interfaces.
    pub namespace: String,
    /// The package for the interfaces.
    pub package: String,
}

impl LinkConstructor {
    pub fn to_wadm_link(&self) -> Trait {
        Trait::new_link(LinkProperty {
            namespace: self.namespace.clone(),
            package: self.package.clone(),
            interfaces: self.interfaces.clone(),
            source: Some(wadm_types::ConfigDefinition::default()),
            target: wadm_types::TargetConfig {
                name: self.post_component_id.clone().unwrap_or_default(),
                config: vec![],
                secrets: vec![],
            },
            name: Some(format!(
                "{}-{}",
                self.pre_component_id,
                self.post_component_id.clone().unwrap_or_default()
            )),
            ..Default::default()
        })
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.interfaces.is_empty() {
            return Err("Link must have at least one interface".to_string());
        }

        // Validate that namespace and package are not empty
        if self.namespace.is_empty() {
            return Err("Namespace cannot be empty".to_string());
        }
        if self.package.is_empty() {
            return Err("Package cannot be empty".to_string());
        }

        Ok(())
    }
}
