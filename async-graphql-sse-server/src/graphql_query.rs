use std::collections::HashMap;

use async_graphql::Request;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum QueryOrSub {
    Subscription(String),
    Query(String),
}

#[derive(Deserialize, Debug)]
pub struct GraphQLQuery {
    #[serde(flatten)]
    query: QueryOrSub,
    #[serde(rename = "operationName")]
    operation_name: Option<String>,
    variables: Option<serde_json::Value>,
    extensions: Option<HashMap<String, async_graphql::Value>>,
}

impl GraphQLQuery {
    pub fn in_query(&self) -> bool {
        matches!(self.query, QueryOrSub::Query(_))
    }
}

impl From<GraphQLQuery> for Request {
    fn from(
        GraphQLQuery {
            query,
            operation_name,
            variables,
            extensions,
        }: GraphQLQuery,
    ) -> Self {
        let mut request = match query {
            QueryOrSub::Subscription(query) => Request::new(format!("subscription {query}")),
            QueryOrSub::Query(query) => Request::new(format!("query {query}")),
        };

        if let Some(operation_name) = operation_name {
            request = request.operation_name(operation_name);
        }

        if let Some(variables) = variables {
            let variables = async_graphql::Variables::from_json(variables);
            request = request.variables(variables);
        }

        if let Some(extensions) = extensions {
            request.extensions = extensions
        }

        request
    }
}
