-- Create the database
CREATE DATABASE self_sensored;

USE self_sensored;

SHOW tables;

-- ############ USERS #####################

-- Create users table
CREATE TABLE IF NOT EXISTS users (
    id INT AUTO_INCREMENT PRIMARY KEY,
    first_name VARCHAR(255) NOT NULL,
    last_name VARCHAR(255) NOT NULL,
    status TINYINT NOT NULL,
    description TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
)  ENGINE=INNODB; 

-- Show what we created
DESCRIBE users;

-- Check exist records.
SELECT * FROM users;

-- ############ STEPS #####################
