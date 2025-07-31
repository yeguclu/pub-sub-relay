use std::{
    error::Error,
    sync::{Arc, atomic::{AtomicUsize, Ordering}},
};
use tokio::{
    net::TcpListener,
    io::{AsyncBufReadExt, AsyncWriteExt, AsyncReadExt, BufReader},
    sync::{mpsc, Mutex},
    time::{interval, Duration},
};
use bytes::BytesMut;

use my_relay::{Consumer, Message};
use std::collections::HashMap;

type ConsumersMap = Arc<Mutex<HashMap<String, Arc<Mutex<Vec<Consumer>>>>>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Relay (producer) socket
    let producer_listener = TcpListener::bind("127.0.0.1:9000").await?;
    println!("Relay listening on 127.0.0.1:9000");

    // Consumer socket
    let consumer_listener = TcpListener::bind("127.0.0.1:9001").await?;
    println!("Consumer listener on 127.0.0.1:9001");

    // Shared hashmap of topic to connected consumers
    let consumers: ConsumersMap = Arc::new(Mutex::new(HashMap::new()));

    let message_count = Arc::new(AtomicUsize::new(0));

    // Spawn task to accept consumers
    {
        let consumers = Arc::clone(&consumers);

        tokio::spawn(async move {
            loop {
                let (socket, addr) = consumer_listener.accept().await.unwrap();
                println!("Consumer connected: {}", addr);

                let (read_half, mut write_half) = socket.into_split();
                let reader = BufReader::new(read_half);
                let mut lines = reader.lines();

                if let Ok(Some(topic)) = lines.next_line().await {
                    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
                    let consumer = Consumer { tx, topic: topic.clone() };

                    // Insert into HashMap by topic
                    {
                        let mut map = consumers.lock().await;
                        let entry = map.entry(topic.clone())
                            .or_insert_with(|| Arc::new(Mutex::new(Vec::new())));
                        entry.lock().await.push(consumer);
                    }

                    // Spawn write task for this consumer
                    tokio::spawn(async move {
                        while let Some(msg) = rx.recv().await {
                            if write_half.write_all(msg.as_bytes()).await.is_err() { break; }
                            if write_half.write_all(b"\n").await.is_err() { break; }
                        }
                        println!("Consumer {} disconnected", addr);
                    });
                }
            }
        });
    }

    // Spawn metrics task
    {
        let consumers = Arc::clone(&consumers);
        let message_count = Arc::clone(&message_count);

        tokio::spawn(async move {
            let mut tick = interval(Duration::from_secs(5));

            loop {
                tick.tick().await;

                let msg_count = message_count.swap(0, Ordering::Relaxed);
                let consumer_count = consumers.lock().await.len();

                println!("[METRICS] {} msgs in last 5s | {} active consumers", msg_count, consumer_count);
            }
        });
    }

    // Main loop: task that accepts producer clients
    loop {
        let (mut socket, addr) = producer_listener.accept().await?;
        println!("Producer connected: {}", addr);

        let consumers = Arc::clone(&consumers);
        let message_count = Arc::clone(&message_count);

        tokio::spawn(async move {
            let mut buf = BytesMut::with_capacity(8192);

            loop {
                let _n = match socket.read_buf(&mut buf).await {
                    Ok(0) => break, // connection closed
                    Ok(n) => n,
                    Err(e) => {
                        println!("Read error from {}: {}", addr, e);
                        break;
                    }
                };

                // Process any full lines in the buffer
                while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                    let line = buf.split_to(pos + 1);
                    let line_str = match std::str::from_utf8(&line) {
                        Ok(s) => s.trim_end(),
                        Err(_) => {
                            println!("Invalid UTF-8 from {}", addr);
                            continue;
                        }
                    };

                    message_count.fetch_add(1, Ordering::Relaxed);

                    let msg: Message = match serde_json::from_str(line_str) {
                        Ok(m) => m,
                        Err(_) => {
                            println!("Invalid JSON from producer: {}", line_str);
                            continue;
                        }
                    };

                    // Lock the map briefly to get the topic Vec
                    let topic_arc_opt = {
                        let map = consumers.lock().await;
                        map.get(&msg.topic).cloned()
                    };

                    // If topic exists, lock its Vec and broadcast
                    if let Some(topic_consumers) = topic_arc_opt {
                        let mut list = topic_consumers.lock().await;
                        list.retain(|c| c.tx.send(msg.payload.clone()).is_ok());
                    }

                    
                }
            }

            println!("Producer {} disconnected", addr);
        });
    }
    
}