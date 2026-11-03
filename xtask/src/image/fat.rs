use std::{
    fs,
    io::{Read as _, Write as _},
};

use anyhow::{Result, anyhow};
use fatfs::{FileSystem, FormatVolumeOptions, FsOptions};
use fscommon::StreamSlice;

use super::PartitionInfo;

pub(super) fn install(
    mut disk_slice: StreamSlice<&mut fs::File>,
    part: PartitionInfo,
    sector_size: u64,
) -> Result<()> {
    // Prepare the label from the partition name.
    let truncated_name: String = part
        .name
        .char_indices()
        .map_while(|(i, c)| {
            if 11 <= i + c.len_utf8() {
                None
            } else {
                Some(c)
            }
        })
        .collect();
    let mut truncated_name_bytes = truncated_name.into_bytes();
    let mut label = [b' '; 11];
    label[..truncated_name_bytes.len()].swap_with_slice(&mut truncated_name_bytes);

    // Format the disk for FAT.
    let options = FormatVolumeOptions::new()
        .bytes_per_sector(sector_size as u16)
        .bytes_per_cluster(2 * sector_size as u32)
        .volume_label(label);
    fatfs::format_volume(&mut disk_slice, options)?;

    // Install components.
    let fs = FileSystem::new(disk_slice, FsOptions::new())?;
    let root = fs.root_dir();
    for component in part.components {
        let mut dir = root.clone();
        if let Some(parent) = component.install.parent() {
            for name in parent {
                dir = dir.create_dir(name)?;
            }
        }

        let mut dst = dir.create_file(
            component
                .install
                .file_name()
                .ok_or_else(|| anyhow!("The specified path is invalid: {}", component.install))?,
        )?;
        let mut src = std::fs::File::open(component.path)?;
        let mut data = vec![];
        let count = src.read_to_end(&mut data)?;
        dst.truncate()?;
        dst.write_all(&data[..count])?;
    }
    Ok(())
}
