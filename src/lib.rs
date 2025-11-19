mod bt;

pub use crate::bt::{
    handle::{NodeError, NodeHandle},
    nodes::NodeType,
};

#[cfg(test)]
#[allow(dead_code)]
pub(crate) mod logging {
    use env_logger::Env;

    pub fn load_logger() {
        let filter = "debug";
        let log_level = Env::default().default_filter_or(filter);
        env_logger::Builder::from_env(log_level)
            .format_timestamp(None)
            .init();
    }
}