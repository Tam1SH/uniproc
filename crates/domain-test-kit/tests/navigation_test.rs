use domain::features::page_status::PageStatusFeature;
use domain::features::test_discovery::TestDiscoveryFeature;
use domain::features::windows_manager::WindowManagerFeature;
use domain_navigation::features::navigation::{NavigationFeature, NavigationRegistryFeature};
use domain_test_kit::generated::*;
use domain_test_kit::test_env::navigation::{MockWindowFeature, TestFeatureState};
use domain_test_kit::utils::{temp_settings_path, DomainTestWindow, FeatureHarness};
use framework::navigation::{Route, RouteRegistry};
use framework::uri::ContextlessAppUri;
use i_slint_core::api::ComponentHandle;
use rstest::{fixture, rstest};
use serial_test::serial;
use std::borrow::Cow;
use std::sync::atomic::Ordering;

#[fixture]
fn h() -> FeatureHarness {
    let temp_path = temp_settings_path();

    FeatureHarness::new(temp_path.clone())
        .app_feature(domain::features::settings::SettingsFeature::with_path(
            temp_path,
        ))
        .unwrap()
        .app_feature(TestDiscoveryFeature)
        .unwrap()
        .app_feature(PageStatusFeature)
        .unwrap()
        .app_feature(NavigationRegistryFeature)
        .unwrap()
        .app_feature(WindowManagerFeature)
        .unwrap()
}

#[rstest]
#[serial]
fn test_navigation_correctly_switches_feature_capabilities(mut h: FeatureHarness) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _guard = rt.enter();

    let registry = h
        .shared()
        .get::<RouteRegistry>()
        .expect("RouteRegistry must be installed");

    registry.replace_routes(vec![
        Route {
            uri: ContextlessAppUri::new(Cow::from("mock_route_a"), vec![Cow::Borrowed("cap_a")]),
        },
        Route {
            uri: ContextlessAppUri::new(Cow::from("mock_route_b"), vec![Cow::Borrowed("cap_b")]),
        },
    ]);

    let nav_stub = NavigationUiStub::new();
    let nav_port = nav_stub.clone();

    let state_a = TestFeatureState::default();
    let state_b = TestFeatureState::default();

    let state_a_c = state_a.clone();
    let state_b_c = state_b.clone();

    h = h
        .window_feature(move || {
            let port = nav_port.clone();
            NavigationFeature::new(move |_: &DomainTestWindow| port.clone())
        })
        .window_feature(move || MockWindowFeature::new("cap_a", state_a_c.clone()))
        .window_feature(move || MockWindowFeature::new("cap_b", state_b_c.clone()));

    let ui_handle = h.0.as_ref().unwrap().ui().clone_strong();
    h.0.as_mut()
        .unwrap()
        .spawn_window(ui_handle)
        .expect("Failed to spawn window");
    nav_stub
        .emit_on_push("mock_route_a".into())
        .stabilize(&mut h);

    assert!(
        state_a.is_active.load(Ordering::SeqCst),
        "Feature A MUST be active on mock_route_a"
    );
    assert!(
        !state_b.is_active.load(Ordering::SeqCst),
        "Feature B MUST be inactive on mock_route_a"
    );

    nav_stub
        .emit_on_push("mock_route_b".into())
        .stabilize(&mut h);

    assert!(
        !state_a.is_active.load(Ordering::SeqCst),
        "Feature A MUST deactivate when navigating to mock_route_b"
    );
    assert!(
        state_b.is_active.load(Ordering::SeqCst),
        "Feature B MUST be active on mock_route_b"
    );
}
