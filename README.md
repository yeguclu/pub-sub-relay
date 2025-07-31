# My Relay

An **asynchronous, high-throughput Pub/Sub relay** written in **Rust** using **Tokio**.  
This relay simulates core components of a **high-performance message bus** like those used in:

- **Blockchain mempool relays** (e.g., Bloxroute / Flashbots)
- **Low-latency trading systems**
- **Custom Pub/Sub systems**

---

## 🚀 Features

- **Asynchronous networking** with [Tokio](https://tokio.rs/)
- **Producer-Consumer model** with topic-based subscriptions
- **Broadcast to multiple consumers**
- **Automatic cleanup of dead consumers**
- **Metrics**: message throughput and active consumer count every 5 seconds
- **Simple JSON message format**:

```json
{
    "topic": "example",
    "payload": "your-message"
}
```

---

## 📦 Project Structure

```
my_relay/
├── src/
│   ├── main.rs       # Main relay entry point
│   └── lib.rs        # Consumer struct & Message type
├── Cargo.toml
└── .gitignore
```

### Key Components

- **Producer Listener (`127.0.0.1:9000`)**  
    Accepts producers sending JSON messages (newline-delimited).

- **Consumer Listener (`127.0.0.1:9001`)**  
    Accepts consumers which subscribe to a topic (first line is topic name).  
    Each consumer receives all messages for its subscribed topic.

- **Shared State**  
    `HashMap<String, Arc<Mutex<Vec<Consumer>>>>`  
    Maps topic → list of consumers, with per-topic locking for minimal contention.

- **Metrics Task**  
    Prints messages per 5 seconds and number of active consumers.

---

## 🛠️ Installation & Running

### 1. Clone and Build

```bash
git clone https://github.com/<your-username>/my_relay.git
cd my_relay
cargo build --release
```

### 2. Run the Relay

```bash
./target/release/my_relay
```

Expected output:

```
Relay listening on 127.0.0.1:9000
Consumer listener on 127.0.0.1:9001
[METRICS] 0 msgs in last 5s | 0 active consumers
```

### Start a Consumer

```bash
nc 127.0.0.1 9001
my_topic
```

### Send Messages from a Producer

```bash
while true; do echo '{"topic":"my_topic","payload":"hello world"}'; done | nc 127.0.0.1 9000
```

Example output:

```
Producer connected: 127.0.0.1:43512
Consumer connected: 127.0.0.1:60324
[METRICS] 25000000 msgs in last 5s | 1 active consumers
```

---

## ⚡ Performance Notes

- Current version processes millions of messages per second in memory.
- Performance bottlenecks:
    - String cloning per consumer
    - JSON parsing on hot path
