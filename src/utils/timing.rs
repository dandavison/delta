use crate::DELTA_DEBUG_TIMING;
use std::cell::RefCell;
use std::convert::TryInto;
use std::time::SystemTime;

#[derive(Debug)]
pub enum Measurement {
    Start = 0,
    ReadConfig = 1,
    Tty = 2,
    // TODO: measure when thread is done, not when process info is requested
    Process = 3,
    FirstPaint = 4,
    _Len = 5,
}

thread_local! {
    static DATAPOINTS: RefCell<[u64; Measurement::_Len as usize]> = const { RefCell::new([0; Measurement::_Len as usize]) };
}

pub struct TimingReporter;

impl Drop for TimingReporter {
    fn drop(&mut self) {
        DATAPOINTS.with(|data| {
            let values = data.take();
            if values[0] != 0 {
                // TODO: report 0 values as "never required"
                eprintln!(
                    "\n    delta timings (ms after start): \
                    tty setup: {:.1} ms, read configs: {:.1} ms, query processes: {:.1} ms, first paint: {:.1}",
                    values[Measurement::Tty as usize] as f32 / 1_000.,
                    values[Measurement::ReadConfig as usize] as f32 / 1_000.,
                    values[Measurement::Process as usize] as f32 / 1_000.,
                    values[Measurement::FirstPaint as usize] as f32 / 1_000.,
                );
            }
        })
    }
}

// After calling with `Start`, collect timestamps relative to this recorded start time. Must be
// called in the main thread (uses Thread Local Storage to avoid `Arc`s etc.)
pub fn measure(what: Measurement) {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    // u128 as u64, microseconds are small enough for simple subtraction
    let usecs: u64 = now.as_micros().try_into().unwrap_or_default();
    use Measurement::*;
    match what {
        Start => {
            if std::env::var_os(DELTA_DEBUG_TIMING).is_some() {
                DATAPOINTS.with(|data| {
                    let mut initial = data.take();
                    initial[Start as usize] = usecs;
                    data.replace(initial)
                });
            }
        }
        ReadConfig | Tty | Process | FirstPaint => DATAPOINTS.with(|data| {
            let mut values = data.take();
            if values[0] == 0 {
                return;
            }
            values[what as usize] = usecs.saturating_sub(values[0]);
            data.replace(values);
        }),
        _Len => unreachable!(),
    }
}

pub fn measure_completion<T>(x: T, what: Measurement) -> T {
    measure(what);
    x
}

pub fn report_on_exit() -> TimingReporter {
    TimingReporter
}
