use crate::pac::CHIPID;

#[derive(Clone, Copy, Debug, defmt::Format)]
pub enum EmbeddedProcessor {
    Arm946Es,
    Arm7Tdmi,
    CortexM3,
    Arm920T,
    Arm926Ejs,
    CortexA5,
    CortexM4,
}

#[derive(Clone, Copy, Debug, defmt::Format)]
pub enum Family {
    AtSam4e,
    AtSam4n,
    AtSam4s,
}

#[derive(Clone, Copy, Debug, defmt::Format)]
pub enum Model {
    AtSam4e8c,
    AtSam4e8e,
    AtSam4e16c,
    AtSam4e16e,

    AtSam4n8a,
    AtSam4n8b,
    AtSam4n8c,
    AtSam4n16b,
    AtSam4n16c,

    AtSam4s2a,
    AtSam4s2b,
    AtSam4s2c,
    AtSam4s4a,
    AtSam4s4b,
    AtSam4s4c,
    AtSam4s8b,
    AtSam4s8c,
    AtSam4s16b,
    AtSam4s16c,
    AtSam4sa16b,
    AtSam4sa16c,
    AtSam4sd16b,
    AtSam4sd16c,
    AtSam4sd32b,
    AtSam4sd32c,
}

#[derive(Clone, Copy, Debug, defmt::Format)]
pub enum FlashMemoryType {
    Rom,
    Romless,
    Sram,
    Flash,
    RomFlash, // flash1_byte_size = ROM size, flash2_byte_size = Flash size
}

#[derive(Debug, defmt::Format)]
pub struct ChipId {
    version: u8,
    embedded_processor: Option<EmbeddedProcessor>,
    flash1_byte_size: Option<usize>,
    flash2_byte_size: Option<usize>,
    internal_sram_byte_size: Option<usize>,
    family: Option<Family>,
    model: Option<Model>,
    flash_memory_type: Option<FlashMemoryType>,
}

impl ChipId {
    pub fn new(chip_id: CHIPID) -> Self {
        let cidr = chip_id.cidr.read();
        let version = cidr.version().bits();
        let embedded_processor = match cidr.eproc().bits() {
            1 => Some(EmbeddedProcessor::Arm946Es),
            2 => Some(EmbeddedProcessor::Arm7Tdmi),
            3 => Some(EmbeddedProcessor::CortexM3),
            4 => Some(EmbeddedProcessor::Arm920T),
            5 => Some(EmbeddedProcessor::Arm926Ejs),
            6 => Some(EmbeddedProcessor::CortexA5),
            7 => Some(EmbeddedProcessor::CortexM4),
            _ => None,
        };
        let flash1_byte_size = Self::decode_flash_size_from_register(cidr.nvpsiz().bits());
        let flash2_byte_size = Self::decode_flash_size_from_register(cidr.nvpsiz2().bits());
        let internal_sram_byte_size = match cidr.sramsiz().bits() {
            0 => Some(48 * 1024),
            1 => Some(192 * 1024),
            2 => Some(2 * 1024),
            3 => Some(6 * 1024),
            4 => Some(24 * 1024),
            5 => Some(4 * 1024),
            6 => Some(80 * 1024),
            7 => Some(160 * 1024),
            8 => Some(8 * 1024),
            9 => Some(16 * 1024),
            10 => Some(32 * 1024),
            11 => Some(64 * 1024),
            12 => Some(128 * 1024),
            13 => Some(256 * 1024),
            14 => Some(96 * 1024),
            15 => Some(512 * 1024),
            _ => None,
        };
        let (family, model) = Self::decode_family_and_model(&chip_id);
        let flash_memory_type = match cidr.nvptyp().bits() {
            0 => Some(FlashMemoryType::Rom),
            1 => Some(FlashMemoryType::Romless),
            2 => Some(FlashMemoryType::Flash),
            3 => Some(FlashMemoryType::RomFlash),
            4 => Some(FlashMemoryType::Sram),
            _ => None,
        };

        ChipId {
            version,
            embedded_processor,
            flash1_byte_size,
            flash2_byte_size,
            internal_sram_byte_size,
            family,
            model,
            flash_memory_type,
        }
    }

    pub fn version(&self) -> u8 {
        self.version
    }

    pub fn embedded_processor(&self) -> Option<EmbeddedProcessor> {
        self.embedded_processor
    }

    pub fn flash1_byte_size(&self) -> Option<usize> {
        self.flash1_byte_size
    }

    pub fn flash2_byte_size(&self) -> Option<usize> {
        self.flash2_byte_size
    }

    pub fn internal_sram_size(&self) -> Option<usize> {
        self.internal_sram_byte_size
    }

    pub fn family(&self) -> Option<Family> {
        self.family
    }

    pub fn model(&self) -> Option<Model> {
        self.model
    }

    pub fn flash_memory_type(&self) -> Option<FlashMemoryType> {
        self.flash_memory_type
    }

    fn decode_flash_size_from_register(register_value: u8) -> Option<usize> {
        match register_value {
            1 => Some(8 * 1024),
            2 => Some(16 * 1024),
            3 => Some(32 * 1024),
            5 => Some(64 * 1024),
            7 => Some(128 * 1024),
            9 => Some(256 * 1024),
            10 => Some(512 * 1024),
            12 => Some(1024 * 1024),
            14 => Some(2048 * 1024),
            _ => None,
        }
    }

    fn decode_family_and_model(chip_id: &CHIPID) -> (Option<Family>, Option<Model>) {
        match chip_id.cidr.read().bits() {
            0x288B_07E0 => (Some(Family::AtSam4s), Some(Model::AtSam4s2a)),
            0x289B_07E0 => (Some(Family::AtSam4s), Some(Model::AtSam4s2b)),
            0x28AB_07E0 => (Some(Family::AtSam4s), Some(Model::AtSam4s2c)),

            0x288B_09E0 => (Some(Family::AtSam4s), Some(Model::AtSam4s4a)),
            0x289B_09E0 => (Some(Family::AtSam4s), Some(Model::AtSam4s4b)),
            0x28AB_09E0 => (Some(Family::AtSam4s), Some(Model::AtSam4s4c)),

            0x289C_0AE0 => (Some(Family::AtSam4s), Some(Model::AtSam4s8b)),
            0x28AC_0AE0 => (Some(Family::AtSam4s), Some(Model::AtSam4s8c)),

            0x289C_0CE0 => (Some(Family::AtSam4s), Some(Model::AtSam4s16b)),
            0x28AC_0CE0 => (Some(Family::AtSam4s), Some(Model::AtSam4s16c)),

            0x2897_0CE0 => (Some(Family::AtSam4s), Some(Model::AtSam4sa16b)),
            0x28A7_0CE0 => (Some(Family::AtSam4s), Some(Model::AtSam4sa16c)),

            0x2997_0CE0 => (Some(Family::AtSam4s), Some(Model::AtSam4sd16b)),
            0x29A7_0CE0 => (Some(Family::AtSam4s), Some(Model::AtSam4sd16c)),

            0x2997_0EE0 => (Some(Family::AtSam4s), Some(Model::AtSam4sd32b)),
            0x29A7_0EE0 => (Some(Family::AtSam4s), Some(Model::AtSam4sd32c)),

            0x293B_0AE0 => (Some(Family::AtSam4n), Some(Model::AtSam4n8a)),
            0x294B_0AE0 => (Some(Family::AtSam4n), Some(Model::AtSam4n8b)),
            0x295B_0AE0 => (Some(Family::AtSam4n), Some(Model::AtSam4n8c)),

            0x2946_0CE0 => (Some(Family::AtSam4n), Some(Model::AtSam4n16b)),
            0x2956_0CE0 => (Some(Family::AtSam4n), Some(Model::AtSam4n16c)),

            0xA3CC_0CE0 => match chip_id.exid.read().bits() {
                0x0012_0209 => (Some(Family::AtSam4e), Some(Model::AtSam4e8c)),
                0x0012_0208 => (Some(Family::AtSam4e), Some(Model::AtSam4e8e)),

                0x0012_0201 => (Some(Family::AtSam4e), Some(Model::AtSam4e16c)),
                0x0012_0200 => (Some(Family::AtSam4e), Some(Model::AtSam4e16e)),
                _ => (Some(Family::AtSam4e), None),
            },

            _ => (None, None),
        }
    }
}
