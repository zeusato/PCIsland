//! System indicators: volume, power/battery, idle screen flag (docs/modules/system-indicators.md).
#![forbid(unsafe_code)]

use std::time::Duration;

use crate::core::activity::payload;
use tokio::sync::watch;

use crate::core::activity::{Activity, ActivityHub};
use crate::core::events::{AppEvent, EventBus};
use crate::core::priority;
use crate::core::settings::Settings;
use crate::platform::audio_endpoint::{self, PowerStatus};

const POWER_POLL: Duration = Duration::from_secs(5);

pub fn setup(hub: ActivityHub, settings: watch::Receiver<Settings>, bus: EventBus) {
    // Volume: event-driven from the endpoint callback.
    let vol_hub = hub.clone();
    let vol_settings = settings.clone();
    let mut last = (-1.0f32, false);
    let mut registered = false;
    audio_endpoint::spawn(move |level, muted, initial| {
        // A re-registration after the first one means the default device changed.
        if initial {
            if registered {
                bus.send(AppEvent::AudioDeviceChanged);
            }
            registered = true;
        }
        let changed = (level - last.0).abs() >= 0.01 || muted != last.1;
        last = (level, muted);
        if initial || !changed || !vol_settings.borrow().indicators.volume_alert {
            return;
        }
        vol_hub.alert(
            Activity::new(
                "indicators",
                "indicators:volume",
                "indicators.volume",
                priority::ALERT,
                payload(&VolumePayload { level, muted }),
            )
            .transient(1_500),
        );
    });

    // Idle screen flag + power polling.
    tauri::async_runtime::spawn(async move {
        let mut settings = settings;
        let mut previous: Option<PowerStatus> = audio_endpoint::power_status();
        let mut warned: Vec<u8> = Vec::new();
        loop {
            publish_idle(&hub, &settings.borrow());
            tokio::select! {
                _ = settings.changed() => continue,
                () = tokio::time::sleep(POWER_POLL) => {}
            }
            let Some(now) = audio_endpoint::power_status() else {
                continue;
            };
            let s = settings.borrow().indicators.clone();
            if let (Some(prev), Some(pct)) = (previous, now.percent) {
                if prev.on_ac != now.on_ac && s.power_alert {
                    warned.clear();
                    hub.alert(
                        Activity::new(
                            "indicators",
                            "indicators:power",
                            "indicators.power",
                            priority::ALERT,
                            payload(&PowerPayload {
                                on_ac: now.on_ac,
                                percent: pct,
                                seconds_left: now.seconds_left,
                            }),
                        )
                        .transient(3_000),
                    );
                }
                if !now.on_ac {
                    if let Some(&threshold) = s
                        .battery_low_thresholds
                        .iter()
                        .filter(|t| pct <= **t && !warned.contains(t))
                        .min()
                    {
                        warned.extend(s.battery_low_thresholds.iter().filter(|t| pct <= **t));
                        hub.alert(
                            Activity::new(
                                "indicators",
                                "indicators:battery-low",
                                "indicators.battery_low",
                                priority::ALERT,
                                payload(&BatteryLowPayload {
                                    percent: pct,
                                    threshold,
                                }),
                            )
                            .transient(if threshold <= 10 {
                                10_000
                            } else {
                                6_000
                            }),
                        );
                    }
                }
            }
            previous = Some(now);
        }
    });
}

fn publish_idle(hub: &ActivityHub, settings: &Settings) {
    match settings.island.idle_display.as_str() {
        mode @ ("eyes" | "clock") => hub.upsert(Activity::new(
            "indicators",
            "indicators:idle",
            "indicators.idle",
            priority::IDLE,
            payload(&IdlePayload { mode: mode.into() }),
        )),
        _ => hub.remove("indicators:idle"),
    }
}

#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct VolumePayload {
    pub level: f32,
    pub muted: bool,
}

#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PowerPayload {
    pub on_ac: bool,
    pub percent: u8,
    pub seconds_left: Option<u32>,
}

#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct BatteryLowPayload {
    pub percent: u8,
    pub threshold: u8,
}

#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct IdlePayload {
    /// "eyes" | "clock"
    pub mode: String,
}
