// use std::collections::HashMap;

// use crate::models::{ComponentInfo, Direction, InterfaceRequirement};

// /// Error types for interface resolution
// #[derive(Debug, thiserror::Error)]
// pub enum ResolverError {
//     #[error("Component not found: {0}")]
//     ComponentNotFound(String),
//     #[error("Interface mismatch: {0}")]
//     InterfaceMismatch(String),
//     #[error("Invalid link: {0}")]
//     InvalidLink(String),
//     #[error("Validation error: {0}")]
//     ValidationError(String),
// }

// /// Tracks and resolves interface requirements between components
// #[derive(Debug)]
// pub struct InterfaceResolver {
//     requirements: HashMap<String, Vec<InterfaceRequirement>>,
//     satisfied: HashMap<String, HashMap<String, String>>,
// }

// impl InterfaceResolver {
//     pub fn new() -> Self {
//         Self {
//             requirements: HashMap::new(),
//             satisfied: HashMap::new(),
//         }
//     }

//     fn make_interface_key(name: &str, namespace: &str, package: &str) -> String {
//         format!("{}:{}:{}", package, namespace, name)
//     }

//     // /// Adds component config from WADM traits
//     // /// Does NOT discover interfaces - those come from WASM only
//     // pub fn hydrate_wadm_components(&mut self, component: &Component) -> Result<(), ResolverError> {
//     //     // First collect all the link configs and requirements we want to update
//     //     let mut to_satisfy = Vec::new();

//     //     // Get the requirements first
//     //     if let Some(reqs) = self.requirements.get(&component.name) {
//     //         if let Some(traits) = &component.traits {
//     //             for trait_def in traits {
//     //                 if trait_def.is_link() {
//     //                     if let TraitProperty::Link(link) = &trait_def.properties {
//     //                         if !link.target.name.is_empty() {
//     //                             // Find matching requirement
//     //                             if let Some(req) = reqs.iter().find(|r| {
//     //                                 r.identifier.name == link.interfaces[0]
//     //                                     && r.identifier.namespace == link.namespace
//     //                                     && r.identifier.package == link.package
//     //                             }) {
//     //                                 // Store the requirement and target
//     //                                 to_satisfy.push((req.clone(), link.target.name.clone()));
//     //                             }
//     //                         }
//     //                     }
//     //                 }
//     //             }
//     //         }
//     //     }

//     //     // Now apply all satisfactions
//     //     for (req, target) in to_satisfy {
//     //         self.mark_satisfied(&component.name, &req, &target);
//     //     }

//     //     Ok(())
//     // }

//     pub fn register_component(
//         &mut self,
//         name: &str,
//         info: &ComponentInfo,
//     ) -> Result<(), ResolverError> {
//         println!("Registering component in resolver: {}", name);

//         let mut requirements = Vec::new();

//         // Add imports as requirements
//         for import in &info.imports {
//             requirements.push(InterfaceRequirement::new(
//                 import.name.clone(),
//                 import.namespace.clone(),
//                 import.package.clone(),
//                 Direction::Import,
//                 name.to_string(),
//             ));
//         }

//         // Add exports as requirements
//         for export in &info.exports {
//             requirements.push(InterfaceRequirement::new(
//                 export.name.clone(),
//                 export.namespace.clone(),
//                 export.package.clone(),
//                 Direction::Export,
//                 name.to_string(),
//             ));
//         }

//         // Store requirements and create satisfied map
//         self.requirements.insert(name.to_string(), requirements);
//         self.satisfied.insert(name.to_string(), HashMap::new());

//         Ok(())
//     }

//     // /// Applies a link between components
//     // pub fn apply_link(&mut self, link: &LinkConstructor) -> Result<(), ResolverError> {
//     //     // Validate pre-component exists
//     //     if !self.requirements.contains_key(&link.pre_component_id) {
//     //         return Err(ResolverError::ComponentNotFound(format!(
//     //             "Source component '{}' not found",
//     //             link.pre_component_id
//     //         )));
//     //     }

//     //     // Validate post-component if specified
//     //     if let Some(post_id) = &link.post_component_id {
//     //         if !self.requirements.contains_key(post_id) {
//     //             return Err(ResolverError::ComponentNotFound(format!(
//     //                 "Target component '{}' not found",
//     //                 post_id
//     //             )));
//     //         }
//     //     }

//     //     // Find matching requirements
//     //     let requirements = self.find_matching_requirements(link)?;
//     //     if requirements.is_empty() {
//     //         return Err(ResolverError::InterfaceMismatch(format!(
//     //             "No matching interfaces found for link from {} to {:?}",
//     //             link.pre_component_id, link.post_component_id
//     //         )));
//     //     }

//     //     // Apply satisfaction with full interface information
//     //     for (req, source_id, target_id) in requirements {
//     //         self.mark_satisfied(&source_id, &req, &target_id);
//     //     }

//     //     Ok(())
//     // }

//     // /// Finds requirements matching a link
//     // fn find_matching_requirements(
//     //     &self,
//     //     link: &LinkConstructor,
//     // ) -> Result<Vec<(InterfaceRequirement, String, String)>, ResolverError> {
//     //     println!("\nAttempting to match link:");
//     //     println!("  From: {}", link.pre_component_id);
//     //     println!("  To: {:?}", link.post_component_id);
//     //     println!(
//     //         "  Interface: {}:{}:{}",
//     //         link.namespace, link.package, link.interfaces[0]
//     //     );

//     //     let reqs = self
//     //         .requirements
//     //         .get(&link.pre_component_id)
//     //         .ok_or_else(|| ResolverError::ComponentNotFound(link.pre_component_id.clone()))?;

//     //     println!("\nAvailable requirements:");
//     //     for req in reqs {
//     //         println!("  Component: {}", req.component);
//     //         println!(
//     //             "  Interface: {}:{}:{}",
//     //             req.identifier.namespace, req.identifier.package, req.identifier.name
//     //         );
//     //         println!("  Direction: {:?}", req.direction);
//     //     }

//     //     Ok(reqs
//     //         .iter()
//     //         .filter(|r| {
//     //             let _basic_match = link.interfaces.contains(&r.identifier.name)
//     //                 && r.identifier.namespace == link.namespace
//     //                 && r.identifier.package == link.package;

//     //             // Component matching
//     //             match &link.post_component_id {
//     //                 Some(post_id) => {
//     //                     if let Some(post_reqs) = self.requirements.get(post_id) {
//     //                         post_reqs.iter().any(|other| {
//     //                             other.identifier.name == r.identifier.name
//     //                                 && other.direction != r.direction
//     //                                 && other.identifier.namespace == r.identifier.namespace
//     //                                 && other.identifier.package == r.identifier.package
//     //                         })
//     //                     } else {
//     //                         false
//     //                     }
//     //                 }
//     //                 None => self.find_matching_component(r).is_some(),
//     //             }
//     //         })
//     //         .map(|req| {
//     //             (
//     //                 req.clone(),
//     //                 link.pre_component_id.clone(),
//     //                 link.post_component_id.clone().unwrap_or_default(),
//     //             )
//     //         })
//     //         .collect())
//     // }

//     // /// Marks an interface as satisfied
//     // fn mark_satisfied(&mut self, component: &str, req: &InterfaceRequirement, satisfied_by: &str) {
//     //     if let Some(satisfied_map) = self.satisfied.get_mut(component) {
//     //         let key = Self::make_interface_key(
//     //             &req.identifier.name,
//     //             &req.identifier.namespace,
//     //             &req.identifier.package,
//     //         );
//     //         satisfied_map.insert(key, satisfied_by.to_string());
//     //     }
//     // }

//     // Checks if an interface is satisfied
//     // fn is_satisfied(&self, component: &str, req: &InterfaceRequirement) -> bool {
//     //     self.satisfied
//     //         .get(component)
//     //         .map(|m| {
//     //             let key = Self::make_interface_key(
//     //                 &req.identifier.name,
//     //                 &req.identifier.namespace,
//     //                 &req.identifier.package,
//     //             );
//     //             m.contains_key(&key)
//     //         })
//     //         .unwrap_or(false)
//     // }

//     // /// Finds potential links for all unsatisfied requirements
//     // pub fn find_potential_links(&self) -> Vec<LinkConstructor> {
//     //     let mut links = Vec::new();

//     //     for (comp_id, reqs) in &self.requirements {
//     //         for req in reqs {
//     //             if !self.is_satisfied(comp_id, req) {
//     //                 if let Some(match_comp) = self.find_matching_component(req) {
//     //                     links.push(LinkConstructor {
//     //                         pre_component_id: comp_id.clone(),
//     //                         post_component_id: Some(match_comp),
//     //                         interfaces: vec![req.identifier.name.clone()],
//     //                         namespace: req.identifier.namespace.clone(),
//     //                         package: req.identifier.package.clone(),
//     //                     });
//     //                 }
//     //             }
//     //         }
//     //     }

//     //     links
//     // }

//     // /// Finds a matching component for an interface requirement
//     // fn find_matching_component(&self, req: &InterfaceRequirement) -> Option<String> {
//     //     for (comp_id, other_reqs) in &self.requirements {
//     //         // First check: Don't match with self
//     //         if comp_id != &req.component {
//     //             // Interface matching
//     //             if other_reqs.iter().any(|other| {
//     //                 // Basic interface name must match
//     //                 other.identifier.name == req.identifier.name &&
//     //                 // Direction must be opposite
//     //                 other.direction != req.direction &&
//     //                 // Check namespace and package
//     //                 req.identifier.namespace == other.identifier.namespace &&
//     //                 req.identifier.package == other.identifier.package
//     //             }) {
//     //                 return Some(comp_id.clone());
//     //             }
//     //         }
//     //     }
//     //     None
//     // }
// }
