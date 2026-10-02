use crate::app_state::Action;
use crossterm::event::Event;

pub trait Mode {
    fn read_event(&mut self, event: Event) -> Action;
}
