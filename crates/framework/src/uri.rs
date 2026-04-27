use derive_more::{Deref, Display, From};
use std::borrow::Cow;
use std::fmt;
use std::fmt::Display;

#[derive(Clone, Debug, Default, PartialEq, Eq, Display, From, Deref)]
pub struct SegmentName(Cow<'static, str>);

impl Display for AppUri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}://{}", self.context_name, self.base)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ContextlessAppUri {
    pub segment: SegmentName,
    pub capabilities: Cow<'static, [Cow<'static, str>]>,
}

impl Display for ContextlessAppUri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.segment)?;

        if !self.capabilities.is_empty() {
            let caps = self.capabilities.join(",");
            write!(f, "?capabilities={}", caps)?;
        }

        Ok(())
    }
}

impl ContextlessAppUri {
    pub fn new(
        segment: impl Into<SegmentName>,
        capabilities: impl Into<Cow<'static, [Cow<'static, str>]>>,
    ) -> Self {
        Self {
            segment: segment.into(),
            capabilities: capabilities.into(),
        }
    }
    pub fn with_context(self, context_name: impl Into<Cow<'static, str>>) -> AppUri {
        AppUri {
            context_name: context_name.into(),
            base: self,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AppUri {
    pub context_name: Cow<'static, str>,
    pub base: ContextlessAppUri,
}

impl AppUri {
    pub fn new(
        context_name: impl Into<Cow<'static, str>>,
        segment: impl Into<SegmentName>,
        capabilities: impl Into<Cow<'static, [Cow<'static, str>]>>,
    ) -> Self {
        Self {
            base: ContextlessAppUri::new(segment, capabilities),
            context_name: context_name.into(),
        }
    }
    pub fn from_parts(context_name: impl Into<Cow<'static, str>>, base: ContextlessAppUri) -> Self {
        Self {
            context_name: context_name.into(),
            base,
        }
    }
}
