use app_contracts::features::environments::AgentConnectionState;

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
    pub from: AgentConnectionState,
    pub event: ConnectionEvent,
    pub to: AgentConnectionState,
    pub effect: TransitionEffect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidTransition {
    pub state: AgentConnectionState,
    pub event: ConnectionEvent,
}

#[derive(Debug)]
pub struct ConnectionMachine {
    state: AgentConnectionState,
    next_retry_delay_secs: u64,
    max_retry_delay_secs: u64,
}

impl Default for ConnectionMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionMachine {
    pub fn new() -> Self {
        Self {
            state: AgentConnectionState::Disconnected,
            next_retry_delay_secs: 1,
            max_retry_delay_secs: 15,
        }
    }

    pub fn apply(&mut self, event: ConnectionEvent) -> Result<Transition, InvalidTransition> {
        let from = self.state;

        let (to, effect) = match (self.state, event) {
            (AgentConnectionState::Disconnected, ConnectionEvent::BeginConnect) => {
                (AgentConnectionState::Connecting, TransitionEffect::None)
            }
            (AgentConnectionState::Connecting, ConnectionEvent::ConnectSucceeded) => {
                self.next_retry_delay_secs = 1;
                (AgentConnectionState::Connected, TransitionEffect::None)
            }
            (AgentConnectionState::Connecting, ConnectionEvent::ConnectFailed) => {
                let delay_secs = self.next_retry_delay_secs;
                self.next_retry_delay_secs =
                    (delay_secs.saturating_mul(2)).min(self.max_retry_delay_secs);
                (
                    AgentConnectionState::WaitingRetry { delay_secs },
                    TransitionEffect::ScheduleRetry { delay_secs },
                )
            }
            (AgentConnectionState::WaitingRetry { .. }, ConnectionEvent::RetryDelayElapsed) => {
                (AgentConnectionState::Connecting, TransitionEffect::None)
            }
            (AgentConnectionState::Connected, ConnectionEvent::ConnectionLost) => {
                (AgentConnectionState::Disconnected, TransitionEffect::None)
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

    pub fn state(&self) -> AgentConnectionState {
        self.state
    }
}
