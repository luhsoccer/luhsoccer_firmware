use az::Az;
use defmt::{error, info, unwrap, warn, Format};
use embassy_executor::task;
use embassy_futures::block_on;
use embassy_rp::{
    i2c::{self, Action, AsyncSlave, I2c},
    peripherals::{I2C1, PIN_10, PIN_11},
};
use embassy_sync::{
    blocking_mutex::raw::{CriticalSectionRawMutex, RawMutex},
    mutex::Mutex,
    signal::Signal,
};
use fixed::types::U16F16;
use fixed_macro::types::U16F16;
use sync::observable::Observable;

use crate::lightbarrier::LightBarrierState;

#[task]
#[allow(clippy::too_many_arguments)]
pub async fn ui_task(
    sda: PIN_10,
    scl: PIN_11,
    i2c1: I2C1,
    config: &'static crate::Config<CriticalSectionRawMutex>,
    save_config: &'static Signal<CriticalSectionRawMutex, ()>,
    voltage_mutex: &'static Mutex<CriticalSectionRawMutex, U16F16>,
    has_ball: &'static Observable<CriticalSectionRawMutex, LightBarrierState, 8>,
    kick_speed: &'static Observable<CriticalSectionRawMutex, crate::KickSpeed, 8>,
    dribbler_speed: &'static Observable<CriticalSectionRawMutex, u16, 8>,
    shutdown: &'static Signal<CriticalSectionRawMutex, ()>,
) {
    const SLAVE_ADDRESS: u8 = 0x42;
    info!("Setting up UI I2C");
    let i2c_config = i2c::Config::default();
    let i2c = I2c::new_async_slave(i2c1, scl, sda, crate::Irqs, i2c_config, SLAVE_ADDRESS);

    ui(
        i2c,
        config,
        save_config,
        voltage_mutex,
        has_ball,
        shutdown,
        kick_speed,
        dribbler_speed,
    )
    .await;
}

#[allow(clippy::too_many_arguments)]
async fn ui<const SUBS1: usize, const SUBS2: usize, const SUBS3: usize>(
    mut i2c: I2c<'_, impl i2c::Instance, AsyncSlave>,
    config: &crate::Config<impl RawMutex>,
    save_config: &Signal<impl RawMutex, ()>,
    voltage_mutex: &Mutex<impl RawMutex, U16F16>,
    has_ball: &Observable<impl RawMutex, LightBarrierState, SUBS1>,
    shutdown: &Signal<impl RawMutex, ()>,
    kick_speed: &Observable<impl RawMutex, crate::KickSpeed, SUBS2>,
    dribbler_speed: &Observable<impl RawMutex, u16, SUBS3>,
) {
    let mut address = None;
    loop {
        i2c.update(|action, data| match action {
            Action::Receive => {
                if let Some(data) = data {
                    if let Some(addr) = address {
                        set_addr(
                            Address::from(addr),
                            data,
                            config,
                            save_config,
                            kick_speed,
                            dribbler_speed,
                        );
                        address = None;
                    } else {
                        address = Some(data);
                    }
                }
                None
            }
            Action::Request => {
                if let Some(addr) = address {
                    let data = get_addr(addr.into(), config, voltage_mutex, has_ball, shutdown);
                    address = None;
                    data
                } else {
                    error!("request for data without address");
                    None
                }
            }
        })
        .await;
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Format)]
enum Address {
    Id,
    RfChannel,
    DribblerSpeed,
    DribblerState,
    Kick,
    Reset,
    LightbarrierState,
    Version,
    EscError,
    ComError,
    KicError,
    BatteryVoltage,
    ResetError,
    HalfDribblerSpeed,
    MaxKickerVoltage,
    Unknown,
}

impl From<u8> for Address {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Id,
            1 | 2 => Self::RfChannel,
            3 => Self::DribblerSpeed,
            4 => Self::DribblerState,
            5 => Self::Kick,
            6 => Self::Reset,
            7 => Self::LightbarrierState,
            8 => Self::Version,
            9 => Self::EscError,
            10 => Self::ComError,
            11 => Self::KicError,
            12 => Self::BatteryVoltage,
            13 => Self::ResetError,
            14 => Self::HalfDribblerSpeed,
            15 => Self::MaxKickerVoltage,
            _ => Self::Unknown,
        }
    }
}

fn set_addr<const SUBS1: usize, const SUBS2: usize>(
    addr: Address,
    data: u8,
    config: &crate::Config<impl RawMutex>,
    save_config: &Signal<impl RawMutex, ()>,
    kick_speed: &Observable<impl RawMutex, crate::KickSpeed, SUBS1>,
    dribbler_speed: &Observable<impl RawMutex, u16, SUBS2>,
) {
    use Address::*;
    match addr {
        Id => {
            config.id.set(data);
            save_config.signal(());
        }
        RfChannel => {
            const RF_BASE: u32 = 2_400;
            const RF_STEP: u32 = 1;
            let frequency = u32::from(data) * RF_STEP + RF_BASE;
            config.rf_frequency.set(frequency);
            save_config.signal(());
        }
        DribblerSpeed => {
            const FACTOR: u16 = u16::MAX / 100;
            config.dribbler_high.set(u16::from(data) * FACTOR);
            save_config.signal(());
        }
        HalfDribblerSpeed => {
            const FACTOR: u16 = u16::MAX / 100;
            config.dribbler_low.set(u16::from(data) * FACTOR);
        }
        DribblerState => match data {
            1 => dribbler_speed.set_if_different(config.dribbler_low.get()),
            2 => dribbler_speed.set_if_different(config.dribbler_high.get()),
            _ => dribbler_speed.set_if_different(0),
        },
        Kick => {
            let duration = u16::from(data) * 50;
            info!("kiking with {}us", duration);
            kick_speed.set(crate::KickSpeed::Raw(duration));
        }
        _ => warn!("unimplemented i2c address {}", addr),
    }
}

fn get_addr<const SUBS: usize>(
    addr: Address,
    config: &crate::Config<impl RawMutex>,
    voltage_mutex: &Mutex<impl RawMutex, U16F16>,
    has_ball: &Observable<impl RawMutex, LightBarrierState, SUBS>,
    shutdown: &Signal<impl RawMutex, ()>,
) -> Option<u8> {
    use Address::*;
    match addr {
        Id => Some(config.id.get()),
        RfChannel => {
            const RF_BASE: u32 = 2_400;
            const RF_STEP: u32 = 1;
            let frequency = config.rf_frequency.get();
            Some(unwrap!(
                u8::try_from(
                    ((frequency - RF_BASE) / RF_STEP).clamp(u32::from(u8::MIN), u32::from(u8::MAX))
                ),
                "clamped"
            ))
        }
        DribblerSpeed => {
            const FACTOR: u16 = u16::MAX / 100;
            Some(unwrap!(
                u8::try_from(config.dribbler_high.get() / FACTOR),
                "is in range 0..=100"
            ))
        }
        HalfDribblerSpeed => {
            const FACTOR: u16 = u16::MAX / 100;
            Some(unwrap!(
                u8::try_from(config.dribbler_low.get() / FACTOR),
                "is in range 0..=100"
            ))
        }
        BatteryVoltage => {
            const FACTOR: U16F16 = U16F16!(5);
            let voltage = *block_on(voltage_mutex.lock());
            let voltage = voltage * FACTOR;
            let voltage = voltage.az();
            Some(voltage)
        }
        LightbarrierState => match has_ball.get() {
            LightBarrierState::HasBall => Some(2),
            LightBarrierState::NoBall => Some(0),
            LightBarrierState::ContactLost => Some(1),
        },
        Reset => {
            shutdown.signal(());
            Some(1)
        }
        _ => {
            warn!("inimplemented i2c address requested {}", addr);
            None
        }
    }
}
