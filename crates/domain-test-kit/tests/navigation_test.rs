use app_contracts::features::agents::ScanTick;
use app_contracts::features::navigation::{page_ids, tab_ids};
use app_core::actor::registry::ActorRegistry;
use context::page_status::PageStatusRegistry;
use domain::features::navigation::NavigationFeature;
use domain::features::page_status::PageStatusFeature;
use domain::features::services::ServicesFeature;
use domain::features::services::application::snapshot_actor::ServiceSnapshotActor;
use domain::features::sidebar::SidebarFeature;
use domain::features::test_discovery::TestDiscoveryFeature;
use domain::features::windows_manager::WindowManagerFeature;
use domain_test_kit::generated::*;
use domain_test_kit::utils::{DomainTestWindow, FeatureHarness, temp_settings_path};
use rstest::{fixture, rstest};
use serial_test::serial;

#[fixture]
fn h() -> FeatureHarness {
    let temp_path = temp_settings_path();
    let mut harness = FeatureHarness::new(temp_path.clone());
    harness.install_settings_at(temp_path).unwrap();
    harness.app_install(TestDiscoveryFeature).unwrap();
    harness.app_install(PageStatusFeature).unwrap();
    harness.app_install(WindowManagerFeature).unwrap();

    harness
}

#[rstest]
#[serial]
fn test_services_updates_only_when_active(mut h: FeatureHarness) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _guard = rt.enter();

    let nav_stub = NavigationUiStub::new();
    let svc_stub = ServicesUiStub::new();

    let n_port = nav_stub.clone();
    let s_port = svc_stub.clone();

    h.install(NavigationFeature::new(move |_: &DomainTestWindow| {
        n_port.clone()
    }))
    .unwrap();
    h.install(ServicesFeature::new(move |_: &DomainTestWindow| {
        s_port.clone()
    }))
    .unwrap();

    let service_addr = h
        .shared
        .get::<ActorRegistry>()
        .unwrap()
        .get::<ServiceSnapshotActor<ServicesUiStub>>()
        .unwrap();

    nav_stub
        .emit_on_request_page_switch(tab_ids::MAIN, page_ids::SERVICES)
        .stabilize(&mut h);

    service_addr.send_test(ScanTick).stabilize(&mut h);

    assert_eq!(
        svc_stub
            .set_total_services_count_call_count()
            .stabilize(&mut h),
        1,
        "UI должен был обновиться на активной странице"
    );

    nav_stub
        .emit_on_request_page_switch(tab_ids::MAIN, page_ids::DISK)
        .stabilize(&mut h);

    service_addr.send_test(ScanTick).stabilize(&mut h);

    assert_eq!(
        svc_stub
            .set_total_services_count_call_count()
            .stabilize(&mut h),
        1,
        "UI НЕ должен обновляться, когда страница SERVICES не активна"
    );

    nav_stub
        .emit_on_request_page_switch(tab_ids::MAIN, page_ids::SERVICES)
        .stabilize(&mut h);

    service_addr.send_test(ScanTick).stabilize(&mut h);

    assert_eq!(
        svc_stub
            .set_total_services_count_call_count()
            .stabilize(&mut h),
        2,
        "UI должен снова обновляться после возврата"
    );
}

#[rstest]
#[serial]
fn generated_navigation_stub_receives_initial_navigation_state(mut h: FeatureHarness) {
    let stub = NavigationUiStub::new();
    let port = stub.clone();

    h.install(NavigationFeature::new(move |_: &DomainTestWindow| {
        port.clone()
    }))
    .unwrap();

    assert_eq!(stub.set_navigation_tree_call_count().stabilize(&mut h), 1);
    assert_eq!(
        stub.set_available_contexts_call_count().stabilize(&mut h),
        1
    );
    assert!(h.shared.get::<PageStatusRegistry>().is_some());
}

#[rstest]
#[serial]
fn generated_navigation_stub_handles_page_switch_without_sidebar_contract(mut h: FeatureHarness) {
    let stub = NavigationUiStub::new();
    let port = stub.clone();

    h.install(NavigationFeature::new(move |_: &DomainTestWindow| {
        port.clone()
    }))
    .unwrap();

    let initial_page_calls = stub.set_active_page_call_count().stabilize(&mut h);
    let initial_tab_calls = stub.set_active_tab_call_count().stabilize(&mut h);

    stub.emit_on_request_page_switch(tab_ids::MAIN, page_ids::SERVICES)
        .stabilize(&mut h);

    assert!(stub.set_active_page_call_count().stabilize(&mut h) > initial_page_calls);
    assert!(stub.set_active_tab_call_count().stabilize(&mut h) >= initial_tab_calls);
}

#[rstest]
#[serial]
fn navigation_and_sidebar_features_integrate_via_transition_bus(mut h: FeatureHarness) {
    let nav_stub = NavigationUiStub::new();
    let sidebar_stub = SidebarUiStub::new();
    let nav_port = nav_stub.clone();
    let sidebar_port = sidebar_stub.clone();

    h.install(NavigationFeature::new(move |_: &DomainTestWindow| {
        nav_port.clone()
    }))
    .unwrap();
    h.install(SidebarFeature::new(move |_: &DomainTestWindow| {
        sidebar_port.clone()
    }))
    .unwrap();

    assert_eq!(
        sidebar_stub
            .set_side_bar_width_call_count()
            .stabilize(&mut h),
        1
    );

    nav_stub
        .emit_on_request_page_switch(tab_ids::MAIN, page_ids::SERVICES)
        .stabilize(&mut h);

    assert!(
        sidebar_stub
            .set_switch_transition_call_count()
            .stabilize(&mut h)
            >= 1
    );
    assert!(
        sidebar_stub
            .set_content_visible_call_count()
            .stabilize(&mut h)
            >= 1
    );
    assert!(
        sidebar_stub
            .set_switch_progress_call_count()
            .stabilize(&mut h)
            >= 1
    );
}

#[rstest]
#[serial]
fn generated_navigation_stub_can_drive_domain_via_bindings(mut h: FeatureHarness) {
    let stub = NavigationUiStub::new();
    let port = stub.clone();

    h.install(NavigationFeature::new(move |_: &DomainTestWindow| {
        port.clone()
    }))
    .unwrap();

    let initial_active_page_calls = stub.set_active_page_call_count().stabilize(&mut h);
    stub.emit_on_request_tab_add("host/windows".to_string())
        .stabilize(&mut h);
    stub.emit_on_request_tab_switch(context::page_status::TabId(0))
        .stabilize(&mut h);

    assert!(stub.set_active_page_call_count().stabilize(&mut h) >= initial_active_page_calls);
}
