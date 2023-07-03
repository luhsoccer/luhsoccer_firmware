use crc::{Crc, CRC_32_ISO_HDLC};
use defmt::{debug, error, info, warn};
use embassy_executor::task;
use embassy_rp::{
    flash::{self, Flash},
    peripherals::FLASH,
};
use embassy_sync::{
    blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex, RawMutex},
    signal::Signal,
};
use embassy_time::{Duration, Timer};
use fixed::types::{I16F16, I24F8};
use fixed_macro::types::{I16F16, I24F8};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sync::observable::{Observable, Subscriber};
use typenum::consts::{N3, P1, Z0};
use units::{
    types::{MetrePerSquareSecond, RadianPerSquareSecond, Volt},
    SiUnit,
};

use crate::kicker::{ADC_230V_POINT, DAC_230V_POINT};

pub struct Parameter<M: RawMutex, T: Clone, const SUBS: usize> {
    inner: Observable<M, T, SUBS>,
}

impl<M: RawMutex, T: Copy, const SUBS: usize> Parameter<M, T, SUBS> {
    pub const fn new(value: T) -> Self {
        Self {
            inner: Observable::new(value),
        }
    }

    /// Set the parameter. Publisches the new value to all subscribers.
    #[allow(dead_code)]
    pub fn set(&self, value: T) {
        self.inner.set(value);
    }

    /// Read the current parameter value.
    #[allow(dead_code)]
    pub fn get(&self) -> T {
        self.inner.get()
    }

    /// Subscribe to the parameter. The subscriber gets notified when the parameter changes.
    #[allow(dead_code)]
    pub fn sub(&self) -> Option<Subscriber<M, T, SUBS>> {
        self.inner.subscriber().ok()
    }
}

impl<M: RawMutex, T: Copy + Serialize, const SUBS: usize> Serialize for Parameter<M, T, SUBS> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.get().serialize(serializer)
    }
}

impl<'a, M: RawMutex, T: Copy + Deserialize<'a>, const SUBS: usize> Deserialize<'a>
    for Parameter<M, T, SUBS>
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        T::deserialize(deserializer).map(Self::new)
    }
}

type MetrePerCubeSecond<T> = SiUnit<T, N3, P1, Z0, Z0, Z0, Z0, Z0>;
type RadianPerCubeSecond<T> = SiUnit<T, N3, Z0, Z0, Z0, Z0, Z0, Z0>;

#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct ConfigV0<M: RawMutex> {
    pub motor_pid_kp: Parameter<M, I24F8, 1>,
    pub motor_pid_ki: Parameter<M, I24F8, 1>,
    pub motor_pid_kd: Parameter<M, I24F8, 1>,
    pub motor_pid_ilimit: Parameter<M, Option<I24F8>, 1>,
    pub motor_pid_limit: Parameter<M, Option<I24F8>, 1>,
    pub linear_accelleration: Parameter<M, MetrePerSquareSecond<I16F16>, 1>,
    pub angular_accelleration: Parameter<M, RadianPerSquareSecond<I16F16>, 1>,
    pub linear_jerk: Parameter<M, MetrePerCubeSecond<I16F16>, 1>,
    pub angular_jerk: Parameter<M, RadianPerCubeSecond<I16F16>, 1>,
    pub kicker_cap_dac_230v: Parameter<M, u16, 1>,
    pub kicker_cap_adc_230v: Parameter<M, u16, 1>,
    pub kicker_charge_voltage: Parameter<M, Volt<u8>, 1>,
    pub kicker_poli4: Parameter<M, I16F16, 1>,
    pub kicker_poli3: Parameter<M, I16F16, 1>,
    pub kicker_poli2: Parameter<M, I16F16, 1>,
    pub kicker_poli1: Parameter<M, I16F16, 1>,
    pub kicker_poli0: Parameter<M, I16F16, 1>,
}

impl<M: RawMutex> ConfigV0<M> {
    pub const fn new() -> Self {
        Self {
            motor_pid_kp: Parameter::new(I24F8!(2000).unwrapped_div(I24F8::TAU)),
            motor_pid_ki: Parameter::new(I24F8!(200).unwrapped_div(I24F8::TAU)),
            motor_pid_kd: Parameter::new(I24F8!(0).unwrapped_div(I24F8::TAU)),
            motor_pid_ilimit: Parameter::new(Some(I24F8!(14000))),
            motor_pid_limit: Parameter::new(Some(I24F8!(2000).unwrapped_mul(I24F8::TAU))),
            linear_accelleration: Parameter::new(MetrePerSquareSecond::new(I16F16!(7))),
            angular_accelleration: Parameter::new(RadianPerSquareSecond::new(I16F16!(42))),
            linear_jerk: Parameter::new(MetrePerCubeSecond::new(I16F16!(50))),
            angular_jerk: Parameter::new(RadianPerCubeSecond::new(I16F16!(300))),
            kicker_cap_dac_230v: Parameter::new(DAC_230V_POINT),
            kicker_cap_adc_230v: Parameter::new(ADC_230V_POINT),
            #[cfg(not(feature = "lupfer"))]
            kicker_charge_voltage: Parameter::new(Volt::new(200)),
            #[cfg(feature = "lupfer")]
            kicker_charge_voltage: Parameter::new(Volt::new(230)),
            kicker_poli4: Parameter::new(I16F16!(1.74646057)),
            kicker_poli3: Parameter::new(I16F16!(-14.2552025)),
            kicker_poli2: Parameter::new(I16F16!(49.25610639)),
            kicker_poli1: Parameter::new(I16F16!(152.85497417)),
            kicker_poli0: Parameter::new(I16F16!(149.71060934)),
        }
    }
}

impl<M: RawMutex> Default for ConfigV0<M> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
enum ConfigSelection<M: RawMutex> {
    V0(ConfigV0<M>),
}

impl<M: RawMutex> Default for ConfigSelection<M> {
    fn default() -> Self {
        Self::V0(ConfigV0::default())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
struct DiscConfig<M: RawMutex> {
    config: ConfigSelection<M>,
    checksum: u32,
}

impl<M: RawMutex> Default for DiscConfig<M> {
    fn default() -> Self {
        let config = ConfigSelection::default();
        let checksum = Self::calc_checksum(&config);
        Self { config, checksum }
    }
}

impl<M: RawMutex> DiscConfig<M> {
    fn new(config: ConfigSelection<M>) -> Self {
        let checksum = Self::calc_checksum(&config);
        Self { config, checksum }
    }

    fn calc_checksum(config: &ConfigSelection<M>) -> u32 {
        let crc = Crc::<u32>::new(&CRC_32_ISO_HDLC);
        let Ok(config_bytes) = postcard::to_vec::<_, {flash::ERASE_SIZE}>(config) else {
            error!("couldn't convert config using postcard");
            return 0;
        };
        crc.checksum(&config_bytes[..])
    }

    fn valid(&self) -> bool {
        let real_checksum = Self::calc_checksum(&self.config);
        debug!("comparing 0x{:x} with 0x{:x}", self.checksum, real_checksum);
        self.checksum == real_checksum
    }

    const CONFIG_FLASH_LOCATION: u32 = 0x200000;
    fn load_from_flash<const FLASH_SIZE: usize>(
        flash: &mut Flash<impl flash::Instance, FLASH_SIZE>,
    ) -> Option<Self> {
        let mut buf = [0; flash::ERASE_SIZE];
        if flash.read(Self::CONFIG_FLASH_LOCATION, &mut buf).is_err() {
            error!("Couldn't read from flash! Using default config");
            return None;
        }

        let Ok(config) = postcard::from_bytes::<Self>(&buf) else {
            error!("Unable to read config from loaded bytes! Using default config");
            return None;
        };

        if config.valid() {
            Some(config)
        } else {
            error!("Loaded config is not valid! Using default config");
            None
        }
    }

    fn save_to_flash<const FLASH_SIZE: usize>(
        &self,
        flash: &mut Flash<impl flash::Instance, FLASH_SIZE>,
    ) {
        let Ok(buf) = postcard::to_vec::<_, {flash::ERASE_SIZE}>(self) else {
            error!("unable to encode config!");
            return;
        };
        if flash
            .erase(Self::CONFIG_FLASH_LOCATION, {
                Self::CONFIG_FLASH_LOCATION + flash::ERASE_SIZE as u32
            })
            .is_err()
        {
            warn!("unable to erase flash");
        }
        if flash.write(Self::CONFIG_FLASH_LOCATION, &buf[..]).is_err() {
            error!("couldn't write config to flash!");
        }
    }
}

#[task]
pub async fn config_task(
    flash: FLASH,
    config: &'static crate::Config<CriticalSectionRawMutex>,
    save: &'static Signal<CriticalSectionRawMutex, ()>,
) {
    const FLASH_SIZE: usize = 16 * 1024 * 1024; // 16MiB

    // add some delay to give an attached debug probe time to parse the defmt RTT header. Reading
    // that header might touch flash memory, which interferes with flash write operations.
    Timer::after(Duration::from_millis(10)).await;

    let flash = Flash::<_, FLASH_SIZE>::new(flash);
    config_inner(flash, config, save).await;
}

async fn config_inner<
    'd,
    T: flash::Instance,
    M: RawMutex,
    MS: RawMutex,
    const FLASH_SIZE: usize,
>(
    mut flash: Flash<'d, T, FLASH_SIZE>,
    config: &crate::Config<M>,
    save: &Signal<MS, ()>,
) {
    if let Some(disc_config) = DiscConfig::<NoopRawMutex>::load_from_flash(&mut flash) {
        info!("Successfully loaded config");
        update_config(config, &disc_config.config);
    }
    loop {
        save.wait().await;
        let temp_config = clone_config::<NoopRawMutex>(config);
        let disc_config = DiscConfig::new(ConfigSelection::V0(temp_config));
        assert!(disc_config.valid());
        disc_config.save_to_flash(&mut flash);
    }
}

macro_rules! clone_config {
    ($from: ident, $to: ident, $($param: ident),*) => {
        $(
            $to.$param.set($from.$param.get());
        )*
    };
}

fn clone_config<M: RawMutex>(config: &crate::Config<impl RawMutex>) -> crate::Config<M> {
    let res = crate::Config::default();
    clone_config!(
        config,
        res,
        motor_pid_kp,
        motor_pid_ki,
        motor_pid_kd,
        motor_pid_ilimit,
        motor_pid_limit,
        linear_accelleration,
        angular_accelleration,
        kicker_cap_dac_230v,
        kicker_cap_adc_230v,
        kicker_poli4,
        kicker_poli3,
        kicker_poli2,
        kicker_poli1,
        kicker_poli0
    );
    res
}

fn update_config(
    config: &crate::Config<impl RawMutex>,
    disc_config: &ConfigSelection<impl RawMutex>,
) {
    match disc_config {
        ConfigSelection::V0(config_v0) => {
            clone_config!(
                config_v0,
                config,
                motor_pid_kp,
                motor_pid_ki,
                motor_pid_kd,
                motor_pid_ilimit,
                motor_pid_limit,
                linear_accelleration,
                angular_accelleration,
                kicker_cap_dac_230v,
                kicker_cap_adc_230v,
                kicker_poli4,
                kicker_poli3,
                kicker_poli2,
                kicker_poli1,
                kicker_poli0
            );
        }
    }
}
