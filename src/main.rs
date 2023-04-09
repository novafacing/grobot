use anyhow::{ensure, Error, Result};
use chrono::{DateTime, Local, Timelike};
use crossbeam_channel::{unbounded, Receiver, Sender};
use ctrlc::set_handler;
use dht22_pi::{read as dht22_read, Reading};
use log::{info, LevelFilter};
use log4rs::{
    append::console::ConsoleAppender,
    config::{Appender, Root},
    encode::pattern::PatternEncoder,
    init_config, Config,
};
use ringbuffer::{AllocRingBuffer, RingBuffer, RingBufferExt, RingBufferWrite};
use rppal::{
    gpio::Gpio,
    pwm::{Channel, Polarity, Pwm},
};
use std::{
    thread::{sleep, spawn},
    time::Duration,
};

// NF-F12 industialPPC Fan PWM Frequency
const FAN_PWM_FREQUENCY: f64 = 25_000.0f64;
// Pin for relay CH1
const LIGHT_PIN: u8 = 26;
// Pin for temp/humidity sensor
const SENSOR_PIN: u8 = 4;
// Number of readings to take from the sensor before starting up
const INITIAL_SENSOR_READINGS: u8 = 8;
// Number of readings to take from the sensor each cycle
const SENSOR_READINGS: u8 = 3;
// Number of seconds to wait between sensor readings
const SENSOR_READING_INTERVAL: f32 = 4.0;
// Number of seconds to wait between time/sensor readings
const MAINTHREAD_CYCLE_INTERVAL: f32 = 90.0;

// Threshold for "temp too high" to turn off lights and turn on fans
const THRESHOLD_TEMP_TOO_HIGH: f32 = 82.0;
// Threshold for "temp too low" to turn off fans and turn on lights
const THRESHOLD_TEMP_TOO_LOW: f32 = 62.0;
// Threshold for "humidity too high" to turn on lights and fans
const THRESHOLD_HUMIDITY_TOO_HIGH: f32 = 80.0;

struct Environment {
    readings: AllocRingBuffer<Reading>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            readings: AllocRingBuffer::with_capacity(INITIAL_SENSOR_READINGS as usize),
        }
    }

    fn ctof(&self, c: f32) -> f32 {
        (c * (9.0 / 5.0)) + 32.0
    }

    // Retrive the temperature in Farenheit
    pub fn temp(&self) -> f32 {
        let sum: f32 = self.readings.iter().map(|r| r.temperature).sum();
        let mean = sum / self.readings.len() as f32;

        let sum_dev_sq: f32 = self
            .readings
            .iter()
            .map(|r| (r.temperature - mean) * (r.temperature - mean))
            .sum();

        let std_dev: f32 = (sum_dev_sq / (self.readings.len() as f32 - 1.0)).sqrt();

        let good_samples = self
            .readings
            .iter()
            .filter(|r| (mean - std_dev) <= r.temperature && r.temperature <= (mean + std_dev))
            .map(|r| r.temperature)
            .collect::<Vec<_>>();

        let temp = self.ctof(good_samples.iter().sum::<f32>() / good_samples.len() as f32);

        info!("Cleaned temperature reading: {}F", temp);

        temp
    }

    pub fn humidity(&self) -> f32 {
        let sum: f32 = self.readings.iter().map(|r| r.humidity).sum();
        let mean = sum / self.readings.len() as f32;

        let sum_dev_sq: f32 = self
            .readings
            .iter()
            .map(|r| (r.humidity - mean) * (r.humidity - mean))
            .sum();

        let std_dev: f32 = (sum_dev_sq / (self.readings.len() as f32 - 1.0)).sqrt();

        let good_samples = self
            .readings
            .iter()
            .filter(|r| (mean - std_dev) <= r.humidity && r.humidity <= (mean + std_dev))
            .map(|r| r.humidity)
            .collect::<Vec<_>>();

        let humidity = good_samples.iter().sum::<f32>() / good_samples.len() as f32;

        info!("Cleaned humidity reading: {}%", humidity);

        humidity
    }

    pub fn add_reading(&mut self, reading: Reading) {
        if reading.humidity >= 0.0
            && reading.humidity <= 100.0
            && !reading.temperature.is_nan()
            && !reading.humidity.is_nan()
        {
            info!("Added new sensor reading: {:?}", reading);
            self.readings.push(reading);
        }
    }
}

struct FanPower {
    power: f64,
}

impl TryFrom<f64> for FanPower {
    type Error = Error;

    fn try_from(value: f64) -> Result<Self> {
        ensure!(
            (0.0..=100.0).contains(&value),
            "FanPower value must be between 0 and 100.0 %"
        );

        let power = value / FanPower::CONVERSION_FACTOR;
        Ok(Self { power })
    }
}

impl FanPower {
    /// Covert from 0-100 percentage to 0.0 - 1.0 value
    const CONVERSION_FACTOR: f64 = 100.0;

    pub fn as_duty_cycle(&self) -> f64 {
        self.power
    }
}

enum Message {
    // Local time
    Time(DateTime<Local>),
    // Temp and humidity
    Environment((f32, f32)),
    // Stop now
    Exit,
}

fn light(rx: Receiver<Message>) -> Result<()> {
    let gpio = Gpio::new()?;
    let light_pin = gpio.get(LIGHT_PIN)?;
    let mut light_pin_output = light_pin.into_output();
    light_pin_output.set_high();

    loop {
        match rx.recv()? {
            Message::Time(time) => {
                info!("Light controller got time: {}", time);
                if (time.hour() >= 6 && time.hour() <= 8)
                    || (time.hour() >= 19 && time.hour() <= 22)
                {
                    info!(
                        "Time {:?} is in set lighting on ranges, enabling light",
                        time
                    );
                    light_pin_output.set_low();
                } else {
                    info!(
                        "Time {:?} is out of set lighting on ranges, disabling light",
                        time
                    );
                    light_pin_output.set_high();
                }
            }
            Message::Environment((temp, humidity)) => {
                // Any environment related processing here
                if temp >= THRESHOLD_TEMP_TOO_HIGH {
                    // Turn off lights if temp gets too high
                    info!("Temperature {} above threshold, disabling light", temp);
                    light_pin_output.set_high();
                } else if temp <= THRESHOLD_TEMP_TOO_LOW {
                    // Turn on lgiths if temp gets too low
                    info!("Temperature {} below threshold, enabling light", temp);
                    light_pin_output.set_low();
                }

                if humidity >= THRESHOLD_HUMIDITY_TOO_HIGH {
                    info!("Humidity {} above threshold, enabling light", humidity);
                    light_pin_output.set_low();
                }
            }
            Message::Exit => {
                // Exit the loop and the thread
                info!("Received exit message on light thread, exiting");
                break;
            }
        }
    }

    Ok(())
}

fn fan(rx: Receiver<Message>) -> Result<()> {
    // Start up the fan at 0% power
    let fan_pwm = Pwm::with_frequency(
        Channel::Pwm0,
        FAN_PWM_FREQUENCY,
        0.00, // 75% Power
        Polarity::Normal,
        true,
    )?;

    // Fan power 75%
    let fan_power = FanPower::try_from(75.0)?;

    loop {
        match rx.recv()? {
            Message::Time(time) => {
                // Run fans for 10 mins at the top of the hour
                if time.minute() <= 10 {
                    info!(
                        "Time {:?} is in first 10 minutes of the hour, running fans",
                        time
                    );
                    fan_pwm.set_duty_cycle(fan_power.as_duty_cycle())?;
                } else {
                    info!(
                        "Time {:?} is not in the first 10 minutes of the hour, not running fans",
                        time
                    );
                }
            }
            Message::Environment((temp, humidity)) => {
                // Do something with env
                if temp >= THRESHOLD_TEMP_TOO_HIGH {
                    info!("Temperature {} over threshold, running fans", temp);
                    fan_pwm.set_duty_cycle(fan_power.as_duty_cycle())?;
                } else if temp <= THRESHOLD_TEMP_TOO_LOW {
                    info!("Temperature {} below threshold, disabling fans", temp);
                    fan_pwm.set_duty_cycle(0.0)?;
                }

                if humidity >= THRESHOLD_HUMIDITY_TOO_HIGH {
                    info!("Humidity {} above threshold, running fans", humidity);
                    fan_pwm.set_duty_cycle(fan_power.as_duty_cycle())?;
                }
            }
            Message::Exit => {
                info!("Received exit message on fan thread, exiting");
                break;
            }
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    let appender = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{h({l})} | {d(%Y-%m-%d %H:%M:%S)} | {m} [{f}]{n}",
        )))
        .build();

    let config = Config::builder()
        .appender(Appender::builder().build("console", Box::new(appender)))
        .build(
            Root::builder()
                .appender("console")
                .build(LevelFilter::Trace),
        )?;

    init_config(config)?;

    let (tx, rx): (Sender<Message>, Receiver<Message>) = unbounded();

    let term_tx = tx.clone();

    set_handler(move || term_tx.send(Message::Exit).expect("Could not send exit"))?;

    let light_rx = rx.clone();
    let fan_rx = rx.clone();

    let light = spawn(|| light(light_rx));
    let fan = spawn(|| fan(fan_rx));

    let mut environment = Environment::new();

    info!("Taking initial sensor readings");

    for _ in 0..INITIAL_SENSOR_READINGS {
        if let Ok(reading) = dht22_read(SENSOR_PIN) {
            environment.add_reading(reading);
        }
        sleep(Duration::from_secs_f32(SENSOR_READING_INTERVAL));
    }

    let main_rx = rx;

    loop {
        info!("Taking sensor readings on main thread");

        for _ in 0..SENSOR_READINGS {
            if let Ok(reading) = dht22_read(SENSOR_PIN) {
                environment.add_reading(reading);
            }

            if let Ok(Message::Exit) = main_rx.try_recv() {
                info!("Got exit message on main thread, exiting");
                break;
            }

            sleep(Duration::from_secs_f32(SENSOR_READING_INTERVAL));
        }

        tx.send(Message::Environment((
            environment.temp(),
            environment.humidity(),
        )))?;

        if let Ok(Message::Exit) = main_rx.try_recv() {
            info!("Got exit message on main thread, exiting");
            break;
        }

        info!("Sleeping for cycle interval");

        sleep(Duration::from_secs_f32(MAINTHREAD_CYCLE_INTERVAL));

        if let Ok(Message::Exit) = main_rx.try_recv() {
            info!("Got exit message on main thread, exiting");
            break;
        }

        tx.send(Message::Time(Local::now()))?;
    }

    light.join().expect("Could not join light thread").ok();
    fan.join().expect("Could not join fan thread").ok();

    info!("grobot done, goodbye");

    Ok(())
}
