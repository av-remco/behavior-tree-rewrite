use crate::{BT, bt::Ready, execution::{dynamic_engine::dynamic_engine::DynamicEngine, static_engine::static_engine::StaticEngine}};

pub(crate) trait Engine {
    async fn run(&mut self) -> bool;
}

pub enum Engines {
    // Event-based, creates a map (node, result) -> next node
    Static,
    // Event-based, looks up next node during run time
    Dynamic,
}

// Wrapper for the factory
pub enum EngineDispatch {
    Static(StaticEngine),
    Dynamic(DynamicEngine),
}

impl Engine for EngineDispatch {
    async fn run(&mut self) -> bool {
        match self {
            EngineDispatch::Static(e)  => e.run().await,
            EngineDispatch::Dynamic(e) => e.run().await,
        }
    }
}

pub(crate) struct EngineFactory {
    pub engine: Engines,
}

impl EngineFactory {
    pub(crate) fn set(&mut self, engine: Engines) {
        self.engine = engine
    }

    pub(crate) fn create(&self, tree: &BT<Ready>) -> EngineDispatch {
        match self.engine {
            Engines::Static => EngineDispatch::Static(StaticEngine::new(tree)),
            Engines::Dynamic => EngineDispatch::Dynamic(DynamicEngine::new(tree)),
        }
    }
}
