use std::{
    str::{from_utf8, Utf8Chunks},
    time::Duration,
};

use esp_idf_svc::hal::{
    delay::FreeRtos,
    i2c::{config::Config, APBTickType},
    prelude::Peripherals,
};

use std::str::FromStr;

#[derive(Debug)]
struct GpggaData {
    time: String,
    latitude: f64,
    latitude_direction: String,
    longitude: f64,
    longitude_direction: String,
    fix_quality: u8,
    num_satellites: u8,
    hdop: f64,
    altitude: f64,
    altitude_units: String,
}

fn parse_gpgga(sentence: &str) -> Result<GpggaData, String> {
    // Ensure the sentence starts with "$GPGGA"
    if !sentence.starts_with("$GPGGA") {
        return Err("Not a GPGGA sentence".to_string());
    }

    // Split the sentence by commas
    let parts: Vec<&str> = sentence.split(',').collect();

    if parts.len() < 15 {
        return Err("Incomplete GPGGA sentence".to_string());
    }

    // Parse each field
    let time = parts[1].to_string();
    let latitude = parse_coordinate(parts[2], parts[3])?;
    let latitude_direction = parts[3].to_string();
    let longitude = parse_coordinate(parts[4], parts[5])?;
    let longitude_direction = parts[5].to_string();
    let fix_quality = parts[6].parse::<u8>().unwrap_or(0);
    let num_satellites = parts[7].parse::<u8>().unwrap_or(0);
    let hdop = parts[8].parse::<f64>().unwrap_or(0.0);
    let altitude = parts[9].parse::<f64>().unwrap_or(0.0);
    let altitude_units = parts[10].to_string();

    Ok(GpggaData {
        time,
        latitude,
        latitude_direction,
        longitude,
        longitude_direction,
        fix_quality,
        num_satellites,
        hdop,
        altitude,
        altitude_units,
    })
}

fn parse_coordinate(value: &str, direction: &str) -> Result<f64, String> {
    if value.is_empty() || direction.is_empty() {
        return Err("Invalid coordinate data".to_string());
    }

    let degrees = value[0..value.len() - 7].parse::<f64>().unwrap_or(0.0);
    let minutes = value[value.len() - 7..].parse::<f64>().unwrap_or(0.0);
    let mut coordinate = degrees + minutes / 60.0;

    if direction == "S" || direction == "W" {
        coordinate = -coordinate;
    }

    Ok(coordinate)
}
fn extract_nmea_sentence(input: &str) -> Option<String> {
    if let Some(start) = input.find("$GPGGA") {
        if let Some(end) = input[start..].find('\n') {
            return Some(input[start..start + end].to_string());
        }
    }
    None
}

fn generate_checksumed(command: &str) -> String {
    let mut checksum = 0u8;
    let mut c_counter = 0;
    for ch in command.chars() {
        if ch == '*' {
            break;
        } else if ch != '$' {
            checksum ^= ch as u8;
        }
        c_counter += 1;
    }
    format!("{}*{:X}\r\n", &command[0..=c_counter], checksum)
}

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();
    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let periphs = Peripherals::take().unwrap();

    let i2c_sda = unsafe { esp_idf_svc::hal::gpio::AnyIOPin::new(21) };
    let i2c_scl = unsafe { esp_idf_svc::hal::gpio::AnyIOPin::new(22) };
    let mut i2c_driver = esp_idf_svc::hal::i2c::I2cDriver::new(
        periphs.i2c0,
        i2c_sda,
        i2c_scl,
        &Config {
            baudrate: 100_000.into(),
            sda_pullup_enabled: true,
            scl_pullup_enabled: true,
            // timeout: Some(APBTickType::from(Duration::from_millis(100))),
            ..Default::default()
        },
    )
    .unwrap();

    // let command = generate_checksumed("$PSTMRESTOREPAR*");
    // let command_bytes = command.as_bytes();

    // println!("The reset command is: {}", command);
    let mut buf = [0u8; 256];

    // let res = i2c_driver.write_read(0x3A, command_bytes, &mut buf, 10000);
    // print!("Restoring result:{}\n", unsafe {
    //     std::str::from_utf8_unchecked(&buf)
    // });
    // FreeRtos::delay_ms(5000);
    let mut req = generate_checksumed("$PSTMNMEAREQUEST,100000,0*");

    loop {
        let res = i2c_driver.write_read(0x3A, req.as_bytes(), &mut buf, 1200);
        // println!("Reading result: {:?}", i2c_driver.write(0x3a, &req, 1000));
        // i2c_driver.write(0x3a, &req, 1000);
        // i2c_driver.read(0x3a, &mut buf, 2000);
        // println!(
        //     "Writing result: {:?}",
        //     i2c_driver.read(0x3a, &mut buf, 2000)
        // );
        let nmea = unsafe { std::str::from_utf8_unchecked(&buf) };
        if let Some(sentence) = extract_nmea_sentence(nmea) {
            println!("{:?}", parse_gpgga(sentence.as_str()));
        }
        // println!("{:?}", from_utf8(&buf));
        FreeRtos::delay_ms(500);
    }
}
