# WAIL (WebAssembly Automated Interface Linker)

A tool for automatically discovering and configuring component links in WebAssembly applications using the WebAssembly Component Model.

## Overview

WAIL analyzes WebAssembly components and automatically detects their interface requirements, creating appropriate links between components that satisfy each other's imports and exports. It handles:

- WASI runtime interfaces automatically
- Component-to-component interface matching
- Explicit link configurations from WADM
- Validation of interface compatibility
- Horizontally scalable for multiple sourced components

## Usage

```bash
# Basic usage with components.yaml
wail --components path/to/components.yaml

# With WADM configuration
wail --components path/to/components.yaml --wadm path/to/wadm.yaml

# Generate complete WADM manifest
wail --components path/to/components.yaml --name my-app --version v0.1.0 > app.yaml
```
