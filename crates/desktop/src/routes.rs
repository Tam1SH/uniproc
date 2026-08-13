use guinea_macros::routes;

use crate::layouts::{ShellLayout, TabsLayout};
use crate::pages::{Processes, Services, Wsl};

routes! {
    Route {
        layout(TabsLayout) {
            layout(ShellLayout) {
                page(Processes, "/")
                page(Services, "/services")
                page(Wsl, "/wsl")
            }
        }
    }
}
