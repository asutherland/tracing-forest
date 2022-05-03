//! This test simulates processing many clients on multithreaded task, where
//! each client has many `await` points.
//!
//! It is intended to maximize the amount of concurrent operations to demonstrate
//! that `tracing-forest` does, in fact, keep each one coherent.
use tracing_forest::util::*;

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

#[tokio::test(flavor = "multi_thread")]
async fn test_n_tasks_blocking() -> Result<()> {
    let num_clients = 10;

    let logs = tracing_forest::capture()
        .set_global(true)
        .build()
        .on(async {
            let mut clients = Vec::with_capacity(10);
            for client in 0..num_clients {
                let handle = tokio::task::spawn_blocking(move || {
                    info_span!("connection").in_scope(|| {
                        info!(%client, "new connection");

                        info_span!("request").in_scope(|| {
                            info!(%client, "sent request");
                            info!(%client, "received response");
                        });

                        info_span!("response").in_scope(|| {
                            info!(%client, "sending response");
                            info!(%client, "response sent");
                        });
                    })
                });

                clients.push(handle);
            }

            for client in clients {
                client.await.unwrap();
            }
        })
        .await;

    assert!(logs.len() == num_clients);

    for tree in logs {
        let connection = tree.span()?;
        assert!(connection.name() == "connection");
        assert!(connection.nodes().len() == 3);

        let client_connect = connection.nodes()[0].event()?;
        assert!(client_connect.message() == Some("new connection"));
        assert!(client_connect.fields().len() == 1);

        let field = &client_connect.fields()[0];
        assert!(field.key() == "client");
        let client_id = field.value();

        for (child, action) in connection.nodes()[1..].iter().zip(["request", "response"]) {
            let span = child.span()?;
            assert!(span.name() == action);
            assert!(span.nodes().len() == 2);

            for child in span.nodes() {
                let event = child.event()?;
                assert!(event.fields().len() == 1);

                let field = &event.fields()[0];
                assert!(field.key() == "client");
                assert!(field.value() == client_id);
            }
        }
    }

    Ok(())
}
