use std::{
    ffi::{OsStr, OsString},
    fs,
    process::Command,
};

use anyhow::Result;
use cargo_metadata::camino::Utf8PathBuf;
use clap::Parser;
use ovmf_prebuilt::{FileType, Prebuilt, Source};

use crate::{Status, common::Arch, image::ImageOptions};

#[derive(Debug, Clone, Parser)]
pub struct RunCommand {
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

    /// QEMU path.
    ///
    /// It is assumed that the directory of qemu is added to the PATH environment variable. If not
    /// you must specify the path through this.
    #[arg(long)]
    qemu_path: Option<Utf8PathBuf>,

    /// Extra arguments for QEMU.
    #[arg(name = "ARGS")]
    extra_qemu_args: Vec<String>,
}

impl RunCommand {
    pub fn run(self) -> Result<Status> {
        let Status::Success(result) = ImageOptions {
            keep_going: self.keep_going,
            locked: self.locked,
            arch: self.arch,
            release: self.release,
            dir: self.dir.as_deref(),
        }
        .build()?
        else {
            return Ok(Status::Failure);
        };

        // We should acquire the path from Context or something.
        let prebuilt_dir = result.base_dir().join("ovmf");
        let prebuilt = Prebuilt::fetch(Source::EDK2_STABLE202605_R1, prebuilt_dir)?;
        let vars_path = result.ovmf_vars_path();
        fs::copy(
            prebuilt.get_file(self.arch.into(), FileType::Vars),
            &vars_path,
        )?;
        let mut ovmf_code_option = OsString::from("if=pflash,format=raw,readonly=on,file=");
        ovmf_code_option.push(prebuilt.get_file(self.arch.into(), FileType::Code));

        let qemu_path = self
            .qemu_path
            .unwrap_or_else(|| format!("qemu-system-{}", self.arch).into());

        let mut qemu = Command::new(qemu_path);
        match self.arch {
            Arch::AArch64 => {
                qemu.args(["-machine", "virt"]).args(["-cpu", "cortex-a35"]);
            }
            Arch::X86_64 => {
                qemu.args(["-machine", "q35"]).arg("-nodefaults");
            }
        }
        qemu.args(["-monitor", "vc"])
            .args(["-serial", "mon:stdio"])
            .args(["-device", "virtio-gpu-pci"])
            .args(["-device", "nec-usb-xhci,id=xhci"])
            .args(["-device", "usb-kbd"])
            .args(["-device", "usb-mouse"])
            .args(["-device", "usb-storage,drive=usb0"])
            .args([OsStr::new("-drive"), &ovmf_code_option])
            .args([
                "-drive",
                &format!("if=pflash,format=raw,file={}", vars_path),
            ])
            .args([
                "-drive",
                &format!(
                    "if=none,id=usb0,media=disk,format=raw,file={}",
                    result.image_path()
                ),
            ])
            .args(self.extra_qemu_args);

        #[cfg(target_os = "macos")]
        if self.arch == Arch::host() {
            qemu.args(["-accel", "hvf"]);
        }
        #[cfg(target_os = "linux")]
        if self.arch == Arch::host() && fs::exists("/dev/kvm")? {
            qemu.args(["-accel", "kvm"]);
        }

        Ok(if qemu.status()?.success() {
            Status::success()
        } else {
            Status::Failure
        })
    }
}
