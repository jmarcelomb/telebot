# Telebot

**Telebot** is a Rust-based Telegram bot framework designed to manage and interact with services. The bot enables users to communicate with various services and receive updates or notifications via Telegram. Currently, the project includes a **Milk Price Service** that scrapes websites to monitor milk prices and notifies users of price changes.

This project uses the [teloxide](https://github.com/teloxide/teloxide) library to handle the Telegram interface, simplifying bot interactions and updates.

## Features

- **Telegram Bot Integration**: Seamless interaction with Telegram users powered by the `teloxide` library.
- **Milk Price Monitoring**: Scrapes milk prices from a predefined website and notifies users of any changes.
- **SQLite Database Support**: Stores data locally in an SQLite database.
- **Containerized Deployment**: Includes a `Dockerfile` and scripts for building and running the bot in a container.
- **Environment Variable Management**: Easily configurable through an environment variable script (`env_vars.sh`).

## Project Structure

```bash
telebot/
├── Cargo.lock            # Rust dependency lockfile
├── Cargo.toml            # Rust project manifest
├── db/                   # Database files and initialization scripts
│   ├── creation.sql      # SQL script for initializing the database
│   └── sqlite.db         # SQLite database file
├── Dockerfile            # Container setup for the bot
├── env_vars.sh           # Script to define environment variables
├── LICENSE               # License file
├── local-db              # Directory for local database persistence
├── README.md             # Project documentation
├── scripts/              # Utility scripts
│   ├── build_image.sh    # Script to build the Docker image
│   ├── get_version.sh    # Script to retrieve the bot version
│   ├── push_image.sh     # Script to push Docker image to a registry
│   └── run_telebot.sh    # Script to run the bot
└── src/                  # Source code for the bot
├── chat.rs           # Telegram chat logic
├── constants.rs      # Constants used throughout the bot
├── db.rs             # Database interaction logic
├── main.rs           # Main entry point of the application
├── milk_price.rs     # Milk price scraping and notifications
└── services.rs       # Service management logic
```

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/) (version 1.70 or later recommended)
- [Docker](https://www.docker.com/) (optional for containerized deployment)
- Telegram Bot API token
- SQLite for database support

### Installation

1. Clone the repository:

```bash
git clone https://github.com/yourusername/telebot.git
cd telebot
```

2.	Set up the environment variables:

```bash
source env_vars.sh
```
4.	Run the bot:

```bash
cargo run
```

Docker Deployment

1.	Build the Docker image:

```bash
./scripts/build_image.sh
```

2.	Run the bot in a container:

```bash
./scripts/run_telebot.sh
```

3.	(Optional) Push the Docker image to a registry:

```bash
./scripts/push_image.sh
```

# Current Services

## Milk Price Service

- Description: Monitors the price of milk on a specific website and notifies users of changes.
- Implementation: Located in src/milk_price.rs.
- Command: /milk_price

## Extending the Bot

To add a new service:

1.	Create a new module in the src directory (e.g., new_service.rs).
2.	Implement the service logic (e.g., web scraping, API integration, etc.).
3.	Register the service in src/services.rs.
4.	Update the bot commands in src/chat.rs.

## Example: Adding a New Service

1.	Create src/weather.rs for weather monitoring:

```rust
pub fn get_weather() -> String {
    "Current weather: Sunny, 25°C".to_string()
}
```

2.	Register it in src/services.rs:

```rust
pub mod weather;
```

3.	Update the chat logic in src/chat.rs:

```rust
if command == "/weather" {
    let response = services::weather::get_weather();
    bot.send_message(chat_id, response).await?;
}
```

# Contributing

Contributions are welcome! Feel free to submit a pull request for new features, bug fixes, or documentation updates.

# License

This project is licensed under Apache-2.0.


