pub mod streaming {
    tonic::include_proto!("grpc.examples.streaming");
}

use futures::Stream;
use std::{error::Error, io::ErrorKind, net::ToSocketAddrs, pin::Pin};
use streaming::{EchoRequest, EchoResponse, Person};
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use tonic::{transport::Server, Request, Response, Status};

#[derive(Debug)]
pub struct StreamingServer {}

type EchoResult<T> = Result<Response<T>, Status>;
type ResponseStream = Pin<Box<dyn Stream<Item = Result<EchoResponse, Status>> + Send>>;

fn match_for_io_error(err_status: &Status) -> Option<&std::io::Error> {
    let mut err: &(dyn Error + 'static) = err_status;

    loop {
        if let Some(io_err) = err.downcast_ref::<std::io::Error>() {
            return Some(io_err);
        }

        // h2::Error do not expose std::io::Error with `source()`
        // https://github.com/hyperium/h2/pull/462
        if let Some(h2_err) = err.downcast_ref::<h2::Error>() {
            if let Some(io_err) = h2_err.get_io() {
                return Some(io_err);
            }
        }

        err = match err.source() {
            Some(err) => err,
            None => return None,
        };
    }
}

#[tonic::async_trait]
impl streaming::streaming_server::Streaming for StreamingServer {
    type EchoStream = ResponseStream;
    async fn echo(
        &self,
        req: Request<tonic::Streaming<EchoRequest>>,
    ) -> EchoResult<Self::EchoStream> {
        println!("Sending response to client....");
        let mut in_stream = req.into_inner();
        let (tx, rx) = mpsc::channel(4);
        tokio::spawn(async move {
            while let Some(result) = in_stream.next().await {
                let mut persons = Vec::new();
                persons.push(Person {
                    name: "John".to_string(),
                    age: 12,
                });
                persons.push(Person {
                    name: "Bob".to_string(),
                    age: 23,
                });
                persons.push(Person {
                    name: "Alex".to_string(),
                    age: 33,
                });
                persons.push(Person {
                    name: "Miranda".to_string(),
                    age: 56,
                });
                match result {
                    Ok(_v) => tx
                        .send(Ok(EchoResponse { person: persons }))
                        .await
                        .expect("working rx"),
                    Err(err) => {
                        if let Some(io_err) = match_for_io_error(&err) {
                            if io_err.kind() == ErrorKind::BrokenPipe {
                                // here you can handle special case when client
                                // disconnected in unexpected way
                                eprintln!("\tclient disconnected: broken pipe");
                                break;
                            }
                        }
                        match tx.send(Err(err)).await {
                            Ok(_) => (),
                            Err(_err) => break, // response was droped
                        }
                    }
                }
            }
        });
        // echo just write the same data that was received
        let out_stream = ReceiverStream::new(rx);

        Ok(Response::new(Box::pin(out_stream) as Self::EchoStream))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = StreamingServer {};
    println!("Starting gRPC server.....");
    Server::builder()
        .add_service(streaming::streaming_server::StreamingServer::new(server))
        .serve("[::1]:50051".to_socket_addrs().unwrap().next().unwrap())
        .await
        .unwrap();

    Ok(())
}
