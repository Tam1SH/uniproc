use macros::feature_settings;

#[feature_settings(prefix = "navigation")]
pub struct NavigationSettings {
    #[setting(default = "processes".to_string())]
    pub default_route_segment: String,
}
