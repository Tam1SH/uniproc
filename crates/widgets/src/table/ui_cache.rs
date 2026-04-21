use slint::{ModelRc, VecModel};
use std::collections::HashMap;
use std::rc::Rc;

pub trait SlintTableRowAdapter<TSlintRow, TSlintField> {
    fn unique_id(&self) -> String;
    fn to_slint_row(&self, cells: ModelRc<TSlintField>) -> TSlintRow;
    fn update_slint_fields(&self, model: &Rc<VecModel<TSlintField>>);
}

pub struct UiTableCache<TSlintRow, TSlintField> {
    entries: HashMap<usize, (String, TSlintRow, Rc<VecModel<TSlintField>>)>,
}

impl<TSlintRow, TSlintField> Default for UiTableCache<TSlintRow, TSlintField> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
}

impl<TSlintRow, TSlintField> UiTableCache<TSlintRow, TSlintField>
where
    TSlintRow: Clone,
    TSlintField: Clone + PartialEq + 'static,
{
    pub fn get_row<T>(&mut self, index: usize, source: &T) -> TSlintRow
    where
        T: SlintTableRowAdapter<TSlintRow, TSlintField>,
    {
        let new_id = source.unique_id();

        if let Some((id, row_obj, fields_model)) = self.entries.get_mut(&index)
            && *id == new_id
        {
            source.update_slint_fields(fields_model);

            *row_obj = source.to_slint_row(ModelRc::from(fields_model.clone()));
            return row_obj.clone();
        }

        let fields_model = Rc::new(VecModel::default());
        source.update_slint_fields(&fields_model);

        let row_obj = source.to_slint_row(ModelRc::from(fields_model.clone()));
        self.entries
            .insert(index, (new_id, row_obj.clone(), fields_model));

        row_obj
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}
