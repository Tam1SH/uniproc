use macros::slint_port;

use super::model::WslDistroDto;

#[slint_port(global = "EnvironmentsFeatureGlobal")]
pub trait UiEnvironmentsPort: 'static {
    #[manual]
    fn set_host_icon_by_key(&self, icon_key: &str);
    #[manual]
    fn set_wsl_distros(&self, distros: Vec<WslDistroDto>);
    fn set_host_name(&self, name: String);
    fn set_selected_env(&self, name: String);
    fn set_has_wsl(&self, has_wsl: bool);
    #[slint(global = "EnvsLoading")]
    fn set_wsl_is_loading(&self, loading: bool);
    #[slint(global = "EnvsLoading")]
    fn set_wsl_distros_is_loading(&self, loading: bool);
}
