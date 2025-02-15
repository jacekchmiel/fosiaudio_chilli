use crate::player::Player;
use crate::resource_catalogue::ResourceCatalogue;
use anyhow::Context;
use chrono::{DateTime, Local};
use log::*;
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

struct SchedulerImpl {
    player: Arc<Player>,
    schedule: BTreeSet<DateTime<Local>>,
}

fn parse_and_filter_schedule(text: &str) -> Result<BTreeSet<DateTime<Local>>, anyhow::Error> {
    let now = Local::now();
    let mut schedule: BTreeSet<DateTime<Local>> =
        serde_yaml::from_str(text).context(format!("Parse schedule from \"{text}\""))?;
    schedule.retain(|tp| *tp >= now);
    info!("now: {:?}", now);
    info!("Schedule: {:?}", schedule);
    Ok(schedule)
}

pub const SCHEDULE_DEFAULT: &str = include_str!("schedule_default.yaml");
impl SchedulerImpl {
    fn new(player: Arc<Player>) -> Result<Self, anyhow::Error> {
        Ok(SchedulerImpl {
            player: player,
            schedule: parse_and_filter_schedule(SCHEDULE_DEFAULT)?,
        })
    }

    fn get_serialized_schedule(&self) -> Result<String, anyhow::Error> {
        if self.schedule.is_empty() {
            Ok("nie idziemy :(".to_string())
        } else {
            Ok(serde_yaml::to_string(&self.schedule).context("Serialize current schedule")?)
        }
    }
}

pub struct Scheduler {
    schedule_impl: Mutex<SchedulerImpl>,
    resources: Arc<ResourceCatalogue>,
}
impl Scheduler {
    pub fn new(
        player: Arc<Player>,
        resources: Arc<ResourceCatalogue>,
    ) -> Result<Self, anyhow::Error> {
        Ok(Scheduler {
            schedule_impl: Mutex::new(SchedulerImpl::new(player)?),
            resources: resources,
        })
    }

    pub fn get_serialized_schedule(&self) -> Result<String, anyhow::Error> {
        self.schedule_impl.lock().unwrap().get_serialized_schedule()
    }

    pub fn set_schedule(&self, text: &str) -> Result<(), anyhow::Error> {
        let schedule =
            parse_and_filter_schedule(text).context(format!("Parse schedule from \"{text}\""))?;
        self.schedule_impl.lock().unwrap().schedule = schedule;
        Ok(())
    }

    pub async fn run_schedule(&self) -> () {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
        info!("Running schedule");
        let mut last_cyclic_log = Local::now() - chrono::Duration::hours(1);
        loop {
            let now = Local::now();
            {
                let mut schedule_impl = self.schedule_impl.lock().unwrap();
                if let Some(closest_event) = schedule_impl.schedule.first() {
                    if *closest_event <= now {
                        info!(
                            "Now: {:?}, closest_event: {:?}. Triggering event.",
                            now, closest_event
                        );
                        let playlist = ["idziemy_na_jednego"]
                            .iter()
                            .flat_map(|sample| self.resources.random_sample(sample))
                            .collect();
                        schedule_impl
                            .player
                            .play_local_playlist(playlist)
                            .context("play from schedule")
                            .unwrap_or_else(|e| log::error!("Failed to play schedule: {e}"));
                        schedule_impl.schedule.pop_first().unwrap();
                        info!("Next closest_event: {:?}", schedule_impl.schedule.first());
                    }
                    if now - last_cyclic_log > chrono::Duration::seconds(60) {
                        info!("Next closest_event: {:?}", schedule_impl.schedule.first());
                        last_cyclic_log = now;
                    }
                }
            }
            interval.tick().await;
        }
    }
}
