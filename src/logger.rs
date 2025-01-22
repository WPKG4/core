use tracing::{level_filters::LevelFilter, Level};

const MAX_LEVEL: LevelFilter = LevelFilter::DEBUG;

pub fn init() {
    better_panic::install();
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();
}
