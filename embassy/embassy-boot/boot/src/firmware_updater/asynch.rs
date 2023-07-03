use digest::Digest;
#[cfg(target_os = "none")]
use embassy_embedded_hal::flash::partition::Partition;
#[cfg(target_os = "none")]
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embedded_storage_async::nor_flash::NorFlash;

use super::FirmwareUpdaterConfig;
use crate::{FirmwareUpdaterError, State, BOOT_MAGIC, STATE_ERASE_VALUE, SWAP_MAGIC};

/// FirmwareUpdater is an application API for interacting with the BootLoader without the ability to
/// 'mess up' the internal bootloader state
pub struct FirmwareUpdater<DFU: NorFlash, STATE: NorFlash> {
    dfu: DFU,
    state: STATE,
}

#[cfg(target_os = "none")]
impl<'a, FLASH: NorFlash>
    FirmwareUpdaterConfig<Partition<'a, NoopRawMutex, FLASH>, Partition<'a, NoopRawMutex, FLASH>>
{
    /// Create a firmware updater config from the flash and address symbols defined in the linkerfile
    pub fn from_linkerfile(flash: &'a embassy_sync::mutex::Mutex<NoopRawMutex, FLASH>) -> Self {
        extern "C" {
            static __bootloader_state_start: u32;
            static __bootloader_state_end: u32;
            static __bootloader_dfu_start: u32;
            static __bootloader_dfu_end: u32;
        }

        let dfu = unsafe {
            let start = &__bootloader_dfu_start as *const u32 as u32;
            let end = &__bootloader_dfu_end as *const u32 as u32;
            trace!("DFU: 0x{:x} - 0x{:x}", start, end);

            Partition::new(flash, start, end - start)
        };
        let state = unsafe {
            let start = &__bootloader_state_start as *const u32 as u32;
            let end = &__bootloader_state_end as *const u32 as u32;
            trace!("STATE: 0x{:x} - 0x{:x}", start, end);

            Partition::new(flash, start, end - start)
        };

        Self { dfu, state }
    }
}

impl<DFU: NorFlash, STATE: NorFlash> FirmwareUpdater<DFU, STATE> {
    /// Create a firmware updater instance with partition ranges for the update and state partitions.
    pub fn new(config: FirmwareUpdaterConfig<DFU, STATE>) -> Self {
        Self {
            dfu: config.dfu,
            state: config.state,
        }
    }

    /// Obtain the current state.
    ///
    /// This is useful to check if the bootloader has just done a swap, in order
    /// to do verifications and self-tests of the new image before calling
    /// `mark_booted`.
    pub async fn get_state(&mut self, aligned: &mut [u8]) -> Result<State, FirmwareUpdaterError> {
        self.state.read(0, aligned).await?;

        if !aligned.iter().any(|&b| b != SWAP_MAGIC) {
            Ok(State::Swap)
        } else {
            Ok(State::Boot)
        }
    }

    /// Verify the DFU given a public key. If there is an error then DO NOT
    /// proceed with updating the firmware as it must be signed with a
    /// corresponding private key (otherwise it could be malicious firmware).
    ///
    /// Mark to trigger firmware swap on next boot if verify suceeds.
    ///
    /// If the "ed25519-salty" feature is set (or another similar feature) then the signature is expected to have
    /// been generated from a SHA-512 digest of the firmware bytes.
    ///
    /// If no signature feature is set then this method will always return a
    /// signature error.
    ///
    /// # Safety
    ///
    /// The `_aligned` buffer must have a size of STATE::WRITE_SIZE, and follow the alignment rules for the flash being read from
    /// and written to.
    #[cfg(feature = "_verify")]
    pub async fn verify_and_mark_updated(
        &mut self,
        _public_key: &[u8],
        _signature: &[u8],
        _update_len: u32,
        _aligned: &mut [u8],
    ) -> Result<(), FirmwareUpdaterError> {
        assert_eq!(_aligned.len(), STATE::WRITE_SIZE);
        assert!(_update_len <= self.dfu.capacity() as u32);

        #[cfg(feature = "ed25519-dalek")]
        {
            use ed25519_dalek::{PublicKey, Signature, SignatureError, Verifier};

            use crate::digest_adapters::ed25519_dalek::Sha512;

            let into_signature_error = |e: SignatureError| FirmwareUpdaterError::Signature(e.into());

            let public_key = PublicKey::from_bytes(_public_key).map_err(into_signature_error)?;
            let signature = Signature::from_bytes(_signature).map_err(into_signature_error)?;

            let mut message = [0; 64];
            self.hash::<Sha512>(_update_len, _aligned, &mut message).await?;

            public_key.verify(&message, &signature).map_err(into_signature_error)?
        }
        #[cfg(feature = "ed25519-salty")]
        {
            use salty::constants::{PUBLICKEY_SERIALIZED_LENGTH, SIGNATURE_SERIALIZED_LENGTH};
            use salty::{PublicKey, Signature};

            use crate::digest_adapters::salty::Sha512;

            fn into_signature_error<E>(_: E) -> FirmwareUpdaterError {
                FirmwareUpdaterError::Signature(signature::Error::default())
            }

            let public_key: [u8; PUBLICKEY_SERIALIZED_LENGTH] = _public_key.try_into().map_err(into_signature_error)?;
            let public_key = PublicKey::try_from(&public_key).map_err(into_signature_error)?;
            let signature: [u8; SIGNATURE_SERIALIZED_LENGTH] = _signature.try_into().map_err(into_signature_error)?;
            let signature = Signature::try_from(&signature).map_err(into_signature_error)?;

            let mut message = [0; 64];
            self.hash::<Sha512>(_update_len, _aligned, &mut message).await?;

            let r = public_key.verify(&message, &signature);
            trace!(
                "Verifying with public key {}, signature {} and message {} yields ok: {}",
                public_key.to_bytes(),
                signature.to_bytes(),
                message,
                r.is_ok()
            );
            r.map_err(into_signature_error)?
        }

        self.set_magic(_aligned, SWAP_MAGIC).await
    }

    /// Verify the update in DFU with any digest.
    pub async fn hash<D: Digest>(
        &mut self,
        update_len: u32,
        chunk_buf: &mut [u8],
        output: &mut [u8],
    ) -> Result<(), FirmwareUpdaterError> {
        let mut digest = D::new();
        for offset in (0..update_len).step_by(chunk_buf.len()) {
            self.dfu.read(offset, chunk_buf).await?;
            let len = core::cmp::min((update_len - offset) as usize, chunk_buf.len());
            digest.update(&chunk_buf[..len]);
        }
        output.copy_from_slice(digest.finalize().as_slice());
        Ok(())
    }

    /// Mark to trigger firmware swap on next boot.
    ///
    /// # Safety
    ///
    /// The `aligned` buffer must have a size of STATE::WRITE_SIZE, and follow the alignment rules for the flash being written to.
    #[cfg(not(feature = "_verify"))]
    pub async fn mark_updated(&mut self, aligned: &mut [u8]) -> Result<(), FirmwareUpdaterError> {
        assert_eq!(aligned.len(), STATE::WRITE_SIZE);
        self.set_magic(aligned, SWAP_MAGIC).await
    }

    /// Mark firmware boot successful and stop rollback on reset.
    ///
    /// # Safety
    ///
    /// The `aligned` buffer must have a size of STATE::WRITE_SIZE, and follow the alignment rules for the flash being written to.
    pub async fn mark_booted(&mut self, aligned: &mut [u8]) -> Result<(), FirmwareUpdaterError> {
        assert_eq!(aligned.len(), STATE::WRITE_SIZE);
        self.set_magic(aligned, BOOT_MAGIC).await
    }

    async fn set_magic(&mut self, aligned: &mut [u8], magic: u8) -> Result<(), FirmwareUpdaterError> {
        self.state.read(0, aligned).await?;

        if aligned.iter().any(|&b| b != magic) {
            // Read progress validity
            self.state.read(STATE::WRITE_SIZE as u32, aligned).await?;

            if aligned.iter().any(|&b| b != STATE_ERASE_VALUE) {
                // The current progress validity marker is invalid
            } else {
                // Invalidate progress
                aligned.fill(!STATE_ERASE_VALUE);
                self.state.write(STATE::WRITE_SIZE as u32, aligned).await?;
            }

            // Clear magic and progress
            self.state.erase(0, self.state.capacity() as u32).await?;

            // Set magic
            aligned.fill(magic);
            self.state.write(0, aligned).await?;
        }
        Ok(())
    }

    /// Write data to a flash page.
    ///
    /// The buffer must follow alignment requirements of the target flash and a multiple of page size big.
    ///
    /// # Safety
    ///
    /// Failing to meet alignment and size requirements may result in a panic.
    pub async fn write_firmware(&mut self, offset: usize, data: &[u8]) -> Result<(), FirmwareUpdaterError> {
        assert!(data.len() >= DFU::ERASE_SIZE);

        self.dfu.erase(offset as u32, (offset + data.len()) as u32).await?;

        self.dfu.write(offset as u32, data).await?;

        Ok(())
    }

    /// Prepare for an incoming DFU update by erasing the entire DFU area and
    /// returning its `Partition`.
    ///
    /// Using this instead of `write_firmware` allows for an optimized API in
    /// exchange for added complexity.
    pub async fn prepare_update(&mut self) -> Result<&mut DFU, FirmwareUpdaterError> {
        self.dfu.erase(0, self.dfu.capacity() as u32).await?;

        Ok(&mut self.dfu)
    }
}

#[cfg(test)]
mod tests {
    use embassy_embedded_hal::flash::partition::Partition;
    use embassy_sync::blocking_mutex::raw::NoopRawMutex;
    use embassy_sync::mutex::Mutex;
    use futures::executor::block_on;
    use sha1::{Digest, Sha1};

    use super::*;
    use crate::mem_flash::MemFlash;

    #[test]
    fn can_verify_sha1() {
        let flash = Mutex::<NoopRawMutex, _>::new(MemFlash::<131072, 4096, 8>::default());
        let state = Partition::new(&flash, 0, 4096);
        let dfu = Partition::new(&flash, 65536, 65536);

        let update = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66];
        let mut to_write = [0; 4096];
        to_write[..7].copy_from_slice(update.as_slice());

        let mut updater = FirmwareUpdater::new(FirmwareUpdaterConfig { dfu, state });
        block_on(updater.write_firmware(0, to_write.as_slice())).unwrap();
        let mut chunk_buf = [0; 2];
        let mut hash = [0; 20];
        block_on(updater.hash::<Sha1>(update.len() as u32, &mut chunk_buf, &mut hash)).unwrap();

        assert_eq!(Sha1::digest(update).as_slice(), hash);
    }
}
