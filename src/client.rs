pub mod streaming {
    tonic::include_proto!("grpc.examples.streaming");
}

use futures::Stream;
use streaming::streaming_client::StreamingClient;
use streaming::EchoRequest;
use tokio_stream::StreamExt;
use tonic::transport::Channel;

fn echo_requests_iter() -> impl Stream<Item = EchoRequest> {
    tokio_stream::iter(1..usize::MAX).map(|i| EchoRequest {
        message: format!("msg {:02}", i),
    })
}

async fn bidirectional_streaming_echo(client: &mut StreamingClient<Channel>, num: usize) {
    let in_stream = echo_requests_iter().take(num);

    let response = client.echo(in_stream).await.unwrap();

    let mut resp_stream = response.into_inner();

    while let Some(received) = resp_stream.next().await {
        let received = received.unwrap();
        println!("\treceived message: `{:?}`", received.person);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = StreamingClient::connect("http://[::1]:50051")
        .await
        .unwrap();

    println!("Streaming echo:");

    // Echo stream that sends 17 requests then graceful end that connection
    println!("\r\nBidirectional stream echo:");
    bidirectional_streaming_echo(&mut client, 1).await;
    Ok(())
}
