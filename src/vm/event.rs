pub trait Observer<T> {
    fn notify(&self, event: T);
}

pub trait Observable<T> {
    fn register(&mut self, observer: Box<dyn Observer<T>>);
}

#[derive(Clone, Copy)]
pub struct VmEvent {
    pub event_type: EventType,
    pub moved_from: Option<usize>,
    pub offset: Option<usize>,
    pub warrior_id: usize,
    pub round: u128,
}

#[derive(Clone, Copy)]
pub enum EventType {
    TerminatedProgram,
    TerminatedThread,
    Change,
    Jump,
}
