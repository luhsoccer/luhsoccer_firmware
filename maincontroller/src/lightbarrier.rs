use defmt::{error, info, Format};
use embassy_executor::task;
use embassy_futures::select::{select, Either};
use embassy_rp::{
    gpio::{Input, Pull},
    peripherals::PIN_15,
};
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, RawMutex};
use embassy_time::{Duration, Timer};
use embedded_hal_async::digital::Wait;
use sync::observable::Observable;

use crate::Config;

enum State {
    NoBall,
    HasBall,
    ContactLost(Timer),
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Format)]
pub enum LightBarrierState {
    HasBall,
    NoBall,
    ContactLost,
}

#[task]
pub async fn lightbarrier_task(
    ball_sense: PIN_15,
    has_ball: &'static Observable<CriticalSectionRawMutex, LightBarrierState, 8>,
    config: &'static Config<CriticalSectionRawMutex>,
) {
    let pin = Input::new(ball_sense, Pull::None);
    lightbarrier(pin, has_ball, config).await;
}

async fn lightbarrier<const SUBS: usize>(
    mut pin: impl Wait,
    has_ball: &Observable<impl RawMutex, LightBarrierState, SUBS>,
    config: &Config<impl RawMutex>,
) {
    let mut state = State::NoBall;
    has_ball.set(LightBarrierState::NoBall);
    info!("Starting lightbarrier loop");

    loop {
        match state {
            State::NoBall => {
                has_ball.set(LightBarrierState::NoBall);
                info!("No ball in dribbler");
                while pin.wait_for_low().await.is_err() {
                    error!("couldn't wait for light barrier being low");
                }
                state = State::HasBall;
            }
            State::HasBall => {
                has_ball.set(LightBarrierState::HasBall);
                info!("Ball in dribbler");
                while pin.wait_for_high().await.is_err() {
                    error!("couldn't wait for light barrier being high");
                }
                // Set a timeout of 200ms when contact to the ball is lost. This prevents the
                // software from thinking there is no ball even though the ball is just bouncing in
                // front of the dribbler.
                state = State::ContactLost(Timer::after(Duration::from_millis(
                    config.lightbarrier_filter_time.get().into(),
                )));
            }
            State::ContactLost(timer) => {
                has_ball.set(LightBarrierState::ContactLost);
                info!("Contact to Ball lost");
                match select(
                    async {
                        while pin.wait_for_low().await.is_err() {
                            error!("couldn't wait for light barrier being low");
                        }
                    },
                    timer,
                )
                .await
                {
                    Either::First(_) => state = State::HasBall,
                    Either::Second(_) => state = State::NoBall,
                }
            }
        }
    }
}
