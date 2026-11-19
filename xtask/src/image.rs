use std::fs::{self, OpenOptions};

use anyhow::Result;
use cargo_metadata::camino::{Utf8Path, Utf8PathBuf};
use clap::Parser;
use fscommon::StreamSlice;
use gpt::{GptConfig, disk::LogicalBlockSize, mbr::ProtectiveMBR, partition::Partition};

use crate::{
    Status,
    build::{BuildOptions, BuildResult, Component},
    common::Arch,
    config::PartitionType,
};

mod fat;

#[derive(Debug, Clone, Parser)]
pub struct ImageCommand {
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

    #[arg(long)]
    dir: Option<Utf8PathBuf>,
}

impl ImageCommand {
    pub fn run(self) -> Result<Status> {
        ImageOptions {
            keep_going: self.keep_going,
            locked: self.locked,
            arch: self.arch,
            release: self.release,
            dir: self.dir.as_deref(),
        }
        .build()
    }
}

#[derive(Debug, Clone)]
pub struct ImageOptions<'a> {
    pub keep_going: bool,
    pub locked: bool,
    pub arch: Arch,
    pub release: bool,
    pub dir: Option<&'a Utf8Path>,
}

impl<'a> ImageOptions<'a> {
    pub fn build(self) -> Result<Status> {
        const LBS: LogicalBlockSize = LogicalBlockSize::Lb512;
        // ProtectiveMBR + (table header + entries) * 2 (primary and backup)
        const EFI_DISK_MIN_SIZE: u64 = LBS.as_u64() + (LBS.as_u64() + (16 << 10)) * 2;

        // Build components.
        let Status::Success(BuildResult {
            build_config,
            image_name,
            data,
            ..
        }) = BuildOptions {
            keep_going: self.keep_going,
            locked: self.locked,
            arch: self.arch,
            release: self.release,
        }
        .build()?
        else {
            return Ok(Status::Failure);
        };

        // Create the image file.
        let dir = self
            .dir
            .unwrap_or(&build_config.dir)
            .join(self.arch.to_string());
        let image_path = dir.join(image_name);
        fs::create_dir_all(dir)?;

        let image_size = data
            .iter()
            .fold(EFI_DISK_MIN_SIZE, |acc, data| acc + data.partition.size);
        let mut image = OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .truncate(false)
            .open(&image_path)?;
        image.set_len(image_size)?;

        // Format image as GPT.
        let mbr = ProtectiveMBR::with_lb_size(
            u32::try_from(image_size / LBS.as_u64() - 1).unwrap_or(u32::MAX),
        );
        mbr.overwrite_lba0(&mut image)?;

        let mut disk = GptConfig::new()
            .writable(true)
            .logical_block_size(LBS)
            .change_partition_count(true)
            .create_from_device(image, None)?;

        let partitions: Vec<_> = data
            .into_iter()
            .map(|data| {
                let part = &data.partition;
                let id =
                    disk.add_partition(part.name.as_ref(), part.size, part.ty.into(), 0, None)?;
                // Partition with ID `id` exists.
                let &Partition {
                    ref name,
                    first_lba,
                    last_lba,
                    ..
                } = disk.partitions().get(&id).unwrap();
                let range = (first_lba * LBS.as_u64())..((last_lba + 1) * LBS.as_u64());
                Ok(PartitionInfo {
                    name: name.clone(),
                    ty: part.ty,
                    range,
                    components: data.components,
                })
            })
            .collect::<Result<_>>()?;
        let mut disk = disk.write()?;

        // Install components to the proper paritions.
        for part in partitions {
            let disk_slice = StreamSlice::new(&mut disk, part.range.start, part.range.end)?;

            match part.ty {
                PartitionType::Efi => {
                    fat::install(disk_slice, part, LBS.as_u64())?;
                }
            }
        }

        Ok(Status::success())
    }
}

pub struct PartitionInfo {
    name: String,
    ty: PartitionType,
    range: std::ops::Range<u64>,
    components: Vec<Component>,
}
