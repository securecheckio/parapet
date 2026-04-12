# SecureCheck Auth API

User management API for SecureCheck Community RPC.

## Features

- User signup with email/password
- API key generation and rotation
- Usage tracking
- Rate limiting per tier

## Endpoints

### POST /signup
Register a new user and get an API key.

```bash
curl -X POST http://localhost:3001/signup \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "secure_password"
  }'
```

Response:
```json
{
  "user_id": "123e4567-e89b-12d3-a456-426614174000",
  "api_key": "sk_abcdef123456..."
}
```

### GET /usage
Get usage statistics.

```bash
curl -X GET http://localhost:3001/usage \
  -H "Content-Type: application/json" \
  -d '{ "api_key": "sk_..." }'
```

Response:
```json
{
  "total_requests": 1234,
  "blocked_requests": 5,
  "current_month_requests": 456
}
```

### POST /api-keys
Create an additional API key.

```bash
curl -X POST http://localhost:3001/api-keys \
  -H "Content-Type: application/json" \
  -d '{
    "api_key": "sk_existing...",
    "name": "Production Key"
  }'
```

## Run

```bash
cargo run --release
```

## Database

Uses the same PostgreSQL database as the reverse-proxy.
See `../reverse-proxy/schema.sql` for the schema.
