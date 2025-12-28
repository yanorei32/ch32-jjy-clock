#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use ch32_hal::Config;
use ch32_hal::Peri;
use ch32_hal::gpio::{AnyPin, Level, Output};
use ch32_hal::println;
use embassy_executor::Spawner;
use embassy_time::{Timer, Instant};
use panic_halt as _;

// ch32-halの外部割り込み機能を試す
// ExtiInputが存在するか確認
use ch32_hal::exti::ExtiInput;

#[embassy_executor::task]
async fn blink(pin: Peri<'static, AnyPin>, interval_ms: u64) {
    let mut led = Output::new(pin, Level::Low, Default::default());

    loop {
        led.set_high();
        Timer::after_millis(interval_ms).await;
        led.set_low();
        Timer::after_millis(interval_ms).await;
    }
}

#[embassy_executor::main(entry = "ch32_hal::entry")]
async fn main(spawner: Spawner) -> ! {
    ch32_hal::debug::SDIPrint::enable();

    let p = ch32_hal::init(Config::default());

    // Adjust the LED GPIO according to your board
    spawner.spawn(blink(p.PA0.into(), 100)).unwrap();

    // 外部割り込みを使用する場合のタスク
    // ExtiInputを作成するために、ペリフェラル、EXTIライン、プル設定が必要
    let exti_button = ExtiInput::new(p.PA1, p.EXTI1, ch32_hal::gpio::Pull::Up);
    spawner.spawn(button_task(exti_button)).unwrap();

    loop {
        Timer::after_millis(1000).await;
    }
}

#[embassy_executor::task]
async fn button_task(mut exti_button: ExtiInput<'static>) {
    let mut last_time_ms = 0u64;

    loop {
        // 外部割り込みでlowになるまで待機（立ち下がりエッジを待つ）
        exti_button.wait_for_falling_edge().await;

        let current_time_ms = Instant::now().as_millis();

        if last_time_ms > 0 {
            let elapsed_ms = current_time_ms - last_time_ms;
            println!("button pressed! elapsed: {} ms", elapsed_ms);
        } else {
            println!("button pressed!");
        }

        last_time_ms = current_time_ms;

        // ボタンが離される（highになる）まで待機（チャタリング防止）
        exti_button.wait_for_rising_edge().await;
    }
}
