#[derive(Clone, Copy)]
pub struct Identifier([u8; 6]);
impl Identifier {
    pub fn new(identifier: [u8; 6]) -> Self {
        Identifier(identifier)
    }

    pub fn as_bytes(&self) -> [u8; 6] {
        self.0
    }

    pub fn is_locally_administered(&self) -> bool {
        self.0[5] & 0x2 != 0
    }

    pub fn is_universally_administered(&self) -> bool {
        !self.is_locally_administered()
    }
}

impl Default for Identifier {
    fn default() -> Self {
        // A default identifier is marked as locally administered.
        Identifier([0x02, 0x00, 0x00, 0x00, 0x00, 0x00])
    }
}
