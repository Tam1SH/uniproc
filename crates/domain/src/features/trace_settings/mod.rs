mod settings;

use app_core::app::{Feature, Window};
use app_core::reactor::Reactor;
use app_core::trace::TracePolicy;
use app_core::SharedState;
use context::settings::reactive::ReactiveSettingSubscription;
use std::sync::Arc;

use self::settings::TraceSettings;

#[derive(Default)]
pub struct TraceSettingsFeature;

impl<TWindow> Feature<TWindow> for TraceSettingsFeature
where
    TWindow: Window,
{
    fn install(
        self,
        _reactor: &mut Reactor,
        _ui: &TWindow,
        shared: &SharedState,
    ) -> anyhow::Result<()> {
        let settings = TraceSettings::new(shared)?;
        apply_trace_policy(&settings);

        let mut subs = Vec::new();

        {
            let settings = settings.clone();
            subs.push(settings.enable_scopes().subscribe(move |_| apply_trace_policy(&settings)));
        }
        {
            let settings = settings.clone();
            subs.push(settings.disable_scopes().subscribe(move |_| apply_trace_policy(&settings)));
        }
        {
            let settings = settings.clone();
            subs.push(
                settings
                    .disable_messages()
                    .subscribe(move |_| apply_trace_policy(&settings)),
            );
        }
        {
            let settings = settings.clone();
            subs.push(
                settings
                    .disable_targets()
                    .subscribe(move |_| apply_trace_policy(&settings)),
            );
        }
        {
            let settings = settings.clone();
            subs.push(
                settings
                    .dump_capacity()
                    .subscribe(move |_| apply_trace_policy(&settings)),
            );
        }

        shared.insert_arc(Arc::new(TraceSettingsRuntime { _subs: subs }));
        Ok(())
    }
}

struct TraceSettingsRuntime {
    _subs: Vec<ReactiveSettingSubscription>,
}

fn apply_trace_policy(settings: &TraceSettings) {
    let builtin = context::trace::builtin_policy();
    let policy = context::trace::normalize_policy(TracePolicy {
        enabled_prefixes: merge_trace_values(builtin.enabled_prefixes, settings.enable_scopes().get()),
        disabled_prefixes: settings.disable_scopes().get(),
        disabled_message_prefixes: merge_trace_values(
            builtin.disabled_message_prefixes,
            settings.disable_messages().get(),
        ),
        disabled_target_prefixes: merge_trace_values(
            builtin.disabled_target_prefixes,
            settings.disable_targets().get(),
        ),
        dump_capacity: settings.dump_capacity().get() as usize,
    });

    app_core::trace::install_policy(policy);
}

fn merge_trace_values(mut builtin: Vec<String>, user: Vec<String>) -> Vec<String> {
    let mut values = Vec::new();
    values.append(&mut builtin);
    values.extend(user);
    values.sort();
    values.dedup();
    values
}
