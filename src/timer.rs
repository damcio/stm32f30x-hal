//! Timers

use cast::{u16, u32};
use nb;
use stm32f30x::{TIM2, TIM3, TIM4, TIM6, TIM7};

use rcc::{APB1, Clocks};
use time::Hertz;

/// Hardware timers
pub struct Timer<TIM> {
    clocks: Clocks,
    tim: TIM,
    timeout: Hertz,
}

/// Interrupt events
pub enum Event {
    /// Timer timed out / count down ended
    TimeOut,
    Update
}

macro_rules! hal {
    ($($TIM:ident: ($tim:ident, $timXen:ident, $timXrst:ident),)+) => {
        $(

            impl Timer<$TIM> {

                // NOTE(allow) `w.psc().bits()` is safe for TIM{6,7} but not for TIM{2,3,4} due to
                // some SVD omission
                pub fn start(&mut self)
                {
                    // pause
                    self.tim.cr1.modify(|_, w| w.cen().clear_bit());
                    // restart counter
                    self.tim.cnt.reset();

                    // start counter
                    self.tim.cr1.modify(|_, w| w.cen().set_bit());
                }

                pub fn wait(&mut self) -> nb::Result<(), !> {
                    if self.tim.sr.read().uif().bit_is_clear() {
                        Err(nb::Error::WouldBlock)
                    } else {
                        self.tim.sr.modify(|_, w| w.uif().clear_bit());
                        Ok(())
                    }
                }

                // XXX(why not name this `new`?) bummer: constructors need to have different names
                // even if the `$TIM` are non overlapping (compare to the `free` function below
                // which just works)
                /// Configures a TIM peripheral as a periodic count down timer
                // pub fn $tim<T>(tim: $TIM, clocks: Clocks, apb1: &mut APB1) -> Self
                pub fn $tim(tim: $TIM, clocks: Clocks, apb1: &mut APB1) -> Self
                {
                    // enable and reset peripheral to a clean slate state
                    apb1.enr().modify(|_, w| w.$timXen().set_bit());
                    apb1.rstr().modify(|_, w| w.$timXrst().set_bit());
                    apb1.rstr().modify(|_, w| w.$timXrst().clear_bit());

                    let timer = Timer {
                        clocks,
                        tim,
                        timeout: Hertz(0),
                    };

                    timer
                }

                /// Starts listening for an `event`
                pub fn listen(&mut self, event: Event) {
                    match event {
                        Event::TimeOut => {
                            // Enable Timeout event interrupt
                            self.tim.cr2.write(|w| {
                                unsafe{w.mms().bits(0b001)}  // set mode to Update Event
                            });
                        },
                        Event::Update => {
                            // Enable update event interrupt
                            self.tim.cr2.write(|w| {
                                unsafe{w.mms().bits(0b010)}  // set mode to Update Event
                            });
                        }
                    }
                    self.tim.dier.write(|w| w.uie().set_bit());
                }

                #[allow(unused_unsafe)]
                pub fn config<T>(&mut self, timeout: T)
                where
                    T: Into<Hertz>,
                {
                    self.timeout = timeout.into();

                    let frequency = self.timeout.0;
                    let ticks = self.clocks.pclk1().0 * if self.clocks.ppre1() == 1 { 1 } else { 2 }
                        / frequency;

                    let psc = u16((ticks - 1) / (1 << 16)).unwrap();
                    self.tim.psc.write(|w| unsafe { w.psc().bits(psc) });

                    let arr = u16(ticks / u32(psc + 1)).unwrap();
                    self.tim.arr.write(|w| unsafe { w.bits(u32(arr)) });
                }

                /// Stops listening for an `event`
                pub fn unlisten(&mut self, event: Event) {
                    match event {
                        Event::TimeOut => {
                            // Enable update event interrupt
                            self.tim.dier.write(|w| w.uie().clear_bit());
                            self.tim.cr2.write(|w| {
                                unsafe{w.mms().bits(0b000)}  // set mode to reset Event
                            });
                        },
                        Event::Update => {
                            // Enable update event interrupt
                            self.tim.cr2.write(|w| {
                                unsafe{w.mms().bits(0b000)}  // set mode to reset Event
                            });
                            self.tim.dier.write(|w| w.uie().clear_bit());
                        }
                    }
                }

                /// Releases the TIM peripheral
                pub fn free(self) -> $TIM {
                    // pause counter
                    self.tim.cr1.modify(|_, w| w.cen().clear_bit());
                    self.tim
                }
            }
        )+
    }
}

hal! {
    TIM2: (tim2, tim2en, tim2rst),
    TIM3: (tim3, tim3en, tim3rst),
    TIM4: (tim4, tim4en, tim4rst),
    TIM6: (tim6, tim6en, tim6rst),
    TIM7: (tim7, tim7en, tim7rst),
}

impl Timer<TIM2> {
    #[inline(always)]
    pub fn get_counter(&self) -> u32
    {
        self.tim.cnt.read().bits()
    }

    #[allow(unused_unsafe)]
    #[allow(exceeding_bitshifts)]
    pub fn reconfig(&mut self) -> ()
    {
        let frequency = self.timeout.0;
        let ticks = self.clocks.pclk1().0 * if self.clocks.ppre1() == 1 { 1 } else { 2 }
            / frequency;

        unsafe {
            let psc = u32((ticks - 1) / u32::max_value());
            self.tim.psc.write(|w| w.psc().bits(psc as u16));

            let arr = u32(ticks / u32(psc + 1));
            self.tim.arr.write(|w|  w.bits(u32(arr)) );
        }
    }

    pub fn stop(&mut self)
    {
        // pause
        self.tim.cr1.modify(|_, w| w.cen().clear_bit());
        // restart counter
        self.tim.cnt.reset();
    }
}