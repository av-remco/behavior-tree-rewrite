use crate::{BT, bt::Processing, execution::executor::Executor};

// TODO: Not how Factory pattern works, but good enough for one executor
pub struct ExecutorFactory {}

impl ExecutorFactory {
    pub(crate) fn create(&self, tree: &BT<Processing>) -> Executor {
        Executor::new(tree)
    }
}
