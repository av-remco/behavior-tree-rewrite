mod bt;
mod conversion;
mod execution;
mod nodes;
mod nodes_bin;

pub use crate::{
    bt::BT,
    nodes::{
        action::{Action, Wait, Success, Failure},
        condition::Condition,
        selector::{Sequence, Fallback},
    },
};

#[cfg(test)]
mod tests;

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