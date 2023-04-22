use anyhow::Result;
use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use grobot::Config;
use toml::from_str;

const CONFIG: &str = include_str!("../configs/default.toml");

const NOMINAL_TEMP: f32 = 72.0;
const NOMINAL_HUMIDITY: f32 = 60.0;

#[test]
fn test_parse_config() -> Result<()> {
    let _: Config = from_str(CONFIG)?;
    Ok(())
}

#[test]
fn test_config_times() -> Result<()> {
    let mut default_config: Config = from_str(CONFIG)?;

    let time_801am_april_23_2023 = "2023-04-23 08:01";
    let parsed_time = NaiveDateTime::parse_from_str(time_801am_april_23_2023, "%Y-%m-%d %H:%M")?;
    let local = Local.from_local_datetime(&parsed_time).unwrap();

    assert!(
        default_config.fan_on(&local, (NOMINAL_TEMP, NOMINAL_HUMIDITY)),
        "Fan expected on at 8am"
    );
    assert!(
        default_config.light_on(&local, (NOMINAL_TEMP, NOMINAL_HUMIDITY)),
        "light expected on at 8am"
    );

    let time_1230pm_april_23_2023 = "2023-04-23 12:30";
    let parsed_time = NaiveDateTime::parse_from_str(time_1230pm_april_23_2023, "%Y-%m-%d %H:%M")?;
    let local = Local.from_local_datetime(&parsed_time).unwrap();

    assert!(
        default_config.fan_off(&local, (NOMINAL_TEMP, NOMINAL_HUMIDITY)),
        "fan expected off at 1230pm"
    );
    assert!(
        default_config.light_off(&local, (NOMINAL_TEMP, NOMINAL_HUMIDITY)),
        "light expected off at 1230pm"
    );

    Ok(())
}
