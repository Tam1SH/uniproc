pub mod actor;
mod model;
mod settings;
mod state;

use crate::features::navigation::actor::{NavigationActor, Push};
use crate::features::navigation::settings::NavigationSettings;
use app_contracts::features::navigation::PAGE_ROUTES;
use app_contracts::features::navigation::{NavigationBinder, UiNavigationBindings};
use app_core::actor::addr::Addr;
use framework::app::Window;
use framework::feature::{
    AppFeature, AppFeatureInitContext, WindowFeature, WindowFeatureInitContext,
};
use framework::navigation::{Route, RouteRegistry};
use framework::uri::ContextlessAppUri;
use macros::window_feature;
use std::borrow::Cow;
use std::sync::Arc;

pub struct NavigationRegistryFeature;

impl AppFeature for NavigationRegistryFeature {
    fn install(self, ctx: &mut AppFeatureInitContext) -> anyhow::Result<()> {
        let registry = Arc::new(RouteRegistry::new());
        registry.replace_routes(
            PAGE_ROUTES
                .iter()
                .map(|route| Route {
                    uri: ContextlessAppUri::new(
                        Cow::from(route.segment),
                        route
                            .features
                            .iter()
                            .map(|&s| Cow::from(s))
                            .collect::<Vec<_>>(),
                    ),
                })
                .collect(),
        );
        ctx.shared.insert_arc(registry);
        Ok(())
    }
}

#[window_feature]
pub struct NavigationFeature;

#[window_feature]
impl<TWindow, F, P> WindowFeature<TWindow> for NavigationFeature<F>
where
    TWindow: Window,
    F: Fn(&TWindow) -> P + 'static + Clone,
    P: UiNavigationBindings + Clone + 'static,
{
    fn install(&mut self, ctx: &mut WindowFeatureInitContext<TWindow>) -> anyhow::Result<()> {
        let settings = NavigationSettings::new(ctx.shared)?;
        let ui_port = (self.make_port)(ctx.ui);
        let token = ctx.ui.new_token();

        let registry = ctx.shared.get::<RouteRegistry>().unwrap();
        let actor = NavigationActor::new(registry.clone(), ctx.window_id);

        let addr = Addr::new_managed(actor, token, &self.tracker);
        let initial_path = settings.default_route_segment().get();
        addr.send(Push(initial_path));

        #[cfg(feature = "test-utils")]
        if let Some(registry) = ctx.shared.get::<app_core::actor::registry::ActorRegistry>() {
            registry.register(addr.clone());
        }

        NavigationBinder::new(&addr, &ui_port).on_push(Push);

        Ok(())
    }
}
