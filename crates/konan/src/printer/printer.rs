use crate::printer::element;
use escpos::utils::{JustifyMode, PageCode};
use escpos::{
    driver::{ConsoleDriver, Driver, NetworkDriver, UsbDriver},
    printer::Printer,
    printer_options::PrinterOptions,
    utils::Protocol,
};

fn build_console_printer() -> escpos::errors::Result<Printer<ConsoleDriver>> {
    build_printer(ConsoleDriver::open(true))
}

fn build_usb_printer(
    vendor_id: u16,
    product_id: u16,
) -> escpos::errors::Result<Printer<UsbDriver>> {
    let driver = UsbDriver::open(vendor_id, product_id, None, None)?;
    build_printer(driver)
}

fn build_network_printer(
    host: String,
    port: u16,
) -> escpos::errors::Result<Printer<NetworkDriver>> {
    let driver = NetworkDriver::open(&host, port, None)?;
    build_printer(driver)
}

fn build_printer<D>(driver: D) -> escpos::errors::Result<Printer<D>>
where
    D: Driver,
{
    let mut printer = Printer::new(
        driver,
        Protocol::default(),
        Some(PrinterOptions::new(
            Some(escpos::utils::PageCode::PC437),
            None, // Some(DebugMode::Dec),
            element::CPL,
        )),
    );
    printer.flip(false)?;
    printer.reset()?;

    Ok(printer)
}

pub fn configured_printer() -> AnyPrinter {
    let driver = std::env::var("KONAN_DRIVER").expect("Missing KONAN_DRIVER");
    match driver.to_lowercase().as_str() {
        "usb" => {
            let vendor_id = std::env::var("KONAN_USB_DRIVER_VENDOR_ID")
                .expect("Missing KONAN_USB_DRIVER_VENDOR_ID")
                .parse()
                .expect("KONAN_USB_DRIVER_VENDOR_ID is not a valid u16");
            let product_id = std::env::var("KONAN_USB_DRIVER_PRODUCT_ID")
                .expect("Missing KONAN_USB_DRIVER_PRODUCT_ID")
                .parse()
                .expect("KONAN_USB_DRIVER_PRODUCT_ID is not a valid u16");
            AnyPrinter::Usb(
                build_usb_printer(vendor_id, product_id).expect("Failed to build USB printer"),
            )
        }
        "network" => {
            let host = std::env::var("KONAN_NETWORK_DRIVER_HOST")
                .expect("Missing KONAN_NETWORK_DRIVER_HOST");
            let port = std::env::var("KONAN_NETWORK_DRIVER_PORT")
                .expect("Missing KONAN_NETWORK_DRIVER_PORT")
                .parse()
                .expect("KONAN_NETWORK_DRIVER_PORT is not a valid u16");
            AnyPrinter::Network(
                build_network_printer(host, port).expect("Failed to build network printer"),
            )
        }
        "console" => {
            AnyPrinter::Console(build_console_printer().expect("Failed to build console printer"))
        }
        other => panic!("Unknown KONAN_DRIVER: '{other}'. Expected one of: usb, network, console"),
    }
}

pub enum AnyPrinter {
    Usb(Printer<UsbDriver>),
    Network(Printer<NetworkDriver>),
    Console(Printer<ConsoleDriver>),
}

macro_rules! delegate_printer_method {
    ($method:ident $(, $arg:ident : $ty:ty)*) => {
        pub fn $method(&mut self $(, $arg: $ty)*) -> escpos::errors::Result<()> {
            match self {
                AnyPrinter::Usb(p) => { p.$method($($arg),*)?; },
                AnyPrinter::Network(p) => { p.$method($($arg),*)?; },
                AnyPrinter::Console(p)=>{ p.$method($($arg),*)?; }
            }
        Ok(())
        }
    };
}

impl AnyPrinter {
    delegate_printer_method!(feed);
    delegate_printer_method!(print);
    delegate_printer_method!(print_cut);
    delegate_printer_method!(write, text: &str);
    delegate_printer_method!(justify, mode: JustifyMode);
    delegate_printer_method!(bold, enabled: bool);
    delegate_printer_method!(size, width:u8, height:u8);
    delegate_printer_method!(reset_size);
    delegate_printer_method!(page_code, code: PageCode);
    delegate_printer_method!(reset_line_spacing);
    delegate_printer_method!(line_spacing, spacing:u8);
}
