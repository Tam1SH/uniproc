use app_contracts::features::processes::FieldDefDto;

slint::include_modules!();

pub mod adapters;

impl From<FieldDefDto> for FieldDef {
    fn from(value: FieldDefDto) -> Self {
        Self {
            id: value.id.into(),
            label: value.label.into(),
            stat_text: value.stat_text.into(),
            stat_numeric: value.stat_numeric,
            threshold: value.threshold,
            width_px: value.width_px,
            stat_detail: value.stat_detail.map(Into::into).unwrap_or_default(),
            show_indicator: value.show_indicator,
        }
    }
}
