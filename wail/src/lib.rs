#![allow(clippy::missing_safety_doc)]
wit_bindgen::generate!();

use crate::cli::Wail;
use crate::exports::wasi::cli::run::Guest as RunGuest;
use crate::wasi::cli::environment;
use clap::builder::ValueParser;
use clap::Arg;
use clap::CommandFactory;
use clap::FromArgMatches;
use core::RUNTIME_INTERFACES;
use exports::wasmcloud::wash::subcommand::{Argument, Guest as SubcommandGuest, Metadata};
use models::ComponentInfo;
use models::ComponentsConfig;
use models::Entity;
use models::InterfaceInfo;
use models::PackageInfo;
use std::path::Path;
use std::path::PathBuf;
use wadm_types::Manifest;
use wadm_types::Trait;

mod cli;
mod core;
mod models;

use core::process_wasm_file;
use core::ConstructorManifest;
use models::Source;

struct WailPlugin;

export!(WailPlugin);

impl From<&Arg> for Argument {
    fn from(arg: &Arg) -> Self {
        Self {
            description: arg.get_help().map(ToString::to_string).unwrap_or_default(),
            is_path: arg.get_value_parser().type_id() == ValueParser::path_buf().type_id(),
            required: arg.is_required_set(),
        }
    }
}

impl WailPlugin {
    fn process_components(
        constructor: &mut ConstructorManifest,
        components_path: &Path,
    ) -> Result<(), ()> {
        let components_config: ComponentsConfig = {
            let content = std::fs::read_to_string(components_path).map_err(|e| {
                eprintln!("Failed to read components file: {}", e);
            })?;
            serde_yaml::from_str(&content).map_err(|e| {
                eprintln!("Failed to parse components file: {}", e);
            })?
        };

        for entity in &components_config.entities {
            Self::process_entity(constructor, entity)?;
        }

        Ok(())
    }

    fn process_entity(constructor: &mut ConstructorManifest, entity: &Entity) -> Result<(), ()> {
        match &entity.source {
            Some(Source::File { path }) => Self::process_file_entity(constructor, entity, path),
            Some(Source::OCI { reference }) => {
                println!("Processing OCI component: {} at {}", entity.name, reference);
                // TODO - add oci handling and known interfaces
                // Handle known capability providers
                if reference.contains("wasmcloud/http-server") {
                    // Create a ComponentInfo for the HTTP server
                    let info = ComponentInfo {
                        imports: vec![
                            // HTTP server IMPORTS incoming-handler (which our component exports)
                            InterfaceInfo {
                                name: "incoming-handler".to_string(),
                                namespace: "wasi".to_string(),
                                package: "http".to_string(),
                            },
                        ],
                        exports: vec![
                            // HTTP server EXPORTS outgoing-handler (which our component imports)
                            InterfaceInfo {
                                name: "outgoing-handler".to_string(),
                                namespace: "wasi".to_string(),
                                package: "http".to_string(),
                            },
                        ],
                        package: Some(PackageInfo {
                            namespace: "wasmcloud".to_string(),
                            name: "httpserver".to_string(),
                        }),
                    };
                    if let Err(e) = constructor.merge_component_info(
                        entity.name.clone(),
                        info,
                        PathBuf::from(reference),
                    ) {
                        eprintln!("Failed to merge component info: {}", e);
                        return Err(());
                    }
                }
                Ok(())
            }
            None => {
                let default_path = entity.get_source();
                Self::process_file_entity(constructor, entity, &default_path)
            }
        }
    }

    fn process_file_entity(
        constructor: &mut ConstructorManifest,
        entity: &Entity,
        path: &Path,
    ) -> Result<(), ()> {
        println!(
            "Processing WASM component: {} at {}",
            entity.name,
            path.display()
        );

        // Get interfaces from WASM file (Source of Truth)
        match process_wasm_file(&entity.name, path) {
            Ok(component_info) => {
                println!("Got component interface info:");
                println!("  Imports: {:?}", component_info.imports);
                println!("  Exports: {:?}", component_info.exports);

                // Add ALL imports and exports from the WASM file
                constructor
                    .merge_component_info(entity.name.clone(), component_info, path.to_path_buf())
                    .map_err(|e| {
                        eprintln!("Failed to merge component interface info: {}", e);
                    })?;
                Ok(())
            }
            Err(e) => {
                eprintln!(
                    "Error: Failed to process WASM file {}: {}",
                    path.display(),
                    e
                );
                Err(()) // Fail if we can't get interfaces
            }
        }
    }

    fn transform_to_wadm(
        input: ConstructorManifest,
        name: String,
        version: String,
        description: String,
    ) -> Manifest {
        let mut components = input.spec.components;

        for component in &mut components {
            // Only include non-WASI link constructors
            let component_links: Vec<Trait> = input
                .link_constructors
                .iter()
                .filter(|c| c.pre_component_id == component.name)
                .filter(|c| {
                    !(c.namespace == "wasi"
                        && RUNTIME_INTERFACES
                            .iter()
                            .any(|(_, pkg, name)| pkg == &c.package && name == &c.interfaces[0]))
                })
                .map(|c| c.to_wadm_link())
                .collect();

            let mut traits = component.traits.clone().unwrap_or_default();
            traits.extend(component_links);
            component.traits = Some(traits);
        }

        let mut annotations = std::collections::BTreeMap::new();
        annotations.insert(wadm_types::VERSION_ANNOTATION_KEY.to_string(), version);
        annotations.insert(
            wadm_types::DESCRIPTION_ANNOTATION_KEY.to_string(),
            description,
        );

        Manifest {
            api_version: input
                .api_version
                .unwrap_or_else(|| wadm_types::OAM_VERSION.to_string()),
            kind: input
                .kind
                .unwrap_or_else(|| wadm_types::APPLICATION_KIND.to_string()),
            metadata: input.metadata.unwrap_or_else(|| wadm_types::Metadata {
                name,
                annotations,
                labels: Default::default(),
            }),
            spec: wadm_types::Specification {
                components,
                policies: input.spec.policies,
            },
        }
    }
}

impl RunGuest for WailPlugin {
    fn run() -> Result<(), ()> {
        let args = environment::get_arguments();
        let cmd = Wail::command();

        // Parse arguments
        let matches = match cmd.try_get_matches_from(args) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Error parsing arguments: {}", e);
                return Err(());
            }
        };

        let args = match Wail::from_arg_matches(&matches) {
            Ok(a) => a,
            Err(e) => {
                eprintln!("Error parsing arguments: {}", e);
                return Err(());
            }
        };

        // Ensure at least one input is provided
        if args.wadm.is_none() && args.components.is_none() {
            eprintln!("Error: Must provide either --wadm or --components or both");
            return Err(());
        }

        // Start with an empty constructor
        let mut constructor = ConstructorManifest::new();

        // Process components.yaml if provided
        if let Some(components_path) = &args.components {
            println!("Processing components from: {}", components_path.display());
            if let Err(_) = Self::process_components(&mut constructor, components_path) {
                eprintln!("Failed to process components");
                return Err(());
            }
        }

        // Process WADM if provided
        if let Some(wadm_path) = &args.wadm {
            println!("Processing WADM manifest from: {}", wadm_path.display());

            let input_content = match std::fs::read_to_string(wadm_path) {
                Ok(content) => content,
                Err(e) => {
                    eprintln!("Failed to read WADM file: {}", e);
                    return Err(());
                }
            };

            let wadm: Manifest = match serde_yaml::from_str(&input_content) {
                Ok(manifest) => manifest,
                Err(e) => {
                    eprintln!("Failed to parse WADM manifest: {}", e);
                    return Err(());
                }
            };

            if let Err(e) = constructor.merge_wadm(&wadm) {
                eprintln!("Failed to merge WADM manifest: {}", e);
                return Err(());
            }
        }

        // Validate and resolve links
        println!("Validating and resolving links...");
        let validation_report = match constructor.validate() {
            Ok(report) => report,
            Err(e) => {
                eprintln!("Validation failed: {}", e);
                return Err(());
            }
        };

        // If validation produced errors, fail
        if !validation_report.is_valid {
            eprintln!("\nValidation errors:");
            for error in &validation_report.errors {
                eprintln!("  - {}", error);
            }
            return Err(());
        }

        // Print warnings if any
        if !validation_report.warnings.is_empty() {
            println!("\nWarnings:");
            for warning in &validation_report.warnings {
                println!("  - {}", warning);
            }
        }

        // Transform to final WADM
        let wadm = Self::transform_to_wadm(constructor, args.name, args.version, args.description);

        // Output result
        let output_content = match serde_yaml::to_string(&wadm) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Failed to serialize manifest: {}", e);
                return Err(());
            }
        };

        println!("\n---");
        println!("{}", output_content);

        Ok(())
    }
}

impl SubcommandGuest for WailPlugin {
    fn register() -> Metadata {
        let cmd = Wail::command();
        let (arguments, flags): (Vec<_>, Vec<_>) =
            cmd.get_arguments().partition(|arg| arg.is_positional());

        let arguments = arguments
            .into_iter()
            .map(|arg| (arg.get_id().to_string(), Argument::from(arg)))
            .collect();

        let flags = flags
            .into_iter()
            .map(|arg| (arg.get_id().to_string(), Argument::from(arg)))
            .collect();

        Metadata {
            name: "Wail".to_string(),
            id: "wail".to_string(),
            description: "Wasm Optimistic Linking".to_string(),
            author: "luk3ark".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            flags,
            arguments,
        }
    }
}

pub struct InputStreamReader<'a> {
    stream: &'a mut crate::wasi::io::streams::InputStream,
}

impl<'a> From<&'a mut crate::wasi::io::streams::InputStream> for InputStreamReader<'a> {
    fn from(stream: &'a mut crate::wasi::io::streams::InputStream) -> Self {
        Self { stream }
    }
}

impl std::io::Read for InputStreamReader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        use crate::wasi::io::streams::StreamError;
        use std::io;

        let n = buf
            .len()
            .try_into()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        match self.stream.blocking_read(n) {
            Ok(chunk) => {
                let n = chunk.len();
                if n > buf.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        "more bytes read than requested",
                    ));
                }
                buf[..n].copy_from_slice(&chunk);
                Ok(n)
            }
            Err(StreamError::Closed) => Ok(0),
            Err(StreamError::LastOperationFailed(e)) => {
                Err(io::Error::new(io::ErrorKind::Other, e.to_debug_string()))
            }
        }
    }
}
