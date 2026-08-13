pub use app_contracts::l10n::L10n;

pub fn tr() -> L10n {
    guinea_core::l10n::L10n::<L10n>::current()
}

pub fn use_tr(cx: &mut windows_reactor::RenderCx) -> L10n {
    guinea::l10n::use_l10n::<L10n>(cx)
}
