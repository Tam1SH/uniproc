use crate::features::agents::AgentConnectionState;
use guinea_core::Load;
use guinea_macros::{actions, port, reducer};
use serde::{Deserialize, Serialize};
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, amethystate::AmeType)]
pub struct ColumnConfig {
    pub width: u64,
    pub min_width: u64,
    pub visible: bool,
}

impl Default for ColumnConfig {
    fn default() -> Self {
        Self {
            width: 110,
            min_width: 80,
            visible: true,
        }
    }
}

/// Which bucket a process belongs to on the Processes page.
///
/// Task Manager's own three-way split (apps / background / Windows), with
/// two refinements: background is split by vendor, because "some Microsoft
/// thing" and "some vendor's updater" are answers to different questions;
/// and Windows processes are split into SCM-managed services and kernel
/// pseudo-processes, which are genuinely different populations - a service
/// is a unit that can share its process with others, while the kernel ones
/// have no image on disk at all.
///
/// A process matches exactly one of these: the order in
/// [`ProcessCategory::classify`] is the tie-break, not a suggestion.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum ProcessCategory {
    /// Owns at least one visible top-level window.
    App,
    /// No window, Microsoft-signed, not a service.
    BackgroundMicrosoft,
    /// No window, not Microsoft-signed, not a service.
    BackgroundThirdParty,
    /// Hosts at least one service registered with the SCM.
    WindowsService,
    /// Kernel pseudo-process: System, Registry, Memory Compression and
    /// friends. Identified by having no image on disk, not by name.
    WindowsKernel,
}

impl ProcessCategory {
    /// First match wins. Windows never gives a kernel pseudo-process a
    /// window, and a service that owns one is a service the user is looking
    /// at - so putting `App` first costs nothing and keeps the rule short.
    pub fn classify(
        has_visible_window: bool,
        is_kernel_process: bool,
        is_service: bool,
        is_microsoft_signed: bool,
    ) -> Self {
        if has_visible_window {
            Self::App
        } else if is_kernel_process {
            Self::WindowsKernel
        } else if is_service {
            Self::WindowsService
        } else if is_microsoft_signed {
            Self::BackgroundMicrosoft
        } else {
            Self::BackgroundThirdParty
        }
    }

    /// Display order of the buckets, and the identity used for grouping.
    ///
    /// Third-party comes before Microsoft inside Background on purpose:
    /// the user's own software is what they came to look at, and the
    /// platform's background noise should not be what they scroll past to
    /// reach it.
    pub const ORDER: [Self; 5] = [
        Self::App,
        Self::BackgroundThirdParty,
        Self::BackgroundMicrosoft,
        Self::WindowsService,
        Self::WindowsKernel,
    ];

    /// The heading this category renders under.
    ///
    /// One flat level, not a parent/child tree. Nesting was tried and
    /// dropped: it cost two extra indent levels and a heading that could not
    /// be collapsed (a parent covering two categories has no single thing to
    /// hide), and bought nothing a longer label does not say outright.
    pub fn label(self) -> &'static str {
        match self {
            Self::App => "Apps",
            // The unqualified name is the third-party one because that is
            // the common case a person is looking for; Microsoft's own
            // background noise is the one that needs calling out.
            Self::BackgroundThirdParty => "Background processes",
            Self::BackgroundMicrosoft => "Background processes (Microsoft)",
            // Not "Windows services": third-party software registers
            // services too, and most of what lands here is theirs.
            Self::WindowsService => "Services",
            Self::WindowsKernel => "Windows kernel",
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct ProcessRow {
    pub pid: u32,
    /// The image name as the OS reports it (`explorer.exe`) - the identity
    /// used for grouping and matching, never for display.
    pub name: String,
    /// What to actually show ("Windows Explorer"). Falls back to `name`
    /// when the agent could not resolve one.
    pub display_name: String,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub disk_bytes: u64,
    pub net_bytes: u64,
    pub exe_path: String,
    pub package_full_name: String,
    pub category: ProcessCategory,
}

#[derive(Clone, PartialEq, Debug, Default)]
pub struct MachineSummary {
    pub cpu_percent: f32,
    pub cpu_current_mhz: u64,
    pub cpu_max_mhz: u64,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
}

#[derive(Clone)]
pub enum ProcessesMsg {
    SetRows {
        rows: Rc<[ProcessRow]>,
        machine: MachineSummary,
        /// Where the agent connection stands as of this batch. Rows survive
        /// a disconnect - they are the last thing that was true - so without
        /// this the page cannot tell a quiet machine from a dead agent.
        agent_state: AgentConnectionState,
    },
    SetSelected(Option<u32>),
    SetSort {
        column: String,
        descending: bool,
    },
}

#[port]
pub trait ProcessesPort: 'static {
    fn send(&self, msg: ProcessesMsg);
}

#[actions]
pub trait ProcessesActions {
    fn on_sort<F>(&self, handler: F)
    where
        F: Fn(String) + 'static;

    fn on_select<F>(&self, handler: F)
    where
        F: Fn(u32) + 'static;

    fn on_deselect<F>(&self, handler: F)
    where
        F: Fn() + 'static;

    fn on_terminate<F>(&self, handler: F)
    where
        F: Fn() + 'static;
}

#[derive(Clone, PartialEq, Debug)]
pub struct ProcessesState {
    /// Agent reports as an async resource: `Load::Loading` until the first
    /// snapshot arrives. An empty vec inside `Ready` is a valid answer (and a
    /// search filter may legitimately produce one) - it must never be read as
    /// "still loading".
    pub rows: Load<Rc<[ProcessRow]>>,
    /// Machine-level summary (CPU and memory totals) delivered together with
    /// each process snapshot.
    pub machine_summary: Load<MachineSummary>,
    pub selected: Option<u32>,
    pub sort_column: String,
    pub descending: bool,
    pub agent_state: AgentConnectionState,
}

impl Default for ProcessesState {
    fn default() -> Self {
        Self {
            rows: Load::Loading,
            machine_summary: Load::Loading,
            selected: None,
            agent_state: AgentConnectionState::Disconnected,
            sort_column: "cpu".to_string(),
            descending: true,
        }
    }
}

impl ProcessesState {
    pub fn rows(&self) -> &[ProcessRow] {
        self.rows.ready().map(|r| r.as_ref()).unwrap_or(&[])
    }

    pub fn total(&self) -> usize {
        self.rows().len()
    }

    pub fn machine_summary(&self) -> Option<&MachineSummary> {
        self.machine_summary.ready()
    }
}

#[reducer]
#[dispatch(ProcessesActions)]
pub fn processes_reducer(state: &mut ProcessesState, msg: ProcessesMsg) {
    match msg {
        ProcessesMsg::SetRows { rows, machine, agent_state } => {
            state.rows = Load::Ready(rows);
            state.machine_summary = Load::Ready(machine);
            state.agent_state = agent_state;
        }
        ProcessesMsg::SetSelected(pid) => {
            state.selected = pid;
        }
        ProcessesMsg::SetSort { column, descending } => {
            state.sort_column = column;
            state.descending = descending;
        }
    }
}
