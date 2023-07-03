//! HAL interface to the Enhanced Embedded Flash Controller (EEFC) peripheral
//!
//! Loosely based off of <https://github.com/nrf-rs/nrf-hal/blob/master/nrf-hal-common/src/nvmc.rs>
//! Many of the functions are named the same as ASF (minus flash_) and should be mostly equivalent.

#[cfg(any(feature = "atsam4e", feature = "atsam4n"))]
use crate::pac::efc;

#[cfg(any(feature = "atsam4e", feature = "atsam4n"))]
use crate::pac::EFC;

#[cfg(feature = "atsam4s")]
use crate::pac::efc0 as efc;

#[cfg(feature = "atsam4s")]
use crate::pac::EFC0 as EFC;

#[cfg(feature = "atsam4sd")]
use crate::pac::EFC1;

use cortex_m::interrupt;
use embedded_storage::nor_flash::{
    ErrorType, NorFlash, NorFlashError, NorFlashErrorKind, ReadNorFlash,
};

// Common EEFC constants for sam4-hal
const FLASH_PAGE_SIZE: u32 = 512;
const USER_SIG_FLASH_SIZE: u32 = 512;
const FLASH_LOCK_REGION_SIZE: u32 = 8192;
const FLASH_READ_SIZE: u32 = 4;
const FLASH_WRITE_SIZE: u32 = 4;

struct FlashParameters {
    gpnvm_num_max: u8,
    flash0_addr: u32,
    flash0_size: u32,
    #[cfg(feature = "atsam4sd")]
    flash1_addr: u32,
    #[cfg(feature = "atsam4sd")]
    flash1_size: u32,
}

#[cfg(any(feature = "atsam4e8c", feature = "atsam4e8e"))]
const FLASH_PARAMS: FlashParameters = FlashParameters {
    gpnvm_num_max: 2,
    flash0_addr: 0x00400000,
    flash0_size: 0x00080000,
};

#[cfg(any(feature = "atsam4e16c", feature = "atsam4e16e"))]
const FLASH_PARAMS: FlashParameters = FlashParameters {
    gpnvm_num_max: 2,
    flash0_addr: 0x00400000,
    flash0_size: 0x00100000,
};

#[cfg(any(feature = "atsam4n8a", feature = "atsam4n8b", feature = "atsam4n8c"))]
const FLASH_PARAMS: FlashParameters = FlashParameters {
    gpnvm_num_max: 2,
    flash0_addr: 0x00400000,
    flash0_size: 0x00080000,
};

#[cfg(any(feature = "atsam4n16b", feature = "atsam4n16c"))]
const FLASH_PARAMS: FlashParameters = FlashParameters {
    gpnvm_num_max: 2,
    flash0_addr: 0x00400000,
    flash0_size: 0x00100000,
};

#[cfg(any(feature = "atsam4s2a", feature = "atsam4s2b", feature = "atsam4s2c"))]
const FLASH_PARAMS: FlashParameters = FlashParameters {
    gpnvm_num_max: 2,
    flash0_addr: 0x00400000,
    flash0_size: 0x00020000,
};

#[cfg(any(feature = "atsam4s4a", feature = "atsam4s4b", feature = "atsam4s4c"))]
const FLASH_PARAMS: FlashParameters = FlashParameters {
    gpnvm_num_max: 2,
    flash0_addr: 0x00400000,
    flash0_size: 0x00040000,
};

#[cfg(any(feature = "atsam4s8b", feature = "atsam4s8c"))]
const FLASH_PARAMS: FlashParameters = FlashParameters {
    gpnvm_num_max: 2,
    flash0_addr: 0x00400000,
    flash0_size: 0x0080000,
};

#[cfg(any(feature = "atsam4sa16b", feature = "atsam4sa16c"))]
const FLASH_PARAMS: FlashParameters = FlashParameters {
    gpnvm_num_max: 2,
    flash0_addr: 0x00400000,
    flash0_size: 0x00100000,
};

#[cfg(any(feature = "atsam4sd16b", feature = "atsam4sd16c"))]
const FLASH_PARAMS: FlashParameters = FlashParameters {
    gpnvm_num_max: 3,
    flash0_addr: 0x00400000,
    flash0_size: 0x00080000,
    flash1_addr: 0x00480000,
    flash1_size: 0x00080000,
};

#[cfg(any(feature = "atsam4sd32b", feature = "atsam4sd32c"))]
const FLASH_PARAMS: FlashParameters = FlashParameters {
    gpnvm_num_max: 3,
    flash0_addr: 0x00400000,
    flash0_size: 0x00100000,
    flash1_addr: 0x00500000,
    flash1_size: 0x00100000,
};

extern "C" {
    /// RAM Function needed for certain EFC accesses (unique id and signature section)
    /// It's currently not possible to do this with pure rust as there can be no flash accesses
    /// while this function is executing.
    /// See: https://github.com/rust-embedded/cortex-m-rt/issues/42#issuecomment-559061416
    ///
    /// Will always return 0 unless the input buf is null
    fn efc_perform_read_sequence(
        efc: *const u32,
        cmd_st: u32,
        cmd_sp: u32,
        buf: *mut u32,
        size: u32,
        flash_addr: *mut u32,
    ) -> u32;

    /// RAM Function alternative to the iap_function
    /// See 3.2.1.3 on how to use the iap_function (haven't had much success so far on atsam4s).
    /// <http://ww1.microchip.com/downloads/en/AppNotes/Atmel-42141-SAM-AT02333-Safe-and-Secure-Bootloader-Implementation-for-SAM3-4_Application-Note.pdf>
    ///
    /// Returns the status of the transfer
    fn efc_perform_fcr(efc: *const u32, fcr: u32) -> u32;
}

/// Interface to an EFC instance
///
/// Partial Programming
/// - Must be done using 32-bit (or higher) boundaries
/// - 8 or 16-bit boundaries must be filled with 0xFF (full 32-bits must be written to the buffer)
/// - See Section 19.4.3.2
/// <https://ww1.microchip.com/downloads/en/DeviceDoc/Atmel-11158-32-bit%20Cortex-M4-Microcontroller-SAM4N16-SAM4N8_Datasheet.pdf>
///
/// Example memory.x configuration (atsam4s8b)
/// ```
/// MEMORY
/// {
///   FLASH (rx) : ORIGIN = 0x00400000, LENGTH = 512K
///   RAM (xrw)  : ORIGIN = 0x20000000, LENGTH = 128K
///   CS0 (xrw)  : ORIGIN = 0x60000000, LENGTH = 16M
///   CS1 (xrw)  : ORIGIN = 0x61000000, LENGTH = 16M
///   CS2 (xrw)  : ORIGIN = 0x62000000, LENGTH = 16M
///   CS3 (xrw)  : ORIGIN = 0x63000000, LENGTH = 16M
/// }
///
/// _flash = ORIGIN(FLASH);
/// ```
///
/// ```rust
/// // 512K flash (unfortunately we need this at compile-time, not link time)
/// const FLASH_CONFIG_SIZE: usize = 524288 / core::mem::size_of::<u32>();
/// extern "C" {
///     #[link_name = "_flash"]
///     static mut FLASH_CONFIG: [u32; FLASH_CONFIG_SIZE];
/// }
///
/// use hal::efc::Efc;
/// use atsam4_hal::pac::Peripherals;
///
/// let peripherals = Peripherals::take().unwrap();
/// // Clock configuration will also do a small bit of the EFC init
/// let _clocks = ClockController::new(
///     peripherals.PMC,
///     &peripherals.SUPC,
///     &peripherals.EFC0,
///     MainClock::Crystal12Mhz,
///     SlowClock::RcOscillator32Khz,
/// );
///
/// // Setup efc driver
/// // FLASH_CONFIG indicates where the usable flash starts
/// let mut efc = Efc::new(cx.device.EFC0, unsafe { &mut FLASH_CONFIG });
///
/// // Retrieve the uid from the efc
/// let uid = efc.read_unique_id().unwrap();
///
/// // Erase user signature
/// efc.erase_user_signature().unwrap();
///
/// // Write to the user signature (max 512 bytes)
/// efc.write_user_signature(&[1,2,3]).unwrap();
///
/// // Read back the user signatfure
/// let mut sig: [u32; 3];
/// efc.read_user_signature(&mut sig, sig.len()).unwrap();
/// ```
pub struct Efc {
    #[cfg(any(feature = "atsam4e", feature = "atsam4n", feature = "atsam4s"))]
    efc: EFC,
    #[cfg(feature = "atsam4sd")]
    efc1: EFC1,
    storage: &'static mut [u32],
}

impl Efc {
    /// Takes ownership of the peripheral and storage area
    #[cfg(any(
        feature = "atsam4e",
        feature = "atsam4n",
        feature = "atsam4s_",
        feature = "atsam4sa"
    ))]
    pub fn new(efc: EFC, storage: &'static mut [u32]) -> Efc {
        Self { efc, storage }
    }

    /// Takes ownership of the peripheral and storage area
    #[cfg(feature = "atsam4sd")]
    pub fn new(efc0: EFC, efc1: EFC1, storage: &'static mut [u32]) -> Efc {
        Self {
            efc: efc0,
            efc1,
            storage,
        }
    }

    /// Consumes `self` and returns back the raw peripheral and associated storage
    #[cfg(any(feature = "atsam4e", feature = "atsam4n", feature = "atsam4s_"))]
    pub fn free(self) -> (EFC, &'static mut [u32]) {
        (self.efc, self.storage)
    }

    /// Consumes `self` and returns back the raw peripheral and associated storage
    #[cfg(feature = "atsam4sd")]
    pub fn free(self) -> (EFC, EFC1, &'static mut [u32]) {
        (self.efc, self.efc1, self.storage)
    }

    #[inline]
    fn wait_ready(&self) {
        while !self.efc.fsr.read().frdy().bit() {}
    }

    /// Translate the given flash address to page and offset values
    /// Returns: (page, offset, bank)
    #[cfg(feature = "atsam4sd")]
    fn translate_address(&self, address: u32) -> Result<(u16, u16, u8), EfcError> {
        if address < FLASH_PARAMS.flash0_addr
            || address > FLASH_PARAMS.flash1_addr + FLASH_PARAMS.flash1_size
        {
            return Err(EfcError::AddressBoundsError);
        }

        // Check if the bank swap gpnvm bit is set
        let gpnvm2 = self.is_gpnvm_set(2)?;
        if address >= FLASH_PARAMS.flash1_addr {
            let bank = if gpnvm2 { 0 } else { 1 }; // Swap banks
            let page = (address - FLASH_PARAMS.flash1_addr) / FLASH_PAGE_SIZE;
            let offset = (address - FLASH_PARAMS.flash1_addr) % FLASH_PAGE_SIZE;
            Ok((page as u16, offset as u16, bank))
        } else {
            let bank = u8::from(gpnvm2);
            let page = (address - FLASH_PARAMS.flash0_addr) / FLASH_PAGE_SIZE;
            let offset = (address - FLASH_PARAMS.flash0_addr) % FLASH_PAGE_SIZE;
            Ok((page as u16, offset as u16, bank))
        }
    }

    /// Translate the given flash address to page and offset values
    /// Returns: (page, offset, bank)
    #[cfg(not(feature = "atsam4sd"))]
    fn translate_address(&self, address: u32) -> Result<(u16, u16, u8), EfcError> {
        if address < FLASH_PARAMS.flash0_addr
            || address > FLASH_PARAMS.flash0_addr + FLASH_PARAMS.flash0_size
        {
            return Err(EfcError::AddressBoundsError);
        }

        let page = (address - FLASH_PARAMS.flash0_addr) / FLASH_PAGE_SIZE;
        let offset = (address - FLASH_PARAMS.flash0_addr) % FLASH_PAGE_SIZE;
        Ok((page as u16, offset as u16, 0))
    }

    /* XXX (HaaTa): Wasn't needed? Probably can just remove this
    /// Compute the address of a flash by the given page and offset
    #[cfg(feature = "atsam4sd")]
    fn compute_address(&self, bank: u8, page: u16, offset: u16) -> Result<u32, EfcError> {
        // Check if the bank swap gpnvm bit is set
        let gpnvm2 = self.is_gpnvm_set(2)?;

        // Determine the address
        Ok(if bank == 0 {
            if gpnvm2 {
                FLASH_PARAMS.flash1_addr + page * FLASH_PAGE_SIZE + offset
            } else {
                FLASH_PARAMS.flash0_addr + page * FLASH_PAGE_SIZE + offset
            }
        } else {
            if gpnvm2 {
                FLASH_PARAMS.flash0_addr + page * FLASH_PAGE_SIZE + offset
            } else {
                FLASH_PARAMS.flash1_addr + page * FLASH_PAGE_SIZE + offset
            }
        })
    }

    /// Compute the address of a flash by the given page and offset
    #[cfg(not(feature = "atsam4sd"))]
    fn compute_address(&self, _bank: u8, page: u16, offset: u16) -> Result<u32, EfcError> {
        Ok(FLASH_PARAMS.flash0_addr + page as u32 * FLASH_PAGE_SIZE + offset as u32)
    }
    */

    /// Compute the lock range associated with the given address range
    /// Returns: (actual_start, actual_end)
    fn compute_lock_range(&self, start: u32, end: u32) -> (u32, u32) {
        let actual_start = start - (start % FLASH_LOCK_REGION_SIZE);
        let actual_end = end - (end % FLASH_LOCK_REGION_SIZE) + FLASH_LOCK_REGION_SIZE - 1;
        (actual_start, actual_end)
    }

    /// Lock all the regions in the given address range.
    /// The actual lock range is reported through two output parameters.
    /// Returns: (actual_start, actual_end)
    pub fn lock(&self, start: u32, end: u32) -> Result<(u32, u32), EfcError> {
        let num_pages_in_region = (FLASH_LOCK_REGION_SIZE / FLASH_PAGE_SIZE) as u16;

        // Compute actual lock range
        let (actual_start, actual_end) = self.compute_lock_range(start, end);

        // Determine page numbers
        let (mut start_page, _, bank) = self.translate_address(actual_start)?;
        let (_, end_page, _) = self.translate_address(actual_end)?;

        // Lock computed pages
        while start_page < end_page {
            self.efc_perform_command(bank, efc::fcr::FCMD_AW::SLB, start_page)?;
            start_page += num_pages_in_region;
        }

        Ok((actual_start, actual_end))
    }

    /// Unlock all the regions in the given address range.
    /// The actual unlock range is reported through two output parameters.
    pub fn unlock(&self, start: u32, end: u32) -> Result<(u32, u32), EfcError> {
        let num_pages_in_region = (FLASH_LOCK_REGION_SIZE / FLASH_PAGE_SIZE) as u16;

        // Compute actual unlock range
        let (actual_start, actual_end) = self.compute_lock_range(start, end);

        // Determine page numbers
        let (mut start_page, _, bank) = self.translate_address(actual_start)?;
        let (_, end_page, _) = self.translate_address(actual_end)?;

        // Unlock computed pages
        while start_page < end_page {
            self.efc_perform_command(bank, efc::fcr::FCMD_AW::CLB, start_page)?;
            start_page += num_pages_in_region;
        }

        Ok((actual_start, actual_end))
    }

    /// Get the number of locked regions inside the given address range.
    pub fn is_locked(&self, start: u32, end: u32) -> Result<u32, EfcError> {
        if end < start
            || start < FLASH_PARAMS.flash0_addr
            || end > FLASH_PARAMS.flash0_addr + FLASH_PARAMS.flash0_size
        {
            return Err(EfcError::AddressBoundsError);
        }

        // Compute page numbers
        let (start_page, _, bank) = self.translate_address(start)?;
        let (_, end_page, _) = self.translate_address(end)?;

        // Compute region numbers
        let num_pages_in_region = FLASH_LOCK_REGION_SIZE / FLASH_PAGE_SIZE;
        let start_region = start_page as u32 / num_pages_in_region;
        let end_region = end_page as u32 / num_pages_in_region;

        // Retrieve lock status
        self.efc_perform_command(bank, efc::fcr::FCMD_AW::GLB, 0)?;

        // Skip unrequested regions (if necessary)
        let mut count = 0;
        let mut status = self.efc_get_result(bank);
        while count <= start_region && start_region < count + 32 {
            status = self.efc_get_result(bank);
            count += 32;
        }

        let mut bit = start_region - count;
        count = end_region - start_region + 1;
        let mut num_locked_regions = 0;

        while count > 0 {
            if status & (1 << bit) != 0 {
                num_locked_regions += 1;
            }

            count -= 1;
            bit += 1;
            if bit == 32 {
                status = self.efc_get_result(bank);
                bit = 0;
            }
        }

        Ok(num_locked_regions)
    }

    /// Set the given GPNVM bit
    pub fn set_gpnvm(&self, gpnvm: u8) -> Result<(), EfcError> {
        // Make sure this is a valid gpnvm bit
        if gpnvm >= FLASH_PARAMS.gpnvm_num_max {
            return Err(EfcError::InvalidGpnvmBitError);
        }

        // Check to see if the bit is already set
        if self.is_gpnvm_set(gpnvm)? {
            return Ok(());
        }

        // Attempt to set the bit
        self.efc_perform_command(0, efc::fcr::FCMD_AW::SGPB, gpnvm as u16)
    }

    /// Clear the given GPNVM bit
    pub fn clear_gpnvm(&self, gpnvm: u8) -> Result<(), EfcError> {
        // Make sure this is a valid gpnvm bit
        if gpnvm >= FLASH_PARAMS.gpnvm_num_max {
            return Err(EfcError::InvalidGpnvmBitError);
        }

        // Check to see if the bit is already clear
        if !self.is_gpnvm_set(gpnvm)? {
            return Ok(());
        }

        // Attempt to set the bit
        self.efc_perform_command(0, efc::fcr::FCMD_AW::CGPB, gpnvm as u16)
    }

    /// Check if the given GPNVM bit is set or not
    pub fn is_gpnvm_set(&self, gpnvm: u8) -> Result<bool, EfcError> {
        // Make sure this is a valid gpnvm bit
        if gpnvm >= FLASH_PARAMS.gpnvm_num_max {
            return Err(EfcError::InvalidGpnvmBitError);
        }

        // Retrieve bit status
        self.efc_perform_command(0, efc::fcr::FCMD_AW::GGPB, gpnvm as u16)?;
        let gpnvm_bits = self.efc_get_result(0);

        // Check bit
        Ok(gpnvm_bits & (1 << gpnvm) != 0)
    }

    /// Set security bit
    pub fn enable_security_bit(&self) -> Result<(), EfcError> {
        self.set_gpnvm(0)
    }

    /// Check if security bit is enabled
    pub fn is_security_bit_enabled(&self) -> Result<bool, EfcError> {
        self.is_gpnvm_set(0)
    }

    /// Read the flash unique ID
    pub fn read_unique_id(&self) -> Result<[u32; 4], EfcError> {
        // Read into uid
        let mut uid: [u32; 4] = [0; 4];
        self.efc_perform_read_sequence(
            0,
            efc::fcr::FCMD_AW::STUI,
            efc::fcr::FCMD_AW::SPUI,
            &mut uid,
            4,
        )?;

        // Prepare id as an arry of 32-bit values
        Ok(uid)
    }

    /// Read the flash user signature
    pub fn read_user_signature(&self, data: &mut [u32], len: usize) -> Result<(), EfcError> {
        // Make sure we're only reading at most 512 bytes
        if len > USER_SIG_FLASH_SIZE as usize / core::mem::size_of::<u32>() {
            return Err(EfcError::InvalidUserSignatureSizeError);
        }

        // Read user signature into the buffer
        self.efc_perform_read_sequence(
            0,
            efc::fcr::FCMD_AW::STUS,
            efc::fcr::FCMD_AW::SPUS,
            data,
            len,
        )
    }

    /// Write the flash user signature
    pub fn write_user_signature(&mut self, data: &[u32]) -> Result<(), EfcError> {
        // Make sure the signature does not exceed the max size
        if data.len() > USER_SIG_FLASH_SIZE as usize / core::mem::size_of::<u32>() {
            return Err(EfcError::InvalidUserSignatureSizeError);
        }

        // Must write signature in chunks of 32-bits (does not support 8 or 16-bit writes)
        self.storage[..data.len()].clone_from_slice(data);

        // Send the write signature command
        self.efc_perform_command(0, efc::fcr::FCMD_AW::WUS, 0)
    }

    /// Erase the flash user signature
    pub fn erase_user_signature(&self) -> Result<(), EfcError> {
        self.efc_perform_command(0, efc::fcr::FCMD_AW::EUS, 0)
    }

    /// Get result of last executed EFC command
    #[cfg(not(feature = "atsam4sd"))]
    fn efc_get_result(&self, _bank: u8) -> u32 {
        self.efc.frr.read().fvalue().bits()
    }

    /// Get result of last executed EFC command
    #[cfg(feature = "atsam4sd")]
    fn efc_get_result(&self, bank: u8) -> u32 {
        if bank == 0 {
            self.efc.frr.read().fvalue().bits()
        } else {
            self.efc1.frr.read().fvalue().bits()
        }
    }

    /// Perform the given command and wait until its completion (or an error).
    ///
    /// NOTE: Unique ID commands are not supported, use efc_perform_read_sequence.
    /// NOTE: This function uses the IAP function (which is contained in ROM)
    fn efc_perform_command(
        &self,
        bank: u8,
        command: efc::fcr::FCMD_AW,
        argument: u16,
    ) -> Result<(), EfcError> {
        // Unique ID commands are not supported
        match command {
            efc::fcr::FCMD_AW::STUI | efc::fcr::FCMD_AW::SPUI => {
                return Err(EfcError::UnsupportedCommandError);
            }
            _ => {}
        }

        self.efc_fcr_command(bank, command, argument)
    }

    /// Convenience function to handle the IAP function
    ///
    /// NOTE: This function uses a RAM function written in C.
    fn efc_fcr_command(
        &self,
        bank: u8,
        command: efc::fcr::FCMD_AW,
        argument: u16,
    ) -> Result<(), EfcError> {
        // Build command for efc_perform_fcr (or possibly the iap_function)
        let fcr_cmd: u32 = ((efc::fcr::FKEY_AW::PASSWD as u32) << 24)
            | ((argument as u32) << 8)
            | (command as u32);

        // Select the flash bank
        #[cfg(not(feature = "atsam4sd"))]
        let efc_ptr = {
            let _ = bank;
            EFC::PTR as *const _
        };
        #[cfg(feature = "atsam4sd")]
        let efc_ptr = if bank == 0 {
            EFC::PTR as *const _
        } else if bank == 1 {
            EFC1::PTR as *const _
        } else {
            return Err(EfcError::InvalidFlashBank);
        };

        // Force processor to flush any pending flash transactions
        cortex_m::asm::dsb();
        cortex_m::asm::isb();

        // Call RAM function
        let status = interrupt::free(|_| unsafe { efc_perform_fcr(efc_ptr, fcr_cmd) });

        // Check for a command error
        if status & (1 << 1) != 0 {
            Err(EfcError::CommandError)
        } else if status & (1 << 2) != 0 {
            Err(EfcError::LockError)
        } else if status & (1 << 3) != 0 {
            Err(EfcError::FlashError)
        } else {
            // Success (though the write may not have fully finished if bit 0 is not set)
            Ok(())
        }
    }

    /// Perform read sequence
    /// Supported sequences are read Unique ID and read User Signature
    ///
    /// NOTE: This function uses a RAM function written in C.
    fn efc_perform_read_sequence(
        &self,
        bank: u8,
        start_cmd: efc::fcr::FCMD_AW,
        stop_cmd: efc::fcr::FCMD_AW,
        bytes: &mut [u32],
        len: usize,
    ) -> Result<(), EfcError> {
        // Check incoming buffer size
        if bytes.len() < len {
            return Err(EfcError::InvalidBufferSizeError);
        }

        // Run RAM function version of the command as we cannot read from flash for any reason
        // until the EEFC mode sequence has finished.
        #[cfg(not(feature = "atsam4sd"))]
        let status = {
            let _ = bank;
            unsafe {
                efc_perform_read_sequence(
                    EFC::PTR as *const _,
                    start_cmd as u32,
                    stop_cmd as u32,
                    bytes.as_mut_ptr(),
                    len as u32,
                    FLASH_PARAMS.flash0_addr as *mut _,
                )
            }
        };
        #[cfg(feature = "atsam4sd")]
        let status = {
            unsafe {
                if bank == 0 {
                    efc_perform_read_sequence(
                        EFC::PTR as *const _,
                        start_cmd as u32,
                        stop_cmd as u32,
                        bytes.as_mut_ptr(),
                        len as u32,
                        FLASH_PARAMS.flash0_addr as *mut _,
                    )
                } else if bank == 1 {
                    efc_perform_read_sequence(
                        EFC1::PTR as *const _,
                        start_cmd as u32,
                        stop_cmd as u32,
                        bytes.as_mut_ptr(),
                        len as u32,
                        FLASH_PARAMS.flash1_addr as *mut _,
                    )
                } else {
                    return Err(EfcError::InvalidFlashBank);
                }
            }
        };

        if status != 0 {
            // The only possible error is a null pointer check for bytes buffer
            Err(EfcError::InvalidBufferSizeError)
        } else {
            Ok(())
        }
    }
}

impl ErrorType for Efc {
    type Error = EfcError;
}

impl ReadNorFlash for Efc {
    const READ_SIZE: usize = FLASH_READ_SIZE as usize;

    /// Reads from atsam4 internal flash
    ///
    /// NOTE: EEFC does not have a requirement that reads must start from an
    ///       aligned address. However we're imposing this restriction due to:
    ///       1. Reads are faster if they are aligned
    ///       2. Less complicated logic
    ///       3. You shouldn't really be using this for unaligned reads anyways
    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        let offset = offset as usize;
        let bytes_len = bytes.len();
        let read_len = bytes_len + (Self::READ_SIZE - (bytes_len % Self::READ_SIZE));
        let target_offset = offset + read_len;
        if offset % Self::READ_SIZE == 0 && target_offset <= self.capacity() {
            self.wait_ready();
            let last_offset = target_offset - Self::READ_SIZE;
            for offset in (offset..last_offset).step_by(Self::READ_SIZE) {
                let word = self.storage[offset >> 2];
                bytes[offset] = (word >> 24) as u8;
                bytes[offset + 1] = (word >> 16) as u8;
                bytes[offset + 2] = (word >> 8) as u8;
                bytes[offset + 3] = (word) as u8;
            }
            let offset = last_offset;
            let word = self.storage[offset >> 2];
            let mut bytes_offset = offset;
            if bytes_offset < bytes_len {
                bytes[bytes_offset] = (word >> 24) as u8;
                bytes_offset += 1;
                if bytes_offset < bytes_len {
                    bytes[bytes_offset] = (word >> 16) as u8;
                    bytes_offset += 1;
                    if bytes_offset < bytes_len {
                        bytes[bytes_offset] = (word >> 8) as u8;
                        bytes_offset += 1;
                        if bytes_offset < bytes_len {
                            bytes[bytes_offset] = (word) as u8;
                        }
                    }
                }
            }
            Ok(())
        } else {
            Err(EfcError::Unaligned)
        }
    }

    #[cfg(not(feature = "atsam4sd"))]
    fn capacity(&self) -> usize {
        FLASH_PARAMS.flash0_size as usize
    }

    #[cfg(feature = "atsam4sd")]
    fn capacity(&self) -> usize {
        (FLASH_PARAMS.flash0_size + FLASH_PARAMS.flash1_size) as usize
    }
}

impl NorFlash for Efc {
    /// 32-bits is the smallest write size
    /// If you'd like to write smaller amounts, you must pad the rest of the 4 bytes
    /// with 0xFFs
    const WRITE_SIZE: usize = FLASH_WRITE_SIZE as usize;

    /// NOTE: We can optimize erase quite a bit by trying to combine multiple erase bounds
    ///       e.g. pages then sectors then pages
    ///
    /// The actual erase will vary depending on the situation
    /// 4 pages  (* 512 ->  2048) - (EPA) Only for 8KB sectors
    /// 8 pages  (* 512 ->  4096) - (EPA) Can be done anywhere
    /// 16 pages (* 512 ->  8192) - (EPA) Can be done anywhere
    /// 32 pages (* 512 -> 16384) - (EPA) Not valid for 8KB sectors
    /// 1 sector                  - (ES) Size depends on which sector
    ///   - Sector 0   (8192)
    ///   - Sector 1   (8192)
    ///   - Sector 2  (49152)
    ///   - Sector 3+ (65536)
    ///   See 2.3.1 for more details
    ///   <http://ww1.microchip.com/downloads/en/Appnotes/Atmel-42218-EEPROM-Emulation-Using-Internal-Flash-SAM4_AT4066_AP-Note.pdf>
    /// All pages                 - For a flash bank (for chips with dual bank flashes)
    ///
    /// If your chip has two banks, you must call erase twice to erase both banks.
    ///
    /// Setting the smallest safe interval as the "default"
    const ERASE_SIZE: usize = 8 * FLASH_PAGE_SIZE as usize;

    /// Erases range of addresses
    /// Will not succeed if the erase bounds are not set to an allowed boundary.
    /// * page
    /// * sector
    /// * bank
    fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        // Nothing to do
        if from == to {
            return Ok(());
        }

        // From must be smaller than to
        if from > to {
            return Err(EfcError::AddressBoundsError);
        }

        // Make sure we're within the address bounds
        if to >= FLASH_PARAMS.flash0_size {
            return Err(EfcError::AddressBoundsError);
        }

        // Check if erasing entire flash, or entire bank
        if from == 0 && to == FLASH_PARAMS.flash0_size {
            return self.efc_perform_command(0, efc::fcr::FCMD_AW::EA, 0);
        }
        #[cfg(feature = "atsam4sd")]
        if from == FLASH_PARAMS.flash1_addr && to == FLASH_PARAMS.flash1_size {
            return self.efc_perform_command(1, efc::fcr::FCMD_AW::EA, 0);
        }
        #[cfg(feature = "atsam4sd")]
        if from == 0 && to == FLASH_PARAMS.flash0_size + FLASH_PARAMS.flash1_size {
            self.efc_perform_command(0, efc::fcr::FCMD_AW::EA, 0)?;
            return self.efc_perform_command(1, efc::fcr::FCMD_AW::EA, 0);
        }

        // Flash must be a multiple of self::ERASE_SIZE
        // TODO: Optimization: 8 kB sectors can have a smaller erase size
        if from % Self::ERASE_SIZE as u32 != 0 || to % Self::ERASE_SIZE as u32 != 0 {
            return Err(EfcError::NotWithinFlashPageBoundsError);
        }

        // TODO: Optimization: Check if 16 kB of pages can be erased (all sectors) or 32 pages can
        //       erased (64 kB sectors)
        // TODO: Optimization: Check if entire sector can be erased
        for address in (from..to).step_by(Self::ERASE_SIZE) {
            // Determine page FARG[15:2] and bank
            // No shifting on page is needed as the page must be a multiple of 4, 8, 16 or 32
            let (page, _, bank) = self.translate_address(address)?;

            // Specifies number of pages to erase FARG[0:1]
            // 0 - 4 pages (only valid on small 8 kB sectors)
            // 1 - 8 pages
            // 2 - 16 pages
            // 3 - 32 pages (not valid on small 16 kB sectors)
            let farg = 1;

            self.efc_perform_command(bank, efc::fcr::FCMD_AW::EPA, farg | page)?;
        }

        Ok(())
    }

    /// Write a data buffer on flash.
    ///
    /// This function works in polling mode, and thus only returns when the
    /// data has been effectively written.
    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        // Make sure write is aligned
        let offset = offset as usize;
        if offset % Self::WRITE_SIZE == 0 && bytes.len() % Self::WRITE_SIZE == 0 {
            // Check to make sure we're not trying to write over the size of one bank
            if bytes.len() + offset > FLASH_PARAMS.flash0_size as usize {
                return Err(EfcError::AddressBoundsError);
            }

            // Write 32-bits at a time into the latched write buffer
            for offset in (offset..(offset + bytes.len())).step_by(Self::WRITE_SIZE) {
                let word = ((bytes[offset] as u32) << 24)
                    | ((bytes[offset + 1] as u32) << 16)
                    | ((bytes[offset + 2] as u32) << 8)
                    | (bytes[offset + 3] as u32);

                // Write word to flash location
                self.storage[offset >> 2] = word;

                // Commit write to flash on page boundaries or on the last partial write
                if (offset + Self::WRITE_SIZE) % FLASH_PAGE_SIZE as usize == 0
                    || offset + Self::WRITE_SIZE == offset + bytes.len()
                {
                    // Translate address to page and offset
                    let (page, _, bank) =
                        self.translate_address(FLASH_PARAMS.flash0_addr + offset as u32)?;

                    self.efc_perform_command(bank, efc::fcr::FCMD_AW::WP, page)?;
                }
            }

            Ok(())
        } else {
            Err(EfcError::Unaligned)
        }
    }
}

#[derive(Debug, defmt::Format)]
pub enum EfcError {
    /// An operation was attempted on an unaligned boundary
    Unaligned,
    /// Bad keyword has been written to the EEFC_FCR register
    CommandError,
    /// Attempted write to a locked page, must be unlocked first to succeed
    LockError,
    /// WriteVerify test of flash memory has failed (possibly EraseVerify)
    FlashError,
    /// Address outside of flash region bounds
    AddressBoundsError,
    /// Unsupported command FMD key for given function
    UnsupportedCommandError,
    /// Invalid GPNVM bit
    InvalidGpnvmBitError,
    /// Invalid buffer sizef
    InvalidBufferSizeError,
    /// Invalid UserSignature size
    InvalidUserSignatureSizeError,
    /// Not within page bounds
    NotWithinFlashPageBoundsError,
    /// Invalid flash bank
    InvalidFlashBank,
}

impl NorFlashError for EfcError {
    fn kind(&self) -> NorFlashErrorKind {
        match self {
            EfcError::AddressBoundsError => NorFlashErrorKind::OutOfBounds,
            EfcError::CommandError => NorFlashErrorKind::Other,
            EfcError::FlashError => NorFlashErrorKind::Other,
            EfcError::InvalidBufferSizeError => NorFlashErrorKind::Other,
            EfcError::InvalidFlashBank => NorFlashErrorKind::Other,
            EfcError::InvalidGpnvmBitError => NorFlashErrorKind::Other,
            EfcError::InvalidUserSignatureSizeError => NorFlashErrorKind::Other,
            EfcError::LockError => NorFlashErrorKind::Other,
            EfcError::NotWithinFlashPageBoundsError => NorFlashErrorKind::Other,
            EfcError::Unaligned => NorFlashErrorKind::NotAligned,
            EfcError::UnsupportedCommandError => NorFlashErrorKind::Other,
        }
    }
}
