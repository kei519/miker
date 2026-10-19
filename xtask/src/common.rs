use clap::ValueEnum;
use serde::Deserialize;

#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize, ValueEnum,
)]
#[serde(rename_all = "lowercase")]
#[value(rename_all = "lower")]
pub enum Arch {
    AArch64,
    #[default]
    #[value(name = "x86_64")]
    X86_64,
}

impl Arch {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    pub fn host() -> Self {
        cfg_select! {
            target_arch = "x86_64" => {
                Self::X86_64
            }
            target_arch = "aarch64" => {
                Self::AArch64
            }
        }
    }
}

impl std::fmt::Display for Arch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value().unwrap().get_name().fmt(f)
    }
}
