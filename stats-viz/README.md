# RSKY Stats Visualizer

A simple TypeScript tool to visualize usage and visitor statistics from the `feedgen` service.

## Prerequisites

- Node.js and npm
- `feedgen` service running

## Installation

```bash
cd stats-viz
npm install
```

## Configuration

You can configure the connection to `feedgen` using environment variables:

- `FEEDGEN_URL`: The base URL of your feed generator (default: `http://localhost:8000`)
- `API_KEY`: The API key for authentication (default: `test-key`)

## Usage

Run the visualizer:

```bash
npm start
```

This will fetch the latest stats and display them in your terminal with:
- Total visits and unique visitor counts.
- A table of the 10 most recent visitors.
- A bar chart of feed popularity based on recent visitors.
