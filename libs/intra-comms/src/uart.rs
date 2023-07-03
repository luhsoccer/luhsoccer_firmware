use core::marker::PhantomData;

use cobs::CobsDecoder;
use crc::{Crc, CRC_16_ISO_IEC_14443_3_A};
use defmt::error;
use embedded_io::{
    asynch::{BufRead, Write},
    Io,
};
use heapless::Vec;
use postcard::{
    de_flavors::{Flavor as DeFlavor, Slice},
    ser_flavors::{Cobs, Flavor as SerFlavor, HVec},
};
use serde::{Deserialize, Serialize};

use crate::definitions::{KickerChargeHint, LocalVelocity, Main2Motor, Motor2Main};

pub struct MotorControllerSender<Tx>
where
    Tx: Write,
{
    sender: Sender<Main2Motor, Tx>,
}

impl<Tx> MotorControllerSender<Tx>
where
    Tx: Write,
{
    pub const fn new(tx: Tx) -> Self {
        Self {
            sender: Sender::new(tx),
        }
    }

    pub async fn drive(&mut self, velocity: LocalVelocity) -> Result<(), SendError<Tx>> {
        self.sender.send::<16>(&Main2Motor::Drive(velocity)).await
    }

    pub async fn kick(&mut self, speed: u16) -> Result<(), SendError<Tx>> {
        self.sender.send::<10>(&Main2Motor::Kick(speed)).await
    }

    pub async fn chip(&mut self, speed: u16) -> Result<(), SendError<Tx>> {
        self.sender.send::<10>(&Main2Motor::Chip(speed)).await
    }

    pub async fn kick_raw(&mut self, duration: u16) -> Result<(), SendError<Tx>> {
        self.sender.send::<10>(&Main2Motor::KickRaw(duration)).await
    }

    pub async fn ball_in_dribbler(&mut self, in_dribbler: bool) -> Result<(), SendError<Tx>> {
        self.sender
            .send::<7>(&if in_dribbler {
                Main2Motor::BallInDribbler
            } else {
                Main2Motor::BallNotInDribbler
            })
            .await
    }

    pub async fn charge_hint(&mut self, hint: KickerChargeHint) -> Result<(), SendError<Tx>> {
        self.sender.send::<8>(&Main2Motor::ChargeHint(hint)).await
    }

    pub async fn calibrate_cap_voltage(&mut self, value: u8) -> Result<(), SendError<Tx>> {
        self.sender
            .send::<8>(&Main2Motor::CalibrateCapVoltage(value))
            .await
    }
}

pub struct MainControllerSender<Tx>
where
    Tx: Write,
{
    sender: Sender<Motor2Main, Tx>,
}

impl<Tx> MainControllerSender<Tx>
where
    Tx: Write,
{
    pub const fn new(tx: Tx) -> Self {
        Self {
            sender: Sender::new(tx),
        }
    }

    pub async fn motor_velocity(&mut self, velocity: LocalVelocity) -> Result<(), SendError<Tx>> {
        self.sender
            .send::<16>(&Motor2Main::MotorVelocity(velocity))
            .await
    }

    pub async fn cap_voltage(&mut self, voltage: u8) -> Result<(), SendError<Tx>> {
        self.sender
            .send::<8>(&Motor2Main::CapVoltage(voltage))
            .await
    }
}

pub struct MotorControllerReceiver<Tx>
where
    Tx: BufRead,
{
    receiver: Receiver<Motor2Main, Tx>,
}

impl<Tx> MotorControllerReceiver<Tx>
where
    Tx: BufRead,
{
    pub const fn new(tx: Tx) -> Self {
        Self {
            receiver: Receiver::new(tx),
        }
    }

    pub async fn receive(&mut self) -> Result<Motor2Main, ReceiveError<Tx>> {
        let mut buf = [0; 16];
        self.receiver.receive(&mut buf).await
    }
}

pub struct MainControllerReceiver<Tx>
where
    Tx: BufRead,
{
    receiver: Receiver<Main2Motor, Tx>,
}

impl<Tx> MainControllerReceiver<Tx>
where
    Tx: BufRead,
{
    pub const fn new(tx: Tx) -> Self {
        Self {
            receiver: Receiver::new(tx),
        }
    }

    pub async fn receive(&mut self) -> Result<Main2Motor, ReceiveError<Tx>> {
        let mut buf = [0; 16];
        self.receiver.receive(&mut buf).await
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SendError<Tx: Io> {
    Postcard(postcard::Error),
    Io(Tx::Error),
}

impl<Tx: Io> From<postcard::Error> for SendError<Tx> {
    fn from(value: postcard::Error) -> Self {
        Self::Postcard(value)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ReceiveError<Rx: Io> {
    Postcard(postcard::Error),
    Cobs,
    Io(Rx::Error),
}

impl<Rx: Io> From<postcard::Error> for ReceiveError<Rx> {
    fn from(value: postcard::Error) -> Self {
        Self::Postcard(value)
    }
}

struct Receiver<T, Rx>
where
    Rx: BufRead,
{
    rx: Rx,
    _marker: PhantomData<T>,
}

impl<T, Rx> Receiver<T, Rx>
where
    Rx: BufRead,
{
    const fn new(rx: Rx) -> Self {
        Self {
            rx,
            _marker: PhantomData,
        }
    }

    async fn receive<'de>(&mut self, buf: &'de mut [u8]) -> Result<T, ReceiveError<Rx>>
    where
        T: Deserialize<'de>,
    {
        let mut decoder = CobsDecoder::new(buf);
        let length = loop {
            let in_buf = self.rx.fill_buf().await.unwrap_or_else(|_| {
                error!("Error getting bytes from uart");
                &[]
            });
            match decoder.push(in_buf) {
                Ok(None) => {
                    let length = in_buf.len();
                    self.rx.consume(length)
                }
                Ok(Some((length, used))) => {
                    self.rx.consume(used);
                    break length;
                }
                Err(_) => {
                    let length = in_buf.len();
                    self.rx.consume(length);
                    return Err(ReceiveError::Cobs);
                }
            }
        };
        let buf = &buf[..length];
        let mut deserializer = postcard::Deserializer::from_flavor(Crc16::new(Slice::new(buf)));
        T::deserialize(&mut deserializer).map_err(Into::into)
    }
}

struct Sender<T, Tx>
where
    T: Serialize,
    Tx: Write,
{
    tx: Tx,
    _marker: PhantomData<T>,
}

impl<T, Tx> Sender<T, Tx>
where
    T: Serialize,
    Tx: Write,
{
    const fn new(tx: Tx) -> Self {
        Self {
            tx,
            _marker: PhantomData,
        }
    }

    async fn send<const N: usize>(&mut self, message: &T) -> Result<(), SendError<Tx>> {
        let buf = postcard::serialize_with_flavor::<T, Crc16<Cobs<HVec<N>>>, Vec<u8, N>>(
            message,
            Crc16::new(Cobs::try_new(HVec::default())?),
        )?;
        self.tx.write_all(&buf[..]).await.map_err(SendError::Io)
    }
}

struct Crc16<B> {
    flav: B,
    crc: Crc<u16>,
}

impl<B> Crc16<B> {
    fn new(flav: B) -> Self {
        let crc = Crc::<u16>::new(&CRC_16_ISO_IEC_14443_3_A);
        Self { flav, crc }
    }
}

impl<B> SerFlavor for Crc16<B>
where
    B: SerFlavor,
{
    type Output = <B as SerFlavor>::Output;

    fn try_push(&mut self, data: u8) -> postcard::Result<()> {
        self.crc.digest().update(&[data]);
        self.flav.try_push(data)
    }

    fn finalize(mut self) -> postcard::Result<Self::Output> {
        let crc = self.crc.digest().finalize();
        self.flav.try_extend(&crc.to_be_bytes())?;
        self.flav.finalize()
    }

    fn try_extend(&mut self, data: &[u8]) -> postcard::Result<()> {
        self.crc.digest().update(data);
        self.flav.try_extend(data)
    }
}

impl<'de, B> DeFlavor<'de> for Crc16<B>
where
    B: DeFlavor<'de>,
{
    type Remainder = <B as DeFlavor<'de>>::Remainder;

    type Source = <B as DeFlavor<'de>>::Source;

    fn pop(&mut self) -> postcard::Result<u8> {
        let byte = self.flav.pop()?;
        self.crc.digest().update(&[byte]);
        Ok(byte)
    }

    fn try_take_n(&mut self, ct: usize) -> postcard::Result<&'de [u8]> {
        let bytes = self.flav.try_take_n(ct)?;
        self.crc.digest().update(bytes);
        Ok(bytes)
    }

    fn finalize(mut self) -> postcard::Result<Self::Remainder> {
        let crc = u16::from_be_bytes([self.flav.pop()?, self.flav.pop()?]);
        if crc != self.crc.digest().finalize() {
            Err(postcard::Error::DeserializeBadEncoding)
        } else {
            self.flav.finalize()
        }
    }
}
