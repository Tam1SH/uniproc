use guinea_core::Load;
use guinea_macros::reducer;
use std::rc::Rc;

use super::messages::{Services, ServicesMsg};
use super::model::ServiceRow;

#[derive(Clone, PartialEq, Debug)]
pub struct ServicesState {
    pub rows: Load<Rc<[ServiceRow]>>,
    pub selected: Option<String>,
    pub sort_column: String,
    pub descending: bool,
}

impl Default for ServicesState {
    fn default() -> Self {
        Self {
            rows: Load::Loading,
            selected: None,
            sort_column: "name".to_string(),
            descending: false,
        }
    }
}

impl ServicesState {
    pub fn rows(&self) -> &[ServiceRow] {
        self.rows.ready().map(|r| r.as_ref()).unwrap_or(&[])
    }

    pub fn total(&self) -> usize {
        self.rows().len()
    }
}

#[reducer]
#[dispatch(Services)]
pub fn services_reducer(state: &mut ServicesState, msg: ServicesMsg) {
    match msg {
        ServicesMsg::SetRows { rows } => {
            state.rows = Load::Ready(rows);
        }
        ServicesMsg::SetSelected(name) => {
            state.selected = name;
        }
        ServicesMsg::SetSort { column, descending } => {
            state.sort_column = column;
            state.descending = descending;
        }
    }
}
