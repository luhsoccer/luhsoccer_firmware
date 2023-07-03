use az::Az;
use defmt::{debug, error, unwrap, warn};
use embassy_executor::task;
use embassy_futures::select::{select3, Either3};
use embassy_rp::{
    gpio::{Input, Level, Output, Pull},
    peripherals::{
        DMA_CH0, DMA_CH1, PIN_0, PIN_1, PIN_14, PIN_2, PIN_3, PIN_4, PIN_5, PIN_6, PIN_7, PIN_8,
        SPI0,
    },
    spi::{self, Spi},
};
use embassy_sync::{
    blocking_mutex::raw::{CriticalSectionRawMutex, RawMutex},
    mutex::Mutex,
};
use embassy_time::{with_timeout, Delay, Duration};
use embedded_hal::digital::v2::{InputPin, OutputPin};
use embedded_hal_async::{digital::Wait, spi::ExclusiveDevice, spi::SpiDevice};
use fixed::types::U16F16;
use fixed_macro::types::U16F16;
use fugit::RateExtU32;
use intra_comms::{
    crate_version,
    definitions::{
        BallState, BasestationToRobot, DribblerSpeedSelection, DribblerState, GameState,
        KickSpeedSelection, LocalVelocity, MovementSelection, RobotToBasestation, Team,
        VelocitySelection,
    },
    ROBOT_BLUE_SYNC_WORDS,
};
use sky66112::{Sky66112, TiedHigh, TiedLow};
use sx1280::{
    definitions::{
        FlrcBitrateBandwidth, FlrcCodingRate, FlrcModulationShaping, GfskFlrcPacketType,
        GfskFlrcPreambleLength, GfskFlrcSyncWordMatch, IrqBit, IrqWriter, PeriodBase, RampTime,
    },
    Sx1280,
};
use sync::observable::Observable;

use crate::{
    lightbarrier::{self, LightBarrierState},
    Config,
};

#[task]
#[allow(clippy::too_many_arguments)]
pub async fn rf_task(
    crx: PIN_0,
    cps: PIN_1,
    ctx: PIN_2,
    reset: PIN_3,
    miso: PIN_4,
    cs: PIN_5,
    sck: PIN_6,
    mosi: PIN_7,
    busy: PIN_8,
    dio1: PIN_14,
    spi: SPI0,
    tx_dma: DMA_CH0,
    rx_dma: DMA_CH1,
    config: &'static Config<CriticalSectionRawMutex>,
    voltage: &'static Mutex<CriticalSectionRawMutex, U16F16>,
    has_ball: &'static Observable<CriticalSectionRawMutex, lightbarrier::LightBarrierState, 8>,
    dribbler_speed: &'static Observable<CriticalSectionRawMutex, u16, 8>,
    command_velocity: &'static Observable<CriticalSectionRawMutex, LocalVelocity, 8>,
    command_kick_speed: &'static Observable<CriticalSectionRawMutex, crate::KickSpeed, 8>,
    actual_velocity: &'static Observable<CriticalSectionRawMutex, LocalVelocity, 8>,
    kicker_voltage: &'static Observable<CriticalSectionRawMutex, u8, 8>,
) {
    let crx = Output::new(crx, Level::Low);
    let cps = Output::new(cps, Level::Low);
    let ctx = Output::new(ctx, Level::Low);
    let reset = Output::new(reset, Level::Low);
    let cs = Output::new(cs, Level::High);
    let busy = Input::new(busy, Pull::None);
    let dio1 = Input::new(dio1, Pull::None);

    let mut spi_config = spi::Config::default();
    spi_config.frequency = 18_000_000;
    let bus = Spi::new(spi, sck, mosi, miso, tx_dma, rx_dma, spi_config);
    let spi_device = ExclusiveDevice::new(bus, cs);

    rf(
        spi_device,
        reset,
        busy,
        dio1,
        cps,
        crx,
        ctx,
        config,
        voltage,
        has_ball,
        dribbler_speed,
        command_velocity,
        command_kick_speed,
        actual_velocity,
        kicker_voltage,
    )
    .await;
}

#[allow(clippy::too_many_arguments)]
async fn rf<
    const SUBS1: usize,
    const SUBS2: usize,
    const SUBS3: usize,
    const SUBS4: usize,
    const SUBS5: usize,
    const SUBS6: usize,
>(
    spi: impl SpiDevice<u8>,
    reset: impl OutputPin,
    busy: impl InputPin + Wait,
    mut dio1: impl InputPin + Wait,
    cps: impl OutputPin,
    crx: impl OutputPin,
    ctx: impl OutputPin,
    config: &Config<impl RawMutex>,
    voltage: &Mutex<impl RawMutex, U16F16>,
    has_ball: &Observable<impl RawMutex, LightBarrierState, SUBS1>,
    dribbler_speed: &Observable<impl RawMutex, u16, SUBS2>,
    command_velocity: &Observable<impl RawMutex, LocalVelocity, SUBS3>,
    command_kick_speed: &Observable<impl RawMutex, crate::KickSpeed, SUBS4>,
    actual_velocity: &Observable<impl RawMutex, LocalVelocity, SUBS5>,
    kicker_voltage: &Observable<impl RawMutex, u8, SUBS6>,
) {
    let sky = Sky66112::new(TiedHigh, cps, crx, ctx, TiedHigh, TiedLow);
    let mut sky_outer = Some(sky.into_sleep_mode2());

    let sx = Sx1280::new(spi, reset, busy);
    let Ok(mut sx) = init(sx, config).await else {
        error!("Can't initialize Sx1280");
        return;
    };
    let Some(mut id_subscriber) = config.id.sub() else {error!("couldn't get id subscriber"); return;};
    let Some(mut frequency_subscriber) = config.rf_frequency.sub() else {error!("couldn't get frequency subscriber"); return;};
    let mut frequency = None;
    let mut sync_word = None;
    let mut rx_timed_out = false;
    loop {
        if let Some(frequency) = frequency.take() {
            debug!("setting new frequency");
            if sx.set_frequency(frequency).await.is_err() {
                error!("couldn't set new frequency");
            }
        }
        if let Some(sync_word) = sync_word.take() {
            debug!("setting new sync word");
            if sx.set_sync_word1(sync_word).await.is_err() {
                error!("couldn't set new sync word")
            }
        }
        let Some(sky) = sky_outer.take() else {error!("The sky66112 got lost"); return;};
        let sky = sky.into_receive_lna_mode();
        if sx
            .start_receive_packet(
                32,
                PeriodBase::MilliSeconds1,
                if rx_timed_out { 0 } else { 50 },
            )
            .await
            .is_err()
        {
            error!("starting receive packet");
            return;
        }
        loop {
            match select3(
                dio1.wait_for_high(),
                id_subscriber.next_value(),
                frequency_subscriber.next_value(),
            )
            .await
            {
                Either3::First(Ok(_)) => break,
                Either3::First(Err(_)) => {
                    error!("Waiting for interrupt");
                    return;
                }
                Either3::Second(id) => sync_word = Some(ROBOT_BLUE_SYNC_WORDS[usize::from(id)]),
                Either3::Third(new_frequency) => frequency = Some(new_frequency.MHz()),
            }
        }
        if dio1.wait_for_high().await.is_err() {
            return;
        }
        let Ok(irq) = sx.irq_status().await else {
            error!("getting rf status");
            sky_outer = Some(sky.into_sleep_mode2());
            continue;
        };
        if irq.is_set(IrqBit::RxTxTimeout) {
            warn!("timeout while receiving packet");
            command_velocity.set(LocalVelocity {
                forward: 0,
                left: 0,
                counterclockwise: 0,
            });
            command_kick_speed.set(crate::KickSpeed::Velocity(0));
            dribbler_speed.set(0);
            sky_outer = Some(sky.into_sleep_mode2());
            rx_timed_out = true;
            continue;
        }
        let Ok((rssi, errors, _sync)) = sx.packet_status().await else {
            error!("getting packet status");
            sky_outer = Some(sky.into_sleep_mode2());
            continue;
        };
        if errors.sync_error {
            error!("rf sync word error!");
            sky_outer = Some(sky.into_sleep_mode2());
            continue;
        }
        if errors.crc_error {
            error!("rf crc error");
            sky_outer = Some(sky.into_sleep_mode2());
            continue;
        }
        if errors.length_error {
            error!("rf length error");
            sky_outer = Some(sky.into_sleep_mode2());
            continue;
        }
        if errors.abort_error {
            error!("rf length error");
            sky_outer = Some(sky.into_sleep_mode2());
            continue;
        }
        rx_timed_out = false;
        let response = RobotToBasestation {
            id: config.id.get(),
            team: Team::Blue,
            battery_voltage: (*voltage.lock().await * U16F16!(8)).az(),
            kicker_voltage: kicker_voltage.get(),
            has_ball: match has_ball.get() {
                LightBarrierState::HasBall | LightBarrierState::ContactLost => {
                    BallState::InDribbler
                }
                LightBarrierState::NoBall => BallState::NotInDribbler,
            },
            error: 0,
            battery_current: None,
            battery_capacity_used: None,
            rssi: unwrap!(u8::try_from(-rssi), "range checked"),
            velocity: Some(VelocitySelection::RobotVelocity(actual_velocity.get())),
            position: None,
            firmware_version: crate_version!(),
        };
        let Ok(feedback_packet) = postcard::to_vec::<_, 24>(&response) else {
            error!("couldn't encode feedback");
        sky_outer = Some(sky.into_sleep_mode2());
        continue;
        };

        let sky = sky.into_transmit_high_power_mode();
        if sx
            .send_packet::<24>(&feedback_packet[..], PeriodBase::MilliSeconds1, 5)
            .await
            .is_err()
        {
            error!("sending rf packet");
            return;
        }
        let _ = dio1.wait_for_high().await;
        sx.clear_interrupts().await.ok();
        sky_outer = Some(sky.into_sleep_mode2());
        let Ok(packet) = sx.read_packet::<32>().await else {
                error!("reading buffer from sx1280");
                return;
            };

        let Ok(packet) = postcard::from_bytes::<BasestationToRobot>(&packet[..]) else {
            error!("couldn't decode packet from basestation");
            continue;
        };

        process(
            &packet,
            config,
            dribbler_speed,
            command_velocity,
            command_kick_speed,
        )
        .await;
    }
}

async fn init<S, R, B>(
    mut sx1280: Sx1280<S, R, B, sx1280::None, u8, sx1280::Async>,
    config: &Config<impl RawMutex>,
) -> Result<
    Sx1280<S, R, B, sx1280::Flrc, u8, sx1280::Async>,
    sx1280::error::Error<S::Error, R::Error, B::Error>,
>
where
    S: SpiDevice,
    R: OutputPin,
    B: Wait,
{
    sx1280.init(&mut Delay).await?;
    let mut sx = sx1280.into_flrc().await;
    sx.set_frequency(config.rf_frequency.get().MHz()).await?;
    sx.set_auto_fs(true).await?;
    let irqs = IrqWriter::new()
        .set(IrqBit::TxDone)
        .set(IrqBit::RxDone)
        .set(IrqBit::RxTxTimeout);
    sx.enable_interrupts(irqs, irqs, irqs, irqs).await?;
    sx.set_buffer_base_address(0, 128).await?;
    sx.set_preamble_length(GfskFlrcPreambleLength::PreambleLength08Bits);
    sx.set_packet_type(GfskFlrcPacketType::PacketLengthVariable);
    sx.set_sync_word_match(GfskFlrcSyncWordMatch::SyncWord1);
    sx.set_sync_word1(ROBOT_BLUE_SYNC_WORDS[usize::from(config.id.get())])
        .await?;
    sx.set_modulation_params(
        FlrcBitrateBandwidth::Bitrate1300Bandwidth12,
        FlrcCodingRate::CodingRate11,
        FlrcModulationShaping::BtOff,
    )
    .await?;
    sx.set_tx_param(0, RampTime::Ramp02us).await?;
    Ok(sx)
}

async fn process<const SUBS1: usize, const SUBS2: usize, const SUBS3: usize>(
    packet: &BasestationToRobot,
    config: &Config<impl RawMutex>,
    command_dribbler_speed: &Observable<impl RawMutex, u16, SUBS1>,
    command_velocity: &Observable<impl RawMutex, LocalVelocity, SUBS2>,
    command_kick_speed: &Observable<impl RawMutex, crate::KickSpeed, SUBS3>,
) {
    match packet.movement {
        MovementSelection::RobotVelocity(velocity) => {
            command_velocity.set_if_different(velocity);
        }
        MovementSelection::CameraVelocity(_) => {
            error!("absolute velocity controll not implemented yet");
        }
        MovementSelection::Position(_) => {
            error!("position controll not implemented yet");
        }
    }

    match packet.kick_speed {
        KickSpeedSelection::Relative(speed) => {
            command_kick_speed.set_if_different(crate::KickSpeed::Velocity(speed));
        }
        KickSpeedSelection::Absolute(_) => {
            error!("absolute kicking speed not implemented yet");
        }
    }

    match packet.dribbler_speed {
        DribblerSpeedSelection::Tristate(state) => {
            command_dribbler_speed.set_if_different(match state {
                DribblerState::Off => 0,
                DribblerState::Half => config.dribbler_low.get(),
                DribblerState::Full => config.dribbler_high.get(),
            })
        }
        DribblerSpeedSelection::Percent(p) => {
            command_dribbler_speed.set_if_different(u16::from(p) * (u16::MAX / 100));
        }
        DribblerSpeedSelection::Rpm(_) => {
            error!("Dribbler RPM controll not implemented yet");
        }
    }

    match packet.game_state {
        GameState::Halt => {
            let halt = async {
                command_kick_speed.set(crate::KickSpeed::Velocity(0));
                command_velocity.set(LocalVelocity {
                    forward: 0,
                    left: 0,
                    counterclockwise: 0,
                });
            };
            if with_timeout(Duration::from_millis(5), halt).await.is_err() {
                error!("timeout sending HALT to motorcontroller");
            }
            command_dribbler_speed.set(0);
        }
        GameState::Stop => (),
        GameState::Normal => (),
    }
}
