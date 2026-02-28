# Configuration Guide

## Development Environment

For local development, use this configuration:

```json
{
  "environment": "development",
  "debug": true,
  "api": {
    "baseUrl": "http://localhost:3000",
    "timeout": 5000
  }
}
```

## Production Environment

For production deployment, use:

```json
{
  "environment": "production",
  "debug": false,
  "api": {
    "baseUrl": "https://api.example.com",
    "timeout": 3000
  }
}
```

## Testing

When running tests, set `timeout` to 3000ms for faster feedback.
