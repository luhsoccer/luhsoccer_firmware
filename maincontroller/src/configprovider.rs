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
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sync::observable::{Observable, Subscriber};

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

#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct ConfigV0<M: RawMutex> {
    pub rf_frequency: Parameter<M, u32, 1>,
    pub id: Parameter<M, u8, 1>,
    pub dribbler_low: Parameter<M, u16, 1>,
    pub dribbler_high: Parameter<M, u16, 1>,
    pub lightbarrier_filter_time: Parameter<M, u32, 1>,
}

impl<M: RawMutex> ConfigV0<M> {
    pub const fn new() -> Self {
        Self {
            rf_frequency: Parameter::new(2_400),
            id: Parameter::new(0),
            dribbler_low: Parameter::new(u16::MAX / 20), // 5%
            dribbler_high: Parameter::new(u16::MAX / 10), // 10%
            lightbarrier_filter_time: Parameter::new(200), // ms
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
            match config.config {
                ConfigSelection::V0(configv0) => {
                    configv0.id.set(configv0.id.get().clamp(0, 15));
                    Some(Self {
                        config: ConfigSelection::V0(configv0),
                        checksum: config.checksum,
                    })
                }
            }
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

fn clone_config<M: RawMutex>(config: &crate::Config<impl RawMutex>) -> crate::Config<M> {
    let res = crate::Config::default();
    res.id.set(config.id.get());
    res.rf_frequency.set(config.rf_frequency.get());
    res.dribbler_low.set(config.dribbler_low.get());
    res.dribbler_high.set(config.dribbler_high.get());
    res.lightbarrier_filter_time
        .set(config.lightbarrier_filter_time.get());
    res
}

fn update_config(
    config: &crate::Config<impl RawMutex>,
    disc_config: &ConfigSelection<impl RawMutex>,
) {
    match disc_config {
        ConfigSelection::V0(config_v0) => {
            config.id.set(config_v0.id.get());
            config.rf_frequency.set(config_v0.rf_frequency.get());
            config.dribbler_low.set(config_v0.dribbler_low.get());
            config.dribbler_high.set(config_v0.dribbler_high.get());
            config
                .lightbarrier_filter_time
                .set(config_v0.lightbarrier_filter_time.get());
        }
    }
}
