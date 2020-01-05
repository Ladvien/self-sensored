-- Create the database
CREATE DATABASE self_sensored;

USE self_sensored;

SHOW tables;

-- ############ DROP FOR REBUILD ##########
DROP TABLE users;
DROP TABLE activities;

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

-- ############ ACTIVITIES #####################

-- Create users table
CREATE TABLE IF NOT EXISTS activities (
    id INT AUTO_INCREMENT PRIMARY KEY,
    user_id INT NOT NULL,
    activity_type VARCHAR(255) NOT NULL,
    date DATETIME,
    quantity INT,
    quantity_type VARCHAR(255) NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
     FOREIGN KEY (user_id)
      REFERENCES users(id)
)  ENGINE=INNODB; 

-- Show what we created
DESCRIBE activities;

-- Check exist records.
SELECT * FROM activities;

