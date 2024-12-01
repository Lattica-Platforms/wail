use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};
use wadm_types::{
    Component, ComponentProperties, Manifest, Properties, Specification, TraitProperty,
};

use validation::ValidationError;

mod link;
mod validation;
pub use link::LinkConstructor;

/// List of WASI interfaces that are automatically satisfied by the runtime
pub const RUNTIME_INTERFACES: &[(&str, &str, &str)] = &[
    ("wasi", "io", "poll"),
    ("wasi", "io", "error"),
    ("wasi", "io", "streams"),
    ("wasi", "http", "types"),
    ("wasi", "cli", "environment"),
    ("wasi", "cli", "exit"),
    ("wasi", "cli", "stdin"),
    ("wasi", "cli", "stdout"),
    ("wasi", "cli", "stderr"),
    ("wasi", "clocks", "wall-clock"),
    ("wasi", "filesystem", "types"),
    ("wasi", "filesystem", "preopens"),
];

use crate::{
    core::process_wasm_file,
    models::{ComponentInfo, InterfaceInfo, PackageInfo},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct ConstructorManifest {
    #[serde(rename = "apiVersion", skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<wadm_types::Metadata>,
    pub spec: Specification,
    #[serde(skip_serializing)]
    pub component_interfaces: HashMap<String, ComponentInfo>, // Track ALL interfaces
    #[serde(skip_serializing)]
    pub link_constructors: Vec<LinkConstructor>, // ONLY for imports that need linking
}

impl ConstructorManifest {
    /// Creates a new empty ConstructorManifest
    pub fn new() -> Self {
        Self {
            api_version: None,
            kind: None,
            metadata: None,
            spec: Specification {
                components: Vec::new(),
                policies: Vec::new(),
            },
            link_constructors: Vec::new(),
            component_interfaces: HashMap::new(),
        }
    }

    /// Merges a WADM manifest into this constructor
    pub fn merge_wadm(&mut self, wadm: &Manifest) -> Result<(), String> {
        // Basic metadata merging stays the same
        if self.metadata.is_none() {
            self.metadata = Some(wadm.metadata.clone());
        }
        if self.api_version.is_none() {
            self.api_version = Some(wadm.api_version.clone());
        }
        if self.kind.is_none() {
            self.kind = Some(wadm.kind.clone());
        }

        // Process each WADM component
        for wadm_component in wadm.components() {
            println!("\nProcessing WADM component: {}", wadm_component.name);

            if self.component_exists(&wadm_component.name) {
                // EXISTING COMPONENT: Only validate and apply link configs
                if let Some(traits) = &wadm_component.traits {
                    for trait_def in traits {
                        if trait_def.is_link() {
                            if let TraitProperty::Link(link) = &trait_def.properties {
                                // Find matching import in component_interfaces
                                if let Some(info) =
                                    self.component_interfaces.get(&wadm_component.name)
                                {
                                    // Verify this is actually an import
                                    if !info.imports.iter().any(|import| {
                                        import.name == link.interfaces[0]
                                            && import.namespace == link.namespace
                                            && import.package == link.package
                                    }) {
                                        return Err(format!(
                                            "Component {} does not import interface {}:{}:{}",
                                            wadm_component.name,
                                            link.namespace,
                                            link.package,
                                            link.interfaces[0]
                                        ));
                                    }

                                    // Find and update matching link constructor
                                    if let Some(existing_link) =
                                        self.link_constructors.iter_mut().find(|l| {
                                            l.pre_component_id == wadm_component.name
                                                && l.interfaces == link.interfaces
                                                && l.namespace == link.namespace
                                                && l.package == link.package
                                        })
                                    {
                                        existing_link.post_component_id =
                                            Some(link.target.name.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                // NEW COMPONENT: Process WASM and add interfaces
                match &wadm_component.properties {
                    Properties::Component { properties } => {
                        if let Some(image) = &properties.image {
                            let path = PathBuf::from(image.trim_start_matches("file://"));
                            match process_wasm_file(&wadm_component.name, &path) {
                                Ok(component_info) => {
                                    self.merge_component_info(
                                        wadm_component.name.clone(),
                                        component_info,
                                        path,
                                    )?;
                                }
                                Err(e) => eprintln!("Failed to process WASM file: {}", e),
                            }
                        }
                    }
                    Properties::Capability { .. } => {
                        if wadm_component.name == "httpserver" {
                            self.process_known_provider(wadm_component)?;
                        }
                    }
                }
            }
        }

        self.spec.policies.extend(wadm.policies().cloned());
        Ok(())
    }

    fn process_known_provider(&mut self, component: &Component) -> Result<(), String> {
        match component.name.as_str() {
            "httpserver" => {
                let info = ComponentInfo {
                    imports: vec![InterfaceInfo {
                        name: "incoming-handler".to_string(),
                        namespace: "wasi".to_string(),
                        package: "http".to_string(),
                    }],
                    exports: vec![InterfaceInfo {
                        name: "outgoing-handler".to_string(),
                        namespace: "wasi".to_string(),
                        package: "http".to_string(),
                    }],
                    package: Some(PackageInfo {
                        namespace: "wasmcloud".to_string(),
                        name: "httpserver".to_string(),
                    }),
                };
                match &component.properties {
                    Properties::Component { properties } => self.merge_component_info(
                        component.name.clone(),
                        info,
                        PathBuf::from(properties.image.as_ref().unwrap_or(&String::new())),
                    ),
                    Properties::Capability { properties } => self.merge_component_info(
                        component.name.clone(),
                        info,
                        PathBuf::from(properties.image.as_ref().unwrap_or(&String::new())),
                    ),
                }
            }
            _ => Ok(()),
        }
    }

    pub fn merge_component_info(
        &mut self,
        name: String,
        info: ComponentInfo,
        file_path: PathBuf,
    ) -> Result<(), String> {
        // Store ALL interface information
        self.component_interfaces.insert(name.clone(), info.clone());

        // Add/update the component
        match self.spec.components.iter().position(|c| c.name == name) {
            Some(index) => {
                let component = &mut self.spec.components[index];
                if let Properties::Component { properties } = &mut component.properties {
                    properties.image = Some(file_path.to_string_lossy().to_string());
                }
            }
            None => {
                self.spec.components.push(Component {
                    name: name.clone(),
                    properties: Properties::Component {
                        properties: ComponentProperties {
                            image: Some(file_path.to_string_lossy().to_string()),
                            application: None,
                            id: Some(name.clone()),
                            config: Vec::new(),
                            secrets: Vec::new(),
                        },
                    },
                    traits: Some(Vec::new()),
                });
            }
        }

        // Create link constructors ONLY for non-WASI imports
        println!("\nCreating link constructors for '{}':", name);
        for import in &info.imports {
            // Skip WASI runtime interfaces
            if import.namespace == "wasi"
                && RUNTIME_INTERFACES
                    .iter()
                    .any(|(_, pkg, name)| pkg == &import.package && name == &import.name)
            {
                println!(
                    "  Skipping WASI runtime interface: {}:{}:{}",
                    import.namespace, import.package, import.name
                );
                continue;
            }

            println!("  Adding link constructor for import:");
            println!("    From: {}", name);
            println!(
                "    Interface: {}:{}:{}",
                import.namespace, import.package, import.name
            );

            self.link_constructors.push(LinkConstructor {
                pre_component_id: name.clone(),
                post_component_id: None, // To be filled by WADM config
                interfaces: vec![import.name.clone()],
                namespace: import.namespace.clone(),
                package: import.package.clone(),
            });
        }

        Ok(())
    }

    /// Convert back to a WADM manifest
    pub fn to_wadm(&self) -> Result<Manifest, ValidationError> {
        let metadata = self.metadata.clone().ok_or_else(|| {
            ValidationError::ComponentError("Manifest metadata is required".to_string())
        })?;

        let mut manifest = Manifest {
            api_version: self.api_version.clone().unwrap_or_else(|| "v1".to_string()),
            kind: self
                .kind
                .clone()
                .unwrap_or_else(|| "Application".to_string()),
            metadata,
            spec: Specification {
                components: Vec::new(),
                policies: Vec::new(),
            },
        };

        // Convert components and their links
        for component in &self.spec.components {
            let mut comp = component.clone();

            // Add link traits for any links where this component is the source
            let component_links: Vec<_> = self
                .link_constructors
                .iter()
                .filter(|l| l.pre_component_id == component.name)
                .collect();

            if !component_links.is_empty() {
                if comp.traits.is_none() {
                    comp.traits = Some(Vec::new());
                }

                if let Some(traits) = &mut comp.traits {
                    for link in component_links {
                        traits.push(link.to_wadm_link());
                    }
                }
            }

            manifest.spec.components.push(comp);
        }

        // Add policies if any
        manifest.spec.policies = self.spec.policies.clone();

        Ok(manifest)
    }

    /// Check if a component exists in the manifest
    pub fn component_exists(&self, name: &str) -> bool {
        self.spec.components.iter().any(|c| c.name == name)
    }

    /// Get a reference to a component by name
    pub fn get_component(&self, name: &str) -> Option<&Component> {
        self.spec.components.iter().find(|c| c.name == name)
    }
}
