// document-generation-service/src/main.rs

mod config;
mod error;
mod generators;
mod models;
mod pubsub;
mod renderers;

use crate::config::Config;
use crate::pubsub::{MessageHandler, Publisher};
use google_cloud_pubsub::client::{Client, ClientConfig};
use google_cloud_pubsub::subscription::Subscription;
use std::sync::Arc;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Print to stderr BEFORE logging initialization to catch early failures
    eprintln!("Starting document-generation-service...");

    // Load configuration
    let config = match Config::load() {
        Ok(cfg) => {
            eprintln!("Configuration loaded successfully");
            cfg
        }
        Err(e) => {
            eprintln!("FATAL: Failed to load configuration: {}", e);
            std::process::exit(1);
        }
    };

    // Initialize logging
    eprintln!("Initializing logging...");
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| config.service.log_level.clone().into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    eprintln!("Logging initialized");

    info!(
        service = %config.service.name,
        version = env!("CARGO_PKG_VERSION"),
        "Starting Document Generation Service"
    );

    // Initialize Pub/Sub client
    eprintln!("Initializing Pub/Sub client...");
    let client_config = match ClientConfig::default().with_auth().await {
        Ok(cfg) => {
            eprintln!("Pub/Sub client config created successfully");
            cfg
        }
        Err(e) => {
            error!("Failed to create Pub/Sub client config: {}", e);
            eprintln!("FATAL: Failed to create Pub/Sub client config: {}", e);
            return Err(e.into());
        }
    };

    let client = match Client::new(client_config).await {
        Ok(c) => {
            eprintln!("Pub/Sub client created successfully");
            c
        }
        Err(e) => {
            error!("Failed to create Pub/Sub client: {}", e);
            eprintln!("FATAL: Failed to create Pub/Sub client: {}", e);
            return Err(e.into());
        }
    };

    info!(
        project_id = %config.pubsub.project_id,
        subscription = %config.pubsub.request_subscription,
        "Initializing Pub/Sub client"
    );

    // Get subscription
    let subscription = client.subscription(&config.pubsub.request_subscription);

    // Initialize publisher for responses
    let publisher = Publisher::new(
        &config.pubsub.project_id,
        &config.pubsub.response_topic,
    )
    .await?;

    // Initialize message handler
    let handler = Arc::new(MessageHandler::new());
    let publisher = Arc::new(publisher);

    info!("Starting message processing loop");

    // Start processing messages
    process_messages(
        subscription,
        handler,
        publisher,
        config.pubsub.max_concurrent_messages,
    )
    .await;

    Ok(())
}

async fn process_messages(
    subscription: Subscription,
    handler: Arc<MessageHandler>,
    publisher: Arc<Publisher>,
    _max_concurrent: usize,
) {
    use tokio_util::sync::CancellationToken;
    use tokio::signal;

    let cancel = CancellationToken::new();
    let cancel_for_signal = cancel.clone();

    // Spawn signal handler
    tokio::spawn(async move {
        match signal::ctrl_c().await {
            Ok(()) => {
                info!("Received shutdown signal, cancelling message processing");
                cancel_for_signal.cancel();
            }
            Err(err) => {
                error!("Unable to listen for shutdown signal: {}", err);
            }
        }
    });

    info!("Starting message processing loop");

    loop {
        if cancel.is_cancelled() {
            info!("Message processing cancelled, exiting loop");
            break;
        }

        let handler_clone = handler.clone();
        let publisher_clone = publisher.clone();

        info!("Calling subscription.receive() to wait for messages...");

        let result = subscription
            .receive(
                move |message, cancel| {
                    let handler = handler_clone.clone();
                    let publisher = publisher_clone.clone();

                    async move {
                        if cancel.is_cancelled() {
                            return;
                        }

                        info!(
                            message_id = %message.message.message_id,
                            "Processing message"
                        );

                        // Process the message
                        let response = handler.handle_message(&message.message.data).await;

                        // Publish response
                        publisher.publish_response(&response).await;

                        // Acknowledge the message
                        if let Err(e) = message.ack().await {
                            error!(
                                message_id = %message.message.message_id,
                                error = %e,
                                "Failed to acknowledge message"
                            );
                        } else {
                            info!(
                                message_id = %message.message.message_id,
                                "Message processed and acknowledged"
                            );
                        }
                    }
                },
                cancel.clone(),
                None,
            )
            .await;

        match result {
            Ok(()) => {
                info!("subscription.receive() completed successfully, continuing loop");
            }
            Err(e) => {
                error!("Error receiving messages: {}", e);
                error!("Retrying in 5 seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        }
    }

    info!("Message processing loop exited");
}
