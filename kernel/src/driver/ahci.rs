/// AHCI HBA (Host Bus Adapter).
pub struct AhciHba {
    base: u64,
}

impl AhciHba {
    pub fn new(base: u64) -> Self {
        Self { base }
    }
}
