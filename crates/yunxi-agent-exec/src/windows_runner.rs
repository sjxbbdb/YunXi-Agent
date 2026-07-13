pub fn runner_boundary_summary() -> &'static str {
    "Windows DirectProcessRunner manages child process lifecycle only; restricted-token filesystem isolation is not enabled in this build"
}

pub fn os_filesystem_isolation_enabled() -> bool {
    false
}
