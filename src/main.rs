#![feature(alloc)]
#![feature(panic_implementation)]
#![feature(alloc_error_handler)]
#![no_main]
#![no_std]

#[macro_use]
extern crate alloc;
extern crate cortex_m;
#[macro_use]
extern crate cortex_m_rt as rt;
extern crate alloc_cortex_m;
extern crate cortex_m_semihosting as sh;
#[macro_use]
extern crate stm32f7;
extern crate stm32f7_discovery;

use alloc_cortex_m::CortexMHeap;
use core::alloc::Layout as AllocLayout;
use core::fmt::Write;
use core::panic::PanicInfo;
use cortex_m::{asm, interrupt};
use rt::ExceptionFrame;
use sh::hio::{self, HStdout};
use stm32f7::stm32f7x6::{CorePeripherals, Interrupt, Peripherals};
use stm32f7_discovery::{
    gpio::{GpioPort, InputPin, OutputPin},
    init::{self, Hz},
    system_clock,
};

#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

const HEAP_SIZE: usize = 1024; // in bytes

entry!(main);

fn main() -> ! {
    let mut stdout = hio::hstdout().unwrap();
    writeln!(stdout, "Hello, world!").unwrap();

    let core_peripherals = CorePeripherals::take().unwrap();
    let mut systick = core_peripherals.SYST;
    let mut nvic = core_peripherals.NVIC;

    let peripherals = Peripherals::take().unwrap();
    let mut rcc = peripherals.RCC;
    let mut pwr = peripherals.PWR;
    let mut flash = peripherals.FLASH;
    let mut fmc = peripherals.FMC;
    let mut ltdc = peripherals.LTDC;

    init::init_system_clock_216mhz(&mut rcc, &mut pwr, &mut flash);
    init::enable_gpio_ports(&mut rcc);

    let gpio_a = GpioPort::new_a(&peripherals.GPIOA);
    let gpio_b = GpioPort::new_b(&peripherals.GPIOB);
    let gpio_c = GpioPort::new(&peripherals.GPIOC);
    let gpio_d = GpioPort::new(&peripherals.GPIOD);
    let gpio_e = GpioPort::new(&peripherals.GPIOE);
    let gpio_f = GpioPort::new(&peripherals.GPIOF);
    let gpio_g = GpioPort::new(&peripherals.GPIOG);
    let gpio_h = GpioPort::new(&peripherals.GPIOH);
    let gpio_i = GpioPort::new(&peripherals.GPIOI);
    let gpio_j = GpioPort::new(&peripherals.GPIOJ);
    let gpio_k = GpioPort::new(&peripherals.GPIOK);
    let mut pins = init::pins(
        gpio_a, gpio_b, gpio_c, gpio_d, gpio_e, gpio_f, gpio_g, gpio_h, gpio_i, gpio_j, gpio_k,
    );

    // configures the system timer to trigger a SysTick exception every second
    init::init_systick(Hz(1), &mut systick, &rcc);
    systick.enable_interrupt();

    init::init_sdram(&mut rcc, &mut fmc);
    init::init_lcd(&mut ltdc, &mut rcc);
    pins.display_enable.set(true);
    pins.backlight.set(true);

    // Initialize the allocator BEFORE you use it
    unsafe { ALLOCATOR.init(rt::heap_start() as usize, HEAP_SIZE) }

    let xs = vec![1, 2, 3];

    nvic.enable(Interrupt::EXTI0);

    let mut previous_button_state = pins.button.get();
    loop {
        let current_button_state = pins.button.get();
        if current_button_state != previous_button_state {
            if current_button_state {
                pins.led.toggle();
            }

            // trigger the `EXTI0` interrupt
            nvic.set_pending(Interrupt::EXTI0);

            previous_button_state = current_button_state;
        }
    }
}

interrupt!(EXTI0, exti0, state: Option<HStdout> = None);

fn exti0(state: &mut Option<HStdout>) {
    if state.is_none() {
        *state = Some(hio::hstdout().unwrap());
    }

    if let Some(hstdout) = state.as_mut() {
        hstdout.write_str("i").unwrap();
    }
}

exception!(SysTick, sys_tick, state: Option<HStdout> = None);

fn sys_tick(_state: &mut Option<HStdout>) {
    system_clock::tick();
}

exception!(HardFault, hard_fault);

fn hard_fault(ef: &ExceptionFrame) -> ! {
    panic!("HardFault at {:#?}", ef);
}

exception!(*, default_handler);

fn default_handler(irqn: i16) {
    panic!("Unhandled exception (IRQn = {})", irqn);
}

// define what happens in an Out Of Memory (OOM) condition
#[alloc_error_handler]
#[no_mangle]
pub fn rust_oom(_: AllocLayout) -> ! {
    if let Ok(mut hstdout) = hio::hstdout() {
        let _ = hstdout.write_str("out of memory");
    }

    // OK to fire a breakpoint here because we know the microcontroller is connected to a debugger
    asm::bkpt();

    loop {}
}

#[panic_implementation]
#[no_mangle]
pub fn panic(info: &PanicInfo) -> ! {
    interrupt::disable();

    if let Ok(mut hstdout) = hio::hstdout() {
        let _ = writeln!(hstdout, "{}", info);
    }

    // OK to fire a breakpoint here because we know the microcontroller is connected to a debugger
    asm::bkpt();

    loop {}
}
