#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

use core::future::Future;
use core::ptr;
use core::task::{Context, Waker, RawWaker, RawWakerVTable, Poll};

use arduino_hal::prelude::*;
use panic_halt as _;
use portable_atomic::{AtomicBool, Ordering};
use pin_utils::pin_mut;
use spin_on::spin_on;

mod event_listener;

static EVENT: event_listener::Event = event_listener::Event::new();
static READY: AtomicBool = AtomicBool::new(false); 

#[avr_device::interrupt(atmega2560)]
#[allow(non_snake_case)]
fn INT4() {
    READY.store(true, Ordering::SeqCst);
    EVENT.notify(1);
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);

    let mut listener = event_listener::EventListener::new();
    pin_mut!(listener);

    // Enable the INT4 pin change interrupt.
    dp.EXINT.eicrb.write(|w| w.isc4().bits(0x02));
    dp.EXINT.eimsk.write(|w| w.int().bits(0b1_0000));

    let mut led = pins.d13.into_output();

    unsafe {
        avr_device::interrupt::enable();
    }

    loop {
        ufmt::uwrite!(&mut serial, "Waiting for interrupt...").void_unwrap();
        led.toggle();

        if READY.swap(false, Ordering::SeqCst) {
            continue;
        }
        
        // Wait until the interrupt is triggered.
        listener.as_mut().listen_to(&EVENT);

        if READY.swap(false, Ordering::SeqCst) {
            continue;
        }

        spin_on({
            let listener = listener.as_mut();
            async move {
                core::hint::spin_loop();
                listener.await;
            }
        });
        READY.store(false, Ordering::SeqCst);
    }
}

struct PollFn<F>(F);

impl<F: Unpin> Future for PollFn<F>
where
    F: FnMut() -> Poll<()>,
{
    type Output = ();

    fn poll(mut self: core::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        (self.0)()
    }
}
