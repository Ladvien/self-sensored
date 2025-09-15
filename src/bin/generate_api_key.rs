use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHasher};

fn main() {
    let api_key = "test_auto_export_key_2024";

    // Generate salt
    let salt = SaltString::generate(&mut OsRng);

    // Create Argon2 hasher
    let argon2 = Argon2::default();

    // Hash the API key
    let hash = argon2.hash_password(api_key.as_bytes(), &salt).unwrap();

    println!("API Key: {api_key}");
    println!("Hash: {hash}");
}
