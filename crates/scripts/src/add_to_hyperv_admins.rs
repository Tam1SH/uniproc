unsafe extern "C" {
    fn fix_hyperv_privileges() -> i32;
}

pub fn ensure_current_user_is_hyperv_admin() -> i32 {
    unsafe { fix_hyperv_privileges() }
}
