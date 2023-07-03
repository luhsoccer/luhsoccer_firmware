use atsam4_hal::{
    gpio::{Floating, Input, Output, Pa11, Pa22, Pa27, Pa5, Pa8, Pd18, Pd28, PushPull},
    spi::{SpiMaster, SpiU8},
};
use defmt::warn;
use intra_comms::definitions::{BasestationToRobot, RobotToBasestation};
use sky66112::Sky66112;
use sx1280::{SimpleSpiDevice, Sx1280};

use crate::{app::monotonics::Monotonic, robot_state::RobotState};

pub type Transceiver = Sx1280<
    SimpleSpiDevice<SpiMaster<SpiU8>, Pa11<Output<PushPull>>>,
    Pa27<Output<PushPull>>,
    Pd18<Input<Floating>>,
    sx1280::Flrc,
    SpiU8,
    sx1280::Blocking,
>;

pub type Amp = Sky66112<
    sky66112::SleepMode,
    sky66112::TiedHigh,
    Pa5<Output<PushPull>>,
    Pa22<Output<PushPull>>,
    Pd28<Output<PushPull>>,
    sky66112::TiedHigh,
    Pa8<Output<PushPull>>,
>;

pub fn transmit_and_receive_feedback(
    state: &mut RobotState,
    transceiver: &mut Transceiver,
    amp: &mut Option<Amp>,
) {
    for buffer_entry in state.send_buffer.iter_mut() {
        if let Some(packet) = buffer_entry {
            let start = Monotonic::now();
            if let Ok(serialized_packet) = postcard::to_vec::<BasestationToRobot, 127>(packet) {
                transceiver
                    .set_sync_word1(intra_comms::ROBOT_BLUE_SYNC_WORDS[(packet.id) as usize])
                    .unwrap();

                let sky_send = amp.take().unwrap().into_transmit_high_power_mode();

                transceiver
                    .send_packet::<40>(
                        &serialized_packet[..],
                        sx1280::definitions::PeriodBase::MilliSeconds1,
                        10,
                    )
                    .unwrap();

                loop {
                    let irq_reader = transceiver.irq_status().unwrap();
                    if irq_reader.is_set(sx1280::definitions::IrqBit::TxDone) {
                        break;
                    } else if irq_reader.is_set(sx1280::definitions::IrqBit::RxTxTimeout) {
                        warn!("Packet sending timed out");
                        break;
                    }
                }

                let sky_receive = sky_send.into_receive_lna_mode();

                transceiver
                    .start_receive_packet(127, sx1280::definitions::PeriodBase::MilliSeconds1, 4)
                    .unwrap();

                loop {
                    let irq_status = transceiver.irq_status().unwrap();
                    if irq_status.is_set(sx1280::definitions::IrqBit::RxDone) {
                        let (rssi, errors, _sync) = transceiver.packet_status().unwrap();
                        if errors.crc_error || errors.length_error || errors.abort_error {
                            warn!("CRC error");
                            break;
                        }
                        let packet = transceiver.read_packet::<40>().unwrap();

                        if let Ok(deserialized_packet) =
                            postcard::from_bytes::<RobotToBasestation>(&packet[..])
                        {
                            if deserialized_packet.id < state.receive_buffer.len() as u8 {
                                let end = Monotonic::now();
                                let rtt = (end - start).to_micros();
                                state.receive_buffer[deserialized_packet.id as usize] =
                                    Some((deserialized_packet, rssi, rtt as u32));
                            } else {
                                warn!("Got invalid robot id");
                            }
                        } else {
                            warn!("Failed to deserialize packet");
                        }

                        break;
                    } else if irq_status.is_set(sx1280::definitions::IrqBit::RxTxTimeout) {
                        warn!("Timeout while waiting for packet");
                        break;
                    }
                }

                *amp = Some(sky_receive.into_sleep_mode2());
            }

            *buffer_entry = None;
        }
    }
}
