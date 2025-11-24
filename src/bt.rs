use anyhow::Error;

use crate::{execution::executor::{Executor, ExecutorFactory}, nodes_bin::{node_handle::NodeHandle, node_index::NodeIndex}};

pub(crate) const CHANNEL_SIZE: usize = 20;

pub struct BT<T: BuildState> {
    name: String,
    pub(crate) root: NodeHandle,
    pub(crate) node_index: NodeIndex,
    executor_factory: ExecutorFactory,

    state_data: T,
}

impl<T: BuildState> BT<T> {
    #[cfg(test)]
    pub fn test_into_state(self) -> BT<Processing> {
        BT::<Processing> {
            name: self.name,
            root: self.root,
            node_index: self.node_index,
            executor_factory: self.executor_factory,
            state_data: Processing {},
        }
    }
}

impl BT<Init> {
    pub fn new<S: Into<String>>(root_node: NodeHandle, name: S) -> Self {
        BT::_new(root_node, name.into())
    }

    fn _new(mut root: NodeHandle, name: String) -> Self {
        let handles = root.take_handles();
        Self {
            name,
            root,
            node_index: NodeIndex::new(handles),
            executor_factory: ExecutorFactory {},
            state_data: Init {},
        }
    }

    pub fn set_executor(&mut self, executor_factory: ExecutorFactory) {
        self.executor_factory = executor_factory;
    }

    pub fn build(self) -> BT<Ready> {
        let bt= 
            BT::<Processing> {
                name: self.name,
                root: self.root,
                node_index: self.node_index,
                executor_factory: self.executor_factory,
                state_data: Processing {},
            }; // State transition quite ugly, litter the code

        let exec = bt.executor_factory.create(&bt);

        BT::<Ready> {
            name: bt.name,
            root: bt.root,
            node_index: bt.node_index,
            executor_factory: bt.executor_factory,
            state_data: Ready { exec },
        }
    }
}

impl BT<Ready> {
    pub async fn execute(self) -> BT<Done> {
        let mut bt= 
            BT::<Executing> {
                name: self.name,
                root: self.root,
                node_index: self.node_index,
                executor_factory: self.executor_factory,
                state_data: Executing { exec: self.state_data.exec },
            };

        let res = bt.state_data.exec.execute().await;

        BT::<Done> {
            name: bt.name,
            root: bt.root,
            node_index: bt.node_index,
            executor_factory: bt.executor_factory,
            state_data: Done { res },
        }
    }
}

impl BT<Done> {
    pub fn result(&self) -> bool {
        self.state_data.res
    }
}

pub trait BuildState {}
pub struct Init;
pub struct Processing;
pub struct Ready {
    exec: Executor,
}
pub struct Executing {
    exec: Executor,
}
pub struct Done {
    res: bool
}

impl BuildState for Init {}
impl BuildState for Processing {}
impl BuildState for Ready {}
impl BuildState for Executing {}
impl BuildState for Done {}