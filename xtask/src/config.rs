use std::{collections::BTreeMap, fs::File, io::Read};

use anyhow::{Error, Result, anyhow, bail};
use cargo_metadata::{
    Metadata, MetadataCommand, Package,
    camino::{Utf8Path, Utf8PathBuf},
};
use gpt::partition_types;
use serde::Deserialize;
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
    pub build: BuildConfig,
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

    pub fn take_arch_config(&mut self, arch: Arch) -> Option<ArchConfig> {
        self.arch.0.remove(&arch)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct BuildConfig {
    pub dir: Utf8PathBuf,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            dir: "build".into(),
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
struct ArchMatrix(BTreeMap<Arch, ArchConfig>);

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
pub struct ArchConfig {
    target: Targets,
    pub image: ImageConfig,
}

impl ArchConfig {
    // This should return `Option<_>` but now the returned value is always converted to
    // `Result<_>`, so we return `Result<_>`.
    pub fn target(&self, kind: &TargetKind) -> Result<ResolvedTarget> {
        if matches!(kind, TargetKind::Host) {
            return Ok(ResolvedTarget::Host);
        }

        self.target
            .0
            .get(kind)
            .map(|spec| ResolvedTarget::Explicit(spec.0.clone()))
            .ok_or_else(|| anyhow!("Target triple of {} is not specified", kind))
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ImageBaseConfig {
    pub image_name: String,
    pub ovmf_vars: String,
}

impl Default for ImageBaseConfig {
    fn default() -> Self {
        Self {
            image_name: "disk.img".into(),
            ovmf_vars: "vars.fd".into(),
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
pub struct ImageConfig {
    #[serde(flatten)]
    pub base: ImageBaseConfig,
    pub partition: Vec<Partition>,
    pub component: Vec<ComponentConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
pub struct PartitionName(String);

impl AsRef<str> for PartitionName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(try_from = "&str")]
pub enum PartitionType {
    Efi,
}

impl TryFrom<&str> for PartitionType {
    type Error = Error;

    fn try_from(value: &str) -> std::prelude::v1::Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "efi" => Ok(Self::Efi),
            name => Err(anyhow!("unknown partition type: {name}")),
        }
    }
}

impl From<PartitionType> for partition_types::Type {
    fn from(value: PartitionType) -> Self {
        match value {
            PartitionType::Efi => partition_types::EFI,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Partition {
    pub name: PartitionName,
    #[serde(rename = "type")]
    pub ty: PartitionType,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
pub struct ComponentName(String);

impl AsRef<str> for ComponentName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ComponentName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ComponentConfig {
    pub name: ComponentName,
    pub partition: PartitionName,
    pub install: Utf8PathBuf,
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
