use crate::albion::{EventCode, OperationCode};

pub fn operation(code: i32) -> Option<OperationCode> {
    OperationCode::try_from(code).ok()
}

pub fn event(code: i32) -> Option<EventCode> {
    EventCode::try_from(code).ok()
}
