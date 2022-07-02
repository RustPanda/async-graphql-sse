use futures_util::stream::StreamExt;
use reqwest::Client;
use reqwest_eventsource::{Event, RequestBuilderExt};

#[tokio::main]
async fn main() {
    let client = Client::builder().http2_prior_knowledge().build().unwrap();

    let mut stream = client
        .get("http://localhost:8080/?subscription={interval}")
        .eventsource()
        .unwrap();

    while let Some(Ok(event)) = stream.next().await {
        match event {
            Event::Open => {}
            Event::Message(event) => {
                println!("received: {}: {}", event.event, event.data)
            }
        }
    }
}
