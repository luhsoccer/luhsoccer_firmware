use intra_comms::definitions::{
    BallState, BasestationToRobot, CameraVelocity, DribblerSpeedSelection, DribblerState,
    GameState, KickSelection, KickSpeedSelection, KickerChargeHint, LocalVelocity,
    MovementSelection, Position, RobotToBasestation, Team,
};
use protobuf::proto::luhsoccer::{
    self, from_basestation_packet::VelocityFeedback, TristateDribblerMode,
};

fn convert_speed(vel: f32) -> i16 {
    (vel * 1000.0) as i16
}

fn convert_rads(rads: f32) -> i16 {
    (rads * 1024.0) as i16
}

fn convert_rad(rads: f32) -> u16 {
    (rads * 1024.0) as u16
}

pub fn parse_server_to_base_station(
    packet: luhsoccer::ToBasestationPacket,
) -> Option<BasestationToRobot> {
    let id = packet.id as u8;
    let team = match packet.team_color() {
        luhsoccer::TeamColor::Blue => Team::Blue,
        luhsoccer::TeamColor::Yellow => Team::Yellow,
    };

    let movement = match packet.movement? {
        luhsoccer::to_basestation_packet::Movement::LocalVelocity(local_vel) => {
            MovementSelection::RobotVelocity(LocalVelocity {
                forward: convert_speed(local_vel.forward),
                left: convert_speed(local_vel.left),
                counterclockwise: convert_rads(local_vel.counter_clockwise),
            })
        }
        luhsoccer::to_basestation_packet::Movement::GlobalVelocity(global_vel) => {
            MovementSelection::CameraVelocity(CameraVelocity {
                x: convert_speed(global_vel.x),
                y: convert_speed(global_vel.y),
                counterclockwise: convert_rads(global_vel.counter_clockwise),
            })
        }
        luhsoccer::to_basestation_packet::Movement::GlobalPosition(global_pos) => {
            MovementSelection::Position(Position {
                x: convert_speed(global_pos.x),
                y: convert_speed(global_pos.y),
                theta: convert_rad(global_pos.theta),
            })
        }
    };

    // Kicker info is a required field
    let kicker_info = packet.kicker_info?;

    let kicker_charge_hint = match kicker_info.charge_hint() {
        luhsoccer::ChargeHint::Charge => KickerChargeHint::Charge,
        luhsoccer::ChargeHint::Discharge => KickerChargeHint::Discharge,
        luhsoccer::ChargeHint::DontCare => KickerChargeHint::DontCare,
    };

    // Kicking speed is a required field
    let kick_speed = match kicker_info.kicking_speed.clone()? {
        // todo this clone needed?
        luhsoccer::kicker_info::KickingSpeed::Relative(vel) => {
            KickSpeedSelection::Relative(convert_speed(vel).try_into().ok()?)
        }
        luhsoccer::kicker_info::KickingSpeed::Absolute(vel) => {
            KickSpeedSelection::Absolute(convert_speed(vel).try_into().ok()?)
        }
    };

    let kick_type = match kicker_info.mode() {
        luhsoccer::KickerMode::Kick => KickSelection::Kick,
        luhsoccer::KickerMode::Chip => KickSelection::Chip,
    };

    // Dribbler info is a required field
    let dribbler_info = packet.dribbler_info?;

    // Dribbler speed is a required field
    let dribbler_speed = match dribbler_info.dribber_mode? {
        luhsoccer::dribbler_info::DribberMode::Percent(per) => {
            DribblerSpeedSelection::Percent(per as u8)
        }
        luhsoccer::dribbler_info::DribberMode::Rpm(rpm) => DribblerSpeedSelection::Rpm(rpm as u16),
        luhsoccer::dribbler_info::DribberMode::TristateMode(mode) => {
            let state = match TristateDribblerMode::from_i32(mode)? {
                TristateDribblerMode::Off => DribblerState::Off,
                TristateDribblerMode::Half => DribblerState::Half,
                TristateDribblerMode::Full => DribblerState::Full,
            };
            DribblerSpeedSelection::Tristate(state)
        }
    };

    Some(BasestationToRobot {
        id,
        team,
        movement,
        kicker_charge_hint,
        kick_speed,
        kick_type,
        dribbler_speed,
        robot_position: None,
        game_state: GameState::Normal,
        time_sync: None,
    })
}

pub fn parse_base_station_to_server(
    packet: RobotToBasestation,
    rssi_basestation: i32,
    measured_rtt: u32,
) -> luhsoccer::FromBasestationPacket {
    let id = packet.id as u32;
    let team_color = match packet.team {
        Team::Blue => luhsoccer::TeamColor::Blue,
        Team::Yellow => luhsoccer::TeamColor::Yellow,
    } as i32;
    let battery_voltage = packet.battery_voltage as f32 / 8.0;
    let kicker_voltage = packet.kicker_voltage as f32;
    let has_ball = match packet.has_ball {
        BallState::InDribbler => true,
        BallState::NotInDribbler => false,
    };
    let error_code = packet.error as u32;
    let rssi_robot = -(packet.rssi as i32);
    let firmware_version = Some(luhsoccer::FirmwareVersion {
        major: packet.firmware_version.major as u32,
        minor: packet.firmware_version.minor as u32,
        patch: packet.firmware_version.patch as u32,
    });

    // TODO: Fix typo in protobuf
    let velocity_feedback = match packet.velocity {
        Some(vel) => match vel {
            intra_comms::definitions::VelocitySelection::RobotVelocity(vel) => Some(
                VelocityFeedback::LocalVelocity(luhsoccer::LocalVelocityFeedback {
                    forward: vel.forward as f32 / 1000.0,
                    left: vel.left as f32 / 1000.0,
                    counter_clockwise: vel.counterclockwise as f32 / 1024.0,
                }),
            ),
            intra_comms::definitions::VelocitySelection::CameraVelocity(vel) => Some(
                VelocityFeedback::GlobalVelocity(luhsoccer::GlobalVelocityFeedback {
                    x: vel.x as f32 / 1000.0,
                    y: vel.y as f32 / 1000.0,
                    counter_clockwise: vel.counterclockwise as f32 / 1024.0,
                }),
            ),
        },
        None => None,
    };

    luhsoccer::FromBasestationPacket {
        id,
        team_color,
        battery_voltage,
        kicker_voltage,
        has_ball,
        error_code,
        battery_current: None,
        battery_capacity_used: None,
        rssi_robot,
        rssi_basestation,
        global_position: None,
        feedback_time: 0,
        firmware_version,
        measured_rtt,
        velocity_feedback,
    }
}
