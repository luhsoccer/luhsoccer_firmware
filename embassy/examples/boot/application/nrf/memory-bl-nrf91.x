MEMORY
{
  /* NOTE 1 K = 1 KiBi = 1024 bytes */
  /* Assumes Secure Partition Manager (SPM) flashed at the start */
  FLASH                             : ORIGIN = 0x00050000, LENGTH = 24K
  BOOTLOADER_STATE                  : ORIGIN = 0x00056000, LENGTH = 4K
  ACTIVE                            : ORIGIN = 0x00057000, LENGTH = 64K
  DFU                               : ORIGIN = 0x00067000, LENGTH = 68K
  RAM                         (rwx) : ORIGIN = 0x20018000, LENGTH = 32K
}

__bootloader_state_start = ORIGIN(BOOTLOADER_STATE);
__bootloader_state_end = ORIGIN(BOOTLOADER_STATE) + LENGTH(BOOTLOADER_STATE);

__bootloader_active_start = ORIGIN(ACTIVE);
__bootloader_active_end = ORIGIN(ACTIVE) + LENGTH(ACTIVE);

__bootloader_dfu_start = ORIGIN(DFU);
__bootloader_dfu_end = ORIGIN(DFU) + LENGTH(DFU);
