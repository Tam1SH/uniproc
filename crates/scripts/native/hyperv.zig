const std = @import("std");

const c = @cImport({
    @cDefine("WIN32_LEAN_AND_MEAN", "1");
    @cInclude("windows.h");
    @cInclude("sddl.h");
    @cInclude("lm.h");
});

export fn fix_hyperv_privileges() i32 {
    var p_sid: c.PSID = undefined;

    // S-1-5-32-578
    if (c.ConvertStringSidToSidW(std.unicode.utf8ToUtf16LeStringLiteral("S-1-5-32-578"), &p_sid) == 0) return -1;

    var name_buf: [256]u16 = undefined;
    var dom_buf: [256]u16 = undefined;
    var n_len: c.DWORD = name_buf.len;
    var d_len: c.DWORD = dom_buf.len;
    var use: c.SID_NAME_USE = undefined;

    if (c.LookupAccountSidW(null, p_sid, &name_buf, &n_len, &dom_buf, &d_len, &use) == 0) return -2;

    var u_buf: [256]u16 = undefined;
    var u_len: c.DWORD = u_buf.len;
    if (c.GetUserNameW(&u_buf, &u_len) == 0) return -3;

    var info = c.LOCALGROUP_MEMBERS_INFO_3{ .lgrmi3_domainandname = &u_buf };

    const res = c.NetLocalGroupAddMembers(null, &name_buf, 3, @ptrCast(&info), 1);

    if (res == c.NERR_Success) return 0;
    if (res == c.ERROR_MEMBER_IN_ALIAS) return 1;
    if (res == c.ERROR_ACCESS_DENIED) return 5;
    return @intCast(res);
}
