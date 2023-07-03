//use core::{convert::Infallible, ops::RangeInclusive};

use blanket::blanket;
use defmt::{trace, Format};
use embedded_hal::digital::v2::{OutputPin, PinState};
/*
use pio::{Assembler, OutDestination};
use rp2040_hal::pio::{
    InstallError, PIOBuilder, PIOExt, PinDir, Running, Rx, StateMachine, StateMachineIndex, Tx,
    UninitStateMachine, PIO,
};
*/

#[blanket(derive(Mut))]
pub trait Dac {
    type Error;

    /// Sets the output voltage of the DAC
    ///
    /// # Errors
    ///
    /// This function will return an error if the output voltage cannot be set
    fn set(&mut self, voltage: u16) -> Result<(), Self::Error>;
}

#[derive(Debug, Format, Copy, Clone)]
pub struct R2R<P, const N: usize> {
    pins: [P; N],
}

impl<P, const N: usize> R2R<P, N> {
    /// Creates a new [`R2RDac<P, N>`].
    pub const fn new(pins: [P; N]) -> Self {
        Self { pins }
    }
}

impl<P, const N: usize> Dac for R2R<P, N>
where
    P: OutputPin,
{
    type Error = P::Error;

    fn set(&mut self, mut voltage: u16) -> Result<(), Self::Error> {
        trace!("setting dac voltage pin by pin");
        for i in (0..N).rev() {
            let state = if voltage > u16::MAX / 2 {
                PinState::High
            } else {
                PinState::Low
            };
            trace!(
                "setting pin {} {}",
                i,
                if state == PinState::High {
                    "High"
                } else {
                    "Low"
                }
            );
            self.pins[i].set_state(state)?;
            voltage <<= 1;
        }
        Ok(())
    }
}

/*
pub struct R2RPio<P, S>
where
    P: PIOExt,
    S: StateMachineIndex,
{
    sm: StateMachine<(P, S), Running>,
    rx: Rx<(P, S)>,
    tx: Tx<(P, S)>,
}

impl<P, S> R2RPio<P, S>
where
    P: PIOExt,
    S: StateMachineIndex,
{
    /// Creates a new R2R DAC using pio to write to the pins
    ///
    /// # Errors
    ///
    /// This function will return an error if it can't install the program in pio memory
    pub fn new(
        pio: &mut PIO<P>,
        sm: UninitStateMachine<(P, S)>,
        pins: RangeInclusive<u8>,
    ) -> Result<Self, InstallError> {
        trace!("creating a new dac using pio");
        let num_pins = pins
            .len()
            .try_into()
            .expect("Range over u8 can't be longer than u8");

        let mut assembler = Assembler::new();
        let mut wrap_target = assembler.label();
        let mut wrap_source = assembler.label();
        assembler.bind(&mut wrap_target);
        assembler.out(OutDestination::PINS, num_pins);
        assembler.bind(&mut wrap_source);
        let program = assembler.assemble_with_wrap(wrap_source, wrap_target);

        let installed = pio.install(&program)?;
        let (mut sm, rx, tx) = PIOBuilder::from_program(installed)
            .out_pins(*pins.start(), num_pins)
            .clock_divisor_fixed_point(1, 0) // fastest clock
            .autopull(true)
            .pull_threshold(num_pins)
            .build(sm);
        sm.set_pindirs(pins.map(|i| (i, PinDir::Output)));
        let sm = sm.start();
        Ok(Self { sm, rx, tx })
    }

    /// Frees the pio state machine and frees pio memory
    pub fn free(self, pio: &mut PIO<P>) -> UninitStateMachine<(P, S)> {
        trace!("freeing pio state machine");
        let sm = self.sm.stop();
        let (sm, installed) = sm.uninit(self.rx, self.tx);
        pio.uninstall(installed);
        sm
    }
}

impl<P, S> Dac for R2RPio<P, S>
where
    P: PIOExt,
    S: StateMachineIndex,
{
    type Error = Infallible;

    fn set(&mut self, voltage: u16) -> Result<(), Self::Error> {
        trace!("setting dac voltage using pio");
        self.tx.write_u16_replicated(voltage);
        Ok(())
    }
}
*/
