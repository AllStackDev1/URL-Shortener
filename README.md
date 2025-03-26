# URL Shortener

A simple and efficient URL shortener service built in Rust.

## Features

- Shortens long URLs into custom or auto-generated codes
- Tracks access count, expiration, and usage metadata
- Supports JSON API requests and query filtering
- Built with Actix Web and SQLx

## Getting Started

1. **Clone the repo**
   ```bash
   git clone https://github.com/your-username/url-shortener.git
   cd url-shortener
   ```

2. **Set up your environment**
   - Copy `.env.example` to `.env` and configure your database URL.

3. **Run migrations**
   ```bash
   sqlx migrate run
   ```

4. **Start the server**
   ```bash
   cargo run
   ```

## API Overview

- `POST /shorten` - Create a new shortened URL
- `GET /{code}` - Redirect to the original URL
- `GET /urls` - List and filter shortened URLs

## Tech Stack

- Rust
- Actix Web
- SQLx (PostgreSQL)
- Serde + Validator

## License

MIT
