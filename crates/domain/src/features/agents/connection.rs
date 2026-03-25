#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    WaitingRetry { delay_secs: u64 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConnectionEvent {
    BeginConnect,
    ConnectSucceeded,
    ConnectFailed,
    RetryDelayElapsed,
    ConnectionLost,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitionEffect {
    None,
    ScheduleRetry { delay_secs: u64 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Transition {
    pub from: ConnectionState,
    pub event: ConnectionEvent,
    pub to: ConnectionState,
    pub effect: TransitionEffect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidTransition {
    pub state: ConnectionState,
    pub event: ConnectionEvent,
}

#[derive(Debug)]
pub struct ConnectionMachine {
    state: ConnectionState,
    next_retry_delay_secs: u64,
    max_retry_delay_secs: u64,
}

impl ConnectionMachine {
    pub fn new() -> Self {
        Self {
            state: ConnectionState::Disconnected,
            next_retry_delay_secs: 1,
            max_retry_delay_secs: 15,
        }
    }

    pub fn apply(&mut self, event: ConnectionEvent) -> Result<Transition, InvalidTransition> {
        let from = self.state;

        let (to, effect) = match (self.state, event) {
            (ConnectionState::Disconnected, ConnectionEvent::BeginConnect) => {
                (ConnectionState::Connecting, TransitionEffect::None)
            }
            (ConnectionState::Connecting, ConnectionEvent::ConnectSucceeded) => {
                self.next_retry_delay_secs = 1;
                (ConnectionState::Connected, TransitionEffect::None)
            }
            (ConnectionState::Connecting, ConnectionEvent::ConnectFailed) => {
                let delay_secs = self.next_retry_delay_secs;
                self.next_retry_delay_secs =
                    (delay_secs.saturating_mul(2)).min(self.max_retry_delay_secs);
                (
                    ConnectionState::WaitingRetry { delay_secs },
                    TransitionEffect::ScheduleRetry { delay_secs },
                )
            }
            (ConnectionState::WaitingRetry { .. }, ConnectionEvent::RetryDelayElapsed) => {
                (ConnectionState::Connecting, TransitionEffect::None)
            }
            (ConnectionState::Connected, ConnectionEvent::ConnectionLost) => {
                (ConnectionState::Disconnected, TransitionEffect::None)
            }
            _ => {
                return Err(InvalidTransition {
                    state: self.state,
                    event,
                });
            }
        };

        self.state = to;
        Ok(Transition {
            from,
            event,
            to,
            effect,
        })
    }

    pub fn state(&self) -> ConnectionState {
        self.state
    }
}
