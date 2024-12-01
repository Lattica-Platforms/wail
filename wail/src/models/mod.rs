mod components;
pub use components::{ComponentInfo, ComponentsConfig, Entity, InterfaceInfo, PackageInfo, Source};

/// Represents a uniquely identifiable interface
#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct InterfaceIdentifier {
    pub name: String,
    pub namespace: String,
    pub package: String,
}

/// Represents an interface requirement (import) or provision (export)
///
/// * `identifier` - Unique identification of the interface (name, namespace, package)
/// * `direction` - Whether this component imports (consumes) or exports (provides) the interface
/// * `component` - The name of the component that declares this requirement. For imports,
///                this is the component that needs the interface. For exports, this is
///                the component that provides the interface.
#[derive(Clone, Debug)]
pub struct InterfaceRequirement {
    pub identifier: InterfaceIdentifier,
    pub direction: Direction,
    pub component: String,
}

impl InterfaceRequirement {
    pub fn new(
        name: String,
        namespace: String,
        package: String,
        direction: Direction,
        component: String,
    ) -> Self {
        Self {
            identifier: InterfaceIdentifier {
                name,
                namespace,
                package,
            },
            direction,
            component,
        }
    }
}

#[derive(Clone, Copy, Hash, Eq, PartialEq, Debug)]
pub enum Direction {
    Import,
    Export,
}

impl Direction {
    pub fn opposite(&self) -> Self {
        match self {
            Direction::Import => Direction::Export,
            Direction::Export => Direction::Import,
        }
    }
}
