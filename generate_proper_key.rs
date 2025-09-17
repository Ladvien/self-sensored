use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHasher};
use uuid::Uuid;

fn main() {
    // Generate API key in the correct format
    let key_uuid = Uuid::new_v4();
    let api_key = format!("hea_{}", key_uuid.simple());

    println!("API Key: {}", api_key);

    // Generate salt
    let salt = SaltString::generate(&mut OsRng);

    // Create Argon2 hasher
    let argon2 = Argon2::default();

    // Hash the API key
    let hash = argon2.hash_password(api_key.as_bytes(), &salt).unwrap();

    println!("Hash: {}", hash);
}