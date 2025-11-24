use crate::{BT, bt::Ready, execution::{dynamic_engine::DynamicEngine, flat_map_engine::FlatMapEngine}};

// TODO: Not how Factory pattern works, but good enough for one executor

pub(crate) trait Engine {
    async fn run(&mut self) -> bool;
}

pub enum Engines {
    Default,
    Dynamic,
}

pub(crate) struct EngineFactory {
    pub engine: Engines,
}

impl EngineFactory {
    pub(crate) fn set(&mut self, engine: Engines) {
        self.engine = engine
    }

    pub(crate) fn create(&self, tree: &BT<Ready>) -> impl Engine {
        match self.engine {
            Engines::Default => FlatMapEngine::new(tree),
            Engines::Dynamic => DynamicEngine::new(tree),
        }
    }
}
