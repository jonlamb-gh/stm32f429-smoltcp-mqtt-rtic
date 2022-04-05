///! Network clock for the TCP/IP stack backed by RTIC monotonic implementation.
///!
///! # Design
///!  `Clock` is implemented using the RTIC `app::monotonics::now()` default `Monotonic`.
///!  That `Monotonic` must tick at 1 kHz.
use minimq::embedded_time::{clock::Error, fraction::Fraction, Clock, Instant};

#[derive(Copy, Clone, Debug)]
pub struct NetworkClock(fn() -> u64);

impl NetworkClock {
    pub fn new(now: fn() -> u64) -> Self {
        Self(now)
    }
}

impl Clock for NetworkClock {
    type T = u32;

    // The duration of each tick in seconds.
    const SCALING_FACTOR: Fraction = Fraction::new(1, 1_000);

    fn try_now(&self) -> Result<Instant<Self>, Error> {
        Ok(Instant::new((self.0)() as _))
    }
}
