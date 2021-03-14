const express = require('express');
const bodyParser = require('body-parser');
const axios = require('axios');
const {connection} = require('./database/connection');
var timeout = require('connect-timeout')

var numRequests = 0;

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
app.use(express.bodyParser.urlencoded());
app.use(express.bodyParser.json());

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

app.listen(port, () => {
    console.log(`Started on port ${port}`);
});

