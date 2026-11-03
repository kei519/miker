use std::{
    collections::BTreeMap,
    io::BufReader,
    process::{Command, Stdio},
};

use anyhow::{Result, anyhow, bail};
use cargo_metadata::{Message, camino::Utf8PathBuf};
use clap::Parser;

use crate::{
    Status,
    common::{Arch, MessageFormat},
    config::{self, BuildConfig, MikerConfig, PackageMetadata, Partition},
};

#[derive(Debug, Clone, Parser)]
pub struct BuildCommand {
    /// Do not abort the build as soon as there is an error.
    #[arg(long)]
    keep_going: bool,

    /// Assert that `Cargo.lock` will remain unchanged.
    #[arg(long)]
    locked: bool,

    #[arg(from_global)]
    arch: Arch,

    #[arg(long)]
    release: bool,
}

impl BuildCommand {
    pub fn run(self) -> Result<Status> {
        BuildOptions {
            keep_going: self.keep_going,
            locked: self.locked,
            arch: self.arch,
            release: self.release,
        }
        .build()
        .map(|result| match result {
            BuildResult::Success { .. } => Status::Success,
            BuildResult::Failure => Status::Failure,
        })
    }
}

pub struct Component {
    pub install: Utf8PathBuf,
    pub path: Utf8PathBuf,
}

// Must be renamed
pub struct PartitionData {
    pub partition: Partition,
    pub components: Vec<Component>,
}

pub enum BuildResult {
    Success {
        build_config: BuildConfig,
        image_name: String,
        data: Vec<PartitionData>,
    },
    Failure,
}

pub struct BuildOptions {
    pub keep_going: bool,
    pub locked: bool,
    pub arch: Arch,
    pub release: bool,
}

impl BuildOptions {
    pub fn build(self) -> Result<BuildResult> {
        let mut failed = false;

        let cargo_metadata = config::metadata()?;
        let mut build_config = MikerConfig::from_root(&cargo_metadata.workspace_root)?;
        let arch_config = build_config
            .take_arch_fonfig(self.arch)
            .ok_or_else(|| anyhow!("Config for {} is not found", self.arch))?;

        let mut executables = BTreeMap::new();
        for package in cargo_metadata.workspace_packages() {
            // Skip cargo-xtask itself
            if package.name == env!("CARGO_PKG_NAME") {
                continue;
            }

            let package_metadata = match PackageMetadata::from_package(package) {
                Ok(metadata) => metadata,
                Err(e) => {
                    eprintln!("skip {}: {:?}", package.name, e);
                    continue;
                }
            };

            let mut cmd = Command::new(env!("CARGO"));
            cmd.args([
                "build",
                &format!("--package={}", package.name),
                &format!("--message-format={}", MessageFormat::JsonRenderDiagnostics),
            ]);

            if let Some(option) = arch_config
                .target(&package_metadata.target)?
                .option_for_cargo()
            {
                cmd.arg(option);
            }

            if self.keep_going {
                cmd.arg("--keep-going");
            }
            if self.locked {
                cmd.arg("--locked");
            }
            if self.release {
                cmd.arg("--release");
            }

            cmd.stdout(Stdio::piped());
            let mut child = cmd.spawn()?;

            let stdout = child
                .stdout
                .take()
                .ok_or_else(|| anyhow!("A child process stdout is not piped"))?;
            let buf = BufReader::new(stdout);
            for result in Message::parse_stream(buf) {
                let artifact = match result {
                    Ok(Message::CompilerArtifact(artifact)) => artifact,
                    Err(e) => {
                        bail!("Failed to read compiler message: {e}");
                    }
                    _ => continue,
                };
                let Some(executable) = artifact.executable else {
                    continue;
                };
                if executables
                    .insert(package.name.as_str(), executable)
                    .is_some()
                {
                    bail!(
                        "Package ({}) outputs executables more than once",
                        package.name
                    );
                }
            }

            if !cmd.status()?.success() {
                failed = true;

                if !self.keep_going {
                    return Ok(BuildResult::Failure);
                }
            }
        }

        if failed {
            return Ok(BuildResult::Failure);
        }

        let image_config = arch_config.image;
        let mut data: Vec<_> = image_config
            .partition
            .into_iter()
            .map(|partition| PartitionData {
                partition,
                components: vec![],
            })
            .collect();
        for component in image_config.component {
            let executable = executables
                .remove(component.name.as_ref())
                .ok_or_else(|| anyhow!("There is no executable for \"{}\"", component.name))?;
            let partition = data
                .iter_mut()
                .find(|data| data.partition.name == component.partition)
                .ok_or_else(|| {
                    anyhow!(
                        "There is no partition settings for component \"{}\"",
                        component.name
                    )
                })?;
            partition.components.push(Component {
                install: component.install,
                path: executable,
            });
        }

        Ok(BuildResult::Success {
            build_config: build_config.build,
            image_name: image_config.image_name,
            data,
        })
    }
}
