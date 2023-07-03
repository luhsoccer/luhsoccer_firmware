use defmt::Format;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum Main2Motor {
    Drive(LocalVelocity),
    /// mm/s
    Kick(u16),
    /// mm/s
    Chip(u16),
    /// us
    KickRaw(u16),
    BallInDribbler,
    BallNotInDribbler,
    CalibrateCapVoltage(u8),
    ChargeHint(KickerChargeHint),
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum Motor2Main {
    MotorVelocity(LocalVelocity),
    // V
    CapVoltage(u8),
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize, Format)]
pub struct BasestationToRobot {
    pub id: u8,
    pub team: Team,
    pub movement: MovementSelection,
    pub kicker_charge_hint: KickerChargeHint,
    pub kick_speed: KickSpeedSelection,
    pub kick_type: KickSelection,
    pub dribbler_speed: DribblerSpeedSelection,
    pub robot_position: Option<Position>,
    pub game_state: GameState,
    pub time_sync: Option<TimesyncTimestamp>,
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize, Format)]
pub struct RobotToBasestation {
    pub id: u8,
    pub team: Team,
    /// V * 8
    pub battery_voltage: u8,
    /// V
    pub kicker_voltage: u8,
    pub has_ball: BallState,
    pub error: u8,
    /// A * 8
    pub battery_current: Option<u8>,
    /// mAh / 8
    pub battery_capacity_used: Option<u8>,
    /// -dBm
    pub rssi: u8,
    pub velocity: Option<VelocitySelection>,
    pub position: Option<Position>,
    pub firmware_version: SemVersion,
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize, Format)]
pub enum Team {
    Blue,
    Yellow,
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize, Format)]
pub enum GameState {
    Halt,
    Stop,
    Normal,
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize, Format)]
pub enum MovementSelection {
    RobotVelocity(LocalVelocity),
    CameraVelocity(CameraVelocity),
    Position(Position),
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize, Format)]
pub enum KickerChargeHint {
    Charge,
    Discharge,
    DontCare,
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize, Format)]
pub enum KickSpeedSelection {
    /// mm/s
    Relative(u16),
    /// mm/s
    Absolute(u16),
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize, Format)]
pub enum KickSelection {
    Kick,
    Chip,
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize, Format)]
pub enum DribblerSpeedSelection {
    Tristate(DribblerState),
    Percent(u8),
    Rpm(u16),
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize, Format)]
pub enum DribblerState {
    Off,
    Half,
    Full,
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize, Format)]
pub struct Position {
    /// mm/s
    pub x: i16,
    /// mm/s
    pub y: i16,
    /// rad * 2^12
    pub theta: u16,
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize, Format)]
pub struct TimesyncTimestamp {
    pub seconds: u32,
    pub fraction: u32,
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize, Format)]
pub enum BallState {
    NotInDribbler,
    InDribbler,
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize, Format)]
pub enum VelocitySelection {
    RobotVelocity(LocalVelocity),
    CameraVelocity(CameraVelocity),
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize, Format)]
pub struct SemVersion {
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize, Format)]
pub struct LocalVelocity {
    /// mm/s
    pub forward: i16,
    /// mm/s
    pub left: i16,
    /// rad/s * 2^10
    pub counterclockwise: i16,
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize, Format)]
pub struct CameraVelocity {
    /// mm/s
    pub x: i16,
    /// mm/s
    pub y: i16,
    /// rad/s * 2^10
    pub counterclockwise: i16,
}
