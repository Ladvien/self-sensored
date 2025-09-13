use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHasher};

fn main() {
    // The actual API key token to hash
    let api_key = "hea_981e5a34ffd84382ba44f6e97e804a39";

    // Generate salt
    let salt = SaltString::generate(&mut OsRng);

    // Create Argon2 hasher with the same parameters as the application
    let argon2 = Argon2::default();

    // Hash the API key
    let hash = argon2.hash_password(api_key.as_bytes(), &salt).unwrap();

    println!("API Key: {}", api_key);
    println!("Hash: {}", hash.to_string());
    println!("\nSQL Update Statement:");
    println!("UPDATE api_keys SET key_hash = '{}' WHERE id = '22222222-2222-2222-2222-222222222222'::uuid;", hash.to_string());
}
