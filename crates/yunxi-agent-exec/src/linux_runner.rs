pub fn runner_boundary_summary() -> &'static str {
    "Linux Landlock runner entrypoint is not verified in this build; policy-only sandbox diagnostics remain authoritative"
}

pub fn os_filesystem_isolation_enabled() -> bool {
    false
}
