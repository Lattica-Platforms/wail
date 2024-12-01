use crate::models::InterfaceRequirement;

use super::{ConstructorManifest, LinkConstructor, RUNTIME_INTERFACES};
use wadm_types::{Component, Properties};

#[derive(Debug)]
pub struct UnlinkedInterface {
    pub component: String,
    pub interface: InterfaceRequirement,
    pub potential_matches: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Component error: {0}")]
    ComponentError(String),
    #[error("Link error: {0}")]
    LinkError(String),
    #[error("Interface error: {0}")]
    InterfaceError(String),
    // #[error("Resolver error: {0}")]
    // ResolverError(#[from] ResolverError),
}

#[derive(Debug, Default)]
pub struct ValidationReport {
    pub discovered_links: Vec<LinkConstructor>,
    pub unlinked_interfaces: Vec<UnlinkedInterface>,
    pub warnings: Vec<String>,
    pub errors: Vec<ValidationError>,
    pub is_valid: bool,
}

impl ValidationReport {
    pub fn new() -> Self {
        Self {
            is_valid: true,
            ..Default::default()
        }
    }

    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    pub fn add_error(&mut self, error: ValidationError) {
        self.is_valid = false;
        self.errors.push(error);
    }

    /// Returns a summary of the validation results
    pub fn summary(&self) -> String {
        let mut summary = Vec::new();

        if !self.discovered_links.is_empty() {
            summary.push(format!(
                "Discovered {} implicit links",
                self.discovered_links.len()
            ));
        }

        if !self.unlinked_interfaces.is_empty() {
            summary.push(format!(
                "Found {} unlinked interfaces",
                self.unlinked_interfaces.len()
            ));
        }

        if !self.warnings.is_empty() {
            summary.push(format!("{} warnings", self.warnings.len()));
        }

        if !self.errors.is_empty() {
            summary.push(format!("{} errors", self.errors.len()));
        }

        if summary.is_empty() {
            "All validations passed successfully".to_string()
        } else {
            summary.join(", ")
        }
    }
}

impl ConstructorManifest {
    /// Validates the manifest and returns a detailed report
    pub fn validate(&mut self) -> Result<ValidationReport, ValidationError> {
        let mut report = ValidationReport::new();
        self.validate_basic_requirements(&mut report)?;

        // Check each link constructor (which represents an import that needs satisfying)
        for link in &mut self.link_constructors {
            // Note: made mutable
            // Skip WASI runtime interfaces
            if RUNTIME_INTERFACES.iter().any(|(ns, pkg, name)| {
                ns == &link.namespace && pkg == &link.package && name == &link.interfaces[0]
            }) {
                println!(
                    "Auto-satisfying WASI interface {}:{}:{} for {}",
                    link.namespace, link.package, link.interfaces[0], link.pre_component_id
                );
                continue;
            }

            // If it has a target, validate the target exists and exports the interface
            if let Some(target) = &link.post_component_id {
                // Explicit target specified - must use this one
                if let Some(target_info) = self.component_interfaces.get(target) {
                    // Check target exports this interface
                    if !target_info.exports.iter().any(|export| {
                        export.name == link.interfaces[0]
                            && export.namespace == link.namespace
                            && export.package == link.package
                    }) {
                        report.add_error(ValidationError::InterfaceError(format!(
                            "Component {} does not export interface {}:{}:{} required by {}",
                            target,
                            link.namespace,
                            link.package,
                            link.interfaces[0],
                            link.pre_component_id
                        )));
                    }
                } else {
                    report.add_error(ValidationError::ComponentError(format!(
                        "Target component {} not found",
                        target
                    )));
                }
            } else {
                // No target specified - find first matching component
                let mut found_match = false;
                for (comp_name, comp_info) in &self.component_interfaces {
                    if comp_name != &link.pre_component_id {
                        // Don't match with self
                        if comp_info.exports.iter().any(|export| {
                            export.name == link.interfaces[0]
                                && export.namespace == link.namespace
                                && export.package == link.package
                        }) {
                            // Found a match - update the link constructor with the target
                            link.post_component_id = Some(comp_name.clone());
                            println!("Saturated link: {} -> {}", link.pre_component_id, comp_name);
                            found_match = true;
                            break; // Take first match
                        }
                    }
                }
                if !found_match {
                    report.add_error(ValidationError::InterfaceError(format!(
                        "No component found that exports interface {}:{}:{} required by {}",
                        link.namespace, link.package, link.interfaces[0], link.pre_component_id
                    )));
                }
            }
        }

        Ok(report)
    }

    fn validate_basic_requirements(
        &self,
        report: &mut ValidationReport,
    ) -> Result<(), ValidationError> {
        // Validate components
        for component in &self.spec.components {
            if let Err(e) = self.validate_component_properties(component) {
                report.add_error(e);
            }
        }

        // Validate link references
        for link in &self.link_constructors {
            if let Err(e) = self.validate_link_references(link) {
                report.add_error(e);
            }
        }

        Ok(())
    }

    fn validate_component_properties(&self, component: &Component) -> Result<(), ValidationError> {
        let missing_requirements = match &component.properties {
            Properties::Component { properties } => {
                properties.image.is_none() && properties.application.is_none()
            }
            Properties::Capability { properties } => {
                properties.image.is_none() && properties.application.is_none()
            }
        };

        if missing_requirements {
            return Err(ValidationError::ComponentError(format!(
                "{} must specify image or application",
                component.name
            )));
        }

        Ok(())
    }

    fn validate_link_references(&self, link: &LinkConstructor) -> Result<(), ValidationError> {
        if !self.component_exists(&link.pre_component_id) {
            return Err(ValidationError::LinkError(format!(
                "Pre-component '{}' not found",
                link.pre_component_id
            )));
        }
        // Check post_component_id only if it's specified
        if let Some(post_id) = &link.post_component_id {
            if !self.component_exists(post_id) {
                return Err(ValidationError::LinkError(format!(
                    "Post-component '{}' not found",
                    post_id
                )));
            }
        }
        link.validate().map_err(ValidationError::LinkError)
    }
}
