use app_contracts::features::agents::AgentConnectionState;

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
            max_retry_delay_secs: 5,
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
                self.next_retry_delay_secs = (delay_secs.saturating_mul(2)).min(self.max_retry_delay_secs);
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
                return Err(InvalidTransition { state: self.state, event });
            }
        };

        self.state = to;
        Ok(Transition { from, event, to, effect })
    }

    pub fn state(&self) -> AgentConnectionState {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waiting_retry_leads_back_to_connecting() {
        let mut machine = ConnectionMachine::new();
        machine.apply(ConnectionEvent::BeginConnect).unwrap();
        machine.apply(ConnectionEvent::ConnectFailed).unwrap();

        assert!(
            machine.apply(ConnectionEvent::BeginConnect).is_err(),
            "BeginConnect is not the way out of WaitingRetry"
        );
        let t = machine
            .apply(ConnectionEvent::RetryDelayElapsed)
            .expect("the retry timer must move the machine on");
        assert_eq!(t.to, AgentConnectionState::Connecting);
    }

    #[test]
    fn a_lost_connection_can_be_reconnected() {
        let mut machine = ConnectionMachine::new();
        machine.apply(ConnectionEvent::BeginConnect).unwrap();
        machine.apply(ConnectionEvent::ConnectSucceeded).unwrap();

        let t = machine.apply(ConnectionEvent::ConnectionLost).unwrap();
        assert_eq!(t.to, AgentConnectionState::Disconnected);
        assert_eq!(
            machine.apply(ConnectionEvent::BeginConnect).unwrap().to,
            AgentConnectionState::Connecting
        );
    }

    #[test]
    fn retry_delay_doubles_up_to_the_cap_and_resets_on_success() {
        let mut machine = ConnectionMachine::new();
        let mut delays = Vec::new();
        for _ in 0..6 {
            machine.apply(ConnectionEvent::BeginConnect).ok();
            let t = machine.apply(ConnectionEvent::ConnectFailed).unwrap();
            if let TransitionEffect::ScheduleRetry { delay_secs } = t.effect {
                delays.push(delay_secs);
            }
            machine.apply(ConnectionEvent::RetryDelayElapsed).unwrap();
        }
        assert_eq!(delays, vec![1, 2, 4, 5, 5, 5], "doubles, then caps");

        machine.apply(ConnectionEvent::ConnectSucceeded).unwrap();
        machine.apply(ConnectionEvent::ConnectionLost).unwrap();
        machine.apply(ConnectionEvent::BeginConnect).unwrap();
        let t = machine.apply(ConnectionEvent::ConnectFailed).unwrap();
        assert_eq!(
            t.effect,
            TransitionEffect::ScheduleRetry { delay_secs: 1 },
            "a successful connect resets the backoff"
        );
    }
}
