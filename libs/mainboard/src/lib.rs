#![no_std]

extern crate cortex_m_rt;

/// The linker will place this boot block at the start of out program image.
/// We need this to help the ROM bootloader get our code up and running
#[link_section = ".boot2"]
#[no_mangle]
#[used]
static BOOT2_FIRMWARE: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

macro_rules! bsp_impl {
    () => {
        pub use rp2040_hal::{self as hal, entry, pac};
        pub const XOSC_CRYSTAL_FREQ: u32 = 12_000_000;

        use hal::{
            adc::Adc,
            clocks::{init_clocks_and_plls, ClocksManager},
            pwm::Slices,
            sio::{spinlock_reset, SioFifo},
            spi::{Disabled, Spi},
            timer::{monotonic::Monotonic, Alarm0},
            Sio, Timer, Watchdog,
        };
        use pac::{
            I2C0, I2C1, PIO0, PIO1, PPB, PSM, RESETS, SPI0, SPI1, UART0, UART1, USBCTRL_DPRAM,
            USBCTRL_REGS,
        };

        pub struct Device {
            pub resets: RESETS,
            pub watchdog: Watchdog,
            pub clocks: ClocksManager,
            pub monotonic: Monotonic<Alarm0>,
            pub pins: Pins,
            pub pio0: PIO0,
            pub pio1: PIO1,
            pub spi0: Spi<Disabled, SPI0, 8>,
            pub spi1: Spi<Disabled, SPI1, 8>,
            pub adc: Adc,
            pub pwm: Slices,
            pub i2c0: I2C0,
            pub i2c1: I2C1,
            pub uart0: UART0,
            pub uart1: UART1,
            pub usb: (USBCTRL_REGS, USBCTRL_DPRAM),
            pub psm: PSM,
            pub ppb: PPB,
            pub fifo: SioFifo,
        }

        /// Initialize the RP2040 for use with rtic
        //#[must_use]
        #[must_use]
        pub fn init_rtic(device: pac::Peripherals) -> Device {
            // # Safety
            //
            // Soft-reset does not release the hardware spinlocks. Release them now to avoid a deadlock
            // after debug or watchdog reset.
            unsafe {
                spinlock_reset();
            }

            let mut resets = device.RESETS;
            let mut watchdog = Watchdog::new(device.WATCHDOG);
            let clocks = init_clocks_and_plls(
                XOSC_CRYSTAL_FREQ,
                device.XOSC,
                device.CLOCKS,
                device.PLL_SYS,
                device.PLL_USB,
                &mut resets,
                &mut watchdog,
            )
            .ok()
            .expect("The hardware should not allow this to fail");

            let mut timer = Timer::new(device.TIMER, &mut resets);
            let alarm = timer
                .alarm_0()
                .expect("the timer was created just now, so alarm 0 could not be set already");
            let monotonic = Monotonic::new(timer, alarm);

            let sio = Sio::new(device.SIO);
            let pins = Pins::new(
                device.IO_BANK0,
                device.PADS_BANK0,
                sio.gpio_bank0,
                &mut resets,
            );

            let pio0 = device.PIO0;
            let pio1 = device.PIO1;

            let spi0: Spi<_, _, 8> = Spi::new(device.SPI0);
            let spi1: Spi<_, _, 8> = Spi::new(device.SPI1);

            let adc = Adc::new(device.ADC, &mut resets);

            let pwm = Slices::new(device.PWM, &mut resets);

            let i2c0 = device.I2C0;
            let i2c1 = device.I2C1;

            let uart0 = device.UART0;
            let uart1 = device.UART1;

            let usb = (device.USBCTRL_REGS, device.USBCTRL_DPRAM);

            let psm = device.PSM;
            let ppb = device.PPB;
            let fifo = sio.fifo;

            Device {
                resets,
                watchdog,
                clocks,
                monotonic,
                pins,
                pio0,
                pio1,
                spi0,
                spi1,
                adc,
                pwm,
                i2c0,
                i2c1,
                uart0,
                uart1,
                usb,
                psm,
                ppb,
                fifo,
            }
        }
    };
}

pub mod main {
    bsp_impl!();

    hal::bsp_pins!(
        Gpio0 {
            name: rf_crx,
            aliases: {
                PushPullOutput: RfCrx,
                FunctionPio0: RfCrxPio0,
                FunctionPio1: RfCrxPio1
            }
        },

        Gpio1 {
            name: rf_cps,
            aliases: {
                PushPullOutput: RfCps,
                FunctionPio0: RfCpsPio0,
                FunctionPio1: RfCpsPio1
            }
        },

        Gpio2 {
            name: rf_ctx,
            aliases: {
                PushPullOutput: RfCtx,
                FunctionPio0: RfCtxPio0,
                FunctionPio1: RfCtxPio1
            }
        },

        Gpio3 {
            name: rf_nreset,
            aliases: {
                PushPullOutput: RfNreset,
                FunctionPio0: RfNresetPio0,
                FunctionPio1: RfNresetPio1
            }
        },

        Gpio4 {
            name: rf_miso,
            aliases: {
                FunctionSpi: RfMiso,
                FunctionPio0: RfMisoPio0,
                FunctionPio1: RfMisoPio1
            }
        },

        Gpio5 {
            name: rf_ncs,
            aliases: {
                PushPullOutput: RfNcs,
                FunctionPio0: RfNcsPio0,
                FunctionPio1: RfNcsPio1
            }
        },

        Gpio6 {
            name: rf_sck,
            aliases: {
                FunctionSpi: RfSck,
                FunctionPio0: RfSckPio0,
                FunctionPio1: RfSckPio1
            }
        },

        Gpio7 {
            name: rf_mosi,
            aliases: {
                FunctionSpi: RfMosi,
                FunctionPio0: RfMosiPio0,
                FunctionPio1: RfMosiPio1
            }
        },

        Gpio8 {
            name: rf_busy,
            aliases: {
                FloatingInput: RfBusy,
                FunctionPio0: RfBusyPio0,
                FunctionPio1: RfBusyPio1
            }
        },

        Gpio9 {
            name: led,
            aliases: {
                FunctionPio0: LedPio0,
                FunctionPio1: LedPio1
            }
        },

        Gpio10 {
            name: ui_sda,
            aliases: {
                FunctionI2C: UiSda,
                FunctionPio0: UiSdaPio0,
                FunctionPio1: UiSdaPio1
            }
        },

        Gpio11 {
            name: ui_scl,
            aliases: {
                FunctionI2C: UiScl,
                FunctionPio0: UiSclPio0,
                FunctionPio1: UiSclPio1
            }
        },

        Gpio12 {
            name: nshutdown,
            aliases: {
                FunctionPio0: NshutdownPio0,
                FunctionPio1: NshutdownPio1
            }
        },

        Gpio13 {
            name: power_switch,
            aliases: {
                FunctionPio0: PowerSwitchPio0,
                FunctionPio1: PowerSwitchPio1
            }
        },

        Gpio14 {
            name: gpio14,
            aliases: {
                FunctionPwm: Gp14Pwm7A,
                FunctionPio0: Gp14Pio0,
                FunctionPio1: Gp14Pio1
            }
        },

        Gpio15 {
            name: ball_sense,
            aliases: {
                FunctionPio0: BallSensePio0,
                FunctionPio1: BallSensePio1
            }
        },

        Gpio16 {
            name: mot_tx,
            aliases: {
                FunctionUart: MotTx,
                FunctionPio0: MotTxPio0,
                FunctionPio1: MotTxPio1
            }
        },

        Gpio17 {
            name: mot_rx,
            aliases: {
                FunctionUart: MotRx,
                FunctionPio0: MotRxPio0,
                FunctionPio1: MotRxPio1
            }
        },

        Gpio18 {
            name: mot_cts,
            aliases: {
                FunctionUart: MotCts,
                FunctionPio0: MotCtsPio0,
                FunctionPio1: MotCtsPio1
            }
        },

        Gpio19 {
            name: mot_rts,
            aliases: {
                FunctionUart: MotRts,
                FunctionPio0: MotRtsPio0,
                FunctionPio1: MotRtsPio1
            }
        },

        Gpio20 {
            name: dribbler,
            aliases: {
                FunctionPwm: DribblerPwm,
                FunctionPio0: DribblerPio0,
                FunctionPio1: DribblerPio1
            }
        },

        Gpio21 {
            name: buzzer,
            aliases: {
                FunctionPwm: BuzzerPwm,
                FunctionPio0: BuzzerPio0,
                FunctionPio1: BuzzerPio1
            }
        },

        Gpio22 {
            name: imu_int2,
            aliases: {
                FunctionPio0: ImuInt2Pio0,
                FunctionPio1: ImuInt2Pio1
            }
        },

        Gpio23 {
            name: imu_int1,
            aliases: {
                FunctionPio0: ImuInt1Pio0,
                FunctionPio1: ImuInt1Pio1
            }
        },

        Gpio24 {
            name: imu_miso,
            aliases: {
                FunctionSpi: ImuMiso,
                FunctionPio0: ImuMisoPio0,
                FunctionPio1: ImuMisoPio1
            }
        },

        Gpio25 {
            name: imu_ncs,
            aliases: {
                FunctionSpi: ImuNcs,
                FunctionPio0: ImuNcsPio0,
                FunctionPio1: ImuNcsPio1
            }
        },

        Gpio26 {
            name: imu_sck,
            aliases: {
                FunctionSpi: ImuSck,
                FunctionPio0: ImuSckPio0,
                FunctionPio1: ImuSckPio1
            }
        },

        Gpio27 {
            name: imu_mosi,
            aliases: {
                FunctionSpi: ImuMosi,
                FunctionPio0: ImuMosiPio0,
                FunctionPio1: ImuMosiPio1
            }
        },

        Gpio28 {
            name: abat,
        },

        Gpio29 {
            name: vbat,
        },
    );
}

pub mod motor {
    bsp_impl!();

    hal::bsp_pins!(
        Gpio0 {
            name: trig2,
            aliases: {
                FunctionPio0: Trig2Pio0,
                FunctionPio1: Trig2Pio1
            }
        },

        Gpio1 {
            name: trig1,
            aliases: {
                FunctionPio0: Trig1Pio0,
                FunctionPio1: Trig1Pio1
            }
        },

        Gpio2 {
            name: dac0,
            aliases: {
                FunctionPio0: Dac0Pio0,
                FunctionPio1: Dac0Pio1
            }
        },

        Gpio3 {
            name: dac1,
            aliases: {
                FunctionPio0: Dac1Pio0,
                FunctionPio1: Dac1Pio1
            }
        },

        Gpio4 {
            name: dac2,
            aliases: {
                FunctionPio0: Dac2Pio0,
                FunctionPio1: Dac2Pio1
            }
        },

        Gpio5 {
            name: dac3,
            aliases: {
                FunctionPio0: Dac3Pio0,
                FunctionPio1: Dac3Pio1
            }
        },

        Gpio6 {
            name: dac4,
            aliases: {
                FunctionPio0: Dac4Pio0,
                FunctionPio1: Dac4Pio1
            }
        },

        Gpio7 {
            name: dac5,
            aliases: {
                FunctionPio0: Dac5Pio0,
                FunctionPio1: Dac5Pio1
            }
        },

        Gpio8 {
            name: dac6,
            aliases: {
                FunctionPio0: Dac6Pio0,
                FunctionPio1: Dac6Pio1
            }
        },

        Gpio9 {
            name: dac7,
            aliases: {
                FunctionPio0: Dac7Pio0,
                FunctionPio1: Dac7Pio1
            }
        },

        Gpio10 {
            name: dac8,
            aliases: {
                FunctionPio0: Dac8Pio0,
                FunctionPio1: Dac8Pio1
            }
        },

        Gpio11 {
            name: dac9,
            aliases: {
                FunctionPio0: Dac9Pio0,
                FunctionPio1: Dac9Pio1
            }
        },

        Gpio12 {
            name: kicker_nfault,
            aliases: {
                FunctionPio0: KickerNfaultPio0,
                FunctionPio1: KickerNfaultPio1
            }
        },

        Gpio13 {
            name: kicker_ndone,
            aliases: {
                FunctionPio0: KickerNdonePio0,
                FunctionPio1: KickerNdonePio1
            }
        },

        Gpio14 {
            name: kicker_clear,
            aliases: {
                FunctionPio0: KickerClearPio0,
                FunctionPio1: KickerClearPio1
            }
        },

        Gpio15 {
            name: kicker_charge,
            aliases: {
                FunctionPio0: KickerChargePio0,
                FunctionPio1: KickerChargePio1
            }
        },

        Gpio16 {
            name: main_tx,
            aliases: {
                FunctionUart: MainTx,
                FunctionPio0: MainTxPio0,
                FunctionPio1: MainTxPio1
            }
        },

        Gpio17 {
            name: main_rx,
            aliases: {
                FunctionUart: MainRx,
                FunctionPio0: MainRxPio0,
                FunctionPio1: MainRxPio1
            }
        },

        Gpio18 {
            name: main_cts,
            aliases: {
                FunctionUart: MainCts,
                FunctionPio0: MainCtsPio0,
                FunctionPio1: MainCtsPio1
            }
        },

        Gpio19 {
            name: main_rts,
            aliases: {
                FunctionUart: MainRts,
                FunctionPio0: MainRtsPio0,
                FunctionPio1: MainRtsPio1
            }
        },

        Gpio20 {
            name: gpio20,
            aliases: {
                FunctionPwm: Gp20Pwm,
                FunctionPio0: Gp20Pio0,
                FunctionPio1: Gp20Pio1
            }
        },

        Gpio21 {
            name: gpio21,
            aliases: {
                FunctionPwm: Gp21Pwm,
                FunctionPio0: Gp21Pio0,
                FunctionPio1: Gp21Pio1
            }
        },

        Gpio22 {
            name: tmc_ncs1,
            aliases: {
                FunctionPio0: TmcNcs1Pio0,
                FunctionPio1: TmcNcs1Pio1
            }
        },

        Gpio23 {
            name: tmc_ncs3,
            aliases: {
                FunctionPio0: TmcNcs3Pio0,
                FunctionPio1: TmcNcs3Pio1
            }
        },

        Gpio24 {
            name: tmc_ncs0,
            aliases: {
                FunctionPio0: TmcNcs0Pio0,
                FunctionPio1: TmcNcs0Pio1
            }
        },

        Gpio25 {
            name: tmc_ncs2,
            aliases: {
                FunctionPio0: TmcNcs2Pio0,
                FunctionPio1: TmcNcs2Pio1
            }
        },

        Gpio26 {
            name: tmc_sck,
            aliases: {
                FunctionSpi: TmcSck,
                FunctionPio0: TmcSckPio0,
                FunctionPio1: TmcSckPio1
            }
        },

        Gpio27 {
            name: tmc_mosi,
            aliases: {
                FunctionSpi: TmcMosi,
                FunctionPio0: TmcMosiPio0,
                FunctionPio1: TmcMosiPio1
            }
        },

        Gpio28 {
            name: tmc_miso,
            aliases: {
                FunctionSpi: TmcMiso,
                FunctionPio0: TmcMisoPio0,
                FunctionPio1: TmcMisoPio1
            }
        },

        Gpio29 {
            name: kicker_vcap,
        },
    );
}
