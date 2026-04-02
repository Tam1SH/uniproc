#[cfg(windows)]
mod windows;

use crate::processes_impl::scanner::base::{DisplayNameRequest, VisitorContext};
use crate::processes_impl::scanner::field_value::{FieldValue, FieldValueKind};
use context::caches::strings::StringsProvider;
use dashmap::DashMap;
use slint::SharedString;

pub struct StatefulContext {
    pub cache: DashMap<(u32, &'static str), FieldValue>,
    pub display_names: DashMap<String, SharedString>,
    pub windows_cache: DashMap<u32, String>,
    pub services_cache: DashMap<u32, String>,
}

impl StatefulContext {
    pub fn new() -> Self {
        Self {
            cache: DashMap::new(),
            display_names: DashMap::new(),
            windows_cache: DashMap::new(),
            services_cache: DashMap::new(),
        }
    }

    //TODO: just use it
    pub fn clear_dead_processes(&self, active_pids: &[u32]) {
        self.cache
            .retain(|(pid, _), _| *pid == 0 || active_pids.contains(pid));
        self.windows_cache
            .retain(|pid, _| active_pids.contains(pid));
        self.services_cache
            .retain(|pid, _| active_pids.contains(pid));
    }

    pub fn refresh_system_metadata(&self) {
        #[cfg(windows)]
        {
            // TODO: Refactor this to be integrated into the main backend process snapshot logic
            //       instead of polling these maps independently to avoid unnecessary overhead.
            self.windows_cache.clear();
            for (pid, title) in windows::get_visible_windows_map() {
                self.windows_cache.insert(pid, title);
            }

            self.services_cache.clear();
            for (pid, name) in windows::get_active_services_map() {
                self.services_cache.insert(pid, name);
            }
        }
    }
}

impl VisitorContext for StatefulContext {
    fn get_field_value(
        &self,
        pid: u32,
        field_id: &'static str,
        kind: FieldValueKind,
    ) -> FieldValue {
        self.cache
            .entry((pid, field_id))
            .or_insert_with(|| FieldValue::new(kind))
            .clone()
    }

    fn resolve_display_name(&self, req: DisplayNameRequest) -> SharedString {
        #[cfg(windows)]
        let pkg_name = req.package_full_name.unwrap_or_default();

        #[cfg(windows)]
        let key = if !pkg_name.is_empty() {
            pkg_name
        } else {
            req.process_name
        };
        #[cfg(not(windows))]
        let key = req.process_name;

        if let Some(cached) = self.display_names.get(key) {
            return cached.clone();
        }

        let display_name = {
            #[cfg(windows)]
            {
                let mut resolved = None;

                if !pkg_name.is_empty() {
                    resolved = windows::get_package_display_name(pkg_name);
                }

                if resolved.is_none()
                    && let Some(title) = self.windows_cache.get(&req.pid)
                {
                    resolved = Some(title.clone());
                }

                if resolved.is_none()
                    && let Some(svc) = self.services_cache.get(&req.pid)
                {
                    resolved = Some(svc.clone());
                }

                if resolved.is_none()
                    && let Some(path) = req.exe_path.filter(|p| !p.is_empty())
                {
                    resolved = windows::get_win32_description(path);
                }

                resolved.unwrap_or_else(|| {
                    windows::get_shell_name(req.process_name)
                        .unwrap_or_else(|| req.process_name.to_string())
                })
            }
            #[cfg(target_os = "linux")]
            {
                req.process_name.to_string()
            }
        };

        let shared = self.intern(&display_name);
        self.display_names.insert(key.to_string(), shared.clone());
        shared
    }

    fn tick(&self) {
        self.refresh_system_metadata()
    }

    fn intern(&self, s: &str) -> SharedString {
        StringsProvider::global().intern(s)
    }
}
