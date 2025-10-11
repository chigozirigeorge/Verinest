# Verinest

Verinest is a Rust backend application built with [Axum](https://github.com/tokio-rs/axum), featuring user authentication, email verification, and PostgreSQL integration. This guide will help you set up Verinest from the [davtech-ge/verinest](https://github.com/davtech-ge/verinest.git) repository.

## 🚀 Getting Started

### Prerequisites
- [Rust](https://www.rust-lang.org/)
- [PostgreSQL](https://www.postgresql.org/)
- [SQLx-CLI](https://crates.io/crates/sqlx-cli)
- [Postman](https://www.postman.com/) (for API testing)

### Installation Steps

1. **Clone the repository:**
   ```bash
   git clone https://github.com/davtech-ge/verinest.git
   cd verinest
   ```

2. **Install dependencies:**
   ```bash
   cargo install --path .
   ```

3. **Set up PostgreSQL:**
   - Create a new database in PostgreSQL.
   - Update the `.env` file with your database URL:
     ```env
     DATABASE_URL=postgres://user:password@localhost/dbname
     ```

4. **Run migrations:**
   ```bash
   sqlx migrate run
   ```

5. **Start the server:**
   ```bash
   cargo run
   ```
   The server will be running on `http://127.0.0.1:8000`.

## 📬 Email Verification Setup
To enable email verification, configure your email provider in the `.env` file:
```env
SMTP_SERVER=smtp.your-email-provider.com
SMTP_PORT=587
SMTP_USER=your-email@example.com
SMTP_PASSWORD=your-email-password
```

## 🧪 API Testing with Postman
- Use the provided Postman collection to test endpoints.
- Main endpoints:
  - `POST /api/auth/register` — Register a new user
  - `POST /api/auth/login` — Login
  - `GET /api/auth/forgot-password` — Request password reset
  - `POST /api/auth/reset-password` — Reset password
  - `GET /api/auth/verify` — Verify email
  - `GET /api/users/me` — Get current user profile (JWT required)

## ⚙️ Configuration
Create a `.env` file with the following variables:
```env
DATABASE_URL=postgresql://postgres:password@localhost:5432/axum_auth
JWT_SECRET_KEY=your_jwt_secret
JWT_MAXAGE=60
SMTP_SERVER=smtp.your-email-provider.com
SMTP_PORT=587
SMTP_USERNAME=your_email@example.com
SMTP_PASSWORD=your_email_password
SMTP_FROM_ADDRESS=no-reply@yourdomain.com
```

## 🎯 Future Enhancements
- Role-based access control (RBAC)
- Rate limiting and input validation
- User profiles and more features

## 📄 License
MIT License. See the [LICENSE](LICENSE) file for details.

## ✨ Acknowledgements
- [Axum](https://github.com/tokio-rs/axum)
- [PostgreSQL](https://www.postgresql.org/)
- [SQLx](https://github.com/launchbadge/sqlx)

Thank you for your support!
