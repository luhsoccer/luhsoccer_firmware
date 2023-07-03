use az::Az;
#[cfg(any(feature = "test_motors", feature = "test_kicker"))]
use defmt::debug;
use defmt::{error, info, unwrap};
use embassy_executor::{task, Spawner};
use embassy_futures::select::{select, Either};
use embassy_rp::{
    peripherals::{PIN_16, PIN_17, PIN_18, PIN_19, UART0},
    uart::{self, BufferedUart, BufferedUartRx},
};
use embassy_sync::{
    blocking_mutex::raw::{CriticalSectionRawMutex, RawMutex},
    signal::Signal,
};
use embassy_time::Duration;
use embedded_io::asynch::{BufRead, Write};
use fixed::types::I16F16;
use intra_comms::{
    definitions::{KickerChargeHint, LocalVelocity, Main2Motor},
    uart::{MainControllerReceiver, MainControllerSender, ReceiveError, SendError},
};
use static_cell::StaticCell;
use sync::observable::Observable;
use units::types::{MetrePerSecond, RadianPerSecond, Volt};

use crate::odometry::Movement;

#[task]
#[allow(clippy::similar_names)]
#[allow(clippy::too_many_arguments)]
pub async fn maincontroller_task(
    uart: UART0,
    tx: PIN_16,
    rx: PIN_17,
    cts: PIN_18,
    rts: PIN_19,
    movement_setpoint: &'static Observable<CriticalSectionRawMutex, Movement, 8>,
    kicker_set_voltage: &'static Observable<CriticalSectionRawMutex, Volt<u8>, 8>,
    has_ball: &'static Observable<CriticalSectionRawMutex, bool, 8>,
    kicker_cap_voltage: &'static Observable<CriticalSectionRawMutex, Volt<u8>, 8>,
    kicker_speed: &'static Observable<CriticalSectionRawMutex, u16, 8>,
    kicker_raw_duration: &'static Observable<CriticalSectionRawMutex, Duration, 8>,
    robot_velocity: &'static Observable<CriticalSectionRawMutex, Movement, 8>,
    save_config: &'static Signal<CriticalSectionRawMutex, ()>,
    config: &'static crate::Config<CriticalSectionRawMutex>,
    spawner: Spawner,
) {
    static UART_RX_BUFFER: StaticCell<[u8; 256]> = StaticCell::new();
    static UART_TX_BUFFER: StaticCell<[u8; 256]> = StaticCell::new();

    let tx_buffer = &mut UART_TX_BUFFER.init([0; 256])[..];
    let rx_buffer = &mut UART_RX_BUFFER.init([0; 256])[..];
    let mut uart_config = uart::Config::default();
    uart_config.baudrate = 1_000_000;
    let uart = BufferedUart::new_with_rtscts(
        uart,
        crate::Irqs,
        tx,
        rx,
        rts,
        cts,
        tx_buffer,
        rx_buffer,
        uart_config,
    );
    let (rx, tx) = uart.split();

    spawner.must_spawn(receive_task(
        MainControllerReceiver::new(rx),
        movement_setpoint,
        kicker_set_voltage,
        has_ball,
        kicker_cap_voltage,
        kicker_speed,
        kicker_raw_duration,
        save_config,
        config,
    ));
    send(
        MainControllerSender::new(tx),
        kicker_cap_voltage,
        robot_velocity,
    )
    .await;
}

#[task]
#[allow(clippy::too_many_arguments)]
async fn receive_task(
    receiver: MainControllerReceiver<BufferedUartRx<'static, UART0>>,
    movement_setpoint: &'static Observable<CriticalSectionRawMutex, Movement, 8>,
    kicker_set_voltage: &'static Observable<CriticalSectionRawMutex, Volt<u8>, 8>,
    has_ball: &'static Observable<CriticalSectionRawMutex, bool, 8>,
    kicker_cap_voltage: &'static Observable<CriticalSectionRawMutex, Volt<u8>, 8>,
    kick_speed: &'static Observable<CriticalSectionRawMutex, u16, 8>,
    kicker_raw_duration: &'static Observable<CriticalSectionRawMutex, Duration, 8>,
    save_config: &'static Signal<CriticalSectionRawMutex, ()>,
    config: &'static crate::Config<CriticalSectionRawMutex>,
) {
    receive(
        receiver,
        movement_setpoint,
        kicker_set_voltage,
        has_ball,
        kicker_cap_voltage,
        kick_speed,
        kicker_raw_duration,
        save_config,
        config,
    )
    .await;
}

#[allow(clippy::too_many_arguments)]
async fn receive<
    const SUBS1: usize,
    const SUBS2: usize,
    const SUBS3: usize,
    const SUBS4: usize,
    const SUBS5: usize,
    const SUBS6: usize,
>(
    mut receiver: MainControllerReceiver<impl BufRead>,
    movement_setpoint: &Observable<impl RawMutex, Movement, SUBS1>,
    kicker_set_voltage: &Observable<impl RawMutex, Volt<u8>, SUBS2>,
    has_ball: &Observable<impl RawMutex, bool, SUBS3>,
    kicker_cap_voltage: &Observable<impl RawMutex, Volt<u8>, SUBS4>,
    kick_speed: &Observable<impl RawMutex, u16, SUBS5>,
    kicker_raw_duration: &Observable<impl RawMutex, Duration, SUBS6>,
    save_config: &Signal<impl RawMutex, ()>,
    config: &crate::Config<impl RawMutex>,
) {
    loop {
        info!("trying to receive packet from maincontroller");
        match receiver.receive().await {
            Err(e) => match e {
                ReceiveError::Postcard(_) => {
                    error!("Couldn't deserialize message using postcard")
                }
                ReceiveError::Cobs => error!("Unable to find valid Cobs packet"),
                ReceiveError::Io(_) => error!("The Uart could not be used"),
            },
            Ok(cmd) => match cmd {
                Main2Motor::Drive(velocity) => {
                    info!("got drive command with velocity {}", velocity);
                    let forward = MetrePerSecond::new(I16F16::from_num(velocity.forward) / 1000);
                    let left = MetrePerSecond::new(I16F16::from_num(velocity.left) / 1000);
                    let counterclockwise = RadianPerSecond::new(
                        I16F16::from_num(velocity.counterclockwise) / (2i32.pow(10)),
                    );
                    let movement = Movement {
                        forward,
                        left,
                        counterclockwise,
                    };
                    #[cfg(not(feature = "test_motors"))]
                    movement_setpoint.set_if_different(movement);
                    #[cfg(feature = "test_motors")]
                    debug!(
                        "Test build. test value {} is not changed to {}",
                        movement_setpoint.get(),
                        movement,
                    );
                }
                Main2Motor::Kick(speed) | Main2Motor::Chip(speed) => {
                    info!("got kick or chip command with speed {}mm/s²", speed);
                    #[cfg(not(feature = "test_kicker"))]
                    kick_speed.set_if_different(speed);
                    #[cfg(feature = "test_kicker")]
                    debug!(
                        "Test build. test value {} is not changed to {}",
                        kick_speed.get(),
                        speed,
                    );
                }
                Main2Motor::KickRaw(duration) => {
                    info!("got raw kick command with duration {}us", duration);
                    let duration = Duration::from_micros_floor(duration.into());
                    #[cfg(not(feature = "test_kicker"))]
                    kicker_raw_duration.set_if_different(duration);
                    #[cfg(feature = "test_kicker")]
                    debug!(
                        "Test build. test value {} is not changed to {}",
                        kicker_raw_duration.get(),
                        duration
                    )
                }
                Main2Motor::BallInDribbler => {
                    info!("ball is in dribbler");
                    #[cfg(not(feature = "test_kicker"))]
                    has_ball.set(true);
                    #[cfg(feature = "test_kicker")]
                    debug!(
                        "Test build. test value {} is not changed to true",
                        has_ball.get(),
                    );
                }
                Main2Motor::BallNotInDribbler => {
                    info!("ball is not in dribbler");
                    #[cfg(not(feature = "test_kicker"))]
                    has_ball.set(false);
                    #[cfg(feature = "test_kicker")]
                    debug!(
                        "Test build. test value {} is not changed to false",
                        has_ball.get(),
                    );
                }
                Main2Motor::CalibrateCapVoltage(measured_voltage) => {
                    info!("calibrating cap voltage: {}", measured_voltage);
                    const TEST_VOLTAGE: u8 = 230;
                    let config_value = config.kicker_cap_dac_230v.get();
                    let scaling = f32::from(TEST_VOLTAGE) / f32::from(measured_voltage);
                    config
                        .kicker_cap_dac_230v
                        .set(((f32::from(config_value) * scaling) as u16).clamp(1, 0x03FF));

                    let adc_voltage = kicker_cap_voltage.get();
                    let scaling = f32::from(measured_voltage) / f32::from(adc_voltage.raw());
                    let config_value = config.kicker_cap_adc_230v.get();
                    config
                        .kicker_cap_adc_230v
                        .set(((f32::from(config_value) * scaling) as u16).clamp(1, 0x0FFF));

                    save_config.signal(());
                }
                Main2Motor::ChargeHint(hint) => {
                    info!("got charg hint {}", hint);
                    let voltage = match hint {
                        KickerChargeHint::Charge | KickerChargeHint::DontCare => {
                            config.kicker_charge_voltage.get()
                        }
                        KickerChargeHint::Discharge => Volt::new(0),
                    };
                    #[cfg(not(feature = "test_kicker"))]
                    kicker_set_voltage.set_if_different(voltage);
                    #[cfg(feature = "test_kicker")]
                    debug!(
                        "Test build. test value {} is not changed to {}",
                        kicker_set_voltage.get(),
                        voltage,
                    );
                }
            },
        }
    }
}

async fn send<const SUBS1: usize, const SUBS2: usize>(
    mut sender: MainControllerSender<impl Write>,
    kicker_cap_voltage: &Observable<impl RawMutex, Volt<u8>, SUBS1>,
    robot_velocity: &Observable<impl RawMutex, Movement, SUBS2>,
) {
    let mut kicker_cap_voltage_sub = unwrap!(kicker_cap_voltage.subscriber());
    let mut robot_velocity_sub = unwrap!(robot_velocity.subscriber());
    loop {
        if let Err(e) = match select(
            kicker_cap_voltage_sub.next_value(),
            robot_velocity_sub.next_value(),
        )
        .await
        {
            Either::First(voltage) => sender.cap_voltage(voltage.raw()).await,
            Either::Second(movement) => {
                sender
                    .motor_velocity(LocalVelocity {
                        forward: (movement.forward.raw() * 1000).az(),
                        left: (movement.left.raw() * 1000).az(),
                        counterclockwise: (movement.counterclockwise.raw() * (2i32.pow(10))).az(),
                    })
                    .await
            }
        } {
            match e {
                SendError::Postcard(_) => error!("Unable to serialize using postcard"),
                SendError::Io(_) => error!("Io error"),
            }
        }
    }
}
