//#![deny(warnings)]
#![no_std]
#![no_main]

mod converter;
mod network;
mod rf;
mod robot_state;
mod status;
mod usb_serial;

use atsam4_hal as _;
use defmt_rtt as _;
use panic_probe as _;

#[global_allocator]
static HEAP: embedded_alloc::Heap = embedded_alloc::Heap::empty();

defmt::timestamp!("{=u64:us}", {
    app::monotonics::now().duration_since_epoch().to_micros()
});

#[rtic::app(device = atsam4_hal::pac, dispatchers = [AES, USART0, USART1, EFC])]
mod app {

    use crate::network;
    use crate::rf;
    use crate::robot_state;
    use crate::status::Status;
    use crate::usb_serial;

    use atsam4_hal as hal;
    use atsam4_hal::ethernet::{EthernetAddress, RxDescriptorTable, TxDescriptorTable};

    use defmt::info;
    use dwt_systick_monotonic::{fugit, DwtSystick};

    use embedded_hal::timer::CountDown;
    use fugit::{ExtU64, RateExtU32};
    use hal::ethernet::Controller;

    use hal::spi::SpiU8;
    use hal::{
        clock::{ClockController, MainClock, SlowClock},
        gpio::{GpioExt, Ports},
        spi::ChipSelectSettings,
        timer::TimerCounter,
        watchdog::Watchdog,
        watchdog::WatchdogDisable,
    };
    use network::Network;
    use robot_state::RobotState;
    use smart_leds::{SmartLedsWrite, RGB};
    use usb_serial::UsbSerial;

    use ws2812_timer_delay as ws2812;

    const MONO_HZ: u32 = 120_000_000;
    #[monotonic(binds = SysTick, default = true, priority = 1)]
    type Monotonic = DwtSystick<MONO_HZ>;

    /// Used to leak a &'static reference on the USB allocator
    static mut USB: Option<crate::usb_serial::UsbAllocator> = None;

    static mut RX_DESC: hal::ethernet::RxDescriptorTable<32> = RxDescriptorTable::new();
    static mut TX_DESC: hal::ethernet::TxDescriptorTable<1> = TxDescriptorTable::new();
    static mut NETWORK_STORAGE: Option<network::Storage> = None;

    #[shared]
    struct Shared {
        state: RobotState,
        network: Network<'static, Controller>,
        serial: UsbSerial<'static>,
        status: Status,
    }

    #[local]
    struct Local {
        ws: ws2812::Ws2812<
            hal::timer::TimerCounterChannel<
                hal::pac::TC0,
                hal::clock::Tc0Clock<hal::clock::Enabled>,
                0,
                15_000_000,
            >,
            hal::gpio::Pd31<hal::gpio::Output<hal::gpio::PushPull>>,
        >,
        rf: rf::Transceiver,
        rf_amp: Option<rf::Amp>,
    }

    #[init]
    fn init(mut ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        {
            // Init the heap. This will be used for protobuf
            use core::mem::MaybeUninit;
            const HEAP_SIZE: usize = 8192; // 8kb should be enough for protobuf
            static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
            unsafe { crate::HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }
        }

        let mut clocks = ClockController::new(
            ctx.device.PMC,
            &ctx.device.SUPC,
            &ctx.device.EFC,
            MainClock::Crystal16Mhz,
            SlowClock::RcOscillator32Khz,
        );

        let master_freq: fugit::Rate<u32, 1, 1> = hal::clock::get_master_clock_frequency();
        info!("Main clock: {} MHz", &master_freq.to_MHz());

        let gpio_ports = Ports::new(
            (
                ctx.device.PIOA,
                clocks.peripheral_clocks.pio_a.into_enabled_clock(),
            ),
            (
                ctx.device.PIOB,
                clocks.peripheral_clocks.pio_b.into_enabled_clock(),
            ),
            (
                ctx.device.PIOD,
                clocks.peripheral_clocks.pio_d.into_enabled_clock(),
            ),
        );

        Watchdog::new(ctx.device.WDT).disable();

        let pins = gpio_ports.split();

        // Safety: This is safe since they are no interrupts enabled yet
        let tx_desc = unsafe { &mut TX_DESC };
        let rx_desc = unsafe { &mut RX_DESC };

        let usb_allocator = crate::usb_serial::new_allocator(
            ctx.device.UDP,
            clocks.peripheral_clocks.udp,
            pins.pb10.into_system_function(&ctx.device.MATRIX),
            pins.pb11.into_system_function(&ctx.device.MATRIX),
        );

        // Safety: This is safe since they are no interrupts enabled yet
        unsafe {
            NETWORK_STORAGE = Some(network::Storage::new());
            USB = Some(usb_allocator);
        }

        // Safety: This is safe since they are no interrupts enabled yet
        let storage = unsafe { NETWORK_STORAGE.as_mut().unwrap() };

        let mac_address = [0x02, 0x6C, 0x75, 0x68, 0x62, 0x73];
        let gmac = hal::ethernet::ControllerBuilder::new()
            .set_ethernet_address(EthernetAddress::new(mac_address))
            .build(
                ctx.device.GMAC,
                clocks.peripheral_clocks.gmac.into_enabled_clock(),
                pins.pd0.into_peripheral_function_a(&ctx.device.MATRIX),
                pins.pd1.into_peripheral_function_a(&ctx.device.MATRIX),
                pins.pd2.into_peripheral_function_a(&ctx.device.MATRIX),
                pins.pd3.into_peripheral_function_a(&ctx.device.MATRIX),
                pins.pd4.into_peripheral_function_a(&ctx.device.MATRIX),
                pins.pd5.into_peripheral_function_a(&ctx.device.MATRIX),
                pins.pd6.into_peripheral_function_a(&ctx.device.MATRIX),
                pins.pd7.into_peripheral_function_a(&ctx.device.MATRIX),
                pins.pd8.into_peripheral_function_a(&ctx.device.MATRIX),
                pins.pd9.into_peripheral_function_a(&ctx.device.MATRIX),
                pins.pd10.into_peripheral_function_a(&ctx.device.MATRIX),
                pins.pd11.into_peripheral_function_a(&ctx.device.MATRIX),
                pins.pd12.into_peripheral_function_a(&ctx.device.MATRIX),
                pins.pd13.into_peripheral_function_a(&ctx.device.MATRIX),
                pins.pd14.into_peripheral_function_a(&ctx.device.MATRIX),
                pins.pd15.into_peripheral_function_a(&ctx.device.MATRIX),
                pins.pd16.into_peripheral_function_a(&ctx.device.MATRIX),
                pins.pd17.into_peripheral_function_a(&ctx.device.MATRIX),
                rx_desc,
                tx_desc,
            );

        let mut delay = hal::delay::Delay::new(ctx.core.SYST);

        let network = network::Network::from_device(gmac, mac_address, storage);

        let mut rf_spi = hal::spi::SpiMaster::<SpiU8>::new(
            ctx.device.SPI,
            clocks.peripheral_clocks.spi.into_enabled_clock(),
            pins.pa12.into_peripheral_function_a(&ctx.device.MATRIX),
            pins.pa13.into_peripheral_function_a(&ctx.device.MATRIX),
            pins.pa14.into_peripheral_function_a(&ctx.device.MATRIX),
            hal::spi::PeripheralSelectMode::Fixed,
            false,
            false,
            0,
            true,
        );

        rf_spi
            .cs_setup(
                1,
                ChipSelectSettings::new(
                    embedded_hal::spi::MODE_0,
                    hal::spi::ChipSelectActive::ActiveAfterTransfer,
                    hal::spi::BitWidth::Width8Bit,
                    18.MHz(),
                    0,
                    0,
                ),
            )
            .unwrap();
        rf_spi.cs_select(1).unwrap();

        let rf_spi = sx1280::SimpleSpiDevice {
            bus: rf_spi,
            cs: pins.pa11.into_push_pull_output(&ctx.device.MATRIX),
        };

        let mut sx1280 = sx1280::Sx1280::new(
            rf_spi,
            pins.pa27.into_push_pull_output(&ctx.device.MATRIX),
            pins.pd18.into_floating_input(&ctx.device.MATRIX),
        );

        //sx1280.init(&mut delay).unwrap();
        sx1280.init(&mut delay).unwrap();

        let sky = sky66112::Sky66112::new(
            sky66112::TiedHigh,
            pins.pa5.into_push_pull_output(&ctx.device.MATRIX),
            pins.pa22.into_push_pull_output(&ctx.device.MATRIX),
            pins.pd28.into_push_pull_output(&ctx.device.MATRIX),
            sky66112::TiedHigh,
            pins.pa8.into_push_pull_output(&ctx.device.MATRIX),
        );

        let sky = sky.into_sleep_mode2();

        let mut sx1280 = sx1280.into_flrc();

        sx1280.set_frequency(2_400u32.MHz()).unwrap();
        sx1280.set_buffer_base_address(0, 128).unwrap();
        sx1280.set_packet_type(sx1280::definitions::GfskFlrcPacketType::PacketLengthVariable);
        sx1280.set_sync_word_match(sx1280::definitions::GfskFlrcSyncWordMatch::SyncWord1);
        sx1280.set_sync_word1(0xdeadbeef).unwrap();
        sx1280.set_auto_fs(true).unwrap();
        sx1280
            .set_preamble_length(sx1280::definitions::GfskFlrcPreambleLength::PreambleLength08Bits);
        sx1280
            .set_modulation_params(
                sx1280::definitions::FlrcBitrateBandwidth::Bitrate1300Bandwidth12,
                sx1280::definitions::FlrcCodingRate::CodingRate11,
                sx1280::definitions::FlrcModulationShaping::BtOff,
            )
            .unwrap();
        sx1280
            .set_tx_param(-2, sx1280::definitions::RampTime::Ramp02us)
            .unwrap();

        let writer = sx1280::definitions::IrqWriter::new()
            .set(sx1280::definitions::IrqBit::TxDone)
            .set(sx1280::definitions::IrqBit::RxDone)
            .set(sx1280::definitions::IrqBit::RxTxTimeout);

        sx1280
            .enable_interrupts(writer, writer, writer, writer)
            .unwrap();

        let tc0 = TimerCounter::new(ctx.device.TC0);
        let tc0_chs = tc0.split::<15_000_000, 15_000_000, 15_000_000>(
            clocks.peripheral_clocks.tc_0.into_enabled_clock(),
            clocks.peripheral_clocks.tc_1.into_enabled_clock(),
            clocks.peripheral_clocks.tc_2.into_enabled_clock(),
        );

        let mut tcc0 = tc0_chs.ch0;
        tcc0.clock_input(hal::timer::ClockSource::MckDiv8);
        let duration = 3u32.MHz::<1, 1>().into_duration::<1, 15_000_000>();
        tcc0.start(duration);
        info!("Init");
        let ws = ws2812::Ws2812::new(tcc0, pins.pd31.into_push_pull_output(&ctx.device.MATRIX));
        write_status::spawn().unwrap();

        let mono = DwtSystick::new(
            &mut ctx.core.DCB,
            ctx.core.DWT,
            delay.free(),
            master_freq.raw(),
        );

        let usb_ref = unsafe { USB.as_ref().unwrap() };
        let mut serial = crate::usb_serial::UsbSerial::new(usb_ref);
        serial.device.force_reset().unwrap();
        serial.on_interrupt();

        print_status::spawn_after(5u64.secs()).unwrap();

        let status = Status::default();

        //test_rf::spawn_after(200u64.millis()).unwrap();
        let state = RobotState::default();

        (
            Shared {
                state,
                network,
                serial,
                status,
            },
            Local {
                ws,
                rf: sx1280,
                rf_amp: Some(sky),
            },
            init::Monotonics(mono),
        )
    }

    #[idle(shared = [state, network, status])]
    fn idle(mut ctx: idle::Context) -> ! {
        loop {
            ctx.shared.status.lock(|status| {
                ctx.shared.network.lock(|network| {
                    network.poll(
                        |_packet| {
                            // TODO implement
                        },
                        |wrapper_packet, endpoint| {
                            ctx.shared.state.lock(|state| {
                                state.update_from_network(wrapper_packet);
                            });
                            transmit::spawn(endpoint).unwrap();
                        },
                        status,
                    );
                });
            });
        }
    }

    #[task(local = [ws], priority = 1)]
    fn write_status(ctx: write_status::Context) {
        const NUM_LEDS: usize = 16;

        let mut data: [RGB<u8>; NUM_LEDS] = [RGB::default(); NUM_LEDS];

        for color in data.iter_mut().skip(0).step_by(3) {
            color.r = 0;
        }

        for color in data.iter_mut().skip(1).step_by(3) {
            color.g = 0;
        }

        for color in data.iter_mut().skip(2).step_by(3) {
            color.b = 0;
        }

        ctx.local.ws.write(data.into_iter()).unwrap();

        write_status::spawn_after(500u64.millis()).unwrap();
    }

    #[task(local = [rf, rf_amp], shared = [state, network])]
    fn transmit(mut ctx: transmit::Context, endpoint: atsam4_hal::smoltcp::wire::IpEndpoint) {
        ctx.shared.state.lock(|state| {
            rf::transmit_and_receive_feedback(state, ctx.local.rf, ctx.local.rf_amp);

            if let Some(feedback) = state.create_network_packet() {
                ctx.shared.network.lock(|network| {
                    network.send_feedback(&feedback, endpoint);
                });
            }
        })
    }

    #[task(shared = [serial, status])]
    fn print_status(mut ctx: print_status::Context) {
        ctx.shared.status.lock(|status| {
            ctx.shared.serial.lock(|serial| {
                status.write_to_serial(serial);
            });
        });
        print_status::spawn_after(1u64.secs()).unwrap();
    }

    #[task(binds = UDP, shared = [serial])]
    fn usb_interrupt(mut ctx: usb_interrupt::Context) {
        ctx.shared.serial.lock(|serial| {
            if serial.on_interrupt() {
                let mut buffer = [0_u8; 64];
                if let Ok(length) = serial.serial.read(&mut buffer[..]) {
                    info!("Got data: {:?}", buffer[..length]);
                }
            }
        });
    }
}
