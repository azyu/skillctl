pub const COMMIT: &str = env!("SKILLCTL_BUILD_COMMIT");
pub const BUILD_TIME: &str = env!("SKILLCTL_BUILD_TIME");

pub const DISPLAY_VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "+",
    env!("SKILLCTL_BUILD_COMMIT")
);
pub const CLAP_LONG_VERSION: &str = concat!(
    "version ",
    env!("CARGO_PKG_VERSION"),
    "+",
    env!("SKILLCTL_BUILD_COMMIT"),
    "\ncommit: ",
    env!("SKILLCTL_BUILD_COMMIT"),
    "\nbuilt: ",
    env!("SKILLCTL_BUILD_TIME")
);

pub fn metadata() -> String {
    format!("skillctl version {DISPLAY_VERSION}\ncommit: {COMMIT}\nbuilt: {BUILD_TIME}\n")
}
