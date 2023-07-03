use defmt::warn;
use intra_comms::definitions::{BasestationToRobot, RobotToBasestation};
use protobuf::proto::luhsoccer::{FromBasestationWrapper, ToBasestationWrapper};

use crate::converter;

#[derive(Default)]
pub struct RobotState {
    feedback_seq_id: u32,
    pub send_buffer: [Option<BasestationToRobot>; 16],
    pub receive_buffer: [Option<(RobotToBasestation, i32, u32)>; 16],
}

impl RobotState {
    pub fn update_from_network(&mut self, packet_wrapper: ToBasestationWrapper) {
        for packet in packet_wrapper.packets {
            if let Some(parsed_packet) = converter::parse_server_to_base_station(packet) {
                if parsed_packet.id < 16 {
                    self.send_buffer[parsed_packet.id as usize] = Some(parsed_packet);
                } else {
                    warn!("Invalid robot id");
                }
            } else {
                warn!("Failed to parse packet"); // TODO implement error codes
            }
        }
    }

    pub fn create_network_packet(&mut self) -> Option<FromBasestationWrapper> {
        let mut packet_wrapper = FromBasestationWrapper::default();

        let mut any_feedback = false;
        for packet in &mut self.receive_buffer {
            if let Some((packet, rssi, rtt)) = packet.take() {
                packet_wrapper
                    .packets
                    .push(converter::parse_base_station_to_server(packet, rssi, rtt));
                any_feedback = true;
            }
        }

        self.feedback_seq_id = self.feedback_seq_id.wrapping_add(1);

        packet_wrapper.error_code = 0;
        packet_wrapper.firmware_version = Some(protobuf::proto::luhsoccer::FirmwareVersion {
            major: 0,
            minor: 0,
            patch: 0,
        });
        packet_wrapper.seq_id = self.feedback_seq_id;

        if any_feedback {
            Some(packet_wrapper)
        } else {
            None
        }
    }
}
