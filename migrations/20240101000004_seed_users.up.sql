-- Password: qwerty (hashed with argon2id via Rust argon2 0.5 crate)
INSERT INTO users (name, email, password)
VALUES
    ('Alice', 'alice@example.com', '$argon2id$v=19$m=19456,t=2,p=1$c2VlZHNhbHQxMjM0NTY$ZwNYThJx3DzYoJWf1GX4YIt7Q+vhSw3+FYJGhUkFZ3M'),
    ('Bob', 'bob@example.com', '$argon2id$v=19$m=19456,t=2,p=1$c2VlZHNhbHQxMjM0NTY$ZwNYThJx3DzYoJWf1GX4YIt7Q+vhSw3+FYJGhUkFZ3M');
