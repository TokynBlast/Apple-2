#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

extern crate alloc;
use bootloader_api::{BootInfo, entry_point};
use core::fmt::Write;
use linked_list_allocator::LockedHeap;
use alloc::boxed::Box;

// Set up a global allocator
#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) -> ! {
    use x86_64::instructions::{nop, port::Port};

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }

    loop {
        nop();
    }
}

pub fn serial() -> uart_16550::SerialPort {
    let mut port = unsafe { uart_16550::SerialPort::new(0x3F8) };
    port.init();
    port
}

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    let mut port = serial();
    writeln!(port, "Hello!").ok();

    if let Some(framebuffer) = boot_info.framebuffer.as_mut() {
        let info = framebuffer.info();
        let mut fbw = fb::Writer::new(framebuffer.buffer_mut(), info);
        writeln!(fbw, "Hello!").ok();
    }

    writeln!(port, "Entered kernel with boot info: {boot_info:?}").unwrap();
    writeln!(port, "Serial port address: 0x3F8").unwrap();
    writeln!(port, "\n=(^.^)= meow\n").unwrap();

    // Print memory map info
    writeln!(port, "Memory regions:").unwrap();
    for region in boot_info.memory_regions.iter() {
        writeln!(port, "{:?}", region).unwrap();
    }

    writeln!(port, "Press 'q' to exit...").unwrap();
    loop {
        // Read keyboard scancode from port 0x60
        let key: u8 = unsafe {
            use x86_64::instructions::port::Port;
            let mut kbd_port = Port::<u8>::new(0x60);
            kbd_port.read()
        };
        // 'q' scancode is 0x10 (make code)
        if key == 0x10 {
            writeln!(port, "Exiting on 'q' key press!").unwrap();
            exit_qemu(QemuExitCode::Success);
        }
        x86_64::instructions::hlt();
    }
}

/// This function is called on panic.
#[panic_handler]
#[cfg(not(test))]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let _ = writeln!(serial(), "PANIC: {info}");
    exit_qemu(QemuExitCode::Failed);
}
