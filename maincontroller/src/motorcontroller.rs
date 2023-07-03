use defmt::{debug, error, unwrap};
use embassy_executor::{task, Spawner};
use embassy_futures::join::join3;
use embassy_rp::{
    peripherals::{PIN_16, PIN_17, PIN_18, PIN_19, UART0},
    uart::{self, BufferedUart, BufferedUartRx},
};
use embassy_sync::{
    blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex, RawMutex},
    mutex::Mutex,
};
use embassy_time::{with_timeout, Duration};
use embedded_io::asynch::{BufRead, Write};
use intra_comms::{
    definitions::{KickerChargeHint, LocalVelocity, Motor2Main},
    uart::{MotorControllerReceiver, MotorControllerSender, ReceiveError, SendError},
};
use static_cell::StaticCell;
use sync::observable::Observable;

use crate::lightbarrier::LightBarrierState;

#[task]
#[allow(clippy::too_many_arguments)]
pub async fn motorcontroller_task(
    uart: UART0,
    tx: PIN_16,
    rx: PIN_17,
    cts: PIN_18,
    rts: PIN_19,
    has_ball: &'static Observable<CriticalSectionRawMutex, LightBarrierState, 8>,
    command_velocity: &'static Observable<CriticalSectionRawMutex, LocalVelocity, 8>,
    command_kick_speed: &'static Observable<CriticalSectionRawMutex, crate::KickSpeed, 8>,
    actual_velocity: &'static Observable<CriticalSectionRawMutex, LocalVelocity, 8>,
    kicker_voltage: &'static Observable<CriticalSectionRawMutex, u8, 8>,
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
        MotorControllerReceiver::new(rx),
        actual_velocity,
        kicker_voltage,
    ));
    send(
        MotorControllerSender::new(tx),
        has_ball,
        command_velocity,
        command_kick_speed,
    )
    .await;
}

#[task]
async fn receive_task(
    receiver: MotorControllerReceiver<BufferedUartRx<'static, UART0>>,
    actual_velocity: &'static Observable<CriticalSectionRawMutex, LocalVelocity, 8>,
    kicker_voltage: &'static Observable<CriticalSectionRawMutex, u8, 8>,
) {
    receive(receiver, actual_velocity, kicker_voltage).await;
}

async fn receive<const SUBS1: usize, const SUBS2: usize>(
    mut receiver: MotorControllerReceiver<impl BufRead>,
    actual_velocity: &Observable<impl RawMutex, LocalVelocity, SUBS1>,
    kicker_voltage: &Observable<impl RawMutex, u8, SUBS2>,
) {
    loop {
        match receiver.receive().await {
            Err(e) => match e {
                ReceiveError::Postcard(_) => {
                    error!("Couldn't deserialize message using postcard")
                }
                ReceiveError::Cobs => error!("Unable to find valid Cobs packet"),
                ReceiveError::Io(_) => error!("The Uart could not be used"),
            },
            Ok(cmd) => match cmd {
                Motor2Main::MotorVelocity(velocity) => actual_velocity.set_if_different(velocity),
                Motor2Main::CapVoltage(voltage) => kicker_voltage.set_if_different(voltage),
            },
        }
    }
}

async fn send<const SUBS1: usize, const SUBS2: usize, const SUBS3: usize>(
    sender: MotorControllerSender<impl Write>,
    has_ball: &Observable<impl RawMutex, LightBarrierState, SUBS1>,
    command_velocity: &Observable<impl RawMutex, LocalVelocity, SUBS2>,
    command_kick_speed: &Observable<impl RawMutex, crate::KickSpeed, SUBS3>,
) {
    const MAX_TIME_BETWEEN_SENDS: Duration = Duration::from_hz(1);

    let mut has_ball_sub = unwrap!(has_ball.subscriber());
    let mut velocity_sub = unwrap!(command_velocity.subscriber());
    let mut kick_speed_sub = unwrap!(command_kick_speed.subscriber());
    let sender = Mutex::<NoopRawMutex, _>::new(sender);

    let has_ball_fut = async {
        loop {
            let value = match with_timeout(MAX_TIME_BETWEEN_SENDS, has_ball_sub.next_value()).await
            {
                Ok(value) => value,
                Err(_) => has_ball.get(),
            };
            debug!("sending {} to motorcontroller", value);
            if let Err(e) = sender
                .lock()
                .await
                .ball_in_dribbler(value == LightBarrierState::HasBall)
                .await
            {
                match e {
                    SendError::Postcard(_) => {
                        error!("unable to encode message using postcard")
                    }
                    SendError::Io(_) => error!("unable to send message using uart"),
                }
            }
        }
    };

    let velocity_fut = async {
        loop {
            let value = match with_timeout(MAX_TIME_BETWEEN_SENDS, velocity_sub.next_value()).await
            {
                Ok(value) => value,
                Err(_) => command_velocity.get(),
            };
            debug!("sending {} to motorcontroller", value);
            if let Err(e) = sender.lock().await.drive(value).await {
                match e {
                    SendError::Postcard(_) => {
                        error!("unable to encode message using postcard")
                    }
                    SendError::Io(_) => error!("unable to send message using uart"),
                }
            }
        }
    };

    let kick_speed_fut = async {
        loop {
            let value =
                match with_timeout(MAX_TIME_BETWEEN_SENDS, kick_speed_sub.next_value()).await {
                    Ok(value) => value,
                    Err(_) => command_kick_speed.get(),
                };
            debug!("sending {} to motorcontroller", value);
            let mut guard = sender.lock().await;
            let res = guard
                .charge_hint(KickerChargeHint::Charge)
                .await
                .and(match value {
                    crate::KickSpeed::Velocity(velocity) => guard.kick(velocity).await,
                    crate::KickSpeed::Raw(duration) => guard.kick_raw(duration).await,
                });
            if let Err(e) = res {
                match e {
                    SendError::Postcard(_) => {
                        error!("unable to encode message using postcard")
                    }
                    SendError::Io(_) => error!("unable to send message using uart"),
                }
            }
        }
    };

    join3(has_ball_fut, velocity_fut, kick_speed_fut).await;
}
