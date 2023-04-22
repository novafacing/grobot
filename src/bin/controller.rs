use anyhow::{bail, Result};
use chrono::{DateTime, Local};
use clap::Parser;
use dht22_pi::read as dht22_read;
use grobot::{Config, Environment, Fan, Light, PORT};
use rppal::{
    gpio::Gpio,
    pwm::{Channel, Polarity, Pwm},
};
use std::{
    net::{Ipv4Addr, SocketAddrV4},
    path::PathBuf,
    time::Duration,
};
use tokio::{
    net::UdpSocket,
    signal::ctrl_c,
    spawn,
    sync::{
        broadcast::{channel as broadcast, Receiver, Sender},
        oneshot::channel as oneshot,
    },
    time::sleep,
};
use tracing::{info, subscriber::set_global_default, Level};
use tracing_appender::{non_blocking, rolling::hourly};
use tracing_subscriber::FmtSubscriber;

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

const BIND_ADDR: Ipv4Addr = Ipv4Addr::new(0, 0, 0, 0);

#[derive(Parser)]
struct Args {
    /// Path to a configuration file in TOML format. Examples of configurations can be found
    /// in the repository.
    config_file: PathBuf,
    #[clap(short, long, default_value_t = Level::INFO)]
    /// Logging level
    log_level: Level,
    #[clap(short, long, default_value_t = PORT)]
    // Port
    port: u16,
    #[clap(short = 'L', long, default_value_t = BIND_ADDR)]
    // Listen address
    listen_addr: Ipv4Addr,
}

#[derive(Clone, Debug)]
enum Message {
    /// Setup Info
    Setup(Config),
    /// Local time
    Time(DateTime<Local>),
    /// Temp and humidity
    Environment((f32, f32)),
    /// Stop now
    Exit,
}

async fn light(mut rx: Receiver<Message>) -> Result<()> {
    let gpio = Gpio::new()?;
    let light_pin = gpio.get(LIGHT_PIN)?;
    let mut light = Light::new(light_pin.into_output());
    light.off();

    let mut config = if let Message::Setup(config) = rx.recv().await? {
        info!(
            "Light thread received setup message with config {:?}",
            config
        );
        config
    } else {
        bail!("Light thread did not receive setup message");
    };

    let mut last_time = None;
    let mut last_env = None;

    loop {
        match rx.recv().await? {
            Message::Time(time) => {
                info!("Light thread received time update with time {:?}", time);
                last_time = Some(time);
            }
            Message::Environment((temp, humidity)) => {
                // Any environment related processing here
                info!(
                    "Light thread received environment update with temp {}F, humidity {}%",
                    temp, humidity
                );
                last_env = Some((temp, humidity));
            }
            Message::Exit => {
                // Exit the loop and the thread
                info!("Received exit message on light thread, exiting");
                break;
            }
            _ => {
                // Ignore other messages
            }
        }

        if let Some(time) = last_time {
            if let Some((temp, humidity)) = last_env {
                if config.light_on(&time, (temp, humidity)) {
                    info!("Light thread turning light on");
                    light.on();
                } else {
                    info!("Light thread turning light off");
                    light.off();
                }
            }
        }
    }

    Ok(())
}

async fn fan(mut rx: Receiver<Message>) -> Result<()> {
    // Start up the fan at 0% power
    let fan_pwm = Pwm::with_frequency(
        Channel::Pwm0,
        FAN_PWM_FREQUENCY,
        0.00, // 75% Power
        Polarity::Normal,
        true,
    )?;

    let mut config = if let Message::Setup(config) = rx.recv().await? {
        info!("Fan thread received setup message with config {:?}", config);
        config
    } else {
        bail!("Fan thread did not receive setup message");
    };

    let mut fan = Fan::new(fan_pwm, config.fan_power());
    let mut last_time = None;
    let mut last_env = None;

    loop {
        match rx.recv().await? {
            Message::Time(time) => {
                // Run fans for 10 mins at the top of the hour
                info!("Fan thread received time update with time {:?}", time);
                last_time = Some(time);
            }
            Message::Environment((temp, humidity)) => {
                // Do something with env
                info!(
                    "Fan thread received environment update with temp {}F, humidity {}%",
                    temp, humidity
                );

                last_env = Some((temp, humidity));
            }
            Message::Exit => {
                info!("Received exit message on fan thread, exiting");
                break;
            }
            _ => {}
        }

        if let Some(time) = last_time {
            if let Some((temp, humidity)) = last_env {
                if config.fan_on(&time, (temp, humidity)) {
                    info!("Fan thread turning fan on");
                    fan.on()?;
                } else {
                    info!("Fan thread turning fan off");
                    fan.off()?;
                }
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let bind_addr = SocketAddrV4::new(args.listen_addr, 0);
    let broadcast_addr = SocketAddrV4::new(Ipv4Addr::new(255, 255, 255, 255), args.port);
    let sock = UdpSocket::bind(bind_addr).await?;

    info!("Listening on {}", bind_addr);
    info!("Broadcasting to {}", broadcast_addr);
    info!("Listening device: {:?}", sock.device()?);

    sock.set_broadcast(true)?;

    let config = Config::from_file(&args.config_file).await?;

    let file_appender = hourly("/var/log", "grobot.log");
    let (non_blocking, _guard) = non_blocking(file_appender);

    let subscriber = FmtSubscriber::builder()
        .with_max_level(args.log_level)
        .with_writer(non_blocking)
        .finish();

    set_global_default(subscriber)?;

    let (tx, _rx): (Sender<Message>, Receiver<Message>) = broadcast(16);
    let fan_rx = tx.subscribe();
    let light_rx = tx.subscribe();

    let (stop_tx, mut stop_rx) = oneshot();

    spawn(async move {
        ctrl_c().await.unwrap();
        // Your handler here
        stop_tx.send(Message::Exit).unwrap();
    });

    spawn(light(light_rx));
    spawn(fan(fan_rx));

    tx.send(Message::Setup(config))?;

    let mut environment = Environment::default();

    info!("Taking initial sensor readings");

    for _ in 0..INITIAL_SENSOR_READINGS {
        if let Ok(reading) = dht22_read(SENSOR_PIN) {
            environment.add_reading(reading);
        }

        sleep(Duration::from_secs_f32(SENSOR_READING_INTERVAL)).await;

        if let Ok(Message::Exit) = stop_rx.try_recv() {
            info!("Got exit message on main thread, exiting");
            tx.send(Message::Exit)?;
            break;
        }
    }

    loop {
        info!("Taking sensor readings on main thread");

        for _ in 0..SENSOR_READINGS {
            if let Ok(reading) = dht22_read(SENSOR_PIN) {
                environment.add_reading(reading);
            }

            sleep(Duration::from_secs_f32(SENSOR_READING_INTERVAL)).await;
        }

        let msg = environment.json()?;

        info!("Broadcasting sensor readings: '{}'", msg);

        sock.send_to(msg.as_bytes(), broadcast_addr).await?;

        tx.send(Message::Environment((
            environment.temp(),
            environment.humidity(),
        )))?;

        if let Ok(Message::Exit) = stop_rx.try_recv() {
            info!("Got exit message on main thread, exiting");
            tx.send(Message::Exit)?;
            break;
        }

        info!("Sleeping for cycle interval");

        if let Ok(Message::Exit) = stop_rx.try_recv() {
            info!("Got exit message on main thread, exiting");
            tx.send(Message::Exit)?;
            break;
        }

        sleep(Duration::from_secs_f32(MAINTHREAD_CYCLE_INTERVAL)).await;

        tx.send(Message::Time(Local::now()))?;
    }

    info!("grobot done, goodbye");

    Ok(())
}
