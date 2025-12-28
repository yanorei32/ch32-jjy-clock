#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use ch32_hal::Config;
use ch32_hal::Peri;
use ch32_hal::exti::ExtiInput;
use ch32_hal::gpio::{AnyPin, Level, Output};
use ch32_hal::println;
use embassy_executor::Spawner;
use embassy_time::{Instant, Timer};
use panic_halt as _;

#[embassy_executor::task]
async fn display_task(
    rs: Peri<'static, AnyPin>,
    rw: Peri<'static, AnyPin>,
    enable: Peri<'static, AnyPin>,
    db0: Peri<'static, AnyPin>,
    db1: Peri<'static, AnyPin>,
    db2: Peri<'static, AnyPin>,
    db3: Peri<'static, AnyPin>,
    db4: Peri<'static, AnyPin>,
    db5: Peri<'static, AnyPin>,
    db6: Peri<'static, AnyPin>,
    db7: Peri<'static, AnyPin>,
) {
    let mut rs = Output::new(rs, Level::Low, Default::default());
    let mut rw = Output::new(rw, Level::Low, Default::default());
    let mut enable = Output::new(enable, Level::Low, Default::default());
    let mut db0 = Output::new(db0, Level::Low, Default::default());
    let mut db1 = Output::new(db1, Level::Low, Default::default());
    let mut db2 = Output::new(db2, Level::Low, Default::default());
    let mut db3 = Output::new(db3, Level::Low, Default::default());
    let mut db4 = Output::new(db4, Level::Low, Default::default());
    let mut db5 = Output::new(db5, Level::Low, Default::default());
    let mut db6 = Output::new(db6, Level::Low, Default::default());
    let mut db7 = Output::new(db7, Level::Low, Default::default());

    Timer::after_millis(100).await;


    // Function Set
    rs.set_low();
    rw.set_low();
    db7.set_low();
    db6.set_low();
    db5.set_high();
    db4.set_high();
    db3.set_high(); // N
    db2.set_low(); // F
    db1.set_low(); // X
    db0.set_low(); // X
    Timer::after_millis(10).await;
    enable.set_high();
    Timer::after_millis(10).await;
    enable.set_low();

    // Display ON/OFF
    rs.set_low();
    rw.set_low();
    db7.set_low();
    db6.set_low();
    db5.set_low();
    db4.set_low();
    db3.set_high();
    db2.set_high(); // Display ON/OFF
    db1.set_low(); // Cursor ON/OFF
    db0.set_high(); // Brink ON/OFF
    Timer::after_millis(10).await;
    enable.set_high();
    Timer::after_millis(10).await;
    enable.set_low();

    // Display Clear
    rs.set_low();
    rw.set_low();
    db7.set_low();
    db6.set_low();
    db5.set_low();
    db4.set_low();
    db3.set_low();
    db2.set_low();
    db1.set_low();
    db0.set_high();
    Timer::after_millis(10).await;
    enable.set_high();
    Timer::after_millis(10).await;
    enable.set_low();


    // Entry Mode Set
    rs.set_low();
    rw.set_low();
    db7.set_low();
    db6.set_low();
    db5.set_low();
    db4.set_low();
    db3.set_low();
    db2.set_high();
    db1.set_high(); // Cursor Moving Direction (high: inc, low: dec)
    db0.set_low(); // Speify Shift of Display
    Timer::after_millis(10).await;
    enable.set_high();
    Timer::after_millis(10).await;
    enable.set_low();

    // Send Data
    rs.set_high();
    rw.set_low();
    db7.set_high();
    db6.set_low();
    db5.set_high();
    db4.set_high();
    db3.set_low();
    db2.set_low();
    db1.set_low();
    db0.set_high();
    Timer::after_millis(10).await;
    enable.set_high();
    Timer::after_millis(10).await;
    enable.set_low();
}

#[embassy_executor::main(entry = "ch32_hal::entry")]
async fn main(spawner: Spawner) -> ! {
    ch32_hal::debug::SDIPrint::enable();

    let p = ch32_hal::init(Config::default());

    spawner.spawn(display_task(
        p.PA2.into(), // rs
        p.PA3.into(), // rw
        p.PA4.into(), // enable
        p.PA5.into(), // d0
        p.PA6.into(), // d1
        p.PA7.into(), // d2
        p.PB0.into(), // d3
        p.PB1.into(), // d4
        p.PA8.into(), // d5
        p.PA9.into(), // d6
        p.PA10.into(), // d7
    )).unwrap();

    // 外部割り込みを使用する場合のタスク
    // ExtiInputを作成するために、ペリフェラル、EXTIライン、プル設定が必要
    let exti_button = ExtiInput::new(p.PA0, p.EXTI0, ch32_hal::gpio::Pull::None);
    spawner.spawn(jjy_task(exti_button)).unwrap();

    loop {
        Timer::after_millis(1000).await;
        // println!("poll");
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum BitWidth {
    Unknown,
    Marker,
    Short,
    Long,
}

impl BitWidth {
    fn as_str(&self) -> &'static str {
        match self {
            BitWidth::Unknown => "Unknown",
            BitWidth::Marker => "Marker",
            BitWidth::Short => "Short",
            BitWidth::Long => "Long",
        }
    }

    fn try_as_bool(&self) -> Option<bool> {
        match self {
            BitWidth::Unknown => None,
            BitWidth::Marker => None,
            BitWidth::Short => Some(true),
            BitWidth::Long => Some(false),
        }
    }
}

#[embassy_executor::task]
async fn jjy_task(mut exti_button: ExtiInput<'static>) {
    const ALLOWED_ERROR: f32 = 0.20;

    let mut buffer = [BitWidth::Unknown; 60];
    let mut cursor = 0;
    let mut recording = false;
    let mut previous_is_marker = false;

    fn is_in_width(left_hand: u32, right_hand: u32) -> bool {
        let max_time = right_hand as f32 * (1.0 + ALLOWED_ERROR);
        let min_time = right_hand as f32 * (1.0 - ALLOWED_ERROR);
        let actual_time = left_hand as f32;
        min_time < actual_time && actual_time < max_time
    }

    loop {
        exti_button.wait_for_falling_edge().await;
        let up_at = Instant::now().as_millis();

        exti_button.wait_for_rising_edge().await;
        let down_at = Instant::now().as_millis();

        let elapsed_ms = (down_at - up_at) as u32;

        let bit = match elapsed_ms {
            ms if is_in_width(ms, 200) => BitWidth::Marker,
            ms if is_in_width(ms, 500) => BitWidth::Short,
            ms if is_in_width(ms, 800) => BitWidth::Long,
            _ => BitWidth::Unknown,
        };

        println!("{} ms ({})", elapsed_ms, bit.as_str());

        if bit == BitWidth::Unknown {
            println!("ABORT! Unknown width is comming");
            cursor = 0;
            recording = false;
            continue;
        }

        if bit == BitWidth::Marker {
            if previous_is_marker {
                println!("Start Bit Detected!");
                recording = true;
                cursor = 0;
            }

            previous_is_marker = true;
        } else {
            previous_is_marker = false;
        }


        if recording {
            if cursor == 38 {
                fn to_minute_hour_day(buf: &[BitWidth]) -> Option<(u32, u32, u32)> {
                    let mut minute = 0;
                    let mut minute_parity = false;
                    let mut hour = 0;
                    let mut hour_parity = false;
                    let mut day = 0;

                    if buf[1].try_as_bool()? {
                        minute += 40;
                        minute_parity = !minute_parity;
                    }

                    if buf[2].try_as_bool()? {
                        minute += 20;
                        minute_parity = !minute_parity;
                    }

                    if buf[3].try_as_bool()? {
                        minute += 10;
                        minute_parity = !minute_parity;
                    }

                    if buf[5].try_as_bool()? {
                        minute += 8;
                        minute_parity = !minute_parity;
                    }

                    if buf[6].try_as_bool()? {
                        minute += 4;
                        minute_parity = !minute_parity;
                    }

                    if buf[7].try_as_bool()? {
                        minute += 2;
                        minute_parity = !minute_parity;
                    }

                    if buf[8].try_as_bool()? {
                        minute += 1;
                        minute_parity = !minute_parity;
                    }

                    if buf[12].try_as_bool()? {
                        hour += 20;
                        hour_parity = !hour_parity;
                    }

                    if buf[13].try_as_bool()? {
                        hour += 10;
                        hour_parity = !hour_parity;
                    }

                    if buf[15].try_as_bool()? {
                        hour += 8;
                        hour_parity = !hour_parity;
                    }

                    if buf[16].try_as_bool()? {
                        hour += 4;
                        hour_parity = !hour_parity;
                    }

                    if buf[17].try_as_bool()? {
                        hour += 2;
                        hour_parity = !hour_parity;
                    }

                    if buf[18].try_as_bool()? {
                        hour += 1;
                        hour_parity = !hour_parity;
                    }

                    if buf[22].try_as_bool()? {
                        day += 200;
                    }

                    if buf[23].try_as_bool()? {
                        day += 100;
                    }

                    if buf[25].try_as_bool()? {
                        day += 80;
                    }

                    if buf[26].try_as_bool()? {
                        day += 40;
                    }

                    if buf[27].try_as_bool()? {
                        day += 20;
                    }

                    if buf[28].try_as_bool()? {
                        day += 10;
                    }

                    if buf[30].try_as_bool()? {
                        day += 8;
                    }

                    if buf[31].try_as_bool()? {
                        day += 4;
                    }

                    if buf[32].try_as_bool()? {
                        day += 2;
                    }

                    if buf[33].try_as_bool()? {
                        day += 1;
                    }

                    if buf[36].try_as_bool()? != hour_parity {
                        return None;
                    }

                    if buf[37].try_as_bool()? != minute_parity {
                        return None;
                    }

                    Some(( minute, hour, day ))
                }

                let Some((minute, hour, day)) = to_minute_hour_day(&buffer) else {
                    cursor = 0;
                    recording = false;
                    continue;
                };

                println!("{hour:0>2}:{minute:0>2} (day: {day})");
            }

            buffer[cursor] = bit;

            cursor += 1;
            cursor %= 60;
        }
    }
}
