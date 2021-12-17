//! RTIC Monotonic implementation for STM32L0 16-bit timers.
//!
//! The 16-bit timer is software-extended to 32 bit by incrementing an overflow
//! counter every time the timer overflows. At an LSE frequency of 32.768 kHz,
//! an overflow will happen every 2 seconds.
//!
//! Note: There might be power-saving potential by not extending the timer
//! (less interrupts), but we'd have to measure whether it's relevant or not.

use core::u32;

pub use fugit::ExtU32;
use rtic::Monotonic;
use stm32l0xx_hal::pac;

const LSE_FREQ: u32 = 32_768;

/// Software-extended LPTIM.
pub struct ExtendedLptim<TIM> {
    timer: TIM,
    overflow: u16,
}

impl ExtendedLptim<pac::LPTIM> {
    pub fn init(timer: pac::LPTIM) -> Self {
        // Enable and reset LPTIM in RCC
        //
        // Correctness: Since we only modify LPTIM related registers in the RCC
        // register block, and since we own pac::LPTIM, we should be safe.
        unsafe {
            let rcc = &*pac::RCC::ptr();

            // Select clock source: LSE
            rcc.ccipr.modify(|_, w| w.lptim1sel().lse());

            // Enable timer clock
            rcc.apb1enr.modify(|_, w| w.lptim1en().set_bit());

            // Reset timer
            rcc.apb1rstr.modify(|_, w| w.lptim1rst().set_bit());
            rcc.apb1rstr.modify(|_, w| w.lptim1rst().clear_bit());
        }

        // Enable the compare-match interrupt
        timer.ier.modify(|_, w| w.cmpmie().set_bit());

        Self { timer, overflow: 0 }
    }

    fn is_overflow(&self) -> bool {
        // Return whether the ARRM (Autoreload match) flag in the ISR
        // (interrupt and status register) is set.
        self.timer.isr.read().arrm().bit_is_set()
    }

    fn clear_overflow_flag(&self) {
        self.timer.icr.write(|w| w.arrmcf().set_bit());
    }
}

impl Monotonic for ExtendedLptim<pac::LPTIM> {
    // Since we are counting overflows we can't let RTIC disable the interrupt.
    const DISABLE_INTERRUPT_ON_EMPTY_QUEUE: bool = false;

    type Instant = fugit::TimerInstantU32<LSE_FREQ>;
    type Duration = fugit::TimerDurationU32<LSE_FREQ>;

    /// Return the current time.
    #[inline(always)]
    fn now(&mut self) -> Self::Instant {
        // Note: The reference manual contains this text:s
        //
        // > It should be noted that for a reliable LPTIM_CNT register read
        // > access, two consecutive read accesses must be performed and
        // > compared. A read access can be considered reliable when the values
        // > of the two consecutive read accesses are equal.
        //
        // However, I think this only applies to asynchronous mode with an
        // external clock source.
        let counter = self.timer.cnt.read().cnt().bits() as u32;

        // If the overflow bit is set, it means that `on_interrupt` (which
        // clears the flag) was not yet called. Compensate for this.
        let overflow = if self.is_overflow() {
            self.overflow + 1
        } else {
            self.overflow
        } as u32;

        Self::Instant::from_ticks(overflow * (1 << 16) + counter)
    }

    /// The time at time zero. Used by RTIC before the monotonic has been initialized.
    #[inline(always)]
    fn zero() -> Self::Instant {
        Self::Instant::from_ticks(0)
    }

    /// Reset the counter to zero for a fixed baseline in a system.
    ///
    /// This method will be called exactly once by the RTIC runtime after
    /// `#[init]` returns and before tasks start.
    ///
    /// # Correctness
    ///
    /// The user may not call this method.
    unsafe fn reset(&mut self) {
        // Enable LPTIM
        self.timer.cr.modify(|_, w| w.enable().set_bit());

        // Set the autoreload register to the max value
        self.timer.arr.write(|w| w.bits(0xffff));

        // Start counting
        self.timer.cr.modify(|_, w| w.cntstrt().set_bit());
    }

    /// Set the compare value of the timer interrupt.
    fn set_compare(&mut self, instant: Self::Instant) {
        let now = self.now();

        // Since the timer may or may not overflow based on the requested
        // compare val, we check how many ticks are left.
        let compare_register_val = match instant.checked_duration_since(now) {
            // If the scheduled instant is too far in the future, we can't set
            // an exact deadline because it's too far in the future. Set it to 0,
            // RTIC will handle re-scheduling.
            Some(duration) if duration.ticks() > 0xffff => 0,

            // Instant is in the past. RTIC will handle this.
            None => 0,

            // Value will not overflow the 16-bit register.
            Some(_) => instant.duration_since_epoch().ticks() as u16,
        };

        // Write value to compare register
        self.timer.cmp.write(|w| w.cmp().bits(compare_register_val));
    }

    /// Clear the compare interrupt flag.
    fn clear_compare_flag(&mut self) {
        self.timer.icr.write(|w| w.cmpmcf().set_bit());
    }

    /// Optional. Commonly used for performing housekeeping of a timer when it
    /// has been extended, e.g. a 16 bit timer extended to 32/64 bits. This
    /// will be called at the end of the interrupt handler after all other
    /// operations have finished.
    fn on_interrupt(&mut self) {
        // If there was an overflow, increment the overflow counter.
        if self.is_overflow() {
            self.clear_overflow_flag();
            self.overflow += 1;
        }
    }
}
