use crate::{BT, bt::Ready, execution::{flat_dynamic_engine::FlatDynamicEngine, flat_static_engine::FlatMapEngine}};

// TODO: Not how Factory pattern works, but good enough for one executor

pub(crate) trait Engine {
    async fn run(&mut self) -> bool;
}

pub enum Engines {
    Static,
    Dynamic,
}

pub enum EngineDispatch {
    Static(FlatMapEngine),
    Dynamic(FlatDynamicEngine),

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
            Engines::Static => EngineDispatch::Static(FlatMapEngine::new(tree)),
            Engines::Dynamic => EngineDispatch::Dynamic(FlatDynamicEngine::new(tree)),
        }
    }
}
