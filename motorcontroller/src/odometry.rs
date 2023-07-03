use core::ops::{Add, Sub};

use az::{Az, SaturatingAs};
use defmt::{debug, error, info, trace, warn};
use defmt::{unwrap, Format};
use embassy_embedded_hal::shared_bus;
use embassy_executor::{task, Spawner};
use embassy_futures::join::{join3, join4, join5};
use embassy_rp::{
    gpio::{Level, Output},
    peripherals::{DMA_CH0, DMA_CH1, PIN_22, PIN_23, PIN_24, PIN_25, PIN_26, PIN_27, PIN_28, SPI1},
    spi::{self, Spi},
};
use embassy_sync::{
    blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex, RawMutex},
    mutex::Mutex,
};
#[cfg(feature = "test_motors")]
use embassy_time::Timer;
use embassy_time::{Delay, Duration, Ticker};
use embedded_hal::spi::{Phase, Polarity};
use embedded_hal_async::spi::SpiDevice;
use fixed::types::{I16F16, I24F8};
use fixed_macro::types::{I16F16, I24F8};
use fugit::ExtU32;
use nalgebra::{matrix, Matrix3x4, Matrix4x3};
use pidcontroller::{Controller as _, PIDController};
use static_cell::StaticCell;
use sync::observable::Observable;
use tmc4671::{
    commands::{Direction, ModeMotion, MotorType, PhiESelectionType, PwmChopperMode},
    nonblocking::Controller,
};
use typenum::{
    consts::{N1, N2, N3, Z0},
    Integer,
};
use units::SiUnit;
use units::{
    prelude::*,
    types::{MetrePerSecond, MetrePerSquareSecond, RadianPerSecond, RadianPerSquareSecond, Second},
};

use crate::Config;

#[task]
async fn config_proxy(
    source: &'static Config<CriticalSectionRawMutex>,
    target: &'static Config<NoopRawMutex>,
) {
    macro_rules! proxy {
        ($param: ident) => {
            async {
                let mut sub = source.$param.sub().expect("To few subscriber slots");
                loop {
                    target.$param.set(sub.next_value().await)
                }
            }
        };
    }
    let a = join5(
        proxy!(motor_pid_kp),
        proxy!(motor_pid_ki),
        proxy!(motor_pid_kd),
        proxy!(motor_pid_ilimit),
        proxy!(motor_pid_limit),
    );
    join3(
        proxy!(linear_accelleration),
        proxy!(angular_accelleration),
        a,
    )
    .await;
}

#[task]
pub async fn calc_robot_speed_task(
    wheel_speeds: &'static Observable<CriticalSectionRawMutex, [RadianPerSecond<I24F8>; 4], 8>,
    robot_velocity: &'static Observable<CriticalSectionRawMutex, Movement, 8>,
) {
    let mut actual_speeds_sub = unwrap!(wheel_speeds.subscriber());
    loop {
        let wheel_speeds = actual_speeds_sub.next_value().await;
        let new_robot_velocity = calculate_velocity(wheel_speeds);
        robot_velocity.set(new_robot_velocity);
    }
}

#[task]
#[allow(clippy::too_many_arguments)]
pub async fn motors_task(
    spi: SPI1,
    clk: PIN_26,
    miso: PIN_28,
    mosi: PIN_27,
    chipselects: (PIN_24, PIN_22, PIN_25, PIN_23),
    dma_tx: DMA_CH0,
    dma_rx: DMA_CH1,
    setpoint: &'static Observable<CriticalSectionRawMutex, Movement, 8>,
    actual_speeds: &'static Observable<CriticalSectionRawMutex, [RadianPerSecond<I24F8>; 4], 8>,
    config: &'static Config<CriticalSectionRawMutex>,
    spawner: Spawner,
) {
    info!("Motors starting");
    let cs0 = Output::new(chipselects.0, Level::High);
    let cs1 = Output::new(chipselects.1, Level::High);
    let cs2 = Output::new(chipselects.2, Level::High);
    let cs3 = Output::new(chipselects.3, Level::High);

    let mut spi_config = spi::Config::default();
    spi_config.frequency = 2_000_000;
    spi_config.polarity = Polarity::IdleHigh;
    spi_config.phase = Phase::CaptureOnSecondTransition;
    let spi = Spi::new(spi, clk, mosi, miso, dma_tx, dma_rx, spi_config);
    let bus_mutex = Mutex::<NoopRawMutex, _>::new(spi);
    let dev0 = shared_bus::asynch::spi::SpiDevice::new(&bus_mutex, cs0);
    let dev1 = shared_bus::asynch::spi::SpiDevice::new(&bus_mutex, cs1);
    let dev2 = shared_bus::asynch::spi::SpiDevice::new(&bus_mutex, cs2);
    let dev3 = shared_bus::asynch::spi::SpiDevice::new(&bus_mutex, cs3);
    static PROXY_CONFIG: StaticCell<Config<NoopRawMutex>> = StaticCell::new();
    let proxy_config_ref = PROXY_CONFIG.init(Config::default());
    spawner.must_spawn(config_proxy(config, proxy_config_ref));

    let mut drivetrain = Drivetrain::new(dev0, dev1, dev2, dev3);
    if drivetrain.init().await.is_ok() {
        debug!("initialized motors");
        drivetrain
            .run(setpoint, actual_speeds, proxy_config_ref)
            .await;
    }
    error!("couldn't initialize motors. Disabling");
}

#[cfg(feature = "test_motors")]
#[task]
pub async fn motors_test_task(
    movement_setpoint: &'static Observable<CriticalSectionRawMutex, Movement, 8>,
) {
    Timer::after(Duration::from_secs(10)).await;
    let movements = [
        Movement {
            forward: MetrePerSecond::new(0.1.az()),
            left: MetrePerSecond::new(0.az()),
            counterclockwise: RadianPerSecond::new(0.az()),
        },
        Movement {
            forward: MetrePerSecond::new((-0.1).az()),
            left: MetrePerSecond::new(0.az()),
            counterclockwise: RadianPerSecond::new(0.az()),
        },
        Movement {
            forward: MetrePerSecond::new(0.az()),
            left: MetrePerSecond::new(0.1.az()),
            counterclockwise: RadianPerSecond::new(0.az()),
        },
        Movement {
            forward: MetrePerSecond::new(0.az()),
            left: MetrePerSecond::new((-0.1).az()),
            counterclockwise: RadianPerSecond::new(0.az()),
        },
        Movement {
            forward: MetrePerSecond::new(0.az()),
            left: MetrePerSecond::new(0.az()),
            counterclockwise: RadianPerSecond::new((core::f32::consts::TAU / 10.0).az()),
        },
        Movement {
            forward: MetrePerSecond::new(0.az()),
            left: MetrePerSecond::new(0.az()),
            counterclockwise: RadianPerSecond::new((-core::f32::consts::TAU / 10.0).az()),
        },
        // faster
        Movement {
            forward: MetrePerSecond::new(1.az()),
            left: MetrePerSecond::new(0.az()),
            counterclockwise: RadianPerSecond::new(0.az()),
        },
        Movement {
            forward: MetrePerSecond::new((-1).az()),
            left: MetrePerSecond::new(0.az()),
            counterclockwise: RadianPerSecond::new(0.az()),
        },
        Movement {
            forward: MetrePerSecond::new(0.az()),
            left: MetrePerSecond::new(1.az()),
            counterclockwise: RadianPerSecond::new(0.az()),
        },
        Movement {
            forward: MetrePerSecond::new(0.az()),
            left: MetrePerSecond::new((-1).az()),
            counterclockwise: RadianPerSecond::new(0.az()),
        },
        Movement {
            forward: MetrePerSecond::new(0.az()),
            left: MetrePerSecond::new(0.az()),
            counterclockwise: RadianPerSecond::new(core::f32::consts::TAU.az()),
        },
        Movement {
            forward: MetrePerSecond::new(0.az()),
            left: MetrePerSecond::new(0.az()),
            counterclockwise: RadianPerSecond::new((-core::f32::consts::TAU).az()),
        },
    ];
    loop {
        for movement in movements {
            movement_setpoint.set(movement);
            info!("driving {} for 5s", movement);
            debug!(
                "{}",
                calculate_wheel_speeds(movement).map(|v| v.raw().az::<f32>())
            );
            Timer::after(Duration::from_secs(5)).await;

            movement_setpoint.set(Movement::new());
            info!("stopping for 5s");
            Timer::after(Duration::from_secs(5)).await;
        }
    }
}

struct Motor<S: SpiDevice> {
    motor: Controller<S>,
    regulator: PIDController<I24F8>,
    direction: Direction,
}

impl<S> Motor<S>
where
    S: SpiDevice,
{
    fn new(spi_device: S) -> Self {
        Self {
            motor: Controller::new(spi_device),
            regulator: PIDController::new(),
            direction: Direction::Positive,
        }
    }

    async fn init(&mut self) -> Result<(), tmc4671::nonblocking::Error<S::Error>> {
        info!("initializing motor");
        #[cfg(debug_assertions)]
        {
            debug!(
                "motordriver hw type: {:?}",
                self.motor.hardware_type().await.ok()
            );
            debug!(
                "motordriver hw version: {:?}",
                self.motor.hardware_version().await.ok()
            );
            debug!(
                "motordriver hw date: {:?}",
                self.motor.hardware_date().await.ok()
            );
            debug!(
                "motordriver hw time: {:?}",
                self.motor.hardware_time().await.ok()
            );
            debug!(
                "motordriver hw variant: {:?}",
                self.motor.hardware_variant().await.ok()
            );
            debug!(
                "motordriver hw build: {:?}",
                self.motor.hardware_build().await.ok()
            );
        }

        // initialize general configuration registers
        trace!("initializing motor registers");
        self.motor
            .set_motor_type_pole_pairs((MotorType::ThreePhaseBldc, 8))
            .await?;
        self.motor.set_mode(ModeMotion::Stopped).await?;
        self.motor.set_bbm(100u32.nanos()).await?;
        self.motor.set_pwm_mode(PwmChopperMode::Centered).await?;

        self.init_encoder().await?;

        info!("successfully initialized motor");
        self.motor.set_mode(ModeMotion::UqUdExt).await
    }

    async fn init_encoder(&mut self) -> Result<(), tmc4671::nonblocking::Error<S::Error>> {
        // initialize decoder
        trace!("initializing motor encoder");
        self.motor.set_decoder_ppr(4000).await?;
        debug!("testing positive encoder direction");
        self.direction = Direction::Positive;
        self.motor.set_decoder_direction(self.direction).await?;
        if let Err(e) = self.motor.calibrate_encoder(4000, &mut Delay, 20).await {
            match e {
                tmc4671::nonblocking::Error::Spi(_) => todo!(),
                tmc4671::nonblocking::Error::Deserialization(_) => todo!(),
                tmc4671::nonblocking::Error::CalibrationValidation => {
                    debug!("testing negative encoder direction");
                    self.direction = Direction::Negative;
                    self.motor.set_decoder_direction(self.direction).await?;
                    self.motor.calibrate_encoder(4000, &mut Delay, 20).await?
                }
            }
        }
        self.motor
            .set_phi_e_selection(PhiESelectionType::PhiEAbn)
            .await?;
        Ok(())
    }

    async fn regulate(
        &mut self,
        target: RadianPerSecond<I24F8>,
    ) -> Result<RadianPerSecond<I24F8>, tmc4671::nonblocking::Error<S::Error>> {
        let target = match self.direction {
            Direction::Positive => target,
            Direction::Negative => -target,
        };
        self.regulator.set_target(&target.raw());
        let velocity = self.get_speed().await?;
        let torque = self.regulator.regulate(&velocity.raw());
        self.motor
            .set_openloop_torque_flux((torque.az(), 0))
            .await?;
        Ok(match self.direction {
            Direction::Positive => velocity,
            Direction::Negative => -velocity,
        })
    }

    async fn get_speed(
        &mut self,
    ) -> Result<RadianPerSecond<I24F8>, tmc4671::nonblocking::Error<S::Error>> {
        const POLE_PAIRS: I24F8 = I24F8!(8);
        const MINUTE: I24F8 = I24F8!(60);
        const FACTOR: I24F8 = POLE_PAIRS.unwrapped_mul(MINUTE).unwrapped_div(I24F8::TAU);
        let velocity = self.motor.velocity().await?;
        Ok(RadianPerSecond::new(velocity.az::<I24F8>() / FACTOR))
    }

    async fn full_stop(&mut self) {
        trace!("starting full stop for motor");
        if (self.motor.set_mode(ModeMotion::Stopped).await).is_err() {
            error!("setting motor to stopped mode");
        }
        trace!("setting freerunning mode");
        if (self
            .motor
            .set_pwm_mode(PwmChopperMode::OffFreeRunning)
            .await)
            .is_err()
        {
            error!("setting motor pwm to free running");
        }
    }
}

struct Drivetrain<S0, S1, S2, S3>
where
    S0: SpiDevice,
    S1: SpiDevice,
    S2: SpiDevice,
    S3: SpiDevice,
{
    motors: (Motor<S0>, Motor<S1>, Motor<S2>, Motor<S3>),
}

impl<S0, S1, S2, S3> Drivetrain<S0, S1, S2, S3>
where
    S0: SpiDevice,
    S1: SpiDevice<Error = S0::Error>,
    S2: SpiDevice<Error = S0::Error>,
    S3: SpiDevice<Error = S0::Error>,
{
    pub fn new(dev0: S0, dev1: S1, dev2: S2, dev3: S3) -> Self {
        Self {
            motors: (
                Motor::new(dev0),
                Motor::new(dev1),
                Motor::new(dev2),
                Motor::new(dev3),
            ),
        }
    }

    async fn init(&mut self) -> Result<(), ()> {
        info!("initializing drivetrain");
        let results = join4(
            async {
                self.motors.0.init().await.map_err(|_| {
                    error!("unable to initialize motor 0");
                })
            },
            async {
                self.motors.1.init().await.map_err(|_| {
                    error!("unable to initialize motor 1");
                })
            },
            async {
                self.motors.2.init().await.map_err(|_| {
                    error!("unable to initialize motor 2");
                })
            },
            async {
                self.motors.3.init().await.map_err(|_| {
                    error!("unable to initialize motor 3");
                })
            },
        )
        .await;
        let result = results.0.and(results.1).and(results.2).and(results.3);
        if result.is_err() {
            warn!("stopping all motors");
            self.motors.0.full_stop().await;
            self.motors.1.full_stop().await;
            self.motors.2.full_stop().await;
            self.motors.3.full_stop().await;
        }
        result
    }

    async fn run<const SUBS1: usize, const SUBS2: usize>(
        &mut self,
        setpoint: &Observable<impl RawMutex, Movement, SUBS1>,
        actual_speeds: &Observable<impl RawMutex, [RadianPerSecond<I24F8>; 4], SUBS2>,
        config: &Config<impl RawMutex>,
    ) {
        macro_rules! set_all {
            ($prop: ident = $value: expr) => {
                let value = $value;
                self.motors.0.regulator.$prop = value;
                self.motors.1.regulator.$prop = value;
                self.motors.2.regulator.$prop = value;
                self.motors.3.regulator.$prop = value;
            };
        }
        const CONTROL_RATE: u32 = 1_000;
        const CONTROL_DURATION: Second<I16F16> =
            Second::new(I16F16::const_from_int(CONTROL_RATE as i32).recip());

        info!("regulating drivetrain");
        let mut current_velocity = Movement::new();
        let mut current_accelleration = (
            MetrePerSquareSecond::new(I16F16!(0)),
            MetrePerSquareSecond::new(I16F16!(0)),
            RadianPerSquareSecond::new(I16F16!(0)),
        );
        let mut ticker = Ticker::every(Duration::from_hz(u64::from(CONTROL_RATE)));

        loop {
            // get current config values
            set_all!(p_gain = config.motor_pid_kp.get());
            set_all!(i_gain = config.motor_pid_ki.get());
            set_all!(d_gain = config.motor_pid_kd.get());
            set_all!(i_sum_limit = config.motor_pid_ilimit.get());
            set_all!(limit = config.motor_pid_limit.get());
            let max_linear_accelleration = config.linear_accelleration.get();
            let max_angular_accelleration = config.angular_accelleration.get();
            let max_linear_jerk = config.linear_jerk.get();
            let max_angular_jerk = config.angular_jerk.get();

            // calculate the error in velocity
            let velocity_error = setpoint.get() - current_velocity;

            // calculate the new accelleration
            current_accelleration.0 = calc_accelleration(
                current_accelleration.0,
                velocity_error.forward,
                max_linear_jerk,
                max_linear_accelleration,
                CONTROL_DURATION,
            );
            current_accelleration.1 = calc_accelleration(
                current_accelleration.1,
                velocity_error.left,
                max_linear_jerk,
                max_linear_accelleration,
                CONTROL_DURATION,
            );
            current_accelleration.2 = calc_accelleration(
                current_accelleration.2,
                velocity_error.counterclockwise,
                max_angular_jerk,
                max_angular_accelleration,
                CONTROL_DURATION,
            );

            current_velocity.forward += current_accelleration.0 * CONTROL_DURATION;
            current_velocity.left += current_accelleration.1 * CONTROL_DURATION;
            current_velocity.counterclockwise += current_accelleration.2 * CONTROL_DURATION;

            let motor_speeds = calculate_wheel_speeds(current_velocity);

            let results = (
                self.motors.0.regulate(motor_speeds[0]).await,
                self.motors.1.regulate(motor_speeds[1]).await,
                self.motors.2.regulate(motor_speeds[2]).await,
                self.motors.3.regulate(motor_speeds[3]).await,
            );
            match results {
                (Ok(speed1), Ok(speed2), Ok(speed3), Ok(speed4)) => {
                    actual_speeds.set_if_different([speed1, speed2, speed3, speed4])
                }
                _ => break,
            }
            ticker.next().await;
        }
        warn!("stopping all motors");
        self.motors.0.full_stop().await;
        self.motors.1.full_stop().await;
        self.motors.2.full_stop().await;
        self.motors.3.full_stop().await;
    }
}

/// Calculates jerk limited accelleration.
///
/// # Arguments
///
/// * `accelleration` - Current accelleration
/// * `velocity_error` - The difference between the target velocity and the current velocity
/// (target - current)
/// * `jerk` - Maximum jerk that is applied
/// * `control_duration` - How much time passed since the last calculation
/// * `max_accelleration` - Maximum accelleration that is applied
///
/// # Function
///
/// This calculates the correct accelleration for a jerk limited velocity path
/// ```text
/// ^ jerk
/// |  ___
/// | |   |
/// | |   |
/// | |   |
/// | |   |
/// ----------------------->
/// |               |   |
/// |               |   |
/// |               |   |
/// |               |___|
/// |               
///
/// ^ accelleration
/// |     ___________
/// |    /           \
/// |   /             \
/// |  /               \
/// | /                 \
/// ----------------------->
///   | A |    B    | C |
/// ```
fn calc_accelleration<N>(
    accelleration: SiUnit<I16F16, N2, N, Z0, Z0, Z0, Z0, Z0>,
    velocity_error: SiUnit<I16F16, N1, N, Z0, Z0, Z0, Z0, Z0>,
    jerk: SiUnit<I16F16, N3, N, Z0, Z0, Z0, Z0, Z0>,
    max_accelleration: SiUnit<I16F16, N2, N, Z0, Z0, Z0, Z0, Z0>,
    control_duration: Second<I16F16>,
) -> SiUnit<I16F16, N2, N, Z0, Z0, Z0, Z0, Z0>
where
    N: Integer + PartialOrd + Add + Ord,
    <N as Add>::Output: Integer + Sub<N>,
    <<N as Add>::Output as Sub<N>>::Output: Integer + PartialOrd,
{
    // The velocity error after which the accelleration needs to go towards 0.
    let velocity_margin = accelleration * accelleration / (jerk * I16F16!(2));
    // Check if we reached the velocity margin.
    if velocity_error.raw().abs().into_unit() <= velocity_margin {
        // We need to go towards 0. This is done by adding or subtracting jerk depending on the
        // sign. The resulting accelleration is clamped at 0 so it doesn't swing around the desired
        // target.
        // This is C in the diagram
        if accelleration < I16F16!(0).into_unit() {
            (accelleration + jerk * control_duration).min(I16F16!(0).into_unit())
        } else {
            (accelleration - jerk * control_duration).max(I16F16!(0).into_unit())
        }
    } else {
        // We don't need to go towards 0 accelleration jet. We try to increase the accelleration
        // and clamp it at the maximum
        // This is A or B in the diagram.
        if velocity_error < I16F16!(0).into_unit() {
            (accelleration - jerk * control_duration).max(-max_accelleration)
        } else {
            (accelleration + jerk * control_duration).min(max_accelleration)
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Movement {
    pub forward: MetrePerSecond<I16F16>,
    pub left: MetrePerSecond<I16F16>,
    pub counterclockwise: RadianPerSecond<I16F16>,
}

impl Sub for Movement {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            forward: self.forward - rhs.forward,
            left: self.left - rhs.left,
            counterclockwise: self.counterclockwise - rhs.counterclockwise,
        }
    }
}

impl Format for Movement {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(
            fmt,
            "Movement {{ forward: {=f32}m/s, left: {=f32}m/s, counterclockwise: {=f32}rad/s }}",
            self.forward.raw().az(),
            self.left.raw().az(),
            self.counterclockwise.raw().az()
        );
    }
}

impl Movement {
    pub const fn new() -> Self {
        Self {
            forward: MetrePerSecond::new(I16F16::ZERO),
            left: MetrePerSecond::new(I16F16::ZERO),
            counterclockwise: RadianPerSecond::new(I16F16::ZERO),
        }
    }
}

const fn deg2rad(degree: i32) -> I16F16 {
    I16F16::const_from_int(degree)
        .unwrapped_mul(I16F16::TAU)
        .unwrapped_div(I16F16::const_from_int(360))
}

#[allow(unused)]
const FRONT_WHEELS_ANGLE: I16F16 = deg2rad(30);
const SIN_FRONT_WHEELS_ANGLE: I16F16 = I16F16!(0.5);
const COS_FRONT_WHEELS_ANGLE: I16F16 = I16F16::SQRT_3.unwrapped_div_int(2);
#[allow(unused)]
const BACK_WHEELS_ANGLE: I16F16 = deg2rad(45);
const SIN_BACK_WHEELS_ANGLE: I16F16 = I16F16::FRAC_1_SQRT_2;
const COS_BACK_WHEELS_ANGLE: I16F16 = I16F16::FRAC_1_SQRT_2;
const ROBOT_RADIUS: I16F16 = I16F16!(0.08);
const WHEEL_RADIUS: I16F16 = I16F16!(0.031);

fn calculate_wheel_speeds(movement: Movement) -> [RadianPerSecond<I24F8>; 4] {
    // See https://wiki.roboteamtwente.nl/technical/control/omnidirectional for more info
    // This Matrix is D in the wiki. It is changed a bit to include the division by the
    // WHEEL_RADIUS and the first and second column are switched because our coordinate system has +x
    // forward and +y left. Additionally the sign of the second column has been switched because it
    // is left not right. The Rows have been rearanged to fit our motor configuration ([m4, m3, m1,
    // m2])
    const VELOCITY_COUPLING: Matrix4x3<I16F16> = matrix![
        COS_BACK_WHEELS_ANGLE.unwrapped_div(WHEEL_RADIUS)                 , SIN_BACK_WHEELS_ANGLE.unwrapped_neg().unwrapped_div(WHEEL_RADIUS) , ROBOT_RADIUS.unwrapped_div(WHEEL_RADIUS);
        COS_BACK_WHEELS_ANGLE.unwrapped_neg().unwrapped_div(WHEEL_RADIUS) , SIN_BACK_WHEELS_ANGLE.unwrapped_neg().unwrapped_div(WHEEL_RADIUS) , ROBOT_RADIUS.unwrapped_div(WHEEL_RADIUS);
        COS_FRONT_WHEELS_ANGLE.unwrapped_div(WHEEL_RADIUS)                , SIN_FRONT_WHEELS_ANGLE.unwrapped_div(WHEEL_RADIUS)                , ROBOT_RADIUS.unwrapped_div(WHEEL_RADIUS);
        COS_FRONT_WHEELS_ANGLE.unwrapped_neg().unwrapped_div(WHEEL_RADIUS), SIN_FRONT_WHEELS_ANGLE.unwrapped_div(WHEEL_RADIUS)                , ROBOT_RADIUS.unwrapped_div(WHEEL_RADIUS);
    ];

    let local_velocity = matrix![
        movement.forward.raw();
        movement.left.raw();
        movement.counterclockwise.raw()
    ];

    let wheel_velocities = VELOCITY_COUPLING * local_velocity;
    [
        RadianPerSecond::new(wheel_velocities[0].az()),
        RadianPerSecond::new(wheel_velocities[1].az()),
        RadianPerSecond::new(wheel_velocities[2].az()),
        RadianPerSecond::new(wheel_velocities[3].az()),
    ]
}

fn calculate_velocity(wheel_speeds: [RadianPerSecond<I24F8>; 4]) -> Movement {
    const SIN_FRONT_SIN_BACK: I16F16 = SIN_BACK_WHEELS_ANGLE.unwrapped_add(SIN_FRONT_WHEELS_ANGLE);
    const COS_FRONT_COS_BACK_SQUARED: I16F16 = COS_BACK_WHEELS_ANGLE
        .unwrapped_mul(COS_BACK_WHEELS_ANGLE)
        .unwrapped_add(COS_FRONT_WHEELS_ANGLE.unwrapped_mul(COS_FRONT_WHEELS_ANGLE));
    const LEFT_FACTOR: I16F16 = WHEEL_RADIUS
        .unwrapped_div(SIN_FRONT_SIN_BACK)
        .unwrapped_div(I16F16!(2));
    const FRONT_FACTOR: I16F16 = COS_FRONT_COS_BACK_SQUARED
        .unwrapped_mul(I16F16!(2))
        .unwrapped_div(WHEEL_RADIUS);
    const ROTATION_FACTOR: I16F16 = SIN_FRONT_SIN_BACK
        .unwrapped_mul(I16F16!(2))
        .unwrapped_mul(ROBOT_RADIUS)
        .unwrapped_div(WHEEL_RADIUS);
    // See https://wiki.roboteamtwente.nl/technical/control/omnidirectional for more info
    // The Matrix is Dt in the wiki. It is changed a bit to include the multiplication by the
    // WHEEL_RADIUS and the first and second row are switched because our coordinate system has +x
    // forward and +y left. Additionally the sign of the second row has been switched because it is
    // left not right. The columns have been rearranged to fit our motor configuration ([m4, m3,
    // m1, m2])
    const PSEUDO_INVERSE: Matrix3x4<I16F16> = matrix![
        COS_BACK_WHEELS_ANGLE.unwrapped_div(FRONT_FACTOR), COS_BACK_WHEELS_ANGLE.unwrapped_div(FRONT_FACTOR).unwrapped_neg(), COS_FRONT_WHEELS_ANGLE.unwrapped_div(FRONT_FACTOR), COS_FRONT_WHEELS_ANGLE.unwrapped_div(FRONT_FACTOR).unwrapped_neg();
        LEFT_FACTOR.unwrapped_neg(), LEFT_FACTOR.unwrapped_neg(), LEFT_FACTOR, LEFT_FACTOR;
        SIN_FRONT_WHEELS_ANGLE.unwrapped_div(ROTATION_FACTOR), SIN_FRONT_WHEELS_ANGLE.unwrapped_div(ROTATION_FACTOR), SIN_BACK_WHEELS_ANGLE.unwrapped_div(ROTATION_FACTOR), SIN_BACK_WHEELS_ANGLE.unwrapped_div(ROTATION_FACTOR);
    ];

    let wheel_speeds = matrix![
        wheel_speeds[0].raw().saturating_as();
        wheel_speeds[1].raw().saturating_as();
        wheel_speeds[2].raw().saturating_as();
        wheel_speeds[3].raw().saturating_as();
    ];

    let local_velocity = PSEUDO_INVERSE * wheel_speeds;
    Movement {
        forward: MetrePerSecond::new(local_velocity[0]),
        left: MetrePerSecond::new(local_velocity[1]),
        counterclockwise: RadianPerSecond::new(local_velocity[2]),
    }
}
