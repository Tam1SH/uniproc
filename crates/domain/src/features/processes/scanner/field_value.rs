use slint::SharedString;
use std::fmt::{Result, Write};
use std::sync::atomic::Ordering;

#[repr(C)]
struct SharedVectorHeader {
    refcount: std::sync::atomic::AtomicIsize,
    size: usize,
    capacity: usize,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum FieldValueKind {
    U64(u64),
    F32(f32),
    Str(SharedString),
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

#[derive(Debug, Clone)]
pub struct FieldValue {
    pub kind: FieldValueKind,
    buffer: SharedString,
}

pub struct RawWriter {
    ptr: *mut u8,
    cap: usize,
    pos: usize,
}

impl Write for RawWriter {
    fn write_str(&mut self, s: &str) -> Result {
        let bytes = s.as_bytes();
        if self.pos + bytes.len() >= self.cap {
            return Err(std::fmt::Error);
        }
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), self.ptr.add(self.pos), bytes.len());
        }
        self.pos += bytes.len();
        Ok(())
    }
}

impl FieldValue {
    pub fn new(kind: FieldValueKind) -> Self {
        let mut buffer = SharedString::default();
        buffer.push_str("00000000000000000000");
        Self { kind, buffer }
    }

    pub fn write_part(
        w: &mut RawWriter,
        value: f64,
        unit: &str,
        formats: &[FieldValueFormat],
    ) -> Result {
        let (no_spaces, no_decimals, no_unit, round_up) = Self::parse_formats(formats);
        let mut scaled = value;
        if round_up {
            scaled = scaled.round();
        }

        let unit = if no_unit { "" } else { unit };
        let space = if no_spaces || unit.is_empty() {
            ""
        } else {
            " "
        };

        if no_decimals {
            write!(w, "{:.0}{}{}", scaled, space, unit)
        } else {
            write!(w, "{:.1}{}{}", scaled, space, unit)
        }
    }

    pub fn format_units_with_params(
        &mut self,
        value: u64,
        step: u64,
        units: &[&str],
        formats: &[FieldValueFormat],
    ) -> SharedString {
        let (no_spaces, no_decimals, no_unit, round_up) = Self::parse_formats(formats);

        let mut scaled = value as f64;
        let mut unit_idx = 0usize;
        let step_f = step.max(2) as f64;
        let units = if units.is_empty() { &[""][..] } else { units };

        while unit_idx + 1 < units.len() && scaled >= step_f {
            scaled /= step_f;
            unit_idx += 1;
        }

        if round_up {
            scaled = scaled.round();
        }

        let unit = if no_unit { "" } else { units[unit_idx] };
        let space = if no_spaces || no_unit || unit.is_empty() {
            ""
        } else {
            " "
        };

        self.write_raw(|w| {
            if unit_idx == 0 || no_decimals {
                write!(w, "{:.0}{}{}", scaled, space, unit)
            } else {
                write!(w, "{:.1}{}{}", scaled, space, unit)
            }
        })
    }

    pub fn to_text(&mut self) -> SharedString {
        if let FieldValueKind::Str(s) = &self.kind {
            return s.clone();
        }

        let kind_copy = self.kind.clone();

        match kind_copy {
            FieldValueKind::Bytes(b) => {
                self.format_units_with_params(b, 1024, &["B", "KB", "MB", "GB", "TB"], &[])
            }
            FieldValueKind::Percent(p) => self.write_raw(|w| write!(w, "{:.1}%", p)),
            FieldValueKind::U64(v) => self.write_raw(|w| write!(w, "{}", v)),
            FieldValueKind::F32(v) => self.write_raw(|w| write!(w, "{:.1}", v)),
            FieldValueKind::Duration(d) => self.write_raw(|w| write!(w, "{}ms", d.as_millis())),
            FieldValueKind::Str(_) => unreachable!(),
        }
    }

    pub fn write_raw<F>(&mut self, mut f: F) -> SharedString
    where
        F: FnMut(&mut RawWriter) -> Result,
    {
        loop {
            unsafe {
                let data_ptr = self.buffer.as_ptr() as *mut u8;
                let header_ptr = (data_ptr as *mut SharedVectorHeader).offset(-1);
                let header = &mut *header_ptr;

                if header.refcount.load(Ordering::Relaxed) < 0 {
                    self.buffer = SharedString::from("                    ");
                    continue;
                }

                let mut writer = RawWriter {
                    ptr: data_ptr,
                    cap: header.capacity,
                    pos: 0,
                };

                if f(&mut writer).is_ok() {
                    std::ptr::write(data_ptr.add(writer.pos), 0);
                    header.size = writer.pos + 1;
                    return self.buffer.clone();
                }
            }
            self.buffer.push_str("          ");
        }
    }

    fn parse_formats(formats: &[FieldValueFormat]) -> (bool, bool, bool, bool) {
        let mut res = (false, false, false, false);
        for f in formats {
            match f {
                FieldValueFormat::WithoutSpaces => res.0 = true,
                FieldValueFormat::WithoutDecimals => res.1 = true,
                FieldValueFormat::WithoutUnit => res.2 = true,
                FieldValueFormat::RoundUp => res.3 = true,
            }
        }
        res
    }
}
