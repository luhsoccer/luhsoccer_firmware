# C Extensions for atsam4-hal

In some minimal cases, it's necessary to use C in order to do things that is not currently possible in Rust.
`build.bash` will compile the C code into the necessary static libraries that can be used during the build process.
The intent is that the static libraries are checked into git so that an embedded C compiler is not needed during a normal build.


## Functions

- efc_perform_read_sequence()
  * Must be run entirely from RAM as the flash controller cannot read processor instructions from flash while accessing special
    regions of EEFC (Enhanced Embedded Flash Controller).
    The chip will crash immediately if you attempt to read instructions from flash during this mode.
- efc_perform_fcr()
  * Alternate to the IAP function that runs in RAM instead of ROM


## Re-Building

**Dependencies**
- arm-none-eabi-gcc (Arch Linux: arm-none-eabi-gcc)
- arm-none-eabi-ar (Arch Linux: arm-none-eabi-binutils)

```bash
./build.bash
```
