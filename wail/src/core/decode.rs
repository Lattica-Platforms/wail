use std::path::Path;

use wit_component::DecodedWasm;
use wit_parser::{WorldItem, WorldKey};

use crate::models::{ComponentInfo, InterfaceInfo, PackageInfo};

fn world_key_to_string(key: &WorldKey) -> String {
    match key {
        WorldKey::Name(name) => name.clone(),
        WorldKey::Interface(id) => format!("interface-{}", id.index()),
    }
}

pub fn process_wasm_file(name: &str, path: &Path) -> anyhow::Result<ComponentInfo> {
    println!("Processing WASM file for {}: {}", name, path.display());
    let bytes = std::fs::read(path)?;

    if &bytes[0..4] != b"\0asm" {
        anyhow::bail!("Not a WASM file: {}", path.display());
    }

    match wit_component::decode(&bytes)? {
        DecodedWasm::Component(resolve, world_id) => {
            let world = &resolve.worlds[world_id];

            // Debug world information
            println!("\nWorld Information:");
            println!("  World Name: {}", world.name);

            // Debug interface definitions
            println!("\nInterface Functions:");
            for (id, interface_def) in resolve.interfaces.iter() {
                println!("Interface[{}]:", id.index());
                if let Some(pkg_id) = interface_def.package {
                    let pkg = &resolve.packages[pkg_id];
                    println!("  Package: {}.{}", pkg.name.namespace, pkg.name.name);
                }
                println!("  Name: {:?}", interface_def.name);
                println!("  Functions:");
                for (fname, func) in &interface_def.functions {
                    println!("    {} -> {:?}", fname, func);
                }
            }

            let mut info = ComponentInfo {
                imports: Vec::new(),
                exports: Vec::new(),
                package: None,
            };

            // Process imports
            println!("\nAdding interfaces for component '{}':", name);
            println!("  Imports:");

            for (world_key, import) in &world.imports {
                match import {
                    WorldItem::Interface { id, .. } => {
                        let interface_def = &resolve.interfaces[*id];
                        if let Some(pkg_id) = interface_def.package {
                            let pkg = &resolve.packages[pkg_id];

                            let interface_name = if let Some(name) = &interface_def.name {
                                name.clone()
                            } else {
                                match world_key {
                                    WorldKey::Name(n) => n.clone(),
                                    WorldKey::Interface(_) => {
                                        if let Some((fname, _)) =
                                            interface_def.functions.iter().next()
                                        {
                                            if fname.starts_with("[method]") {
                                                fname.split('.').last().unwrap_or(fname).to_string()
                                            } else {
                                                fname.to_string()
                                            }
                                        } else {
                                            format!("{}-{}", pkg.name.name, id.index())
                                        }
                                    }
                                }
                            };

                            println!(
                                "    - {}:{}:{}",
                                pkg.name.namespace, pkg.name.name, interface_name
                            );

                            info.imports.push(InterfaceInfo {
                                name: interface_name,
                                namespace: pkg.name.namespace.clone(),
                                package: pkg.name.name.clone(),
                            });
                        }
                    }
                    WorldItem::Function(func) => {
                        println!("Found function import: {}", world_key_to_string(world_key));
                        println!("  Function: {:?}", func);
                    }
                    WorldItem::Type(_) => {
                        println!("Found type import: {}", world_key_to_string(world_key));
                    }
                }
            }

            // Process exports
            println!("  Exports:");
            for (world_key, export) in &world.exports {
                match export {
                    WorldItem::Interface { id, .. } => {
                        let interface_def = &resolve.interfaces[*id];
                        if let Some(pkg_id) = interface_def.package {
                            let pkg = &resolve.packages[pkg_id];

                            let interface_name = if let Some(name) = &interface_def.name {
                                name.clone()
                            } else {
                                match world_key {
                                    WorldKey::Name(n) => n.clone(),
                                    WorldKey::Interface(_) => {
                                        if let Some((fname, _)) =
                                            interface_def.functions.iter().next()
                                        {
                                            if fname.starts_with("[method]") {
                                                fname.split('.').last().unwrap_or(fname).to_string()
                                            } else {
                                                fname.to_string()
                                            }
                                        } else {
                                            format!("{}-{}", pkg.name.name, id.index())
                                        }
                                    }
                                }
                            };

                            println!(
                                "    - {}:{}:{}",
                                pkg.name.namespace, pkg.name.name, interface_name
                            );

                            info.exports.push(InterfaceInfo {
                                name: interface_name,
                                namespace: pkg.name.namespace.clone(),
                                package: pkg.name.name.clone(),
                            });
                        }
                    }
                    WorldItem::Function(func) => {
                        println!("Found function export: {}", world_key_to_string(world_key));
                        println!("  Function: {:?}", func);
                    }
                    WorldItem::Type(_) => {
                        println!("Found type export: {}", world_key_to_string(world_key));
                    }
                }
            }

            // Add component package info if available
            if let Some(pkg_id) = world.package {
                let package = &resolve.packages[pkg_id];
                info.package = Some(PackageInfo {
                    namespace: package.name.namespace.clone(),
                    name: package.name.name.clone(),
                });
            }

            Ok(info)
        }
        DecodedWasm::WitPackage(resolve, pkg_id) => {
            println!("Found WIT package for {}", name);
            // For WIT packages, we only set the package info
            let package = &resolve.packages[pkg_id];
            let info = ComponentInfo {
                imports: Vec::new(),
                exports: Vec::new(),
                package: Some(PackageInfo {
                    namespace: package.name.namespace.clone(),
                    name: package.name.name.clone(),
                }),
            };
            println!("  Package: {:?}", info.package);
            Ok(info)
        }
    }
}
