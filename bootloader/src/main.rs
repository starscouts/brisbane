#![no_main]
#![no_std]

use alloc::string::ToString;
use alloc::vec::Vec;
use core::panic::PanicInfo;
use uefi::prelude::*;
use uefi::proto::device_path::text::{AllowShortcuts, DisplayOnly};
use uefi::table::boot::LoadImageSource;
use uefi::{CString16, println};
use uefi::fs::{FileSystem, FileSystemResult};
use uefi::fs::Error::Io;
use uefi::prelude::BootServices;
use uefi::proto::device_path::LoadedImageDevicePath;
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::table::boot::ScopedProtocol;

extern crate alloc;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("\n{}", info.to_string().replace('\n', "\r\n"));
    loop {}
}

#[entry]
fn main(_image: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi::helpers::init(&mut system_table).unwrap();
    let boot_services = system_table.boot_services();

    unsafe {
        let mut st = system_table.unsafe_clone();
        let stdout = st.stdout();
        stdout.reset(false).expect("Failed to clear screen buffer");
    }

    println!("{}", &build_info::format!("Welcome to Brisbane Bootloader\nversion {} ({}, {})\n",
        $.crate_info.version, $.compiler, $.timestamp));

    let loaded_image = boot_services
        .open_protocol_exclusive::<LoadedImageDevicePath>(boot_services.image_handle())
        .unwrap();
    let image_device_path = loaded_image
        .to_string(boot_services, DisplayOnly(false), AllowShortcuts(false)).unwrap()
        .to_string();

    println!("Detected filesystem at {}", image_device_path);

    println!("Reading kernel...");
    let kernel_image = get_kernel_image(boot_services);

    match kernel_image {
        Ok(data) => {
            println!("Loading kernel...");

            match boot_services.load_image(boot_services.image_handle(), LoadImageSource::FromBuffer {
                buffer: data.as_slice(),
                file_path: Some(&**loaded_image)
            }) {
                Ok(handle) => {
                    println!("Starting kernel...");

                    match boot_services.start_image(handle) {
                        Ok(_) => {
                            panic!("Operation completed successfully.");
                        }
                        Err(e) => {
                            match e.status() {
                                Status::UNSUPPORTED => panic!("System error encountered in kernel (look above)."),
                                _ => panic!("Internal system error while starting kernel: {:?}", e)
                            }
                        }
                    }
                },
                Err(e) => {
                    panic!("Internal system error while loading kernel: {:?}", e);
                }
            }
        },
        Err(e) => {
            match e {
                Io(e) => {
                    match e.uefi_error.status() {
                        Status::NOT_FOUND => panic!("Kernel not found."),
                        Status::OUT_OF_RESOURCES => panic!("Not enough system memory to load the kernel."),
                        _ => panic!("Internal system error while reading kernel: {:?}", e)
                    }
                },
                _ => {
                    panic!("Internal system error while reading kernel: {:?}", e);
                }
            }
        }
    }
}

fn get_kernel_image(boot_services: &BootServices) -> FileSystemResult<Vec<u8>> {
    let path: CString16 = CString16::try_from("\\brisbane\\boot\\kernel").unwrap();
    let fs: ScopedProtocol<SimpleFileSystem> = boot_services.get_image_file_system(boot_services.image_handle()).unwrap();
    let mut fs = FileSystem::new(fs);
    fs.read(path.as_ref())
}
