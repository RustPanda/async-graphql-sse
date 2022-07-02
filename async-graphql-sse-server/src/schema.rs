use async_graphql::*;

pub type ExampleSchema = Schema<QueryRoot, EmptyMutation, SubscriptionRoot>;
pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn first_name(&self) -> &str {
        "Matvei"
    }
    async fn second_name(&self) -> &str {
        "Golubev"
    }

    async fn age(&self) -> u8 {
        28
    }
}

pub struct SubscriptionRoot;

#[Subscription]
impl SubscriptionRoot {
    async fn interval(
        &self,
        #[graphql(default = 1)] n: i32,
    ) -> impl futures_util::Stream<Item = i32> {
        let mut value = 0;
        async_stream::stream! {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                value += n;
                tracing::info!("send value");
                yield value
            }
        }
    }
}

pub fn build() -> SchemaBuilder<QueryRoot, EmptyMutation, SubscriptionRoot> {
    ExampleSchema::build(QueryRoot, EmptyMutation, SubscriptionRoot)
}
