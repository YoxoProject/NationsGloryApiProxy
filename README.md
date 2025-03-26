# NationsGlory API Proxy

## Project Overview

NationsGlory API Proxy is a Rust-based intermediary service designed to act as a proxy between clients and the
NationsGlory API. Its primary purpose is to handle rate limiting and caching, ensuring that clients do not encounter
rate limit errors and can benefit from cached responses for improved performance.

## How to Use

### Obtaining an API Key

First, obtain an NationsGlory API key from [the official NationsGlory API](https://publicapi.nationsglory.fr/).

### Making Requests

Once you have your API key, you can make requests to the proxy server by including your API key(s) in the
`Authorization` header.

#### Example

To fetch notations for a specific week and server, you can use the following `curl` command:

```sh
curl -H "Authorization: <your_api_key>" "http://localhost:8000/notations?week=2880&server=red&country=france"
```

You can include *multiple* API keys by separating them with commas. This is useful to have less waiting time between
requests.:

```sh
curl -H "Authorization: <your_api_key1>,<your_api_key2>" "http://localhost:8000/notations?week=2880&server=red&country=france"
```

> [!WARNING]
> Don't forget to replace `<your_api_key>` with your actual NationsGlory API key.

## Installation on Ubuntu

### Prerequisites

Ensure you have the following dependencies installed:

- `redis-server`
- `gcc`
- `libssl-dev`
- `pkg-config`
- `rust` (with `cargo`)

### Steps

#### Clone the Repository

```sh
git clone https://github.com/YoxoProject/NationsGloryApiProxy.git
cd NationsGloryApiProxy
```

#### Install Dependencies

```sh
# Add Redis officiel APT repository
curl -fsSL https://packages.redis.io/gpg | sudo gpg --dearmor -o /usr/share/keyrings/redis-archive-keyring.gpg
echo "deb [signed-by=/usr/share/keyrings/redis-archive-keyring.gpg] https://packages.redis.io/deb $(lsb_release -cs) main" | sudo tee /etc/apt/sources.list.d/redis.list

sudo apt update
sudo apt install redis-server gcc libssl-dev pkg-config
```

#### Install Rust

If you don't have Rust installed, you can install it using `rustup`:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

#### Build the Project

```sh
cargo build --release
```

#### Run the Project

Before running the project, ensure you have a `.env` file in the root directory with the following content:

```
REDIS_URL=redis://127.0.0.1/
```

Then, start the Redis server and run the project:

```sh
sudo service redis-server start
cargo run --release
```

## API Endpoints

Only endpoint that not use personal API key information are implemented in this proxy.
For more information, please check the [NationsGlory public API documentation](https://publicapi.nationsglory.fr/).

### `GET /planning?<server>&<month>&<year>`

Fetches the planning for a given server, month, and year.

#### Parameters:

- `server` (required): The server for which to fetch planning.
- `month` (required): The month for which to fetch planning.
- `year` (required): The year for which to fetch planning.

#### Example:

```sh
curl "http://localhost:8000/planning?server=red&month=06&year=2024"
```

### `GET /playercount`

Fetches the current player count.

#### Example:

```sh
curl "http://localhost:8000/playercount"
```

### `GET /hdv/<server>/list`

Fetches the list of items available in the in-game auction house for a specific server.

#### Parameters:

- `server` (required): The server for which to fetch the auction house data.

#### Example:

```sh
curl "http://localhost:8000/hdv/red/list"
```

### `GET /notations?<week>&<server>&<country>`

Fetches notations for a specific week and server, optionally filtered by country.

#### Parameters:

- `week` (required): The week for which to fetch notations (It's the number of weeks since 01/01/1970).
- `server` (required): The server for which to fetch notations.
- `country` (optional): The country to filter the notations by.

#### Example:

```sh
curl "http://localhost:8000/notations?week=2880&server=red&country=france"
```

### `GET /country/<server>/<country>`

Fetches information about a specific country on a specific server.

#### Parameters:

- `server` (required): The server on which the country is located.
- `country` (required): The country to fetch information about.

#### Example:

```sh
curl "http://localhost:8000/country/red/france"
```

### `GET /country/list/<server>`

Fetches a list of all countries on a specific server.

#### Parameters:

- `server` (required): The server for which to fetch the country list.

#### Example:

```sh
curl "http://localhost:8000/country/list/red"
```

### `GET /user/<username>`

Fetches information about a specific user.

#### Parameters:

- `username` (required): The username of the player.

#### Example:

```sh
curl "http://localhost:8000/user/exampleUser"
```

### `GET /ngisland/list?<page>`

Fetches a paginated list of islands on NGIsland.

#### Parameters:

- `page` (optional): The page number to fetch (for pagination).

#### Example:

```sh
curl "http://localhost:8000/ngisland/list?page=1"
```

## Additional Information

- **Caching**: The proxy uses Redis to cache responses, reducing the number of requests sent to the NationsGlory API and
  improving response times.
- **Rate Limiting**: The proxy manages API key usage to avoid hitting rate limits, ensuring smooth operation even under
  high load.

Feel free to contribute to the project by submitting issues or pull requests on the GitHub repository.