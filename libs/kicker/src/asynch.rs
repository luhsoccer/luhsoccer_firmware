use defmt::warn;
use embassy_rp::{
    pio::{
        self as pio_mod, Common, Direction, FifoJoin, Instance, PioPin, ShiftConfig,
        ShiftDirection, StateMachine,
    },
    relocate::RelocatedProgram,
    Peripheral,
};

pub struct PioDac<'a, PIO, const SM: usize>
where
    PIO: Instance,
{
    sm: StateMachine<'a, PIO, SM>,
}

impl<'d, PIO, const SM: usize> PioDac<'d, PIO, SM>
where
    PIO: Instance,
{
    pub fn new(
        mut sm: StateMachine<'d, PIO, SM>,
        pio: &mut Common<'d, PIO>,
        pins: (
            impl Peripheral<P = impl PioPin + 'd> + 'd,
            impl Peripheral<P = impl PioPin + 'd> + 'd,
            impl Peripheral<P = impl PioPin + 'd> + 'd,
            impl Peripheral<P = impl PioPin + 'd> + 'd,
            impl Peripheral<P = impl PioPin + 'd> + 'd,
            impl Peripheral<P = impl PioPin + 'd> + 'd,
            impl Peripheral<P = impl PioPin + 'd> + 'd,
            impl Peripheral<P = impl PioPin + 'd> + 'd,
            impl Peripheral<P = impl PioPin + 'd> + 'd,
            impl Peripheral<P = impl PioPin + 'd> + 'd,
        ),
    ) -> Self {
        let prog = pio_proc::pio_asm!(".wrap_target", "out pins,10", ".wrap");
        let relocated = RelocatedProgram::new(&prog.program);
        let out_pins = [
            pio.make_pio_pin(pins.0),
            pio.make_pio_pin(pins.1),
            pio.make_pio_pin(pins.2),
            pio.make_pio_pin(pins.3),
            pio.make_pio_pin(pins.4),
            pio.make_pio_pin(pins.5),
            pio.make_pio_pin(pins.6),
            pio.make_pio_pin(pins.7),
            pio.make_pio_pin(pins.8),
            pio.make_pio_pin(pins.9),
        ];
        let pio_out_pins = [
            &out_pins[0],
            &out_pins[1],
            &out_pins[2],
            &out_pins[3],
            &out_pins[4],
            &out_pins[5],
            &out_pins[6],
            &out_pins[7],
            &out_pins[8],
            &out_pins[9],
        ];
        let mut cfg = pio_mod::Config::default();
        cfg.set_out_pins(&pio_out_pins);
        cfg.use_program(&pio.load_program(&relocated), &[]);
        cfg.clock_divider = 1u8.into();
        cfg.shift_out = ShiftConfig {
            auto_fill: true,
            threshold: 10,
            direction: ShiftDirection::default(),
        };
        cfg.fifo_join = FifoJoin::TxOnly;
        sm.set_config(&cfg);
        sm.set_pin_dirs(Direction::Out, &pio_out_pins);
        sm.set_enable(true);
        Self { sm }
    }

    pub fn set(&mut self, value: u16) {
        if value > 0x03FF {
            warn!("dac value over 10bit maximum");
        }
        let value = value.min(0x03FF);
        self.sm.tx().push(u32::from(value));
    }
}
