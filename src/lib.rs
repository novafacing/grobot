use std::path::Path;

use anyhow::{ensure, Context, Error, Result};
use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use dht22_pi::{read as dht22_read, Reading};
use ringbuffer::{AllocRingBuffer, RingBuffer, RingBufferExt, RingBufferWrite};
use rppal::{gpio::OutputPin, pwm::Pwm};
use serde::Deserialize;
use tokio::{fs::File, io::AsyncReadExt};
use toml::from_str;
use tracing::{info, warn};

pub struct Environment {
    readings: AllocRingBuffer<Reading>,
}

impl Default for Environment {
    fn default() -> Self {
        Self::with_readings(Environment::DEFAULT_INITIAL_READINGS)
    }
}

impl Environment {
    const DEFAULT_INITIAL_READINGS: usize = 8;

    pub fn with_readings(initial_readings: usize) -> Self {
        Self {
            readings: AllocRingBuffer::with_capacity(initial_readings),
        }
    }

    /// Do the initial set of readings to fill the ring buffer
    pub async fn init(&mut self, pin: u8) -> Result<()> {
        for _ in 0..self.readings.capacity() {
            if let Ok(reading) = dht22_read(pin) {
                self.add_reading(reading);
            } else {
                warn!("Failed to read from sensor");
            }
        }

        Ok(())
    }

    /// Do a single reading from the sensor,
    pub async fn read(&mut self, pin: u8) {
        if let Ok(reading) = dht22_read(pin) {
            self.add_reading(reading);
        } else {
            warn!("Failed to read from sensor");
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

#[derive(Debug, Clone)]
pub struct FanPower {
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

    fn parse_fan_power<'de, D>(deserializer: D) -> Result<FanPower, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = f64::deserialize(deserializer)?;
        FanPower::try_from(s).map_err(serde::de::Error::custom)
    }
}

pub struct Light(OutputPin);

impl Light {
    pub fn new(pin: OutputPin) -> Self {
        Self(pin)
    }

    pub fn on(&mut self) {
        self.0.set_low();
    }

    pub fn off(&mut self) {
        self.0.set_high();
    }
}

pub struct Fan((Pwm, FanPower));

impl Fan {
    pub fn new(pwm: Pwm, power: FanPower) -> Self {
        Self((pwm, power))
    }

    pub fn on(&mut self) -> Result<()> {
        self.0 .0.set_duty_cycle(self.0 .1.as_duty_cycle())?;
        Ok(())
    }

    pub fn off(&mut self) -> Result<()> {
        self.0 .0.set_duty_cycle(0.0)?;
        Ok(())
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Action {
    On,
    Off,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Event {
    #[serde(deserialize_with = "Event::parse_time")]
    time: DateTime<Local>,
    action: Action,
}

impl Event {
    // Parse a time string in %H:%M format with strftime
    fn parse_time<'de, D>(deserializer: D) -> Result<DateTime<Local>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let now = Local::now();
        let now_date = now.format("%Y-%m-%d").to_string();
        let s = String::deserialize(deserializer)?;
        let time = NaiveDateTime::parse_from_str(&format!("{}T{}", now_date, s), "%Y-%m-%dT%H:%M")
            .map_err(serde::de::Error::custom)?;

        let local = Local.from_local_datetime(&time).single().ok_or_else(|| {
            serde::de::Error::custom(format!("Failed to convert {} to local time", time))
        })?;

        Ok(local)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct FanConfig {
    #[serde(deserialize_with = "FanPower::parse_fan_power")]
    power: FanPower,
    schedule: Vec<Event>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct LightConfig {
    schedule: Vec<Event>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ThresholdConfig {
    min_temp: f32,
    min_humidity: f32,
    max_temp: f32,
    max_humidity: f32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    fan: FanConfig,
    light: LightConfig,
    thresholds: ThresholdConfig,
}

impl Config {
    pub fn light_on(&mut self, time: &DateTime<Local>, environment: (f32, f32)) -> bool {
        let (temp, humidity) = environment;
        // Check if the light should be on at the given time by:
        // * Sorting the schedule by time
        // * Bucketing the schedule into pairs of on/off events
        // * Checking if the time is between any of the on/off pairs
        let light_on_schedule = self.light.schedule.chunks(2).any(|pair| {
            let on = &pair[0];
            let off = &pair[1];

            on.time.time() <= time.time() && off.time.time() > time.time()
        });

        // Check if the light should be on due to the humidity
        // If humidity is too high, we turn on to burn off the excess
        // If temperature is too high, we turn off the light to prevent overheating
        // If temperature is too low, we turn on the light to increase the temperature
        let light_on_environment =
            humidity > self.thresholds.max_humidity || temp < self.thresholds.min_temp;

        let light_off_environment = temp > self.thresholds.max_temp;

        (light_on_schedule || light_on_environment) && !light_off_environment
    }

    pub fn light_off(&mut self, time: &DateTime<Local>, environment: (f32, f32)) -> bool {
        !self.light_on(time, environment)
    }

    pub fn fan_on(&mut self, time: &DateTime<Local>, environment: (f32, f32)) -> bool {
        let (temp, humidity) = environment;
        // Check if the fan should be on at the given time by:
        // * Bucketing the schedule into pairs of on/off events
        // * Checking if the time is between any of the on/off pairs
        let fan_on_schedule = self.fan.schedule.chunks(2).any(|pair| {
            let on = &pair[0];
            let off = &pair[1];

            on.time.time() <= time.time() && off.time.time() > time.time()
        });

        // Check if the fan should be on due to the humidity
        // If humidity is too high, we turn on to circulate and lower humidity
        // If humidity is too low, we turn off to avoid dehumidifying
        // If temperature is too high, we turn on to circulate and lower temperature
        // If temperature is too low, we turn off to avoid lowering it further
        let fan_on_environment =
            humidity > self.thresholds.max_humidity || temp > self.thresholds.max_temp;

        let fan_off_environment =
            humidity < self.thresholds.min_humidity || temp < self.thresholds.min_temp;

        (fan_on_schedule || fan_on_environment) && !fan_off_environment
    }

    pub fn fan_off(&mut self, time: &DateTime<Local>, environment: (f32, f32)) -> bool {
        !self.fan_on(time, environment)
    }

    pub fn fan_power(&self) -> FanPower {
        self.fan.power.clone()
    }

    pub fn setup(&mut self) -> Result<()> {
        // Sort the schedules by time ascending
        self.light.schedule.sort_by(|a, b| a.time.cmp(&b.time));
        self.fan.schedule.sort_by(|a, b| a.time.cmp(&b.time));

        // Ensure the schedule is valid (this reduces to the same as checking open/close parens lol)
        // We can start with either an on or off event, they just need to be balanced
        let first_light = self
            .light
            .schedule
            .first()
            .context("Must have something in the schedule")?;

        ensure!(
            first_light.action == Action::On,
            "Schedule must start with an On or Off event"
        );

        // Ensure the schedule is valid (this reduces to the same as checking open/close parens lol)
        // We can start with either an on or off event, they just need to be balanced
        let first_fan = self
            .fan
            .schedule
            .first()
            .context("Must have something in the schedule")?;

        ensure!(
            first_fan.action == Action::On,
            "Schedule must start with an On or Off event"
        );

        Ok(())
    }

    pub async fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        // Use toml::from_str
        let mut file = File::open(path).await?;
        let mut s = String::new();
        file.read_to_string(&mut s).await?;
        let mut config: Config = from_str(&s)?;
        config.setup()?;

        Ok(config)
    }
}
