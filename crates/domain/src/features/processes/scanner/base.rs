use async_trait::async_trait;

#[derive(Debug, Clone)]
pub enum FieldValue {
    U64(u64),
    F32(f32),
    Str(String),
    Bytes(u64),
    Percent(f32),
    Duration(std::time::Duration),
}

pub enum FieldValueFormat {
    WithoutSpaces,
    WithoutDecimals,
    WithoutUnit,
    RoundUp,
}

impl FieldValue {
    pub fn format_bytes(b: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = 1024 * KB;
        const GB: u64 = 1024 * MB;
        match b {
            b if b >= GB => format!("{:.1} GB", b as f64 / GB as f64),
            b if b >= MB => format!("{:.1} MB", b as f64 / MB as f64),
            b if b >= KB => format!("{:.1} KB", b as f64 / KB as f64),
            b => format!("{} B", b),
        }
    }

    pub fn format_bytes_with_params(b: u64, formats: &[FieldValueFormat]) -> String {
        Self::format_units_with_params(
            b,
            1024,
            &["B", "KB", "MB", "GB", "TB", "PB"],
            formats,
        )
    }

    pub fn format_units_with_params(
        value: u64,
        step: u64,
        units: &[&str],
        formats: &[FieldValueFormat],
    ) -> String {
        let units = if units.is_empty() { &[""][..] } else { units };
        let step = step.max(2) as f64;

        let no_spaces = formats
            .iter()
            .any(|f| matches!(f, FieldValueFormat::WithoutSpaces));
        let no_decimals = formats
            .iter()
            .any(|f| matches!(f, FieldValueFormat::WithoutDecimals));
        let no_unit = formats
            .iter()
            .any(|f| matches!(f, FieldValueFormat::WithoutUnit));
        let round_up = formats
            .iter()
            .any(|f| matches!(f, FieldValueFormat::RoundUp));

        let mut scaled = value as f64;
        let mut unit_idx = 0usize;
        while unit_idx + 1 < units.len() && scaled >= step {
            scaled /= step;
            unit_idx += 1;
        }

        if round_up {
            scaled = scaled.round();
        }

        let unit = if no_unit { "" } else { units[unit_idx] };
        let space = if no_spaces || no_unit { "" } else { " " };

        if unit_idx == 0 || no_decimals {
            format!("{:.0}{space}{unit}", scaled, space = space)
        } else {
            format!("{:.1}{space}{unit}", scaled, space = space)
        }
    }

    pub fn format_value_with_params(
        value: f64,
        unit: &str,
        formats: &[FieldValueFormat],
    ) -> String {
        let no_spaces = formats
            .iter()
            .any(|f| matches!(f, FieldValueFormat::WithoutSpaces));
        let no_decimals = formats
            .iter()
            .any(|f| matches!(f, FieldValueFormat::WithoutDecimals));
        let no_unit = formats
            .iter()
            .any(|f| matches!(f, FieldValueFormat::WithoutUnit));
        let round_up = formats
            .iter()
            .any(|f| matches!(f, FieldValueFormat::RoundUp));

        let value = if round_up { value.round() } else { value };
        let unit = if no_unit { "" } else { unit };
        let space = if no_spaces || no_unit { "" } else { " " };

        if no_decimals {
            format!("{:.0}{space}{unit}", value, space = space)
        } else {
            format!("{:.1}{space}{unit}", value, space = space)
        }
    }

    pub fn to_text(&self) -> String {
        match self {
            FieldValue::Bytes(b) => Self::format_bytes(*b),
            FieldValue::Percent(p) => format!("{:.1}%", p),
            FieldValue::U64(v) => v.to_string(),
            FieldValue::F32(v) => format!("{:.1}", v),
            FieldValue::Str(s) => s.clone(),
            FieldValue::Duration(d) => format!("{}ms", d.as_millis()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Field {
    pub id: &'static str,
    pub label: &'static str,
    pub value: FieldValue,
    pub stat_detail: Option<String>,
    pub show_indicator: bool,
    pub numeric: f32,
    pub threshold: f32,
}

pub trait VisitorContext {
    fn get(&self, key: &str) -> Option<f32>;
}

pub trait ProcessVisitor {
    fn pid(&self) -> u32;
    fn name(&self) -> &str;
    fn parent_pid(&self) -> u32;
    fn exe_path(&self) -> Option<&str>;
    fn visit(&self, ctx: &dyn VisitorContext, visitor: &mut dyn FnMut(Field));
}

pub trait ScanResult: Send + Sync {
    fn context(&self) -> Box<dyn VisitorContext>;
    fn visit_processes(&self, visitor: &mut dyn FnMut(&dyn ProcessVisitor));
    fn visit_stats(&self, visitor: &mut dyn FnMut(Field));
}

#[async_trait]
pub trait ProcessScanner: Send {
    fn schema_id(&self) -> &'static str;
    async fn scan(&mut self) -> Box<dyn ScanResult>;
}
