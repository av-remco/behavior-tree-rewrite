#[cfg(test)]
#[allow(unused_imports)]
mod tests {
    use std::{collections::HashMap, prelude::rust_2024::Future, time::Duration};

    use actify::Handle;
    use anyhow::{Error, Ok, Result};
    use log::debug;
    use tokio::time::sleep;
    use macros::bt_action;

    use crate::{BT, Condition, Failure, Success, Wait, bt::Ready, logging::load_logger, nodes::action::{Executor, mocking::MockAction}, nodes_bin::{node::Node, node_status::Status}};

    struct TestExecutor {}

    impl Executor for TestExecutor {
        fn get_name(&self) -> String {
            "test_executor".to_string()
        }
    
        fn execute(&mut self) -> impl Future<Output = Result<bool>> + Send {
            async move {
                sleep(Duration::from_millis(400)).await;
                Ok(true)
            }
        }
    }

    impl TestExecutor {
        pub fn new() -> TestExecutor {
            Self {}
        }
    }

    #[tokio::test]
    async fn test_simple_action_seq() {
        let result = BT::new()
            .name("test_tree")
            .root(
                BT::seq(vec![
                    BT::action(TestExecutor::new()),
                    BT::action(TestExecutor::new()),
                    BT::action(TestExecutor::new())
                ])
            )
            .run().await
            .result();
        assert_eq!(result, true);
    }

    #[bt_action]
    async fn foo() -> Result<bool, Error> {
        sleep(Duration::from_millis(400)).await;
        Ok(true)
    }

    #[tokio::test]
    async fn test_macro_no_args() {
        let result = BT::new()
            .name("test_tree")
            .root(
                BT::seq(vec![
                    BT::action(FooExecutor::new()),
                    BT::action(FooExecutor::new()),
                    BT::action(FooExecutor::new())
                ])
            )
            .run().await
            .result();
        assert_eq!(result, true);
    }

    #[bt_action]
    async fn bar(arg: u64) -> Result<bool, Error> {
        sleep(Duration::from_millis(arg)).await;
        Ok(true)
    }

    #[tokio::test]
    async fn test_macro_args() {
        let result = BT::new()
            .name("test_tree")
            .root(
                BT::seq(vec![
                    BT::action(BarExecutor::new(400)),
                    BT::action(BarExecutor::new(100)),
                    BT::action(BarExecutor::new(200))
                ])
            )
            .run().await
            .result();
        assert_eq!(result, true);
    }

    #[bt_action]
    async fn sleep_handle(arg: Handle<u64>) -> Result<bool, Error> {
        let duration = arg.get().await;
        debug!("Sleeping {:?} millis", duration);
        sleep(Duration::from_millis(duration)).await;
        arg.set(duration + 100).await;
        Ok(true)
    }

    #[tokio::test]
    async fn test_macro_handle_args() {
        load_logger();
        let handle = Handle::new(100);
        let result = BT::new()
            .name("test_tree")
            .root(
                BT::seq(vec![
                    BT::action(SleepHandleExecutor::new(handle.clone())),
                    BT::action(SleepHandleExecutor::new(handle.clone())),
                    BT::action(SleepHandleExecutor::new(handle.clone())),
                ])
            )
            .run().await
            .result();
        assert_eq!(result, true);
    }
}