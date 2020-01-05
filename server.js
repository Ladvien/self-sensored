const express = require('express');
const bodyParser = require('body-parser');
const axios = require('axios');
const {connection} = require('./database/connection');
var timeout = require('connect-timeout')

// Server setup.
var app = express();
const port = 3000;


// Add request parameters.
app.use((req, res, next) => {
    res.setHeader('Access-Control-Allow-Origin', '*');
    res.setHeader('Access-Control-Allow-Headers', 
                  'Origin, X-Requested-With, Content-Type, Accept'); 
    res.setHeader('Access-Control-Allow-Methods', 'GET, POST, PUT, PATCH, DELETE, OPTIONS');    
    next();
});

// Add the middleware.
app.use(bodyParser.json())

// Create user
app.post('/user', (req, res) => {
    if (!req.body) { return { 'message': 'No request provided.' }};
    try {
        // Attempt to create a user.
        connection.query(`INSERT INTO users SET ?;`, req.body, function (error, results, fields) {
          if (error) {
            // If there's an error in the insert, notify the caller.
            res.send({"error": "User creation was unsuccessful.", "message": error });
          }
          if (results) {
            // Look up the newly created record.
            connection.query('SELECT * FROM users WHERE id = ?', results['insertId'], function(error, results, fields) {
                if (error) {
                    // If there's an error retrieving the new record, notify caller.
                    res.send({"error": "User creation was unsuccessful.", "message": error});
                }
                // Return the new record to the user for confirmation of its creation.
                res.send( {"success": results} );
            });
          }
        });
    } catch (err) {
        res.send({'error': 'Error with request shape.', err})
    }
});

// Get user.
app.get('/user', (req, res) => {
    if (!req.body) { return { 'message': 'No request provided.' }};
    try {
        console.log(req.body);
        // Look up the newly created record.
        connection.query('SELECT * FROM users WHERE id = ?', req.body['user_id'], function(error, results, fields) {
            if (error) {
                // If there's an error retrieving the new record, notify caller.
                res.send({"error": "User creation was unsuccessful.", "message": error});
            }
            // Check if there was a user with that id.
            console.log(results)
            if (results.length > 0) {
                // Return the new record to the user for confirmation of its creation.
                res.send( {"user": results[0]} );
            } else {
                res.send({'error': `no user found with id ${req.body['user_id']}`})
            }

        });
    } catch (err) {
        res.send({'error': 'Error with request shape.', err})
    }
});


/*
    This route will be used for storing HealthKit data.
*/
app.post('/activities/:type', (req, res) => {
    if (!req.body) { return { 'message': 'No request provided.' }};
    try {
        let tableName = 'activities'
        let activityType = Object.keys(req.body)[0];

        let entry = req.body[activityType];
        console.log(entry)
        delete entry['device']
        // Create restingHeartRate entry.
        connection.query(`INSERT INTO ${tableName} SET ?;`, entry, function (error, results, fields) {
            if (error) {
                // If there's an error in the insert, notify the caller.
                console.log(error)
                res.send({"error": "Activities creation was unsuccessful.", "message": error });
            }
            if (results) {
            // Look up the newly created record.
            connection.query(`SELECT * FROM ${tableName} WHERE id = ?`, results['insertId'], function(error, results, fields) {
                if (error) {
                    // If there's an error retrieving the new record, notify caller.
                    res.send({"error": "User creation was unsuccessful.", "message": error});
                }
                // Return the new record to the user for confirmation of its creation.
                res.send( {"success": results} );
            });
            }
        });
    } catch (err) {
        res.send({'error': 'Error with request shape.', 'error_message': err})
    }
});

app.listen(port, () => {
    console.log(`Started on port ${port}`);
});


   
// connection.end();