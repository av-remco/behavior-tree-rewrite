#[cfg(test)]
#[allow(unused_imports)]
mod tests {
    use std::{collections::HashMap, prelude::rust_2024::Future, time::Duration};

    use actify::Handle;
    use anyhow::{Ok, Result};
    use tokio::time::sleep;

    use crate::{BT, Condition, Failure, Success, Wait, bt::Ready, logging::load_logger, nodes::action::{Executor, mocking::MockAction}, nodes_bin::{node::Node, node_status::Status}};

    struct TestExecutor {}

    impl Executor for TestExecutor {
        fn get_name(&self) -> String {
            "test_executor".to_string()
        }
    
        fn execute(&mut self) -> impl Future<Output = Result<bool>> + Send {
            async move {
                sleep(Duration::from_millis(1000)).await;
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
}