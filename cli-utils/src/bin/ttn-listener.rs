use std::{process, thread, time::Duration};

use clap::Clap;
use paho_mqtt as mqtt;

/// Attempt to reconnect to the broker. It can be called after connection is lost. In this example,
/// we try to reconnect several times, with a few second pause between each attempt. A real system
/// might keep trying indefinitely, with a backoff, or something like that.
fn try_reconnect(client: &mqtt::Client) -> bool {
    println!("Connection lost. Waiting to retry connection");
    for _ in 0..12 {
        thread::sleep(Duration::from_millis(5000));
        if client.reconnect().is_ok() {
            println!("Successfully reconnected");
            return true;
        }
    }
    println!("Unable to reconnect after several attempts.");
    false
}

#[derive(Clap)]
struct Opts {
    #[clap(short, long, default_value = "tcp://eu1.cloud.thethings.network:1883")]
    host: String,
    #[clap(short, long, default_value = "gfroerli-test@ttn")]
    user_name: String,
    #[clap(short, long)]
    password: String,
}

fn main() {
    env_logger::init();

    let opts: Opts = Opts::parse();

    let host = opts.host;

    // Create the client
    let create_opts = mqtt::CreateOptionsBuilder::new()
        .server_uri(host)
        .finalize();

    let mut client = mqtt::Client::new(create_opts).unwrap_or_else(|e| {
        println!("Error creating the client: {:?}", e);
        process::exit(1);
    });

    // Initialize the consumer before connecting
    let rx = client.start_consuming();

    let conn_opts = mqtt::ConnectOptionsBuilder::new()
        .keep_alive_interval(Duration::from_secs(20))
        .clean_session(false)
        .user_name(opts.user_name)
        .password(opts.password)
        .finalize();

    let subscriptions = ["v3/+/devices/+/activations", "v3/+/devices/+/up"];
    let qos = [1, 1];

    // Make the connection to the broker
    println!("Connecting to the MQTT broker...");
    match client.connect(conn_opts) {
        Ok(rsp) => {
            if let Some(conn_rsp) = rsp.connect_response() {
                println!(
                    "Connected to: '{}' with MQTT version {}",
                    conn_rsp.server_uri, conn_rsp.mqtt_version
                );
                if !conn_rsp.session_present {
                    // Register subscriptions on the server
                    println!("Subscribing to topics, with requested QoS: {:?}...", qos);

                    match client.subscribe_many(&subscriptions, &qos) {
                        Ok(qosv) => println!("QoS granted: {:?}", qosv),
                        Err(e) => {
                            println!("Error subscribing to topics: {:?}", e);
                            client.disconnect(None).unwrap();
                            process::exit(1);
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("Error connecting to the broker: {:?}", e);
            process::exit(1);
        }
    }

    // Just loop on incoming messages.
    // If we get a None message, check if we got disconnected,
    // and then try a reconnect.
    println!("Waiting for messages...");
    for msg in rx.iter() {
        if let Some(msg) = msg {
            println!("{}", msg);
        } else if client.is_connected() || !try_reconnect(&client) {
            break;
        }
    }

    // If we're still connected, then disconnect now,
    // otherwise we're already disconnected.
    if client.is_connected() {
        println!("Disconnecting");
        client.unsubscribe_many(&subscriptions).unwrap();
        client.disconnect(None).unwrap();
    }
    println!("Exiting");
}
