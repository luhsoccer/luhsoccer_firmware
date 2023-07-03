//! PID controller
//!
//! See [Wikipedia](https://en.wikipedia.org/wiki/PID_controller) for more details.

#![cfg_attr(any(not(test), target_arch = "arm"), no_std)]

use core::ops::Neg;

use array_init::array_init;
use blanket::blanket;
use num_traits::{clamp, Num, NumAssign, NumOps};

/// Controller controlling an output using an input
#[blanket(derive(Mut))]
pub trait Controller {
    type Input;
    type Output;
    /// Regulate the output using the input
    fn regulate(&mut self, input: &Self::Input) -> Self::Output;
    /// Set the target of the controller
    fn set_target(&mut self, target: &Self::Input);
}

impl<C: Controller, const N: usize> Controller for [C; N] {
    type Input = [C::Input; N];

    type Output = [C::Output; N];

    fn regulate(&mut self, input: &Self::Input) -> Self::Output {
        array_init(|i| self[i].regulate(&input[i]))
    }

    fn set_target(&mut self, target: &Self::Input) {
        for i in 0..N {
            self[i].set_target(&target[i]);
        }
    }
}

/// Two controllers in parallel. Each controller is executed on its own and the both results are
/// returned
pub struct ParallelController<C1, C2>(pub C1, pub C2)
where
    C1: Controller,
    C2: Controller;

impl<C1, C2> Controller for ParallelController<C1, C2>
where
    C1: Controller,
    C2: Controller,
{
    type Input = (C1::Input, C2::Input);

    type Output = (C1::Output, C2::Output);

    fn regulate(&mut self, input: &Self::Input) -> (C1::Output, C2::Output) {
        (self.0.regulate(&input.0), self.1.regulate(&input.1))
    }

    fn set_target(&mut self, target: &Self::Input) {
        self.0.set_target(&target.0);
        self.1.set_target(&target.1);
    }
}

#[macro_export]
macro_rules! parallel_controller {
    ($x:expr) => {
        $x
    };
    ($x:expr, $( $y:expr ),+ ) => {
        $crate::ParallelController($x, parallel_controller!($($y),+))
    };
}

/// two controllers in series. The first controller regulates the input. The output of the first
/// controller sets the target of the second controller. The result of the second controller is
/// returned.
pub struct SeriesController<C1, C2>(pub C1, pub C2)
where
    C1: Controller,
    C2: Controller;

impl<C1, C2> Controller for SeriesController<C1, C2>
where
    C1: Controller,
    C2: Controller,
    C1::Output: Into<C2::Input>,
{
    type Input = (C1::Input, C2::Input);

    type Output = C2::Output;

    fn regulate(&mut self, input: &Self::Input) -> C2::Output {
        let setpoint = self.0.regulate(&input.0);
        self.1.set_target(&setpoint.into());
        self.1.regulate(&input.1)
    }

    fn set_target(&mut self, target: &Self::Input) {
        self.0.set_target(&target.0);
        self.1.set_target(&target.1);
    }
}

#[macro_export]
macro_rules! series_controller {
    ($x:expr) => {
        $x
    };
    ($x:expr, $($y:expr),+) => {
        $crate::SeriesController($x, series_controller!($($y),+))
    }
}

/// PID Controller
#[derive(Default, Copy, Clone)]
pub struct PIDController<T>
where
    T: Num + NumOps,
{
    /// p gain
    pub p_gain: T,
    /// i gain
    pub i_gain: T,
    /// d gain
    pub d_gain: T,
    setpoint: T,
    i_sum: T,
    /// limit the integral
    pub i_sum_limit: Option<T>,
    last_error: T,
    /// limit the output
    pub limit: Option<T>,
}

impl<T> PIDController<T>
where
    T: Num + NumOps,
{
    /// Creates a new PID controller with default values.
    #[must_use]
    pub fn new() -> Self {
        Self {
            p_gain: T::zero(),
            i_gain: T::zero(),
            d_gain: T::zero(),
            setpoint: T::zero(),
            i_sum: T::zero(),
            i_sum_limit: None,
            last_error: T::zero(),
            limit: None,
        }
    }

    /// Adds p gain.
    ///
    /// Controlls the output propotional to the error.
    #[allow(clippy::missing_const_for_fn)]
    #[must_use]
    pub fn with_p_gain(mut self, p_gain: T) -> Self {
        self.p_gain = p_gain;
        self
    }

    /// Adds i gain.
    ///
    /// Controlls the output using the integral of the error.
    #[allow(clippy::missing_const_for_fn)]
    #[must_use]
    pub fn with_i_gain(mut self, i_gain: T) -> Self {
        self.i_gain = i_gain;
        self
    }

    /// Adds d gain.
    ///
    /// Controlls the output using the derivative of the error.
    #[allow(clippy::missing_const_for_fn)]
    #[must_use]
    pub fn with_d_gain(mut self, d_gain: T) -> Self {
        self.d_gain = d_gain;
        self
    }

    /// Set the target.
    #[allow(clippy::missing_const_for_fn)]
    #[must_use]
    pub fn with_setpoint(mut self, setpoint: T) -> Self {
        self.setpoint = setpoint;
        self
    }

    /// Adds integral limit.
    ///
    /// Limits the integral so it doesn't grow to infinity.
    #[allow(clippy::missing_const_for_fn)]
    #[must_use]
    pub fn with_i_sum_limit(mut self, i_sum_limit: T) -> Self {
        self.i_sum_limit = Some(i_sum_limit);
        self
    }

    /// Adds output limit.
    ///
    /// Limits the output of the controller.
    #[allow(clippy::missing_const_for_fn)]
    #[must_use]
    pub fn with_limit(mut self, limit: T) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Clear the integral.
    ///
    /// The integral can grow to an uncontrolable size so it might be set to 0 by the user.
    pub fn clear_integral(&mut self) {
        self.i_sum = T::zero();
    }
}

impl<T> Controller for PIDController<T>
where
    T: NumAssign + Copy + PartialOrd + Neg<Output = T>,
    <T as Neg>::Output: Into<T>,
{
    type Input = T;

    type Output = T;

    fn regulate(&mut self, input: &Self::Input) -> T {
        let error = self.setpoint - *input;

        let p = if self.p_gain == T::zero() {
            T::zero()
        } else {
            self.p_gain * error
        };

        let i = if self.i_gain == T::zero() {
            T::zero()
        } else {
            self.i_sum += error;
            self.i_sum = if let Some(limit) = self.i_sum_limit {
                clamp(self.i_sum, -limit, limit)
            } else {
                self.i_sum
            };
            self.i_gain * self.i_sum
        };

        let d = if self.d_gain == T::zero() {
            T::zero()
        } else {
            let d = self.d_gain * (self.last_error - error);
            self.last_error = error;
            d
        };

        let output = p + i + d;

        self.limit
            .map_or(output, |limit| clamp(output, -limit, limit))
    }

    fn set_target(&mut self, target: &Self::Input) {
        self.setpoint = *target;
    }
}

#[cfg(not(any(not(test), target_arch = "arm")))]
mod tests {
    use crate::{Controller, PIDController, ParallelController, SeriesController};

    #[test]
    fn p_gain() {
        let mut controller = PIDController::new().with_p_gain(1).with_setpoint(1);
        assert_eq!(controller.regulate(&0), 1);
        assert_eq!(controller.regulate(&0), 1);
        let mut controller = PIDController::new().with_p_gain(2).with_setpoint(1);
        assert_eq!(controller.regulate(&0), 2);
    }

    #[test]
    fn i_gain() {
        let mut controller = PIDController::new().with_i_gain(1).with_setpoint(1);
        assert_eq!(controller.regulate(&0), 1);
        assert_eq!(controller.regulate(&0), 2);
        controller.clear_integral();
        assert_eq!(controller.regulate(&0), 1);
        let mut controller = PIDController::new().with_i_gain(2).with_setpoint(1);
        assert_eq!(controller.regulate(&0), 2);
    }

    #[test]
    fn d_gain() {
        let mut controller = PIDController::new().with_d_gain(1);
        assert_eq!(controller.regulate(&0), 0);
        assert_eq!(controller.regulate(&1), 1);
        assert_eq!(controller.regulate(&2), 1);
        assert_eq!(controller.regulate(&2), 0);
    }

    #[test]
    fn set_target() {
        let mut controller = PIDController::new().with_p_gain(1);
        controller.set_target(&1);
        assert_eq!(controller.setpoint, 1);
    }

    #[test]
    fn i_sum_limit() {
        let mut controller = PIDController::new()
            .with_i_gain(1)
            .with_i_sum_limit(5)
            .with_setpoint(2);
        assert_eq!(controller.regulate(&0), 2);
        assert_eq!(controller.regulate(&0), 4);
        assert_eq!(controller.regulate(&0), 5);
        assert_eq!(controller.regulate(&0), 5);
        assert_eq!(controller.regulate(&7), 0);
        assert_eq!(controller.regulate(&5), -3);
        assert_eq!(controller.regulate(&5), -5);
    }

    #[test]
    fn limit() {
        let mut controller = PIDController::new()
            .with_p_gain(1)
            .with_limit(5)
            .with_setpoint(0);
        assert_eq!(controller.regulate(&-4), 4);
        assert_eq!(controller.regulate(&-6), 5);
        assert_eq!(controller.regulate(&4), -4);
        assert_eq!(controller.regulate(&6), -5);
    }

    #[test]
    fn parallel() {
        let mut controller = ParallelController(
            PIDController::new().with_p_gain(1),
            PIDController::new().with_p_gain(2),
        );
        assert_eq!(controller.regulate(&(1, 1)), (-1, -2));
        let mut controller = parallel_controller!(
            PIDController::new().with_p_gain(1),
            PIDController::new().with_p_gain(2)
        );
        assert_eq!(controller.regulate(&(1, 1)), (-1, -2));
        controller.set_target(&(3, 4));
        assert_eq!(controller.0.setpoint, 3);
        assert_eq!(controller.1.setpoint, 4);
    }

    #[test]
    fn series() {
        let mut controller = SeriesController(
            PIDController::new().with_p_gain(1),
            PIDController::new().with_p_gain(2),
        );
        assert_eq!(controller.regulate(&(1, 1)), -4);
        let mut controller = series_controller!(
            PIDController::new().with_p_gain(1),
            PIDController::new().with_p_gain(2)
        );
        assert_eq!(controller.regulate(&(1, 1)), -4);
        controller.set_target(&(3, 4));
        assert_eq!(controller.0.setpoint, 3);
        assert_eq!(controller.1.setpoint, 4);
    }
}
