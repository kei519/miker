use std::{collections::BTreeMap, fs::File, io::Read};

use anyhow::{Error, Result, anyhow, bail};
use cargo_metadata::{
    Metadata, MetadataCommand, Package,
    camino::{Utf8Path, Utf8PathBuf},
};
use gpt::partition_types::Type as PartitionType;
use serde::{
    Deserialize, Deserializer,
    de::{Unexpected, Visitor},
};
use serde_json::Value as JsonValue;

use crate::common::Arch;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TargetKind {
    Host,
    NonHost(String),
}

impl std::fmt::Display for TargetKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Host => f.write_str("host"),
            Self::NonHost(target) => f.write_str(target),
        }
    }
}

impl<'de> Deserialize<'de> for TargetKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = String;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("A character string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(v.into())
            }
        }

        deserializer.deserialize_str(Visitor).map(|s| {
            if s == "host" {
                Self::Host
            } else {
                Self::NonHost(s)
            }
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct PackageMetadata {
    pub target: TargetKind,
}

impl PackageMetadata {
    pub fn from_package(package: &Package) -> Result<Self> {
        const KEY_NAME: &str = "miker";

        let JsonValue::Object(metadata) = &package.metadata else {
            bail!("Package ({}) does not have metadata", package.name);
        };
        let Some(metadata) = metadata.get(KEY_NAME) else {
            bail!(
                "Package ({}) does not have metadata for {}",
                package.name,
                KEY_NAME
            );
        };
        serde_json::from_value(metadata.clone()).map_err(Error::new)
    }
}

pub fn metadata() -> Result<Metadata> {
    MetadataCommand::new().exec().map_err(Error::new)
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
pub struct MikerConfig {
    build: BuildConfig,
    arch: ArchMatrix,
}

impl MikerConfig {
    pub fn from_root(path: impl AsRef<Utf8Path>) -> Result<Self> {
        Self::from_path(path.as_ref().join("miker.toml"))
    }

    pub fn from_path(path: impl AsRef<Utf8Path>) -> Result<Self> {
        let path = path.as_ref().as_std_path();
        let mut toml_file = File::open(path)?;
        let mut toml_bytes = Vec::new();
        toml_file.read_to_end(&mut toml_bytes)?;
        toml::from_slice(&toml_bytes).map_err(Error::new)
    }

    pub fn resolve(&self, arch: Arch, kind: TargetKind) -> Result<ResolvedTarget> {
        if matches!(kind, TargetKind::Host) {
            return Ok(ResolvedTarget::Host);
        }

        let Some(mapping) = self.arch.0.get(&arch) else {
            bail!("There is no config for {}", arch);
        };
        mapping
            .target
            .0
            .get(&kind)
            .map(|spec| ResolvedTarget::Explicit(spec.0.clone()))
            .ok_or_else(|| anyhow!("Missing target mapping for {}.{}", arch, kind))
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
struct BuildConfig {
    build_dir: Utf8PathBuf,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            build_dir: "build".into(),
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
struct ArchMatrix(BTreeMap<Arch, ArchConfig>);

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
struct ArchConfig {
    target: Targets,
    image: ImageConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
struct ImageConfig {
    image_name: String,
    ovmf_vars: String,
    partition: Vec<Partition>,
    component: Vec<Component>,
}

impl Default for ImageConfig {
    fn default() -> Self {
        Self {
            image_name: "image.disk".into(),
            ovmf_vars: "vars.fd".into(),
            partition: vec![],
            component: vec![],
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct PartitionName(String);

#[derive(Debug, Clone, Deserialize)]
struct Partition {
    name: PartitionName,
    #[serde(
        rename = "type",
        deserialize_with = "Partition::deserialize_partition_type"
    )]
    ty: PartitionType,
    size: u64,
}

impl Partition {
    fn deserialize_partition_type<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<PartitionType, D::Error> {
        use serde::de::Error as _;

        struct TypeVisitor;
        impl<'d> Visitor<'d> for TypeVisitor {
            type Value = String;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a partition type name or GUID is required")
            }

            fn visit_str<E>(self, v: &str) -> std::prelude::v1::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(v.into())
            }
        }
        let name = deserializer.deserialize_str(TypeVisitor)?;

        PartitionType::from_name(&name)
            .map_err(|e| D::Error::invalid_value(Unexpected::Str(&name), &e.as_str()))
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ComponentName(String);

#[derive(Debug, Clone, Deserialize)]
struct Component {
    name: ComponentName,
    partition: PartitionName,
    install: Utf8PathBuf,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
struct Targets(BTreeMap<TargetKind, TargetSpec>);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
struct TargetSpec(String);

impl std::fmt::Display for TargetSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone)]
pub enum ResolvedTarget {
    Host,
    Explicit(String),
}

impl ResolvedTarget {
    pub fn option_for_cargo(&self) -> Option<String> {
        match self {
            Self::Host => None,
            Self::Explicit(target) => Some(format!("--target={target}")),
        }
    }
}
