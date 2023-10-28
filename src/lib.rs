use std::collections::HashSet;

use asr::{
    timer, 
    timer::TimerState, 
    time::Duration, 
    Process, 
    future::{next_tick, IntoOption}, 
    Error, Address, 
    watcher::Pair, 
    settings::Gui,
};
use bytemuck::Pod;

const PROCESS_NAME: &str = "null_1.0.exe";

asr::async_main!(stable);

#[derive(Gui)]
struct Settings {
    /// Split after each level
    #[default = false]
    split_level: bool,

    /// Split after each chapter
    #[default = true]
    split_chapter: bool
}

struct Watcher<T> {
    watcher: asr::watcher::Watcher<T>,
    address: Address,
}

impl<T : Pod> Watcher<T> {
    fn new(address: Address) -> Self {
        Self {
            watcher: asr::watcher::Watcher::new(),
            address,
        }
    }

    fn update(&mut self, process: &Process) -> Option<&Pair<T>> {
        self.watcher.update(process.read(self.address).into_option())
    }
}

struct Watchers {
    is_main_menu: Watcher<u8>,
    is_finished: Watcher<u8>,
    room_number: Watcher<u8>,
    hours: Watcher<u8>,
    minutes: Watcher<u8>,
    seconds: Watcher<u8>,
    sub_second_frame: Watcher<u8>,
}

impl Watchers {
    fn new(process: &Process) -> Result<Self, Error> {
        let main_module = process.get_module_address(PROCESS_NAME)?;
        Ok(Watchers { 
            is_main_menu: Watcher::new(main_module.add(0x43C924)),
            is_finished: Watcher::new(main_module.add(0x43C925)),
            room_number: Watcher::new(main_module.add(0x43C926)),
            hours: Watcher::new(main_module.add(0x43C927)),
            minutes: Watcher::new(main_module.add(0x43C928)),
            seconds: Watcher::new(main_module.add(0x43C929)),
            sub_second_frame: Watcher::new(main_module.add(0x43C92A)),
        })
    }

    fn update(&mut self, process: &Process) -> Option<State> {
        let is_main_menu = self.is_main_menu.update(process)?;
        timer::set_variable("is_main_menu", &is_main_menu.to_string());

        let is_finished = self.is_finished.update(process)?;
        timer::set_variable("is_finished", &is_finished.to_string());

        let room_number = self.room_number.update(process)?;
        timer::set_variable("room_number", &room_number.to_string());

        let hours = self.hours.update(process)?;
        timer::set_variable("hours", &hours.to_string());

        let minutes = self.minutes.update(process)?;
        timer::set_variable("minutes", &minutes.to_string());

        let seconds = self.seconds.update(process)?;
        timer::set_variable("seconds", &seconds.to_string());

        let sub_second_frames = self.sub_second_frame.update(process)?;
        timer::set_variable("sub_second_frames", &sub_second_frames.to_string());

        let timer = Pair {
            old: time_components_to_duration(
                hours.old, 
                minutes.old, 
                seconds.old, 
                sub_second_frames.old
            ),
            current: time_components_to_duration(
                hours.current, 
                minutes.current, 
                seconds.current, 
                sub_second_frames.current
            ),
        };
        Some(State { 
            is_main_menu, 
            is_finished, 
            room_number, 
            timer
        })
    }
}

struct State<'a> {
    #[allow(dead_code)]
    is_main_menu: &'a Pair<u8>,
    is_finished: &'a Pair<u8>,
    room_number: &'a Pair<u8>,
    timer: Pair<Duration>,
}

#[allow(dead_code)]
async fn main() {
    let mut settings = Settings::register();
    loop {
        let process = Process::wait_attach(PROCESS_NAME).await;
        if let Ok(mut watchers) = Watchers::new(&process) {
            let mut splits_done: HashSet<u8> = HashSet::new();
            process.until_closes(async {
                loop {
                    settings.update();
                    if let Some(state) = watchers.update(&process) {
                        timer::set_game_time(state.timer.current);
                        timer::pause_game_time();
                        match timer::state() {
                            TimerState::NotRunning => {
                                if should_start(&state) {
                                    timer::start();
                                }
                            },
                            TimerState::Running => {
                                if should_reset(&state) {
                                    timer::reset();
                                }
                                if should_split(&state, &settings, &mut splits_done) {
                                    timer::split();
                                }
                            },
                            _ => (),
                        }
                    }
                    next_tick().await;
                }
            }).await;
        }
    }
}

fn should_start(state: &State) -> bool {
    state.is_main_menu.changed_to(&0)
}

fn should_split(
    state: &State, 
    settings: &Settings,
    splits_done: &mut HashSet<u8>,
) -> bool {
    if state.is_finished.changed_to(&1) {
        return true;
    }

    if state.room_number.increased() {
        let is_chapter_end = state.room_number.current % 4 == 0;
        if settings.split_level || (settings.split_chapter && is_chapter_end) {
            return splits_done.insert(state.room_number.old);
        }
    }

    return false;
}

fn should_reset(state: &State) -> bool {
    state.is_main_menu.changed_to(&1)
}

fn time_components_to_duration(
    hours: u8,
    minutes: u8,
    seconds: u8,
    sub_second_frames: u8
) -> Duration {
    let total_seconds: f32 = 
        hours as f32 * 3600.0 +
        minutes as f32 * 60.0 +
        seconds as f32 +
        sub_second_frames as f32 / 60.0;

    Duration::seconds_f32(total_seconds)
}
